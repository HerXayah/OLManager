mod news;
mod post_match;
mod round_summary;

use crate::board_objectives;
use crate::champions;
use crate::end_of_season;
use crate::game::Game;
use crate::player_events;
use crate::potential;
use crate::random_events;
use crate::schedule;
use crate::scouting;
use crate::training;
use crate::transfers;
use chrono::Datelike;
use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, MatchResult};
use domain::player::Position as DomainPosition;
use domain::stats::StatsState;
use log::{debug, info};
use std::collections::HashMap;
use uuid::Uuid;

// Re-export public items
pub use news::generate_matchday_news;
pub use post_match::{apply_match_report, apply_match_report_with_capture};
pub use round_summary::{
    NotableUpset, RoundResultSummary, RoundSummary, StandingDelta, TopScorerDelta,
    build_round_summary,
};

/// Progress injury recovery by one day for all currently injured players.
/// Players with 1 day remaining are cleared (fully recovered).
fn progress_injury_recovery(game: &mut Game) {
    for player in game.players.iter_mut() {
        if let Some(mut injury) = player.injury.take()
            && injury.days_remaining > 1
        {
            injury.days_remaining -= 1;
            player.injury = Some(injury);
        }
    }
}

/// Process a single day advance.
pub fn process_day(game: &mut Game) {
    process_day_with_capture(game, &mut |_| {});
}

pub fn process_day_with_capture<F>(game: &mut Game, on_capture: &mut F)
where
    F: FnMut(StatsState),
{
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    let has_match_today = game.league.as_ref().is_some_and(|league| {
        league
            .fixtures
            .iter()
            .any(|f| f.date == today && f.status == FixtureStatus::Scheduled)
    });

    if has_match_today {
        info!("[turn] process_day {}: matchday", today);
        simulate_matchday_with_capture(game, &today, on_capture);
        maybe_schedule_playoffs(game);
    } else {
        let weekday_num = game.clock.current_date.weekday().num_days_from_monday();
        training::process_training(game, weekday_num);
        training::check_squad_fitness_warnings(game);
    }

    crate::contracts::process_contract_expiries(game);

    // Weekly financial processing (wages, matchday income, warnings)
    crate::finances::process_weekly_finances(game);

    // Board objectives (generate if missing, update progress)
    board_objectives::generate_objectives(game);
    board_objectives::update_objective_progress(game);

    // Player conversations, random events, and scouting
    player_events::check_player_events(game);
    progress_injury_recovery(game);
    random_events::check_random_events(game);
    scouting::process_scouting(game);
    transfers::generate_incoming_transfer_offers(game);

    news::generate_weekly_digest_news(game, &today);
    news::generate_pre_match_messages(game, &today);

    crate::firing::check_manager_firing(game);
    crate::job_offers::check_job_offers(game);
    potential::process_potential_research(game);
    champions::process_daily_champion_system(game);

    debug!("[turn] process_day {}: complete, advancing clock", today);
    game.clock.advance_days(1);
    crate::season_context::refresh_game_context(game);
}

/// Called after a live match finishes to complete the day:
/// generates matchday news, pre-match messages, and advances the clock by one day.
pub fn finish_live_match_day(game: &mut Game) {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    info!("[turn] finish_live_match_day: {}", today);
    generate_matchday_news(game, &today);
    maybe_schedule_playoffs(game);

    crate::contracts::process_contract_expiries(game);

    board_objectives::generate_objectives(game);
    board_objectives::update_objective_progress(game);

    player_events::check_player_events(game);
    progress_injury_recovery(game);
    random_events::check_random_events(game);
    scouting::process_scouting(game);
    transfers::generate_incoming_transfer_offers(game);
    news::generate_weekly_digest_news(game, &today);
    news::generate_pre_match_messages(game, &today);

    crate::firing::check_manager_firing(game);
    crate::job_offers::check_job_offers(game);
    potential::process_potential_research(game);
    champions::process_daily_champion_system(game);

    game.clock.advance_days(1);
    crate::season_context::refresh_game_context(game);
}

// ---------------------------------------------------------------------------
// Domain → Engine type conversion
// ---------------------------------------------------------------------------

