use log::info;

use ofm_core::contracts::contract_warning_stage;
use ofm_core::game::Game;
use ofm_core::player_rating::{effective_rating_for_assignment, formation_slots, natural_ovr};

fn user_team_context<'a>(
    game: &'a Game,
) -> Option<(&'a domain::team::Team, Vec<&'a domain::player::Player>)> {
    let user_team_id = game.manager.team_id.as_deref()?;
    let team = game.teams.iter().find(|team| team.id == user_team_id)?;
    let roster = game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(user_team_id))
        .collect();

    Some((team, roster))
}

fn build_blocker(id: &str, severity: &str, text: String, tab: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "severity": severity,
        "text": text,
        "tab": tab
    })
}

fn build_effective_healthy_starting_xi_ids(
    saved_xi_ids: &[String],
    roster: &[&domain::player::Player],
    formation: &str,
) -> Vec<String> {
    let healthy_roster: Vec<&domain::player::Player> = roster
        .iter()
        .copied()
        .filter(|player| player.injury.is_none())
        .collect();
    let by_id: std::collections::HashMap<&str, &domain::player::Player> = healthy_roster
        .iter()
        .map(|player| (player.id.as_str(), *player))
        .collect();
    let mut used = std::collections::HashSet::new();
    let mut valid_saved_ids = Vec::new();

    for id in saved_xi_ids {
        if by_id.contains_key(id.as_str()) && used.insert(id.clone()) {
            valid_saved_ids.push(id.clone());
        }
    }

    let mut remaining_players: Vec<&domain::player::Player> = healthy_roster
        .iter()
        .copied()
        .filter(|player| !used.contains(&player.id))
        .collect();
    remaining_players.sort_by(|left, right| {
        natural_ovr(right)
            .partial_cmp(&natural_ovr(left))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let slots = formation_slots(formation);

    if valid_saved_ids.len() >= 8 {
        let mut xi_ids = valid_saved_ids;
        while xi_ids.len() < 11 {
            let slot = slots.get(xi_ids.len());
            let best_index = remaining_players
                .iter()
                .enumerate()
                .max_by(|(_, left), (_, right)| {
                    let left_rating = slot.map_or_else(
                        || natural_ovr(left),
                        |slot| effective_rating_for_assignment(left, slot),
                    );
                    let right_rating = slot.map_or_else(
                        || natural_ovr(right),
                        |slot| effective_rating_for_assignment(right, slot),
                    );
                    left_rating
                        .partial_cmp(&right_rating)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(index, _)| index);

            let Some(best_index) = best_index else {
                break;
            };

            let player = remaining_players.remove(best_index);
            xi_ids.push(player.id.clone());
        }
        xi_ids.truncate(11);
        return xi_ids;
    }

    let mut xi_ids = Vec::new();

    for slot in slots.iter().take(11) {
        let best_player = healthy_roster
            .iter()
            .copied()
            .filter(|player| !used.contains(&player.id))
            .max_by(|left, right| {
                effective_rating_for_assignment(left, slot)
                    .partial_cmp(&effective_rating_for_assignment(right, slot))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        let Some(player) = best_player else {
            break;
        };

        if used.insert(player.id.clone()) {
            xi_ids.push(player.id.clone());
        }
    }

    xi_ids
}

fn injured_starting_xi_blocker(
    xi_ids: &[String],
    roster: &[&domain::player::Player],
) -> Option<serde_json::Value> {
    let injured_in_xi: Vec<_> = xi_ids
        .iter()
        .filter_map(|id| {
            roster
                .iter()
                .find(|player| player.id == *id && player.injury.is_some())
        })
        .map(|player| player.match_name.clone())
        .collect();

    (!injured_in_xi.is_empty()).then(|| {
        build_blocker(
            "injured_xi",
            "warn",
            format!(
                "{} injured player(s) in Starting XI: {}",
                injured_in_xi.len(),
                injured_in_xi.join(", ")
            ),
            "Squad",
        )
    })
}

fn incomplete_starting_xi_blocker(
    effective_healthy_xi_ids: &[String],
    roster: &[&domain::player::Player],
) -> Option<serde_json::Value> {
    let healthy_xi = effective_healthy_xi_ids.len();

    (healthy_xi < 11 && roster.len() >= 11).then(|| {
        build_blocker(
            "incomplete_xi",
            "warn",
            format!(
                "Starting XI has only {} healthy players — set your lineup",
                healthy_xi
            ),
            "Squad",
        )
    })
}

fn urgent_unread_messages_blocker(game: &Game) -> Option<serde_json::Value> {
    let urgent_unread = game
        .messages
        .iter()
        .filter(|message| {
            !message.read && message.priority == domain::message::MessagePriority::Urgent
        })
        .count();

    (urgent_unread > 0).then(|| {
        build_blocker(
            "urgent_messages",
            "info",
            format!("{} urgent unread message(s)", urgent_unread),
            "Inbox",
        )
    })
}

fn key_contract_risk_blocker(
    roster: &[&domain::player::Player],
    effective_healthy_xi_ids: &[String],
    current_date: chrono::NaiveDate,
) -> Option<serde_json::Value> {
    let effective_xi_id_set: std::collections::HashSet<&str> = effective_healthy_xi_ids
        .iter()
        .map(String::as_str)
        .collect();

    let mut effective_xi_players: Vec<&domain::player::Player> = roster
        .iter()
        .copied()
        .filter(|player| effective_xi_id_set.contains(player.id.as_str()))
        .collect();
    effective_xi_players.sort_by(|left, right| {
        natural_ovr(right)
            .partial_cmp(&natural_ovr(left))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let risky_key_players: Vec<&str> = effective_xi_players
        .into_iter()
        .take(3)
        .filter(|player| {
            contract_warning_stage(player.contract_end.as_deref(), current_date).is_some()
        })
        .map(|player| player.match_name.as_str())
        .collect();

    (!risky_key_players.is_empty()).then(|| {
        build_blocker(
            "key_contract_risk",
            "warn",
            format!(
                "Key player contract risk in squad planning: {}",
                risky_key_players.join(", ")
            ),
            "Squad",
        )
    })
}

fn contract_wage_risk_blocker(
    team: &domain::team::Team,
    roster: &[&domain::player::Player],
    current_date: chrono::NaiveDate,
) -> Option<serde_json::Value> {
    let at_risk_wages: u32 = roster
        .iter()
        .copied()
        .filter(|player| {
            contract_warning_stage(player.contract_end.as_deref(), current_date).is_some()
        })
        .map(|player| player.wage)
        .sum();

    let wage_budget = team.wage_budget.max(0) as u32;
    (wage_budget > 0 && at_risk_wages > wage_budget).then(|| {
        build_blocker(
            "contract_wage_risk",
            "warn",
            format!(
                "{} of wages are tied to at-risk contracts — review your wage budget",
                at_risk_wages
            ),
            "Finances",
        )
    })
}

fn minimum_main_roster_blocker(roster: &[&domain::player::Player]) -> Option<serde_json::Value> {
    (roster.len() < 5).then(|| {
        build_blocker(
            "main_roster_minimum",
            "warn",
            format!(
                "Plantel principal incompleto: tenes {} jugador(es). Necesitas al menos 5 para avanzar.",
                roster.len()
            ),
            "Squad",
        )
    })
}

fn main_role_coverage_blocker(roster: &[&domain::player::Player]) -> Option<serde_json::Value> {
    let role_set: std::collections::HashSet<&'static str> = roster
        .iter()
        .map(|player| lol_role_for_position(&player.natural_position))
        .collect();
    let required_roles = ["TOP", "JUNGLE", "MID", "ADC", "SUPPORT"];
    let missing_roles: Vec<&str> = required_roles
        .iter()
        .copied()
        .filter(|role| !role_set.contains(role))
        .collect();

    (!missing_roles.is_empty()).then(|| {
        build_blocker(
            "main_missing_roles",
            "warn",
            format!(
                "Plantel principal incompleto por roles: falta al menos un jugador en {}. No podes avanzar hasta cubrir TOP, JUNGLE, MID, ADC y SUPPORT.",
                missing_roles.join(", ")
            ),
            "Squad",
        )
    })
}

fn lol_role_for_position(position: &domain::player::Position) -> &'static str {
    use domain::player::Position;
    match position {
        Position::Defender
        | Position::RightBack
        | Position::CenterBack
        | Position::LeftBack
        | Position::RightWingBack
        | Position::LeftWingBack => "TOP",
        Position::AttackingMidfielder | Position::RightMidfielder | Position::LeftMidfielder => {
            "MID"
        }
        Position::Forward | Position::RightWinger | Position::LeftWinger | Position::Striker => {
            "ADC"
        }
        Position::Goalkeeper | Position::DefensiveMidfielder => "SUPPORT",
        Position::Midfielder | Position::CentralMidfielder => "JUNGLE",
    }
}

fn academy_role_coverage_blocker(
    game: &Game,
    team: &domain::team::Team,
) -> Option<serde_json::Value> {
    let academy_team_id = team.academy_team_id.clone().or_else(|| {
        game.teams
            .iter()
            .find(|candidate| {
                candidate.team_kind == domain::team::TeamKind::Academy
                    && candidate.parent_team_id.as_deref() == Some(team.id.as_str())
            })
            .map(|academy| academy.id.clone())
    })?;

    let role_set: std::collections::HashSet<&'static str> = game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(academy_team_id.as_str()))
        .map(|player| lol_role_for_position(&player.natural_position))
        .collect();
    let required_roles = ["TOP", "JUNGLE", "MID", "ADC", "SUPPORT"];
    let missing_roles: Vec<&str> = required_roles
        .iter()
        .copied()
        .filter(|role| !role_set.contains(role))
        .collect();

    (!missing_roles.is_empty()).then(|| {
        build_blocker(
            "academy_missing_roles",
            "warn",
            format!(
                "Academia incompleta: falta al menos un jugador en {}. No podes avanzar hasta cubrir todos los roles.",
                missing_roles.join(", ")
            ),
            "YouthAcademy",
        )
    })
}

pub fn compute_blocking_actions(game: &Game) -> Vec<serde_json::Value> {
    let mut blockers = Vec::new();
    let (team, roster) = match user_team_context(game) {
        Some(context) => context,
        None => {
            info!("[cmd] compute_blocking_actions: no user team context");
            return blockers;
        }
    };
    let saved_xi_ids = &team.starting_xi_ids;
    let current_date = game.clock.current_date.date_naive();
    let effective_healthy_xi_ids =
        build_effective_healthy_starting_xi_ids(saved_xi_ids, &roster, &team.formation);

    if let Some(blocker) = injured_starting_xi_blocker(saved_xi_ids, &roster) {
        blockers.push(blocker);
    }

    if let Some(blocker) = incomplete_starting_xi_blocker(&effective_healthy_xi_ids, &roster) {
        blockers.push(blocker);
    }

    if let Some(blocker) = minimum_main_roster_blocker(&roster) {
        blockers.push(blocker);
    }

    if let Some(blocker) = main_role_coverage_blocker(&roster) {
        blockers.push(blocker);
    }

    if let Some(blocker) =
        key_contract_risk_blocker(&roster, &effective_healthy_xi_ids, current_date)
    {
        blockers.push(blocker);
    }

    if let Some(blocker) = contract_wage_risk_blocker(team, &roster, current_date) {
        blockers.push(blocker);
    }

    if let Some(blocker) = academy_role_coverage_blocker(game, team) {
        blockers.push(blocker);
    }

    if let Some(blocker) = urgent_unread_messages_blocker(game) {
        blockers.push(blocker);
    }

    let blocker_ids: Vec<String> = blockers
        .iter()
        .filter_map(|blocker| blocker.get("id").and_then(|id| id.as_str()))
        .map(|id| id.to_string())
        .collect();

    info!(
        "[cmd] compute_blocking_actions: date={}, team={}, roster={}, xi={}, blockers={:?}",
        game.clock.current_date.format("%Y-%m-%d"),
        team.id,
        roster.len(),
        effective_healthy_xi_ids.len(),
        blocker_ids
    );

    blockers
}
