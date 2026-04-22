use crate::game::Game;
use crate::messages;
use domain::league::{
    CompactMatchEvent, CompactMatchReport, CompactTeamMatchStats, FixtureStatus, MatchEndReason,
    MatchResult,
};
use domain::player::{
    PlayerIssue, PlayerIssueCategory, PlayerPromiseKind, Position as DomainPosition,
};
use domain::stats::{
    LolRole, MatchOutcome, PlayerMatchStatsRecord, StatsState, TeamMatchStatsRecord, TeamSide,
};
use log::debug;

fn compact_team_stats(stats: &engine::TeamStats) -> CompactTeamMatchStats {
    CompactTeamMatchStats {
        kills: stats.kills,
        deaths: stats.deaths,
        gold_earned: stats.gold_earned,
        damage_dealt: stats.damage_dealt,
        objectives: stats.objectives,
    }
}

fn compact_match_report(report: &engine::MatchReport) -> CompactMatchReport {
    let events = report
        .events
        .iter()
        .filter(|event| {
            matches!(
                &event.event_type,
                engine::EventType::Kill
                    | engine::EventType::ObjectiveTaken
                    | engine::EventType::TowerDestroyed
                    | engine::EventType::InhibitorDestroyed
                    | engine::EventType::NexusTowerDestroyed
                    | engine::EventType::NexusDestroyed
            )
        })
        .map(|event| CompactMatchEvent {
            minute: event.minute,
            event_type: format!("{:?}", event.event_type),
            side: format!("{:?}", event.side),
            player_id: event.player_id.clone(),
            secondary_player_id: event.secondary_player_id.clone(),
        })
        .collect();

    CompactMatchReport {
        game_duration_seconds: report.game_duration_seconds,
        home_stats: compact_team_stats(&report.home_stats),
        away_stats: compact_team_stats(&report.away_stats),
        events,
    }
}

fn report_end_reason(reason: engine::MatchReportEndReason) -> MatchEndReason {
    match reason {
        engine::MatchReportEndReason::NexusDestroyed => MatchEndReason::NexusDestroyed,
        engine::MatchReportEndReason::TimeLimit => MatchEndReason::TimeLimit,
    }
}

fn team_side(team_id: &str, home_team_id: &str) -> TeamSide {
    if team_id == home_team_id {
        TeamSide::Blue
    } else {
        TeamSide::Red
    }
}

fn outcome_for_side(side: TeamSide, report: &engine::MatchReport) -> MatchOutcome {
    let (team_score, opponent_score) = match side {
        TeamSide::Blue => (report.home_wins, report.away_wins),
        TeamSide::Red => (report.away_wins, report.home_wins),
    };
    MatchOutcome::from_scores(team_score, opponent_score)
}

fn domain_role(role: Option<engine::live_match::LolRole>) -> LolRole {
    match role {
        Some(engine::live_match::LolRole::Top) => LolRole::Top,
        Some(engine::live_match::LolRole::Jungle) => LolRole::Jungle,
        Some(engine::live_match::LolRole::Mid) => LolRole::Mid,
        Some(engine::live_match::LolRole::Adc) => LolRole::Adc,
        Some(engine::live_match::LolRole::Support) => LolRole::Support,
        None => LolRole::Unknown,
    }
}

/// Apply a completed match report to the game state: update fixture, standings,
/// player stats, stamina, and generate messages. Public so Tauri can call it
/// after a live match finishes.
pub fn apply_match_report(
    game: &mut Game,
    fixture_index: usize,
    home_team_id: &str,
    away_team_id: &str,
    report: &engine::MatchReport,
) {
    apply_match_report_with_capture(
        game,
        fixture_index,
        home_team_id,
        away_team_id,
        report,
        &mut |_| {},
    );
}