fn build_engine_team(game: &Game, team_id: &str) -> engine::TeamData {
    let team = game.teams.iter().find(|t| t.id == team_id);
    let (name, formation, play_style) = match team {
        Some(t) => (
            t.name.clone(),
            t.formation.clone(),
            match t.play_style {
                domain::team::PlayStyle::Attacking => engine::PlayStyle::Attacking,
                domain::team::PlayStyle::Defensive => engine::PlayStyle::Defensive,
                domain::team::PlayStyle::Possession => engine::PlayStyle::Possession,
                domain::team::PlayStyle::Counter => engine::PlayStyle::Counter,
                domain::team::PlayStyle::HighPress => engine::PlayStyle::HighPress,
                _ => engine::PlayStyle::Balanced,
            },
        ),
        None => (
            "Unknown".into(),
            "4-4-2".into(),
            engine::PlayStyle::Balanced,
        ),
    };

    let players: Vec<engine::PlayerData> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id))
        .map(|p| {
            let pos = match p.position.to_group_position() {
                DomainPosition::Goalkeeper => engine::Position::Goalkeeper,
                DomainPosition::Defender => engine::Position::Defender,
                DomainPosition::Midfielder => engine::Position::Midfielder,
                DomainPosition::Forward => engine::Position::Forward,
                _ => engine::Position::Midfielder,
            };
            engine::PlayerData {
                id: p.id.clone(),
                name: p.match_name.clone(),
                position: pos,
                lol_role: Some(lol_role_from_position(&p.natural_position).to_string()),
                condition: p.condition,
                fitness: p.fitness,
                pace: p.attributes.pace,
                stamina: p.attributes.stamina,
                strength: p.attributes.strength,
                agility: p.attributes.agility,
                passing: p.attributes.passing,
                shooting: p.attributes.shooting,
                tackling: p.attributes.tackling,
                dribbling: p.attributes.dribbling,
                defending: p.attributes.defending,
                positioning: p.attributes.positioning,
                vision: p.attributes.vision,
                decisions: p.attributes.decisions,
                composure: p.attributes.composure,
                aggression: p.attributes.aggression,
                teamwork: p.attributes.teamwork,
                leadership: p.attributes.leadership,
                handling: p.attributes.handling,
                reflexes: p.attributes.reflexes,
                aerial: p.attributes.aerial,
                traits: p.traits.iter().map(|t| format!("{:?}", t)).collect(),
            }
        })
        .collect();

    engine::TeamData {
        id: team_id.to_string(),
        name,
        formation,
        play_style,
        players,
    }
}

fn maybe_schedule_playoffs(game: &mut Game) {
    let Some(league) = game.league.as_mut() else {
        return;
    };

    let split = match schedule::parse_lec_split(&league.name) {
        Some(split) => split,
        None => return,
    };

    let playoff_fixtures_exist = league
        .fixtures
        .iter()
        .any(|fixture| fixture.competition == FixtureCompetition::Playoffs);

    if !playoff_fixtures_exist {
        if !regular_season_complete(league) {
            return;
        }

        let sorted = league.sorted_standings();
        let required_seeds = match split {
            schedule::LecSplit::Winter => 8,
            schedule::LecSplit::Spring | schedule::LecSplit::Summer => 6,
        };
        if sorted.len() < required_seeds {
            return;
        }

        let seeded_team_ids: Vec<String> = sorted
            .iter()
            .take(required_seeds)
            .map(|entry| entry.team_id.clone())
            .collect();

        let start_date = game.clock.current_date + chrono::Duration::days(1);
        let start_matchday = league
            .fixtures
            .iter()
            .map(|fixture| fixture.matchday)
            .max()
            .unwrap_or(0)
            + 1;

        let opening_pairings = match split {
            schedule::LecSplit::Winter => vec![
                (seeded_team_ids[0].clone(), seeded_team_ids[7].clone()),
                (seeded_team_ids[3].clone(), seeded_team_ids[4].clone()),
                (seeded_team_ids[1].clone(), seeded_team_ids[6].clone()),
                (seeded_team_ids[2].clone(), seeded_team_ids[5].clone()),
            ],
            schedule::LecSplit::Spring | schedule::LecSplit::Summer => vec![
                (seeded_team_ids[0].clone(), seeded_team_ids[3].clone()),
                (seeded_team_ids[1].clone(), seeded_team_ids[2].clone()),
            ],
        };

        let best_of = schedule::playoff_best_of(split, false);
        let opening_round = build_playoff_round_fixtures(
            start_matchday,
            start_date.format("%Y-%m-%d").to_string(),
            opening_pairings,
            best_of,
        );
        schedule::append_fixtures(league, opening_round);
        return;
    }

    let has_pending_playoffs = league.fixtures.iter().any(|fixture| {
        fixture.competition == FixtureCompetition::Playoffs && fixture.status != FixtureStatus::Completed
    });
    if has_pending_playoffs {
        return;
    }

    let required_seeds = match split {
        schedule::LecSplit::Winter => 8,
        schedule::LecSplit::Spring | schedule::LecSplit::Summer => 6,
    };
    let seeded_team_ids: Vec<String> = league
        .sorted_standings()
        .iter()
        .take(required_seeds)
        .map(|entry| entry.team_id.clone())
        .collect();
    if seeded_team_ids.len() < required_seeds {
        return;
    }

    let next_matchday = league
        .fixtures
        .iter()
        .filter(|fixture| fixture.competition == FixtureCompetition::Playoffs)
        .map(|fixture| fixture.matchday)
        .max()
        .unwrap_or(0)
        + 1;
    let next_date = (game.clock.current_date + chrono::Duration::days(7))
        .format("%Y-%m-%d")
        .to_string();

    let next_pairings = match split {
        schedule::LecSplit::Winter => next_winter_playoff_pairings(league, &seeded_team_ids),
        schedule::LecSplit::Spring | schedule::LecSplit::Summer => {
            next_spring_summer_playoff_pairings(league, &seeded_team_ids)
        }
    };

    let Some((pairings, is_grand_final)) = next_pairings else {
        return;
    };

    let best_of = schedule::playoff_best_of(split, is_grand_final);
    let fixtures = build_playoff_round_fixtures(next_matchday, next_date, pairings, best_of);
    if fixtures.is_empty() {
        return;
    }

    schedule::append_fixtures(league, fixtures);
}

