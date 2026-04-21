use log::info;
use tauri::State;

use ofm_core::game::Game;
use ofm_core::state::StateManager;

#[tauri::command]
pub fn set_formation(state: State<'_, StateManager>, formation: String) -> Result<Game, String> {
    info!("[cmd] set_formation: {}", formation);
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    // Parse formation into (def, mid, fwd) counts
    let parts: Vec<usize> = formation
        .split('-')
        .filter_map(|s| s.parse().ok())
        .collect();
    let (num_def, num_mid, num_fwd) = match parts.len() {
        3 => (parts[0], parts[1], parts[2]),
        4 => (parts[0], parts[1] + parts[2], parts[3]),
        _ => (4, 4, 2),
    };

    if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
        team.formation = formation;
    }

    // Reassign positions for outfield players on this team
    let player_ids: Vec<String> = game
        .players
        .iter()
        .filter(|p| {
            p.team_id.as_deref() == Some(&team_id)
                && p.position != domain::player::Position::Goalkeeper
        })
        .map(|p| p.id.clone())
        .collect();

    // Sort by defensive ability (most defensive first)
    let mut sorted_ids = player_ids.clone();
    sorted_ids.sort_by(|a_id, b_id| {
        let pa = game.players.iter().find(|p| p.id == *a_id).unwrap();
        let pb = game.players.iter().find(|p| p.id == *b_id).unwrap();
        let def_a = pa.attributes.defending as u16
            + pa.attributes.tackling as u16
            + pa.attributes.strength as u16;
        let def_b = pb.attributes.defending as u16
            + pb.attributes.tackling as u16
            + pb.attributes.strength as u16;
        def_b.cmp(&def_a)
    });

    // Assign positions
    for (slot, pid) in sorted_ids.iter().enumerate() {
        let new_pos = if slot < num_def {
            domain::player::Position::Defender
        } else if slot < num_def + num_mid {
            domain::player::Position::Midfielder
        } else if slot < num_def + num_mid + num_fwd {
            domain::player::Position::Forward
        } else {
            continue;
        };
        if let Some(player) = game.players.iter_mut().find(|p| p.id == *pid) {
            player.position = new_pos;
        }
    }

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn set_starting_xi(
    state: State<'_, StateManager>,
    player_ids: Vec<String>,
) -> Result<Game, String> {
    info!("[cmd] set_starting_xi: {} players", player_ids.len());
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
        team.starting_xi_ids = player_ids;
    }

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn set_play_style(state: State<'_, StateManager>, play_style: String) -> Result<Game, String> {
    info!("[cmd] set_play_style: {}", play_style);
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    let style = match play_style.as_str() {
        "Attacking" => domain::team::PlayStyle::Attacking,
        "Defensive" => domain::team::PlayStyle::Defensive,
        "Possession" => domain::team::PlayStyle::Possession,
        "Counter" => domain::team::PlayStyle::Counter,
        "HighPress" => domain::team::PlayStyle::HighPress,
        _ => domain::team::PlayStyle::Balanced,
    };

    if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
        team.play_style = style;
    }

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn set_lol_tactics(
    state: State<'_, StateManager>,
    lol_tactics: domain::team::LolTactics,
) -> Result<Game, String> {
    info!("[cmd] set_lol_tactics");
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
        team.lol_tactics = lol_tactics;
    }

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn set_team_match_roles(
    state: State<'_, StateManager>,
    match_roles: domain::team::MatchRoles,
) -> Result<Game, String> {
    info!("[cmd] set_team_match_roles");
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
        team.match_roles = match_roles;
    }

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn set_training(
    state: State<'_, StateManager>,
    focus: String,
    intensity: String,
) -> Result<Game, String> {
    info!(
        "[cmd] set_training: focus={}, intensity={}",
        focus, intensity
    );
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    let training_focus = domain::team::TrainingFocus::from_id(&focus).unwrap_or_default();

    let training_intensity = match intensity.as_str() {
        "Low" => domain::team::TrainingIntensity::Low,
        "Medium" => domain::team::TrainingIntensity::Medium,
        "High" => domain::team::TrainingIntensity::High,
        _ => domain::team::TrainingIntensity::Medium,
    };

    if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
        team.training_focus = training_focus;
        team.training_intensity = training_intensity;
    }

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn set_training_schedule(
    state: State<'_, StateManager>,
    schedule: String,
) -> Result<Game, String> {
    info!("[cmd] set_training_schedule: {}", schedule);
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    let training_schedule = match schedule.as_str() {
        "Intense" => domain::team::TrainingSchedule::Intense,
        "Balanced" => domain::team::TrainingSchedule::Balanced,
        "Light" => domain::team::TrainingSchedule::Light,
        _ => domain::team::TrainingSchedule::Balanced,
    };

    if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
        team.training_schedule = training_schedule;
    }

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn set_training_groups(
    state: State<'_, StateManager>,
    groups: Vec<domain::team::TrainingGroup>,
) -> Result<Game, String> {
    info!("[cmd] set_training_groups: {} groups", groups.len());
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
        team.training_groups = groups;
    }

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn set_player_training_focus(
    state: State<'_, StateManager>,
    player_id: String,
    focus: Option<String>,
) -> Result<Game, String> {
    info!(
        "[cmd] set_player_training_focus: player={}, focus={:?}",
        player_id, focus
    );
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let training_focus = focus.and_then(|f| domain::team::TrainingFocus::from_id(&f));

    if let Some(player) = game.players.iter_mut().find(|p| p.id == player_id) {
        player.training_focus = training_focus;
    } else {
        return Err(format!("Player not found: {}", player_id));
    }

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn reroll_player_lol_role(
    state: State<'_, StateManager>,
    player_id: String,
    role: String,
) -> Result<Game, String> {
    info!(
        "[cmd] reroll_player_lol_role: player={}, role={}",
        player_id, role
    );

    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let manager_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    let (next_natural, next_position) = match role.as_str() {
        "TOP" => (
            domain::player::Position::Defender,
            domain::player::Position::Defender,
        ),
        "JUNGLE" => (
            domain::player::Position::Midfielder,
            domain::player::Position::Midfielder,
        ),
        "MID" => (
            domain::player::Position::AttackingMidfielder,
            domain::player::Position::Midfielder,
        ),
        "ADC" => (
            domain::player::Position::Forward,
            domain::player::Position::Forward,
        ),
        "SUPPORT" => (
            domain::player::Position::DefensiveMidfielder,
            domain::player::Position::Midfielder,
        ),
        _ => return Err(format!("Unknown LoL role: {}", role)),
    };

    let player = game
        .players
        .iter_mut()
        .find(|candidate| candidate.id == player_id)
        .ok_or_else(|| format!("Player not found: {}", player_id))?;

    if player.team_id.as_deref() != Some(manager_team_id.as_str()) {
        return Err("Player does not belong to manager team".to_string());
    }

    let previous_natural = player.natural_position.clone();

    if previous_natural != next_natural
        && !player
            .alternate_positions
            .iter()
            .any(|position| position == &previous_natural)
    {
        player.alternate_positions.push(previous_natural);
        if player.alternate_positions.len() > 4 {
            player.alternate_positions.truncate(4);
        }
    }

    player.natural_position = next_natural;
    player.position = next_position;

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn auto_select_set_pieces(
    state: State<'_, StateManager>,
    player_ids: Vec<String>,
) -> Result<serde_json::Value, String> {
    log::debug!("[cmd] auto_select_set_pieces: {} players", player_ids.len());
    let game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let (captain, penalty, free_kick, corner) =
        ofm_core::live_match_manager::auto_select_set_pieces(&game, &player_ids);

    Ok(serde_json::json!({
        "captain": captain,
        "penalty_taker": penalty,
        "free_kick_taker": free_kick,
        "corner_taker": corner,
    }))
}