pub fn apply_match_report_with_capture<F>(
    game: &mut Game,
    fixture_index: usize,
    home_team_id: &str,
    away_team_id: &str,
    report: &engine::MatchReport,
    on_capture: &mut F,
) where
    F: FnMut(StatsState),
{
    debug!(
        "[turn] apply_match_report: fixture #{}, result {} - {}",
        fixture_index, report.home_wins, report.away_wins
    );

    let result = MatchResult {
        home_wins: report.home_wins,
        away_wins: report.away_wins,
        ended_by: report_end_reason(report.ended_by),
        game_duration_seconds: report.game_duration_seconds,
        report: Some(compact_match_report(report)),
    };
    let mut counts_for_standings = false;

    // Update fixture status, standings
    if let Some(league) = game.league.as_mut() {
        let fixture = &mut league.fixtures[fixture_index];
        fixture.status = FixtureStatus::Completed;
        counts_for_standings = fixture.counts_for_league_standings();

        if counts_for_standings {
            if let Some(entry) = league
                .standings
                .iter_mut()
                .find(|e| e.team_id == home_team_id)
            {
                entry.record_result(result.home_wins, result.away_wins);
            }
            if let Some(entry) = league
                .standings
                .iter_mut()
                .find(|e| e.team_id == away_team_id)
            {
                entry.record_result(result.away_wins, result.home_wins);
            }
        }

        fixture.result = Some(result);
    }

    on_capture(build_stats_state_capture(
        game,
        fixture_index,
        home_team_id,
        away_team_id,
        report,
    ));

    // Update player season stats from the engine report
    apply_player_stats(game, report, home_team_id, away_team_id);
    resolve_post_match_promises(game, report, home_team_id, away_team_id);

    // Deplete stamina for players who played, scaled by minutes on pitch
    deplete_match_stamina(game, home_team_id, report);
    deplete_match_stamina(game, away_team_id, report);

    // Update morale based on result and individual performance
    update_post_match_morale(game, report, home_team_id, away_team_id);

    // Update team form (last 5 results)
    if counts_for_standings {
        update_team_form(game, report, home_team_id, away_team_id);
    }

    // Update board satisfaction based on match result
    if counts_for_standings
        && let Some(user_team_id) = &game.manager.team_id
        && (*user_team_id == home_team_id || *user_team_id == away_team_id)
    {
        let user_wins = if *user_team_id == home_team_id {
            report.home_wins
        } else {
            report.away_wins
        };
        let opp_wins = if *user_team_id == home_team_id {
            report.away_wins
        } else {
            report.home_wins
        };
        let sat_delta: i8 = if user_wins > opp_wins {
            2
        } else {
            -3
        };
        let new_sat = (game.manager.satisfaction as i16 + sat_delta as i16).clamp(0, 100) as u8;
        game.manager.satisfaction = new_sat;

        let fan_delta: i8 = if user_wins > opp_wins {
            5
        } else {
            -8
        };
        let goal_diff = (user_wins as i8) - (opp_wins as i8);
        let fan_bonus: i8 = if goal_diff >= 3 {
            3
        } else if goal_diff <= -3 {
            -3
        } else {
            0
        };
        let new_fan = (game.manager.fan_approval as i16 + fan_delta as i16 + fan_bonus as i16)
            .clamp(0, 100) as u8;
        game.manager.fan_approval = new_fan;
    }

    // Generate match result message for user's team
    if counts_for_standings
        && let Some(user_team_id) = &game.manager.team_id
        && (*user_team_id == home_team_id || *user_team_id == away_team_id)
    {
        let fixture = &game.league.as_ref().unwrap().fixtures[fixture_index];
        let res = fixture.result.as_ref().unwrap();
        let home_name = game
            .teams
            .iter()
            .find(|t| t.id == home_team_id)
            .map(|t| t.name.as_str())
            .unwrap_or("Home");
        let away_name = game
            .teams
            .iter()
            .find(|t| t.id == away_team_id)
            .map(|t| t.name.as_str())
            .unwrap_or("Away");

        let msg = messages::match_result_message(
            &fixture.id,
            home_name,
            away_name,
            res.home_wins,
            res.away_wins,
            home_team_id,
            away_team_id,
            user_team_id,
            fixture.matchday,
            &game.clock.current_date.to_rfc3339(),
        );
        game.messages.push(msg);
    }

    // Generate match report news article
    if counts_for_standings {
        super::news::generate_match_news(game, fixture_index, home_team_id, away_team_id, report);
    }
}