fn regular_season_complete(league: &League) -> bool {
    end_of_season::season_has_started(league)
        && end_of_season::has_full_schedule(league)
        && league
            .fixtures
            .iter()
            .filter(|fixture| fixture.counts_for_league_standings())
            .all(|fixture| fixture.status == FixtureStatus::Completed)
}

fn build_playoff_round_fixtures(
    matchday: u32,
    date: String,
    pairings: Vec<(String, String)>,
    best_of: u8,
) -> Vec<Fixture> {
    pairings
        .into_iter()
        .map(|(home_team_id, away_team_id)| Fixture {
            id: Uuid::new_v4().to_string(),
            matchday,
            date: date.clone(),
            home_team_id,
            away_team_id,
            competition: FixtureCompetition::Playoffs,
            best_of,
            status: FixtureStatus::Scheduled,
            result: None,
        })
        .collect()
}

fn playoff_round_fixtures(league: &League, round: u32) -> Vec<&Fixture> {
    let Some(start_matchday) = league
        .fixtures
        .iter()
        .filter(|fixture| fixture.competition == FixtureCompetition::Playoffs)
        .map(|fixture| fixture.matchday)
        .min()
    else {
        return Vec::new();
    };

    let target_matchday = start_matchday + round.saturating_sub(1);
    league
        .fixtures
        .iter()
        .filter(|fixture| {
            fixture.competition == FixtureCompetition::Playoffs && fixture.matchday == target_matchday
        })
        .collect()
}

fn fixture_winner_loser(fixture: &Fixture) -> Option<(String, String)> {
    let result: &MatchResult = fixture.result.as_ref()?;
    if result.home_wins > result.away_wins {
        Some((fixture.home_team_id.clone(), fixture.away_team_id.clone()))
    } else if result.away_wins > result.home_wins {
        Some((fixture.away_team_id.clone(), fixture.home_team_id.clone()))
    } else {
        None
    }
}

fn outcome_for_pair(fixtures: &[&Fixture], a: &str, b: &str) -> Option<(String, String)> {
    fixtures
        .iter()
        .find(|fixture| {
            (fixture.home_team_id == a && fixture.away_team_id == b)
                || (fixture.home_team_id == b && fixture.away_team_id == a)
        })
        .and_then(|fixture| fixture_winner_loser(fixture))
}

