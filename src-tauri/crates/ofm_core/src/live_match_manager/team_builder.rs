use crate::game::Game;
use crate::potential::calculate_lol_ovr;
use domain::player::Position as DomainPosition;
use engine::{PlayStyle, PlayerData, Position, TeamData};

// ---------------------------------------------------------------------------
// Domain → Engine conversion (LoL: 5 titulares + banca)
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
    let mut ordered_players = available_players;
    ordered_players.sort_by(|left, right| {
        calculate_lol_ovr(right)
            .cmp(&calculate_lol_ovr(left))
            .then_with(|| right.condition.cmp(&left.condition))
    });

    let mut starters = ordered_players;
    let bench_domain = if starters.len() > 5 {
        starters.split_off(5)
    } else {
        Vec::new()
    };

    // Keep LoL lane order stable for draft/pre-match UIs.
    // Selection stays top-5 by OVR+condition; this only reorders those five.
    starters.sort_by(|left, right| {
        lol_role_rank(&left.natural_position)
            .cmp(&lol_role_rank(&right.natural_position))
            .then_with(|| calculate_lol_ovr(right).cmp(&calculate_lol_ovr(left)))
            .then_with(|| right.condition.cmp(&left.condition))
    });

    let starting_xi = starters.into_iter().map(to_engine_player).collect::<Vec<_>>();
    let bench = bench_domain
        .into_iter()
        .map(to_engine_player)
        .collect::<Vec<_>>();

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
        lol_role: Some(map_position_to_lol_role(&p.natural_position).to_string()),
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

fn map_position_to_lol_role(position: &DomainPosition) -> &'static str {
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

fn lol_role_rank(position: &DomainPosition) -> u8 {
    match map_position_to_lol_role(position) {
        "TOP" => 0,
        "JUNGLE" => 1,
        "MID" => 2,
        "ADC" => 3,
        "SUPPORT" => 4,
        _ => 5,
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