fn build_stats_state_capture(
    game: &Game,
    fixture_index: usize,
    home_team_id: &str,
    away_team_id: &str,
    report: &engine::MatchReport,
) -> StatsState {
    let Some(league) = game.league.as_ref() else {
        return StatsState::default();
    };
    let Some(fixture) = league.fixtures.get(fixture_index) else {
        return StatsState::default();
    };

    let team_by_player_id: std::collections::HashMap<&str, &str> = game
        .players
        .iter()
        .filter_map(|player| {
            player
                .team_id
                .as_deref()
                .map(|team_id| (player.id.as_str(), team_id))
        })
        .collect();
    let champion_by_player_id: std::collections::HashMap<&str, &str> =
        std::collections::HashMap::new();

    let player_matches = report
        .player_stats
        .iter()
        .filter_map(|(player_id, stats)| {
            let team_id = *team_by_player_id.get(player_id.as_str())?;
            if team_id != home_team_id && team_id != away_team_id {
                return None;
            }

            let opponent_team_id = if team_id == home_team_id {
                away_team_id
            } else {
                home_team_id
            };
            let side = team_side(team_id, home_team_id);

            Some(PlayerMatchStatsRecord {
                fixture_id: fixture.id.clone(),
                season: league.season,
                matchday: fixture.matchday,
                date: fixture.date.clone(),
                competition: fixture.competition.clone(),
                player_id: player_id.clone(),
                team_id: team_id.to_string(),
                opponent_team_id: opponent_team_id.to_string(),
                side,
                result: outcome_for_side(side, report),
                role: domain_role(stats.role),
                champion: champion_by_player_id
                    .get(player_id.as_str())
                    .map(|champion_id| (*champion_id).to_string()),
                duration_seconds: stats.duration_seconds,
                kills: stats.kills,
                deaths: stats.deaths,
                assists: stats.assists,
                creep_score: stats.creep_score,
                gold_earned: stats.gold_earned,
                damage_dealt: stats.damage_dealt,
                vision_score: stats.vision_score,
                wards_placed: stats.wards_placed,
            })
        })
        .collect();

    let team_matches = vec![
        TeamMatchStatsRecord {
            fixture_id: fixture.id.clone(),
            season: league.season,
            matchday: fixture.matchday,
            date: fixture.date.clone(),
            competition: fixture.competition.clone(),
            team_id: home_team_id.to_string(),
            opponent_team_id: away_team_id.to_string(),
            side: TeamSide::Blue,
            result: outcome_for_side(TeamSide::Blue, report),
            duration_seconds: report.game_duration_seconds,
            kills: report.home_stats.kills,
            deaths: report.home_stats.deaths,
            gold_earned: report.home_stats.gold_earned,
            damage_dealt: report.home_stats.damage_dealt,
            objectives: report.home_stats.objectives,
        },
        TeamMatchStatsRecord {
            fixture_id: fixture.id.clone(),
            season: league.season,
            matchday: fixture.matchday,
            date: fixture.date.clone(),
            competition: fixture.competition.clone(),
            team_id: away_team_id.to_string(),
            opponent_team_id: home_team_id.to_string(),
            side: TeamSide::Red,
            result: outcome_for_side(TeamSide::Red, report),
            duration_seconds: report.game_duration_seconds,
            kills: report.away_stats.kills,
            deaths: report.away_stats.deaths,
            gold_earned: report.away_stats.gold_earned,
            damage_dealt: report.away_stats.damage_dealt,
            objectives: report.away_stats.objectives,
        },
    ];

    StatsState {
        player_matches,
        team_matches,
    }
}

// ---------------------------------------------------------------------------
// Post-match: feed engine report stats back into domain Player models
// ---------------------------------------------------------------------------