fn next_spring_summer_playoff_pairings(
    league: &League,
    seeds: &[String],
) -> Option<(Vec<(String, String)>, bool)> {
    if seeds.len() < 6 {
        return None;
    }

    let r1 = playoff_round_fixtures(league, 1);
    let r2 = playoff_round_fixtures(league, 2);
    let r3 = playoff_round_fixtures(league, 3);
    let r4 = playoff_round_fixtures(league, 4);
    let r5 = playoff_round_fixtures(league, 5);
    let r6 = playoff_round_fixtures(league, 6);

    if r1.is_empty() {
        return None;
    }
    if r2.is_empty() {
        let (_w1, l1) = outcome_for_pair(&r1, &seeds[0], &seeds[3])?;
        let (_w2, l2) = outcome_for_pair(&r1, &seeds[1], &seeds[2])?;
        return Some((vec![(l1, seeds[5].clone()), (l2, seeds[4].clone())], false));
    }
    if r3.is_empty() {
        let (w1, _l1) = outcome_for_pair(&r1, &seeds[0], &seeds[3])?;
        let (w2, _l2) = outcome_for_pair(&r1, &seeds[1], &seeds[2])?;
        return Some((vec![(w1, w2)], false));
    }
    if r4.is_empty() {
        let (w_r2_a, _l_r2_a) = outcome_for_pair(&r2, &seeds[5], &seeds[0])
            .or_else(|| outcome_for_pair(&r2, &seeds[5], &seeds[3]))?;
        let (w_r2_b, _l_r2_b) = outcome_for_pair(&r2, &seeds[4], &seeds[1])
            .or_else(|| outcome_for_pair(&r2, &seeds[4], &seeds[2]))?;
        return Some((vec![(w_r2_a, w_r2_b)], false));
    }
    if r5.is_empty() {
        let (_w_r3, l_r3) = fixture_winner_loser(*r3.first()?)?;
        let (w_r4, _l_r4) = fixture_winner_loser(*r4.first()?)?;
        return Some((vec![(l_r3, w_r4)], false));
    }
    if r6.is_empty() {
        let (w_r3, _l_r3) = fixture_winner_loser(*r3.first()?)?;
        let (w_r5, _l_r5) = fixture_winner_loser(*r5.first()?)?;
        return Some((vec![(w_r3, w_r5)], true));
    }

    None
}

fn next_winter_playoff_pairings(
    league: &League,
    seeds: &[String],
) -> Option<(Vec<(String, String)>, bool)> {
    if seeds.len() < 8 {
        return None;
    }

    let r1 = playoff_round_fixtures(league, 1);
    let r2 = playoff_round_fixtures(league, 2);
    let r3 = playoff_round_fixtures(league, 3);
    let r4 = playoff_round_fixtures(league, 4);
    let r5 = playoff_round_fixtures(league, 5);
    let r6 = playoff_round_fixtures(league, 6);
    let r7 = playoff_round_fixtures(league, 7);
    let r8 = playoff_round_fixtures(league, 8);

    if r1.is_empty() {
        return None;
    }

    let (w1, l1) = outcome_for_pair(&r1, &seeds[0], &seeds[7])?;
    let (w2, l2) = outcome_for_pair(&r1, &seeds[3], &seeds[4])?;
    let (w3, l3) = outcome_for_pair(&r1, &seeds[1], &seeds[6])?;
    let (w4, l4) = outcome_for_pair(&r1, &seeds[2], &seeds[5])?;

    if r2.is_empty() {
        return Some((vec![(l1.clone(), l2.clone()), (l3.clone(), l4.clone())], false));
    }
    if r3.is_empty() {
        return Some((vec![(w1.clone(), w2.clone()), (w3.clone(), w4.clone())], false));
    }

    let (wlb1_a, _llb1_a) = outcome_for_pair(&r2, &l1, &l2)?;
    let (wlb1_b, _llb1_b) = outcome_for_pair(&r2, &l3, &l4)?;
    let (wwb2_a, lwb2_a) = outcome_for_pair(&r3, &w1, &w2)?;
    let (wwb2_b, lwb2_b) = outcome_for_pair(&r3, &w3, &w4)?;

    if r4.is_empty() {
        return Some((vec![(wlb1_a.clone(), lwb2_a.clone()), (wlb1_b.clone(), lwb2_b.clone())], false));
    }
    if r5.is_empty() {
        return Some((vec![(wwb2_a.clone(), wwb2_b.clone())], false));
    }

    let (wlb2_a, _llb2_a) = outcome_for_pair(&r4, &wlb1_a, &lwb2_a)?;
    let (wlb2_b, _llb2_b) = outcome_for_pair(&r4, &wlb1_b, &lwb2_b)?;
    let (wwbf, lwbf) = outcome_for_pair(&r5, &wwb2_a, &wwb2_b)?;

    if r6.is_empty() {
        return Some((vec![(wlb2_a.clone(), wlb2_b.clone())], false));
    }

    let (wlb3, _llb3) = outcome_for_pair(&r6, &wlb2_a, &wlb2_b)?;
    if r7.is_empty() {
        return Some((vec![(lwbf.clone(), wlb3.clone())], false));
    }

    let (wlb_final, _llb_final) = outcome_for_pair(&r7, &lwbf, &wlb3)?;
    if r8.is_empty() {
        return Some((vec![(wwbf, wlb_final)], true));
    }

    None
}

