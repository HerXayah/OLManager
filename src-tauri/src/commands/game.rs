use log::info;
use serde::Serialize;
use tauri::State;
use chrono::{Datelike, TimeZone};

use db::save_index::SaveEntry;
use domain::manager::Manager;
use domain::stats::StatsState;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::state::StateManager;

use crate::SaveManagerState;

#[derive(Debug, Clone, Serialize)]
pub struct TeamSelectionData {
    pub manager: Manager,
    pub teams: Vec<domain::team::Team>,
    pub players: Vec<domain::player::Player>,
}

fn resolve_default_world_path() -> Result<std::path::PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("Failed to read current dir: {}", e))?;
    let candidates = [
        cwd.join("src-tauri").join("databases").join("lec_world.json"),
        cwd.join("databases").join("lec_world.json"),
    ];

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err("Default LEC world database not found (lec_world.json).".to_string())
}

/// Step 1: Create manager + generate world. No team assigned yet.
/// Returns the Game object so the frontend can show team selection.
/// world_source: "random" (default) or a file path to a JSON world database.
#[tauri::command]
pub async fn start_new_game(
    state: State<'_, StateManager>,
    nickname: Option<String>,
    first_name: String,
    last_name: String,
    dob: String,
    nationality: String,
    world_source: Option<String>,
) -> Result<String, String> {
    info!(
        "[cmd] start_new_game: {} {} (nickname={:?}, nationality={}, world_source={:?})",
        first_name, last_name, nickname, nationality, world_source
    );
    // Validate inputs
    let first_name = first_name.trim().to_string();
    let last_name = last_name.trim().to_string();
    let nickname = nickname.unwrap_or_default().trim().to_string();
    if first_name.is_empty() || last_name.is_empty() {
        return Err("First name and last name are required.".to_string());
    }
    if first_name.len() > 30 || last_name.len() > 30 {
        return Err("First name and last name must not exceed 30 characters.".to_string());
    }
    if nickname.len() > 20 {
        return Err("Nickname must not exceed 20 characters.".to_string());
    }
    let nationality = nationality.trim().to_string();
    if nationality.is_empty() {
        return Err("Nationality is required.".to_string());
    }

    // Validate DOB: must be a valid date and within a sensible range
    let birth_date = chrono::NaiveDate::parse_from_str(&dob, "%Y-%m-%d")
        .map_err(|_| "Invalid date of birth. Use YYYY-MM-DD format.".to_string())?;
    let today = chrono::Utc::now().date_naive();
    let age = today.signed_duration_since(birth_date).num_days() / 365;
    if age > 99 {
        return Err("Invalid date of birth.".to_string());
    }

    let mut manager = Manager::new(
        "mgr_user".to_string(),
        first_name,
        last_name,
        dob,
        nationality,
    );
    manager.nickname = nickname;

    use chrono::TimeZone;
    let start_date = chrono::Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let clock = GameClock::new(start_date);

    // Load world based on source
    let world_source = world_source.unwrap_or_else(|| "lec-default".to_string());
    let (teams, players, staff) = if world_source == "random" {
        ofm_core::generator::generate_world(None)
    } else if world_source == "lec-default" {
        let path = resolve_default_world_path()?;
        let json = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read default LEC world database: {}", e))?;
        let world = ofm_core::generator::load_world_from_json(&json)?;
        (world.teams, world.players, world.staff)
    } else {
        // Try to load from file path (strip "file:" prefix if present)
        let path = world_source.strip_prefix("file:").unwrap_or(&world_source);
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read world database: {}", e))?;
        let world = ofm_core::generator::load_world_from_json(&json)?;
        (world.teams, world.players, world.staff)
    };

    let new_game = Game::new(clock, manager, teams, players, staff, vec![]);

    info!(
        "[cmd] start_new_game: world generated with {} teams, {} players, {} staff",
        new_game.teams.len(),
        new_game.players.len(),
        new_game.staff.len()
    );
    info!("[cmd] start_new_game: storing game in state");
    state.set_game(new_game);
    state.set_stats_state(StatsState::default());
    info!("[cmd] start_new_game: completed");
    Ok("ok".to_string())
}

