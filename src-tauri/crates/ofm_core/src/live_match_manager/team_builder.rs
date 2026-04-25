use crate::game::Game;
use crate::player_rating::natural_ovr;
use domain::player::Position as DomainPosition;
use engine::{PlayStyle, PlayerData, Position, TeamData};

#[derive(Clone, Copy, PartialEq, Eq)]
enum LolRole {
    Top,
    Jungla,
    Mid,
    Adc,
    Support,
}

const LOL_ROLE_ORDER: [LolRole; 5] = [
    LolRole::Top,
    LolRole::Jungla,
    LolRole::Mid,
    LolRole::Adc,
    LolRole::Support,
];

fn role_from_player(player: &domain::player::Player) -> LolRole {
    match player.natural_position {
        DomainPosition::Defender
        | DomainPosition::CenterBack
        | DomainPosition::RightBack
        | DomainPosition::LeftBack
        | DomainPosition::RightWingBack
        | DomainPosition::LeftWingBack => LolRole::Top,
        DomainPosition::Midfielder | DomainPosition::CentralMidfielder => LolRole::Jungla,
        DomainPosition::AttackingMidfielder
        | DomainPosition::RightMidfielder
        | DomainPosition::LeftMidfielder => LolRole::Mid,
        DomainPosition::Forward
        | DomainPosition::Striker
        | DomainPosition::RightWinger
        | DomainPosition::LeftWinger => LolRole::Adc,
        DomainPosition::DefensiveMidfielder | DomainPosition::Goalkeeper => LolRole::Support,
    }
}

// ---------------------------------------------------------------------------
// Domain → Engine conversion (LoL: quinteto titular, sin banca)
// ---------------------------------------------------------------------------

pub(super) fn build_team_with_bench(game: &Game, team_id: &str) -> (TeamData, Vec<PlayerData>) {
    let team = game.teams.iter().find(|t| t.id == team_id);
    let (name, formation, play_style) = match team {
        Some(t) => (
            t.name.clone(),
            t.formation.clone(),
            match t.play_style {
                domain::team::PlayStyle::Attacking => PlayStyle::Attacking,
                domain::team::PlayStyle::Defensive => PlayStyle::Defensive,
                domain::team::PlayStyle::Possession => PlayStyle::Possession,
                domain::team::PlayStyle::Counter => PlayStyle::Counter,
                domain::team::PlayStyle::HighPress => PlayStyle::HighPress,
                _ => PlayStyle::Balanced,
            },
        ),
        None => ("Unknown".into(), "4-4-2".into(), PlayStyle::Balanced),
    };

    // Collect all players for this team.
    // NOTE: For LoL/live prototype we should not apply football injury filtering,
    // otherwise rosters can drop below 5 and UI shows empty player slots.
    let available_players: Vec<&domain::player::Player> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id))
        .collect();
    let mut used_ids = std::collections::HashSet::new();
    let mut starting_xi = Vec::with_capacity(5);

    // Ensure first 5 starters are stable LoL core (Top/Jungla/Mid/ADC/Support)
    // because several UI flows (draft/profile context) rely on `players[0..5]`.
    let saved_starters = team.map(|t| t.starting_xi_ids.clone()).unwrap_or_default();
    for starter_id in saved_starters.iter().take(5) {
        if let Some(player) = available_players
            .iter()
            .copied()
            .find(|player| player.id == *starter_id)
        {
            if used_ids.insert(player.id.clone()) {
                starting_xi.push(to_engine_player(player));
            }
        }
    }

    if starting_xi.len() < 5 {
        for role in LOL_ROLE_ORDER {
            if starting_xi.len() >= 5 {
                break;
            }

            let candidate = available_players
                .iter()
                .copied()
                .filter(|player| !used_ids.contains(&player.id) && role_from_player(player) == role)
                .max_by(|left, right| {
                    natural_ovr(left)
                        .partial_cmp(&natural_ovr(right))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

            if let Some(player) = candidate {
                used_ids.insert(player.id.clone());
                starting_xi.push(to_engine_player(player));
            }
        }
    }

    if starting_xi.len() < 5 {
        let mut fallback = available_players
            .iter()
            .copied()
            .filter(|player| !used_ids.contains(&player.id))
            .collect::<Vec<_>>();
        fallback.sort_by(|left, right| {
            natural_ovr(right)
                .partial_cmp(&natural_ovr(left))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for player in fallback {
            if starting_xi.len() >= 5 {
                break;
            }
            used_ids.insert(player.id.clone());
            starting_xi.push(to_engine_player(player));
        }
    }
    starting_xi.truncate(5);
    let bench = Vec::new();

    let team_data = TeamData {
        id: team_id.to_string(),
        name,
        formation,
        play_style,
        players: starting_xi,
    };

    (team_data, bench)
}

fn to_engine_player(p: &domain::player::Player) -> PlayerData {
    let pos = match p.position.to_group_position() {
        DomainPosition::Goalkeeper => Position::Goalkeeper,
        DomainPosition::Defender => Position::Defender,
        DomainPosition::Midfielder => Position::Midfielder,
        DomainPosition::Forward => Position::Forward,
        _ => Position::Midfielder,
    };

    PlayerData {
        id: p.id.clone(),
        name: p.match_name.clone(),
        position: pos,
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
}

/// Auto-select set-piece takers from a set of player IDs.
/// Returns (captain_id, penalty_taker_id, free_kick_taker_id, corner_taker_id).
pub fn auto_select_set_pieces(
    game: &Game,
    player_ids: &[String],
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let players: Vec<&domain::player::Player> = player_ids
        .iter()
        .filter_map(|id| game.players.iter().find(|p| &p.id == id))
        .collect();

    if players.is_empty() {
        return (None, None, None, None);
    }

    // Captain: highest leadership + teamwork
    let captain = players
        .iter()
        .max_by_key(|p| (p.attributes.leadership as u16) + (p.attributes.teamwork as u16))
        .map(|p| p.id.clone());

    // Penalty taker: highest shooting + composure (exclude GK)
    let penalty = players
        .iter()
        .filter(|p| p.position != DomainPosition::Goalkeeper)
        .max_by_key(|p| (p.attributes.shooting as u16) + (p.attributes.composure as u16))
        .map(|p| p.id.clone());

    // Free kick taker: highest passing + vision + shooting (exclude GK)
    let free_kick = players
        .iter()
        .filter(|p| p.position != DomainPosition::Goalkeeper)
        .max_by_key(|p| {
            (p.attributes.passing as u16)
                + (p.attributes.vision as u16)
                + (p.attributes.shooting as u16) / 2
        })
        .map(|p| p.id.clone());

    // Corner taker: highest passing + vision (exclude GK, prefer different from FK)
    let corner = players
        .iter()
        .filter(|p| p.position != DomainPosition::Goalkeeper)
        .max_by_key(|p| {
            let base = (p.attributes.passing as u16) + (p.attributes.vision as u16);
            // Small penalty if same as free kick taker to encourage variety
            if free_kick.as_ref() == Some(&p.id) {
                base.saturating_sub(5)
            } else {
                base
            }
        })
        .map(|p| p.id.clone());

    (captain, penalty, free_kick, corner)
}