fn lol_role_from_position(position: &DomainPosition) -> &'static str {
    match position {
        DomainPosition::Defender
        | DomainPosition::RightBack
        | DomainPosition::CenterBack
        | DomainPosition::LeftBack
        | DomainPosition::RightWingBack
        | DomainPosition::LeftWingBack => "TOP",
        DomainPosition::AttackingMidfielder
        | DomainPosition::RightMidfielder
        | DomainPosition::LeftMidfielder => "MID",
        DomainPosition::Forward
        | DomainPosition::RightWinger
        | DomainPosition::LeftWinger
        | DomainPosition::Striker => "ADC",
        DomainPosition::Goalkeeper | DomainPosition::DefensiveMidfielder => "SUPPORT",
        DomainPosition::Midfielder | DomainPosition::CentralMidfielder => "JUNGLE",
    }
}

// ---------------------------------------------------------------------------
// Matchday simulation using the engine crate
// ---------------------------------------------------------------------------

fn simulate_matchday_with_capture<F>(game: &mut Game, today: &str, on_capture: &mut F)
where
    F: FnMut(StatsState),
{
    info!("[turn] simulate_matchday: {}", today);
    simulate_other_matches_with_capture(game, today, None, on_capture);
    generate_matchday_news(game, today);
}

/// Simulate all scheduled matches for `today`, optionally skipping one fixture
/// (the user's live match). Called by both process_day and advance_time_with_mode.
pub fn simulate_other_matches(game: &mut Game, today: &str, skip_fixture: Option<usize>) {
    simulate_other_matches_with_capture(game, today, skip_fixture, &mut |_| {});
}

pub fn simulate_other_matches_with_capture<F>(
    game: &mut Game,
    today: &str,
    skip_fixture: Option<usize>,
    on_capture: &mut F,
) where
    F: FnMut(StatsState),
{
    debug!(
        "[turn] simulate_other_matches: date={}, skip={:?}",
        today, skip_fixture
    );
    let fixture_indices: Vec<usize> = game.league.as_ref().map_or(vec![], |league| {
        league
            .fixtures
            .iter()
            .enumerate()
            .filter(|(i, f)| {
                f.date == today
                    && f.status == FixtureStatus::Scheduled
                    && (skip_fixture != Some(*i))
            })
            .map(|(i, _)| i)
            .collect()
    });

    for idx in fixture_indices {
        simulate_single_match_with_capture(game, idx, on_capture);
    }
}

fn simulate_single_match_with_capture<F>(game: &mut Game, idx: usize, on_capture: &mut F)
where
    F: FnMut(StatsState),
{
    let (home_team_id, away_team_id, best_of) = {
        let f = &game.league.as_ref().unwrap().fixtures[idx];
        (f.home_team_id.clone(), f.away_team_id.clone(), f.best_of)
    };

    let home_name = game
        .teams
        .iter()
        .find(|t| t.id == home_team_id)
        .map(|t| t.name.as_str())
        .unwrap_or("?");
    let away_name = game
        .teams
        .iter()
        .find(|t| t.id == away_team_id)
        .map(|t| t.name.as_str())
        .unwrap_or("?");
    debug!(
        "[turn] simulate_single_match: {} vs {} (fixture #{})",
        home_name, away_name, idx
    );

    let home_data = build_engine_team(game, &home_team_id);
    let away_data = build_engine_team(game, &away_team_id);
    let config = engine::MatchConfig::default();
    let report = if best_of <= 1 {
        engine::simulate(&home_data, &away_data, &config)
    } else {
        simulate_series(&home_data, &away_data, &config, best_of)
    };

    info!(
        "[turn] match result: {} {} - {} {} (fixture #{})",
        home_name, report.home_wins, report.away_wins, away_name, idx
    );
    apply_match_report_with_capture(game, idx, &home_team_id, &away_team_id, &report, on_capture);
}