/// Step 2: User picks a team. Assigns manager, generates welcome message, saves to DB.
#[tauri::command]
pub async fn select_team(
    state: State<'_, StateManager>,
    sm_state: State<'_, SaveManagerState>,
    team_id: String,
) -> Result<Game, String> {
    info!("[cmd] select_team: team_id={}", team_id);
    let mut game = state
        .get_game(|g: &Game| g.clone())
        .ok_or("No active game session".to_string())?;

    // Validate team exists
    let team = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .ok_or("Team not found".to_string())?;
    let team_name = team.name.clone();

    // Assign manager to team
    game.manager.hire(team_id.clone());
    if let Some(t) = game.teams.iter_mut().find(|t| t.id == team_id) {
        t.manager_id = Some(game.manager.id.clone());
    }

    // Generate Winter schedule (LEC):
    // - Regular season: single round-robin (9 matchdays with 10 teams)
    // - Superweeks: Sat/Sun/Mon blocks (3 rounds per superweek)
    //
    // Reference windows:
    // Winter 2025: 2025-01-18 → 2025-03-02
    // Spring 2025: 2025-03-29 → 2025-06-08
    // Summer 2025: 2025-08-02 → 2025-09-28
    let season_year = game.clock.current_date.year();
    let season_start = chrono::Utc
        .with_ymd_and_hms(season_year, 1, 18, 0, 0, 0)
        .unwrap();
    // 9 rounds in 3 superweeks (Sat/Sun/Mon, then +7 days)
    let winter_round_offsets: [i64; 9] = [0, 1, 2, 7, 8, 9, 14, 15, 16];
    let team_ids: Vec<String> = game.teams.iter().map(|t| t.id.clone()).collect();
    let mut league = ofm_core::schedule::generate_single_round_league_with_offsets(
        "LEC Winter",
        season_year as u32,
        &team_ids,
        season_start,
        Some(&winter_round_offsets),
    );

    // IMPORTANT: playoffs are generated later from real standings.
    // Do not pre-seed playoff fixtures at game start (would leak teams before matches are played).

    let opponents: Vec<String> = team_ids
        .iter()
        .filter(|candidate_team_id| candidate_team_id.as_str() != team_id)
        .cloned()
        .collect();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let mut friendlies =
        ofm_core::schedule::generate_preseason_friendlies(&team_id, &opponents, season_start, 3);
    // Avoid scheduling preseason fixtures in the past relative to game start.
    friendlies.retain(|fixture| fixture.date >= today);
    ofm_core::schedule::append_fixtures(&mut league, friendlies);
    game.league = Some(league);
    ofm_core::season_context::refresh_game_context(&mut game);

    // Rich templated messages
    let date_str = game.clock.current_date.to_rfc3339();
    let welcome_msg = ofm_core::messages::welcome_message(&team_name, &team_id, &date_str);
    game.messages.push(welcome_msg);

    let season_msg = ofm_core::messages::season_schedule_message(
        "LEC Winter",
        &season_start.format("%B %d, %Y").to_string(),
        &date_str,
    );
    game.messages.push(season_msg);

    let team_names: Vec<String> = game.teams.iter().map(|team| team.name.clone()).collect();
    game.news.push(ofm_core::news::season_preview_article(
        &team_names,
        &date_str,
    ));

    let staff_msg = ofm_core::messages::staff_advice_message(&team_name, &team_id, &date_str);
    game.messages.push(staff_msg);

    ofm_core::player_events::generate_contract_concern_messages(&mut game, false);

    // Save to new per-save DB
    let manager_name = game.manager.display_name();
    let save_name = format!("{}'s Career", manager_name);

    let mut sm = sm_state
        .0
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    let save_id = sm.create_save(&game, &save_name)?;
    state.set_save_id(save_id);

    state.set_game(game.clone());
    state.set_stats_state(StatsState::default());
    Ok(game)
}

#[tauri::command]
pub async fn get_saves(sm_state: State<'_, SaveManagerState>) -> Result<Vec<SaveEntry>, String> {
    log::debug!("[cmd] get_saves");
    let sm = sm_state
        .0
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    Ok(sm.list_saves().to_vec())
}

#[tauri::command]
pub async fn delete_save(
    sm_state: State<'_, SaveManagerState>,
    save_id: String,
) -> Result<bool, String> {
    info!("[cmd] delete_save: save_id={}", save_id);
    let mut sm = sm_state
        .0
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    sm.delete_save(&save_id)
}

#[tauri::command]
pub async fn load_game(
    state: State<'_, StateManager>,
    sm_state: State<'_, SaveManagerState>,
    save_id: String,
) -> Result<String, String> {
    info!("[cmd] load_game: save_id={}", save_id);
    let mut sm = sm_state
        .0
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    let mut game = sm.load_game(&save_id)?;
    let stats_state = sm.load_stats_state(&save_id)?;
    ofm_core::season_context::refresh_game_context(&mut game);

    let mgr_name = game.manager.display_name();

    state.set_save_id(save_id);
    state.set_game(game);
    state.set_stats_state(stats_state);
    Ok(mgr_name)
}

#[tauri::command]
pub async fn get_active_game(state: State<'_, StateManager>) -> Result<Game, String> {
    log::debug!("[cmd] get_active_game");
    state
        .get_game(|g: &Game| g.clone())
        .ok_or("No active game session".to_string())
}

#[tauri::command]
pub async fn get_team_selection_data(
    state: State<'_, StateManager>,
) -> Result<TeamSelectionData, String> {
    log::debug!("[cmd] get_team_selection_data");
    state
        .get_game(|game| TeamSelectionData {
            manager: game.manager.clone(),
            teams: game.teams.clone(),
            players: game.players.clone(),
        })
        .ok_or("No active game session".to_string())
}

#[tauri::command]
pub async fn save_game(
    state: State<'_, StateManager>,
    sm_state: State<'_, SaveManagerState>,
) -> Result<(), String> {
    info!("[cmd] save_game");
    let game = state
        .get_game(|g: &Game| g.clone())
        .ok_or("No active game session".to_string())?;

    let save_id = state
        .get_save_id()
        .ok_or("No active save session".to_string())?;

    let mut sm = sm_state
        .0
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    sm.save_game(&game, &save_id)?;
    let stats_state = state
        .get_stats_state(|stats| stats.clone())
        .unwrap_or_default();
    sm.save_stats_state(&stats_state, &save_id)
}

/// Save the current game and clear the active session so the player returns to the main menu.
#[tauri::command]
pub async fn exit_to_menu(
    state: State<'_, StateManager>,
    sm_state: State<'_, SaveManagerState>,
) -> Result<(), String> {
    info!("[cmd] exit_to_menu");
    let game = state
        .get_game(|g: &Game| g.clone())
        .ok_or("No active game session")?;

    // Auto-save
    if let Some(save_id) = state.get_save_id() {
        let mut sm = sm_state
            .0
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        sm.save_game(&game, &save_id)?;
        let stats_state = state
            .get_stats_state(|stats| stats.clone())
            .unwrap_or_default();
        sm.save_stats_state(&stats_state, &save_id)?;
    }

    // Clear the in-memory game state
    state.clear_game();
    state.clear_save_id();

    Ok(())
}