fn apply_player_stats(
    game: &mut Game,
    report: &engine::MatchReport,
    home_team_id: &str,
    away_team_id: &str,
) {
    for player in game.players.iter_mut() {
        if let Some(ps) = report.player_stats.get(&player.id) {
            player.stats.appearances += 1;
            player.stats.goals += ps.kills as u32;
            player.stats.assists += ps.assists as u32;
            player.stats.minutes_played += ps.duration_seconds / 60;

            let match_rating = 6.0 + ((ps.kills + ps.assists) as f32 * 0.35)
                - (ps.deaths as f32 * 0.25)
                + ((ps.damage_dealt as f32 / 10_000.0).min(1.5));
            if player.stats.appearances == 1 {
                player.stats.avg_rating = match_rating.clamp(0.0, 10.0);
            } else {
                let n = player.stats.appearances as f32;
                player.stats.avg_rating =
                    (player.stats.avg_rating * (n - 1.0) + match_rating.clamp(0.0, 10.0)) / n;
            }

            if matches!(player.position, DomainPosition::Goalkeeper) {
                let tid = player.team_id.as_deref().unwrap_or("");
                let conceded_zero = if tid == home_team_id {
                    report.away_stats.kills == 0
                } else if tid == away_team_id {
                    report.home_stats.kills == 0
                } else {
                    false
                };
                if conceded_zero {
                    player.stats.clean_sheets += 1;
                }
            }
        }
    }
}

fn resolve_post_match_promises(
    game: &mut Game,
    report: &engine::MatchReport,
    home_team_id: &str,
    away_team_id: &str,
) {
    for player in game.players.iter_mut() {
        let Some(team_id) = player.team_id.as_deref() else {
            continue;
        };
        if team_id != home_team_id && team_id != away_team_id {
            continue;
        }

        let Some(promise) = player.morale_core.pending_promise.clone() else {
            continue;
        };

        let played = report.player_stats.contains_key(&player.id);

        match promise.kind {
            PlayerPromiseKind::PlayingTime => {
                if played {
                    player.morale_core.pending_promise = None;
                    player.morale_core.manager_trust =
                        (i16::from(player.morale_core.manager_trust) + 3).clamp(0, 100) as u8;

                    if player
                        .morale_core
                        .unresolved_issue
                        .as_ref()
                        .is_some_and(|issue| issue.category == PlayerIssueCategory::PlayingTime)
                    {
                        player.morale_core.unresolved_issue = None;
                    }
                } else if promise.matches_remaining <= 1 {
                    player.morale_core.pending_promise = None;
                    player.morale_core.manager_trust =
                        (i16::from(player.morale_core.manager_trust) - 12).clamp(0, 100) as u8;
                    player.morale_core.unresolved_issue = Some(PlayerIssue {
                        category: PlayerIssueCategory::PlayingTime,
                        severity: 75,
                    });
                } else {
                    player.morale_core.pending_promise = Some(domain::player::PlayerPromise {
                        kind: PlayerPromiseKind::PlayingTime,
                        matches_remaining: promise.matches_remaining - 1,
                    });
                }
            }
        }
    }
}

fn capped_positive_recovery(delta: i16, player: &domain::player::Player) -> i16 {
    let Some(issue) = player.morale_core.unresolved_issue.as_ref() else {
        return delta;
    };

    if delta <= 0 {
        return delta;
    }

    if issue.severity >= 75 {
        return 0;
    }

    if issue.severity >= 50 {
        return ((delta + 1) / 2).max(1);
    }

    delta
}