fn simulate_series(
    home_data: &engine::TeamData,
    away_data: &engine::TeamData,
    config: &engine::MatchConfig,
    best_of: u8,
) -> engine::MatchReport {
    let target_wins = (best_of / 2) + 1;
    let mut home_wins = 0_u8;
    let mut away_wins = 0_u8;
    let mut reports: Vec<engine::MatchReport> = Vec::new();

    while home_wins < target_wins && away_wins < target_wins {
        let report = engine::simulate(home_data, away_data, config);
        home_wins = home_wins.saturating_add(report.home_wins);
        away_wins = away_wins.saturating_add(report.away_wins);
        reports.push(report);
    }

    let mut merged = reports
        .last()
        .cloned()
        .unwrap_or_else(|| engine::simulate(home_data, away_data, config));
    merged.home_wins = home_wins;
    merged.away_wins = away_wins;

    merged.home_stats = engine::TeamStats::default();
    merged.away_stats = engine::TeamStats::default();
    merged.events.clear();
    merged.kill_feed.clear();
    merged.player_stats = HashMap::new();
    merged.game_duration_seconds = 0;

    let mut possession_sum = 0.0_f64;
    for report in reports {
        merged.home_stats.kills = merged.home_stats.kills.saturating_add(report.home_stats.kills);
        merged.home_stats.deaths = merged.home_stats.deaths.saturating_add(report.home_stats.deaths);
        merged.home_stats.gold_earned = merged
            .home_stats
            .gold_earned
            .saturating_add(report.home_stats.gold_earned);
        merged.home_stats.damage_dealt = merged
            .home_stats
            .damage_dealt
            .saturating_add(report.home_stats.damage_dealt);
        merged.home_stats.objectives = merged
            .home_stats
            .objectives
            .saturating_add(report.home_stats.objectives);
        merged.home_stats.possession_ticks = merged
            .home_stats
            .possession_ticks
            .saturating_add(report.home_stats.possession_ticks);

        merged.away_stats.kills = merged.away_stats.kills.saturating_add(report.away_stats.kills);
        merged.away_stats.deaths = merged.away_stats.deaths.saturating_add(report.away_stats.deaths);
        merged.away_stats.gold_earned = merged
            .away_stats
            .gold_earned
            .saturating_add(report.away_stats.gold_earned);
        merged.away_stats.damage_dealt = merged
            .away_stats
            .damage_dealt
            .saturating_add(report.away_stats.damage_dealt);
        merged.away_stats.objectives = merged
            .away_stats
            .objectives
            .saturating_add(report.away_stats.objectives);
        merged.away_stats.possession_ticks = merged
            .away_stats
            .possession_ticks
            .saturating_add(report.away_stats.possession_ticks);

        merged.events.extend(report.events);
        merged.kill_feed.extend(report.kill_feed);
        merged.game_duration_seconds = merged
            .game_duration_seconds
            .saturating_add(report.game_duration_seconds);
        possession_sum += report.home_possession;

        for (player_id, stats) in report.player_stats {
            let entry = merged.player_stats.entry(player_id).or_default();
            if entry.role.is_none() {
                entry.role = stats.role;
            }
            entry.duration_seconds = entry.duration_seconds.saturating_add(stats.duration_seconds);
            entry.kills = entry.kills.saturating_add(stats.kills);
            entry.deaths = entry.deaths.saturating_add(stats.deaths);
            entry.assists = entry.assists.saturating_add(stats.assists);
            entry.creep_score = entry.creep_score.saturating_add(stats.creep_score);
            entry.gold_earned = entry.gold_earned.saturating_add(stats.gold_earned);
            entry.damage_dealt = entry.damage_dealt.saturating_add(stats.damage_dealt);
            entry.vision_score = entry.vision_score.saturating_add(stats.vision_score);
            entry.wards_placed = entry.wards_placed.saturating_add(stats.wards_placed);
        }
    }

    if home_wins + away_wins > 0 {
        merged.home_possession = possession_sum / f64::from(home_wins + away_wins);
    }

    merged
}
