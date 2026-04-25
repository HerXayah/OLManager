use chrono::{Datelike, TimeZone};
use domain::player::Player;
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;
use tauri::State;

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
        cwd.join("src-tauri")
            .join("databases")
            .join("lec_world.json"),
        cwd.join("databases").join("lec_world.json"),
    ];

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err("Default LEC world database not found (lec_world.json).".to_string())
}

#[derive(Clone, Copy)]
struct LolSeedRatings {
    mechanics: u8,
    laning: u8,
    teamfighting: u8,
    macro_play: u8,
    consistency: u8,
    shotcalling: u8,
    champion_pool: u8,
    discipline: u8,
    mental_resilience: u8,
}

#[derive(Debug, Deserialize)]
struct DraftSeedRoot {
    data: DraftSeedData,
}

#[derive(Debug, Deserialize)]
struct DraftSeedData {
    rostered_seeds: Vec<DraftPlayerSeed>,
}

#[derive(Debug, Deserialize)]
struct DraftPlayerSeed {
    ign: String,
    potential: u8,
}

fn draft_potential_map() -> &'static HashMap<String, u8> {
    static POTENTIALS: OnceLock<HashMap<String, u8>> = OnceLock::new();
    POTENTIALS.get_or_init(|| {
        let content = include_str!("../../../data/lec/draft/players.json");
        let parsed: DraftSeedRoot = serde_json::from_str(content).unwrap_or(DraftSeedRoot {
            data: DraftSeedData {
                rostered_seeds: vec![],
            },
        });

        parsed
            .data
            .rostered_seeds
            .into_iter()
            .map(|seed| (normalize_seed_name(&seed.ign), seed.potential))
            .collect()
    })
}

fn potential_seed_for_player(match_name: &str) -> Option<u8> {
    let key = normalize_seed_name(match_name);
    draft_potential_map().get(&key).copied().or_else(
        || {
            if key == "kyeahoo" {
                Some(89)
            } else {
                None
            }
        },
    )
}