/// Update player morale based on match result and individual performance.
fn update_post_match_morale(
    game: &mut Game,
    report: &engine::MatchReport,
    home_team_id: &str,
    away_team_id: &str,
) {
    use rand::RngExt;
    let mut rng = rand::rng();

    let home_won = report.home_wins > report.away_wins;
    let away_won = report.away_wins > report.home_wins;

    for player in game.players.iter_mut() {
        let tid = match player.team_id.as_deref() {
            Some(t) if t == home_team_id || t == away_team_id => t.to_string(),
            _ => continue,
        };

        let is_home = tid == home_team_id;
        let base_morale = player.morale as i16;

        let goal_diff = (report.home_wins as i16 - report.away_wins as i16).abs();
        let result_delta: i16 = if (is_home && home_won) || (!is_home && away_won) {
            rng.random_range(3..=8)
        } else {
            let base_loss = rng.random_range(-5..=-2);
            let margin_penalty = (goal_diff - 1).max(0) * -3;
            base_loss + margin_penalty
        };

        let mut individual_delta: i16 = 0;
        if let Some(ps) = report.player_stats.get(&player.id) {
            individual_delta += ps.kills as i16 * 3;
            individual_delta += ps.assists as i16 * 2;
            if ps.deaths >= 5 {
                individual_delta -= 3;
            } else if ps.kills + ps.assists >= 6 {
                individual_delta += 2;
            }
        }

        let total_delta = capped_positive_recovery(result_delta + individual_delta, player);
        let new_morale = (base_morale + total_delta).clamp(10, 100) as u8;
        player.morale = new_morale;
    }
}

/// Update team form vectors after a match result. Keeps last 5 results.
/// Also applies streak-based morale bonus/penalty to all players on teams with streaks.
fn update_team_form(
    game: &mut Game,
    report: &engine::MatchReport,
    home_team_id: &str,
    away_team_id: &str,
) {
    use rand::RngExt;
    let mut rng = rand::rng();

    let home_result = if report.home_wins > report.away_wins {
        "W"
    } else {
        "L"
    };
    let away_result = if report.away_wins > report.home_wins {
        "W"
    } else {
        "L"
    };

    // Update form for both teams
    for (team_id_str, result) in [(home_team_id, home_result), (away_team_id, away_result)] {
        if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id_str) {
            team.form.push(result.to_string());
            if team.form.len() > 5 {
                team.form.remove(0);
            }
        }
    }

    // Apply streak-based morale bonus/penalty
    for team_id_str in [home_team_id, away_team_id] {
        let form = game
            .teams
            .iter()
            .find(|t| t.id == team_id_str)
            .map(|t| t.form.clone())
            .unwrap_or_default();

        if form.len() >= 3 {
            let last3: Vec<&str> = form.iter().rev().take(3).map(|s| s.as_str()).collect();
            let streak_delta: i16 = if last3.iter().all(|r| *r == "W") {
                rng.random_range(2..=5) // 3+ win streak: small global morale boost
            } else if last3.iter().all(|r| *r == "L") {
                rng.random_range(-10..=-5) // 3+ loss streak: significant morale drop
            } else {
                0
            };

            if streak_delta != 0 {
                for player in game.players.iter_mut() {
                    if player.team_id.as_deref() == Some(team_id_str) {
                        let base = player.morale as i16;
                        let adjusted_delta = capped_positive_recovery(streak_delta, player);
                        player.morale = (base + adjusted_delta).clamp(10, 100) as u8;
                    }
                }
            }
        }
    }
}

fn deplete_match_stamina(game: &mut Game, team_id: &str, report: &engine::MatchReport) {
    for player in game.players.iter_mut() {
        if player.team_id.as_deref() == Some(team_id) {
            let minutes = report
                .player_stats
                .get(&player.id)
                .map(|ps| (ps.duration_seconds / 60) as u8)
                .unwrap_or(0);
            if minutes == 0 {
                continue;
            }
            let minutes_factor = minutes as f64 / 90.0;
            let stamina_factor = player.attributes.stamina as f64 / 100.0;
            let base_depletion = 40.0 * (1.0 - stamina_factor * 0.4);
            let depletion = (base_depletion * minutes_factor) as u8;
            player.condition = player.condition.saturating_sub(depletion);

            if minutes >= 60 {
                use rand::RngExt;
                let mut rng = rand::rng();
                if rng.random_bool(0.3) && player.fitness < 100 {
                    player.fitness = player.fitness.saturating_add(1);
                }
            }
        }
    }
}