fn normalize_seed_name(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

fn lol_ratings_seed_for_player(match_name: &str) -> Option<LolSeedRatings> {
    let key = normalize_seed_name(match_name);
    let ratings = match key.as_str() {
        "myrwn" => LolSeedRatings {
            mechanics: 86,
            laning: 85,
            teamfighting: 85,
            macro_play: 80,
            consistency: 75,
            shotcalling: 75,
            champion_pool: 90,
            discipline: 78,
            mental_resilience: 85,
        },
        "elyoya" => LolSeedRatings {
            mechanics: 87,
            laning: 90,
            teamfighting: 85,
            macro_play: 87,
            consistency: 85,
            shotcalling: 90,
            champion_pool: 84,
            discipline: 88,
            mental_resilience: 90,
        },
        "jojopyun" => LolSeedRatings {
            mechanics: 91,
            laning: 93,
            teamfighting: 88,
            macro_play: 87,
            consistency: 82,
            shotcalling: 86,
            champion_pool: 82,
            discipline: 80,
            mental_resilience: 82,
        },
        "supa" => LolSeedRatings {
            mechanics: 84,
            laning: 82,
            teamfighting: 82,
            macro_play: 80,
            consistency: 82,
            shotcalling: 74,
            champion_pool: 79,
            discipline: 82,
            mental_resilience: 83,
        },
        "alvaro" => LolSeedRatings {
            mechanics: 85,
            laning: 84,
            teamfighting: 88,
            macro_play: 85,
            consistency: 80,
            shotcalling: 84,
            champion_pool: 80,
            discipline: 84,
            mental_resilience: 83,
        },
        "brokenblade" => LolSeedRatings {
            mechanics: 84,
            laning: 86,
            teamfighting: 84,
            macro_play: 86,
            consistency: 80,
            shotcalling: 84,
            champion_pool: 88,
            discipline: 83,
            mental_resilience: 85,
        },
        "skewmond" => LolSeedRatings {
            mechanics: 87,
            laning: 88,
            teamfighting: 86,
            macro_play: 84,
            consistency: 86,
            shotcalling: 82,
            champion_pool: 83,
            discipline: 86,
            mental_resilience: 88,
        },
        "caps" => LolSeedRatings {
            mechanics: 85,
            laning: 85,
            teamfighting: 88,
            macro_play: 93,
            consistency: 90,
            shotcalling: 93,
            champion_pool: 86,
            discipline: 90,
            mental_resilience: 90,
        },
        "hanssama" => LolSeedRatings {
            mechanics: 84,
            laning: 83,
            teamfighting: 84,
            macro_play: 84,
            consistency: 83,
            shotcalling: 82,
            champion_pool: 78,
            discipline: 82,
            mental_resilience: 82,
        },
        "labrov" => LolSeedRatings {
            mechanics: 83,
            laning: 84,
            teamfighting: 83,
            macro_play: 78,
            consistency: 78,
            shotcalling: 75,
            champion_pool: 80,
            discipline: 82,
            mental_resilience: 82,
        },
        "canna" => LolSeedRatings {
            mechanics: 84,
            laning: 88,
            teamfighting: 86,
            macro_play: 88,
            consistency: 88,
            shotcalling: 82,
            champion_pool: 85,
            discipline: 88,
            mental_resilience: 86,
        },
        "yike" => LolSeedRatings {
            mechanics: 89,
            laning: 86,
            teamfighting: 87,
            macro_play: 83,
            consistency: 84,
            shotcalling: 84,
            champion_pool: 85,
            discipline: 84,
            mental_resilience: 86,
        },
        "kyeahoo" => LolSeedRatings {
            mechanics: 84,
            laning: 83,
            teamfighting: 82,
            macro_play: 80,
            consistency: 81,
            shotcalling: 79,
            champion_pool: 84,
            discipline: 80,
            mental_resilience: 81,
        },
        "caliste" => LolSeedRatings {
            mechanics: 87,
            laning: 85,
            teamfighting: 84,
            macro_play: 80,
            consistency: 84,
            shotcalling: 76,
            champion_pool: 80,
            discipline: 84,
            mental_resilience: 86,
        },
        "busio" => LolSeedRatings {
            mechanics: 88,
            laning: 89,
            teamfighting: 85,
            macro_play: 84,
            consistency: 84,
            shotcalling: 85,
            champion_pool: 84,
            discipline: 83,
            mental_resilience: 84,
        },
        "naaknako" => LolSeedRatings {
            mechanics: 90,
            laning: 88,
            teamfighting: 86,
            macro_play: 85,
            consistency: 84,
            shotcalling: 82,
            champion_pool: 86,
            discipline: 84,
            mental_resilience: 85,
        },
        "lyncas" => LolSeedRatings {
            mechanics: 84,
            laning: 82,
            teamfighting: 84,
            macro_play: 82,
            consistency: 80,
            shotcalling: 84,
            champion_pool: 78,
            discipline: 82,
            mental_resilience: 83,
        },
        "humanoid" => LolSeedRatings {
            mechanics: 88,
            laning: 86,
            teamfighting: 86,
            macro_play: 85,
            consistency: 75,
            shotcalling: 85,
            champion_pool: 85,
            discipline: 80,
            mental_resilience: 79,
        },
        "carzzy" => LolSeedRatings {
            mechanics: 83,
            laning: 84,
            teamfighting: 83,
            macro_play: 81,
            consistency: 76,
            shotcalling: 80,
            champion_pool: 81,
            discipline: 77,
            mental_resilience: 76,
        },
        "fleshy" => LolSeedRatings {
            mechanics: 84,
            laning: 83,
            teamfighting: 82,
            macro_play: 78,
            consistency: 78,
            shotcalling: 80,
            champion_pool: 80,
            discipline: 80,
            mental_resilience: 77,
        },
        "lot" => LolSeedRatings {
            mechanics: 82,
            laning: 80,
            teamfighting: 79,
            macro_play: 76,
            consistency: 75,
            shotcalling: 78,
            champion_pool: 76,
            discipline: 78,
            mental_resilience: 75,
        },
        "isma" => LolSeedRatings {
            mechanics: 79,
            laning: 77,
            teamfighting: 78,
            macro_play: 79,
            consistency: 78,
            shotcalling: 80,
            champion_pool: 76,
            discipline: 82,
            mental_resilience: 80,
        },
        "jackies" => LolSeedRatings {
            mechanics: 84,
            laning: 82,
            teamfighting: 83,
            macro_play: 76,
            consistency: 75,
            shotcalling: 79,
            champion_pool: 77,
            discipline: 78,
            mental_resilience: 80,
        },
        "noah" => LolSeedRatings {
            mechanics: 85,
            laning: 85,
            teamfighting: 83,
            macro_play: 81,
            consistency: 80,
            shotcalling: 77,
            champion_pool: 80,
            discipline: 83,
            mental_resilience: 76,
        },
        "jun" => LolSeedRatings {
            mechanics: 85,
            laning: 86,
            teamfighting: 85,
            macro_play: 84,
            consistency: 82,
            shotcalling: 80,
            champion_pool: 80,
            discipline: 82,
            mental_resilience: 82,
        },
        "maynter" => LolSeedRatings {
            mechanics: 76,
            laning: 81,
            teamfighting: 78,
            macro_play: 77,
            consistency: 82,
            shotcalling: 76,
            champion_pool: 75,
            discipline: 82,
            mental_resilience: 78,
        },
        "rhilech" => LolSeedRatings {
            mechanics: 85,
            laning: 81,
            teamfighting: 84,
            macro_play: 80,
            consistency: 80,
            shotcalling: 82,
            champion_pool: 79,
            discipline: 80,
            mental_resilience: 84,
        },
        "poby" => LolSeedRatings {
            mechanics: 80,
            laning: 81,
            teamfighting: 80,
            macro_play: 82,
            consistency: 84,
            shotcalling: 78,
            champion_pool: 80,
            discipline: 80,
            mental_resilience: 77,
        },
        "samd" => LolSeedRatings {
            mechanics: 81,
            laning: 78,
            teamfighting: 82,
            macro_play: 76,
            consistency: 80,
            shotcalling: 78,
            champion_pool: 80,
            discipline: 81,
            mental_resilience: 76,
        },
        "parus" => LolSeedRatings {
            mechanics: 82,
            laning: 84,
            teamfighting: 82,
            macro_play: 85,
            consistency: 81,
            shotcalling: 84,
            champion_pool: 82,
            discipline: 81,
            mental_resilience: 82,
        },
        "empyros" => LolSeedRatings {
            mechanics: 74,
            laning: 73,
            teamfighting: 77,
            macro_play: 75,
            consistency: 78,
            shotcalling: 74,
            champion_pool: 78,
            discipline: 79,
            mental_resilience: 76,
        },
        "razork" => LolSeedRatings {
            mechanics: 88,
            laning: 83,
            teamfighting: 82,
            macro_play: 80,
            consistency: 78,
            shotcalling: 82,
            champion_pool: 83,
            discipline: 82,
            mental_resilience: 84,
        },
        "vladi" => LolSeedRatings {
            mechanics: 82,
            laning: 79,
            teamfighting: 80,
            macro_play: 79,
            consistency: 76,
            shotcalling: 80,
            champion_pool: 77,
            discipline: 75,
            mental_resilience: 76,
        },
        "upset" => LolSeedRatings {
            mechanics: 85,
            laning: 84,
            teamfighting: 80,
            macro_play: 81,
            consistency: 82,
            shotcalling: 79,
            champion_pool: 76,
            discipline: 82,
            mental_resilience: 80,
        },
        "lospa" => LolSeedRatings {
            mechanics: 83,
            laning: 84,
            teamfighting: 80,
            macro_play: 82,
            consistency: 78,
            shotcalling: 75,
            champion_pool: 77,
            discipline: 78,
            mental_resilience: 80,
        },
        "wunder" => LolSeedRatings {
            mechanics: 75,
            laning: 76,
            teamfighting: 78,
            macro_play: 76,
            consistency: 74,
            shotcalling: 80,
            champion_pool: 83,
            discipline: 72,
            mental_resilience: 73,
        },
        "skeanz" => LolSeedRatings {
            mechanics: 74,
            laning: 72,
            teamfighting: 72,
            macro_play: 73,
            consistency: 76,
            shotcalling: 75,
            champion_pool: 76,
            discipline: 78,
            mental_resilience: 75,
        },
        "lider" => LolSeedRatings {
            mechanics: 80,
            laning: 78,
            teamfighting: 78,
            macro_play: 72,
            consistency: 69,
            shotcalling: 74,
            champion_pool: 68,
            discipline: 70,
            mental_resilience: 72,
        },
        "jopa" => LolSeedRatings {
            mechanics: 82,
            laning: 80,
            teamfighting: 82,
            macro_play: 76,
            consistency: 80,
            shotcalling: 77,
            champion_pool: 78,
            discipline: 80,
            mental_resilience: 82,
        },
        "mikyx" => LolSeedRatings {
            mechanics: 78,
            laning: 79,
            teamfighting: 78,
            macro_play: 83,
            consistency: 77,
            shotcalling: 84,
            champion_pool: 82,
            discipline: 78,
            mental_resilience: 77,
        },
        "rooster" => LolSeedRatings {
            mechanics: 80,
            laning: 82,
            teamfighting: 76,
            macro_play: 72,
            consistency: 75,
            shotcalling: 67,
            champion_pool: 72,
            discipline: 78,
            mental_resilience: 78,
        },
        "boukada" => LolSeedRatings {
            mechanics: 72,
            laning: 69,
            teamfighting: 72,
            macro_play: 67,
            consistency: 71,
            shotcalling: 70,
            champion_pool: 68,
            discipline: 70,
            mental_resilience: 71,
        },
        "nuc" => LolSeedRatings {
            mechanics: 79,
            laning: 80,
            teamfighting: 80,
            macro_play: 81,
            consistency: 80,
            shotcalling: 80,
            champion_pool: 76,
            discipline: 78,
            mental_resilience: 77,
        },
        "paduck" => LolSeedRatings {
            mechanics: 80,
            laning: 78,
            teamfighting: 78,
            macro_play: 73,
            consistency: 76,
            shotcalling: 68,
            champion_pool: 70,
            discipline: 78,
            mental_resilience: 77,
        },
        "trymbi" => LolSeedRatings {
            mechanics: 72,
            laning: 72,
            teamfighting: 76,
            macro_play: 74,
            consistency: 75,
            shotcalling: 77,
            champion_pool: 78,
            discipline: 75,
            mental_resilience: 73,
        },
        "tracyn" => LolSeedRatings {
            mechanics: 80,
            laning: 76,
            teamfighting: 74,
            macro_play: 74,
            consistency: 76,
            shotcalling: 77,
            champion_pool: 72,
            discipline: 80,
            mental_resilience: 80,
        },
        "daglas" => LolSeedRatings {
            mechanics: 76,
            laning: 71,
            teamfighting: 73,
            macro_play: 70,
            consistency: 73,
            shotcalling: 72,
            champion_pool: 73,
            discipline: 78,
            mental_resilience: 76,
        },
        "serin" => LolSeedRatings {
            mechanics: 77,
            laning: 80,
            teamfighting: 76,
            macro_play: 75,
            consistency: 78,
            shotcalling: 75,
            champion_pool: 75,
            discipline: 78,
            mental_resilience: 75,
        },
        "ice" => LolSeedRatings {
            mechanics: 84,
            laning: 80,
            teamfighting: 80,
            macro_play: 80,
            consistency: 82,
            shotcalling: 72,
            champion_pool: 78,
            discipline: 80,
            mental_resilience: 80,
        },
        "way" => LolSeedRatings {
            mechanics: 70,
            laning: 72,
            teamfighting: 73,
            macro_play: 78,
            consistency: 74,
            shotcalling: 68,
            champion_pool: 74,
            discipline: 75,
            mental_resilience: 78,
        },
        _ => return None,
    };

    Some(ratings)
}

fn apply_lol_seed_ratings(players: &mut [Player]) {
    for player in players.iter_mut() {
        let Some(seed) = lol_ratings_seed_for_player(&player.match_name) else {
            continue;
        };

        // Keep legacy schema compatibility but use a strict 1:1 mapping to LoL stats.
        // These are now treated as the source for LoL profile/training progression.
        player.attributes.dribbling = seed.mechanics;
        player.attributes.shooting = seed.laning;
        player.attributes.teamwork = seed.teamfighting;
        player.attributes.vision = seed.macro_play;
        player.attributes.decisions = seed.consistency;
        player.attributes.leadership = seed.shotcalling;
        player.attributes.agility = seed.champion_pool;
        player.attributes.composure = seed.discipline;
        player.attributes.stamina = seed.mental_resilience;

        if let Some(potential_base) = potential_seed_for_player(&player.match_name) {
            player.potential_base = potential_base.min(99);
        }
        player.potential_revealed = None;
        player.potential_research_started_on = None;
        player.potential_research_eta_days = None;
    }
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
    let (teams, mut players, staff) = if world_source == "random" {
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

    apply_lol_seed_ratings(&mut players);

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
    ofm_core::champions::bootstrap_champion_state(&mut game);
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
    apply_lol_seed_ratings(&mut game.players);
    ofm_core::champions::bootstrap_champion_state(&mut game);
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
    let mut game = state
        .get_game(|g: &Game| g.clone())
        .ok_or("No active game session".to_string())?;
    ofm_core::champions::bootstrap_champion_state(&mut game);
    state.set_game(game.clone());
    Ok(game)
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
