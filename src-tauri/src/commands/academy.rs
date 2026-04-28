use chrono::Utc;
use domain::message::{InboxMessage, MessageCategory, MessageContext, MessagePriority};
use domain::team::{AcademyLifecycle, AcademyMetadata, ErlAssignment, Team, TeamKind};
use log::info;
use ofm_core::academy::{
    eligible_academy_acquisition_options, validate_academy_acquisition, AcademyAcquisitionOption,
    ErlAcademyCandidate, ErlLeagueDefinition,
};
use ofm_core::game::Game;
use ofm_core::state::StateManager;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::game::{
    academy_seed_team_id, ensure_example_academy_pool, example_academy_seed_catalog,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AcademyAcquisitionOptionsResponse {
    pub parent_team_id: String,
    pub acquisition_allowed: bool,
    pub blocked_reason: Option<String>,
    pub options: Vec<AcademyAcquisitionOption>,
}

pub type AcademyCreationOptionsResponse = AcademyAcquisitionOptionsResponse;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcquireAcademyTeamRequest {
    pub parent_team_id: String,
    pub source_team_id: String,
    pub custom_name: Option<String>,
    pub custom_short_name: Option<String>,
    pub custom_logo_url: Option<String>,
}

#[tauri::command]
pub fn get_academy_acquisition_options(
    state: State<'_, StateManager>,
    parent_team_id: String,
) -> Result<AcademyAcquisitionOptionsResponse, String> {
    info!(
        "[cmd] get_academy_acquisition_options: parent_team_id={}",
        parent_team_id
    );
    let mut game = state
        .get_game(|game| game.clone())
        .ok_or("No active game session".to_string())?;

    ensure_example_academy_pool(&mut game);
    state.set_game(game.clone());

    get_academy_acquisition_options_for_game(&game, &parent_team_id)
}

#[tauri::command]
pub fn acquire_academy_team(
    state: State<'_, StateManager>,
    request: AcquireAcademyTeamRequest,
) -> Result<Game, String> {
    info!(
        "[cmd] acquire_academy_team: parent_team_id={}, source_team_id={}",
        request.parent_team_id, request.source_team_id
    );
    let mut game = state
        .get_game(|game| game.clone())
        .ok_or("No active game session".to_string())?;
    ensure_example_academy_pool(&mut game);
    let updated = acquire_academy_team_in_game(&mut game, request)?;
    state.set_game(updated.clone());
    Ok(updated)
}

#[tauri::command]
pub fn promote_academy_player(
    state: State<'_, StateManager>,
    player_id: String,
) -> Result<Game, String> {
    info!("[cmd] promote_academy_player: player_id={}", player_id);
    let mut game = state
        .get_game(|game| game.clone())
        .ok_or("No active game session".to_string())?;

    let parent_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    let academy_team_id = resolve_manager_academy_team_id(&game, &parent_team_id)?;

    let (moved_player_id, moved_player_name) = {
        let player = game
            .players
            .iter_mut()
            .find(|candidate| candidate.id == player_id)
            .ok_or_else(|| format!("Player '{}' not found", player_id))?;

        if player.team_id.as_deref() != Some(academy_team_id.as_str()) {
            return Err("Player does not belong to your academy team".to_string());
        }

        player.team_id = Some(parent_team_id.clone());
        (player.id.clone(), player.match_name.clone())
    };

    push_academy_player_moved_message(
        &mut game,
        "academy-promote",
        &parent_team_id,
        &moved_player_id,
        &moved_player_name,
        "Promocion desde la academia",
        "Subiste al jugador {player} desde la academia al equipo principal.",
    );

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn demote_main_player_to_academy(
    state: State<'_, StateManager>,
    player_id: String,
) -> Result<Game, String> {
    info!(
        "[cmd] demote_main_player_to_academy: player_id={}",
        player_id
    );
    let mut game = state
        .get_game(|game| game.clone())
        .ok_or("No active game session".to_string())?;

    let parent_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    let academy_team_id = resolve_manager_academy_team_id(&game, &parent_team_id)?;

    let (moved_player_id, moved_player_name) = {
        let player = game
            .players
            .iter_mut()
            .find(|candidate| candidate.id == player_id)
            .ok_or_else(|| format!("Player '{}' not found", player_id))?;

        if player.team_id.as_deref() != Some(parent_team_id.as_str()) {
            return Err("Player does not belong to your main team".to_string());
        }

        player.team_id = Some(academy_team_id.clone());
        (player.id.clone(), player.match_name.clone())
    };

    push_academy_player_moved_message(
        &mut game,
        "academy-demote",
        &parent_team_id,
        &moved_player_id,
        &moved_player_name,
        "Jugador enviado a la academia",
        "Bajaste al jugador {player} del equipo principal a la academia.",
    );

    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn get_academy_creation_options(
    state: State<'_, StateManager>,
    parent_team_id: String,
) -> Result<AcademyCreationOptionsResponse, String> {
    info!(
        "[cmd] get_academy_creation_options: parent_team_id={}",
        parent_team_id
    );
    let game = state
        .get_game(|game| game.clone())
        .ok_or("No active game session".to_string())?;

    get_academy_acquisition_options_for_game(&game, &parent_team_id)
}

#[tauri::command]
pub fn create_academy(
    _state: State<'_, StateManager>,
    parent_team_id: String,
    erl_league_id: String,
) -> Result<Game, String> {
    info!(
        "[cmd] create_academy: parent_team_id={}, erl_league_id={}",
        parent_team_id, erl_league_id
    );
    Err(format!(
        "create_academy is deprecated; use acquire_academy_team with a source team candidate instead of ERL '{}'.",
        erl_league_id
    ))
}

pub(crate) fn get_academy_acquisition_options_for_game(
    game: &Game,
    parent_team_id: &str,
) -> Result<AcademyAcquisitionOptionsResponse, String> {
    let parent = find_team(game, parent_team_id)?;
    let normalize_key = |value: &str| {
        value
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .flat_map(|ch| ch.to_lowercase())
            .collect::<String>()
    };
    let occupied_source_ids: std::collections::HashSet<String> = game
        .teams
        .iter()
        .filter(|team| team.team_kind == TeamKind::Academy && team.parent_team_id.is_some())
        .flat_map(|team| {
            let mut ids = vec![team.id.clone()];
            if let Some(metadata) = team.academy.as_ref() {
                ids.push(metadata.source_team_id.clone());
            }
            ids
        })
        .collect();
    let taken_original_names: std::collections::HashSet<String> = game
        .teams
        .iter()
        .filter(|team| team.team_kind == TeamKind::Academy && team.parent_team_id.is_some())
        .filter_map(|team| {
            team.academy
                .as_ref()
                .map(|metadata| normalize_key(&metadata.original_name))
        })
        .collect();

    let options = eligible_academy_acquisition_options(
        &parent.country,
        academy_erl_catalog(),
        academy_candidate_catalog(),
    )
    .into_iter()
    .filter(|option| {
        !occupied_source_ids.contains(&option.source_team_id)
            && !taken_original_names.contains(&normalize_key(&option.name))
    })
    .collect::<Vec<_>>();
    let blocked_reason = if !parent.is_main() {
        Some("Academy can only be acquired for a main team".to_string())
    } else if parent.academy_team_id.is_some() {
        Some("Parent team already has academy".to_string())
    } else if options.is_empty() {
        Some("No free academy candidates available in LES, LFL, or Prime League".to_string())
    } else if options
        .iter()
        .all(|option| parent.finance < option.acquisition_cost)
    {
        Some("Insufficient funds for all eligible academy acquisition options".to_string())
    } else {
        None
    };

    Ok(AcademyAcquisitionOptionsResponse {
        parent_team_id: parent.id.clone(),
        acquisition_allowed: blocked_reason.is_none(),
        blocked_reason,
        options,
    })
}

#[allow(dead_code)]
pub(crate) fn get_academy_creation_options_for_game(
    game: &Game,
    parent_team_id: &str,
) -> Result<AcademyCreationOptionsResponse, String> {
    get_academy_acquisition_options_for_game(game, parent_team_id)
}

pub(crate) fn acquire_academy_team_in_game(
    game: &mut Game,
    request: AcquireAcademyTeamRequest,
) -> Result<Game, String> {
    let option = get_academy_acquisition_options_for_game(game, &request.parent_team_id)?
        .options
        .into_iter()
        .find(|option| option.source_team_id == request.source_team_id)
        .ok_or_else(|| {
            format!(
                "Academy candidate '{}' is not eligible for this team",
                request.source_team_id
            )
        })?;

    let parent_snapshot = find_team(game, &request.parent_team_id)?.clone();
    validate_academy_acquisition(&parent_snapshot, &option).map_err(format_academy_error)?;

    let academy_id = option.source_team_id.clone();

    let created_at = game.clock.current_date.with_timezone(&Utc).to_rfc3339();
    let metadata = academy_metadata(&option, created_at.clone(), request.custom_logo_url.clone());

    let existing_academy_index = game
        .teams
        .iter()
        .position(|team| team.id == academy_id && team.team_kind == TeamKind::Academy);
    if let Some(academy_index) = existing_academy_index {
        if game
            .teams
            .get(academy_index)
            .and_then(|academy| academy.parent_team_id.as_ref())
            .is_some()
        {
            return Err(format!("Academy team id '{}' already linked", academy_id));
        }
    }

    let parent_index = game
        .teams
        .iter()
        .position(|team| team.id == request.parent_team_id)
        .ok_or("Parent team not found".to_string())?;

    {
        let parent = game
            .teams
            .get_mut(parent_index)
            .ok_or("Parent team not found".to_string())?;
        parent.finance -= option.acquisition_cost;
        parent.season_expenses += option.acquisition_cost;
        parent.academy_team_id = Some(academy_id.clone());
    }

    if let Some(academy_index) = existing_academy_index {
        let academy = game
            .teams
            .get_mut(academy_index)
            .ok_or("Academy team not found".to_string())?;
        academy.name = request.custom_name.unwrap_or_else(|| option.name.clone());
        academy.short_name = request
            .custom_short_name
            .unwrap_or_else(|| option.short_name.clone());
        academy.parent_team_id = Some(parent_snapshot.id.clone());
        academy.academy = Some(metadata);
        academy.reputation = u32::from(option.reputation) * 100;
        academy.finance = 0;
        academy.wage_budget = 0;
        academy.transfer_budget = 0;
    } else {
        let mut academy = Team::new(
            academy_id.clone(),
            request.custom_name.unwrap_or_else(|| option.name.clone()),
            request
                .custom_short_name
                .unwrap_or_else(|| option.short_name.clone()),
            option.country_code.clone(),
            parent_snapshot.city.clone(),
            format!("{} Performance Centre", option.short_name),
            2_500,
        );
        academy.team_kind = TeamKind::Academy;
        academy.parent_team_id = Some(parent_snapshot.id.clone());
        academy.academy = Some(metadata);
        academy.finance = 0;
        academy.wage_budget = 0;
        academy.transfer_budget = 0;
        academy.reputation = u32::from(option.reputation) * 100;
        game.teams.push(academy);
    }

    push_academy_acquired_message(
        game,
        &parent_snapshot,
        &option.name,
        option.acquisition_cost,
    );
    Ok(game.clone())
}

fn push_academy_acquired_message(game: &mut Game, parent: &Team, academy_name: &str, cost: i64) {
    let date = game.clock.current_date.format("%Y-%m-%d").to_string();
    let message = InboxMessage::new(
        format!("academy-acquired-{}-{}", parent.id, academy_name.to_lowercase().replace(' ', "-")),
        format!("Academia financiada: {}", academy_name),
        format!(
            "La operacion se completo con exito. {} ahora tiene una academia vinculada ({}) por un costo de €{}.",
            parent.name, academy_name, cost
        ),
        "Direccion Deportiva".to_string(),
        date,
    )
    .with_category(MessageCategory::Finance)
    .with_priority(MessagePriority::High)
    .with_sender_role("Director Deportivo")
    .with_context(MessageContext {
        team_id: Some(parent.id.clone()),
        ..Default::default()
    });

    game.messages.push(message);
}

fn push_academy_player_moved_message(
    game: &mut Game,
    id_prefix: &str,
    parent_team_id: &str,
    player_id: &str,
    player_name: &str,
    subject: &str,
    body_template: &str,
) {
    let date = game.clock.current_date.format("%Y-%m-%d").to_string();
    let message = InboxMessage::new(
        format!("{}-{}", id_prefix, player_id),
        subject.to_string(),
        body_template.replace("{player}", player_name),
        "Staff Academia".to_string(),
        date,
    )
    .with_category(MessageCategory::Training)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("Coordinador de Academia")
    .with_context(MessageContext {
        team_id: Some(parent_team_id.to_string()),
        player_id: Some(player_id.to_string()),
        ..Default::default()
    });

    game.messages.push(message);
}

#[allow(dead_code)]
pub(crate) fn create_academy_in_game(
    _game: &mut Game,
    _parent_team_id: &str,
    erl_league_id: &str,
) -> Result<Game, String> {
    Err(format!(
        "create_academy_in_game is deprecated; use acquire_academy_team_in_game with a source team candidate instead of ERL '{}'.",
        erl_league_id
    ))
}

fn find_team<'game>(game: &'game Game, team_id: &str) -> Result<&'game Team, String> {
    game.teams
        .iter()
        .find(|team| team.id == team_id)
        .ok_or_else(|| format!("Team '{}' not found", team_id))
}

fn resolve_manager_academy_team_id(game: &Game, parent_team_id: &str) -> Result<String, String> {
    let parent_team = find_team(game, parent_team_id)?;
    if !parent_team.is_main() {
        return Err("Academy actions are only available for main teams".to_string());
    }

    if let Some(academy_team_id) = parent_team.academy_team_id.clone() {
        return Ok(academy_team_id);
    }

    game.teams
        .iter()
        .find(|team| {
            team.team_kind == TeamKind::Academy
                && team.parent_team_id.as_deref() == Some(parent_team_id)
        })
        .map(|team| team.id.clone())
        .ok_or("No academy team linked to manager team".to_string())
}

fn academy_metadata(
    option: &AcademyAcquisitionOption,
    acquired_at: String,
    current_logo_url: Option<String>,
) -> AcademyMetadata {
    AcademyMetadata {
        lifecycle: AcademyLifecycle::Active,
        erl_assignment: ErlAssignment {
            erl_league_id: option.erl_league_id.clone(),
            country_rule: option.assignment_rule.clone(),
            fallback_reason: option.fallback_reason.clone(),
            reputation: option.reputation,
            acquisition_cost: option.acquisition_cost,
            acquired_at: acquired_at.clone(),
            creation_cost: 0,
            created_at: String::new(),
        },
        source_team_id: option.source_team_id.clone(),
        original_name: option.name.clone(),
        original_short_name: option.short_name.clone(),
        original_logo_url: option.logo_url.clone(),
        current_logo_url,
        acquisition_cost: option.acquisition_cost,
        acquired_at,
    }
}

fn format_academy_error(error: ofm_core::academy::AcademyError) -> String {
    match error {
        ofm_core::academy::AcademyError::ParentMustBeMainTeam { team_id } => {
            format!("Team '{}' is not a main team", team_id)
        }
        ofm_core::academy::AcademyError::AcademyAlreadyExists {
            parent_team_id,
            academy_team_id,
        } => format!(
            "Parent team '{}' already has academy '{}'",
            parent_team_id, academy_team_id
        ),
        ofm_core::academy::AcademyError::InsufficientFunds {
            available,
            required,
        } => format!(
            "Insufficient funds for academy acquisition: available {}, required {}",
            available, required
        ),
        ofm_core::academy::AcademyError::UnrelatedAcademy {
            parent_team_id,
            academy_team_id,
        } => format!(
            "Academy '{}' is not linked to parent team '{}'",
            academy_team_id, parent_team_id
        ),
    }
}

fn academy_erl_catalog() -> &'static [ErlLeagueDefinition] {
    static CATALOG: std::sync::OnceLock<Vec<ErlLeagueDefinition>> = std::sync::OnceLock::new();
    CATALOG.get_or_init(|| {
        vec![
            erl(
                "lfl",
                "La Ligue Française",
                "FR",
                "EMEA",
                5,
                &["BE", "LU", "MC"],
            ),
            erl(
                "liga-espanola",
                "Liga Española de League of Legends",
                "ES",
                "EMEA",
                4,
                &["PT", "AD"],
            ),
            erl(
                "prime-league",
                "Prime League",
                "DE",
                "EMEA",
                5,
                &["AT", "CH"],
            ),
            erl(
                "nlc",
                "Northern League of Legends Championship",
                "GB",
                "EMEA",
                4,
                &["IE"],
            ),
            erl("ultraliga", "Ultraliga", "PL", "EMEA", 3, &["UA", "LT"]),
            erl(
                "hitpoint-masters",
                "Hitpoint Masters",
                "CZ",
                "EMEA",
                2,
                &["SK"],
            ),
            erl(
                "elite-series",
                "Elite Series",
                "BE",
                "EMEA",
                3,
                &["NL", "LU"],
            ),
            erl(
                "greek-legends-league",
                "Greek Legends League",
                "GR",
                "EMEA",
                2,
                &["CY"],
            ),
            erl(
                "lplol",
                "Liga Portuguesa de League of Legends",
                "PT",
                "EMEA",
                2,
                &["ES"],
            ),
            erl(
                "tcl",
                "Turkish Championship League",
                "TR",
                "EMEA",
                4,
                &["AZ"],
            ),
            erl("lit", "Italian ERL / LIT", "IT", "EMEA", 2, &["MT", "SM"]),
        ]
    })
}

fn academy_candidate_catalog() -> &'static [ErlAcademyCandidate] {
    static CATALOG: std::sync::OnceLock<Vec<ErlAcademyCandidate>> = std::sync::OnceLock::new();
    CATALOG.get_or_init(|| {
        example_academy_seed_catalog()
            .iter()
            .map(|seed| {
                let (reputation, development_level) = match seed.league_id.as_str() {
                    "lfl" => (5, 4),
                    "prime-league" => (5, 3),
                    "les" | "liga-espanola" => (4, 3),
                    _ => (4, 3),
                };

                ErlAcademyCandidate {
                    source_team_id: academy_seed_team_id(&seed.league_id, &seed.team_name),
                    name: seed.team_name.clone(),
                    short_name: seed.short_name.clone(),
                    logo_url: seed.logo_url.clone(),
                    erl_league_id: seed.league_id.clone(),
                    country_code: seed.country_code.clone(),
                    reputation,
                    development_level,
                }
            })
            .collect()
    })
}

fn erl(
    id: &str,
    name: &str,
    country_code: &str,
    region: &str,
    reputation: u8,
    nearby_country_codes: &[&str],
) -> ErlLeagueDefinition {
    ErlLeagueDefinition {
        id: id.to_string(),
        name: name.to_string(),
        country_code: country_code.to_string(),
        region: region.to_string(),
        reputation,
        nearby_country_codes: nearby_country_codes
            .iter()
            .map(|country| country.to_string())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        acquire_academy_team_in_game, get_academy_acquisition_options_for_game,
        AcquireAcademyTeamRequest,
    };
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::team::{
        AcademyLifecycle, AcademyMetadata, ErlAssignment, ErlAssignmentRule, Team, TeamKind,
    };
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;

    fn source_id_by_name(game: &Game, parent_team_id: &str, team_name: &str) -> String {
        get_academy_acquisition_options_for_game(game, parent_team_id)
            .expect("options")
            .options
            .into_iter()
            .find(|option| option.name == team_name)
            .map(|option| option.source_team_id)
            .expect("expected academy option by name")
    }

    fn team(id: &str, country: &str, finance: i64) -> Team {
        let mut team = Team::new(
            id.to_string(),
            format!("{} Esports", id),
            id.chars().take(3).collect::<String>().to_uppercase(),
            country.to_string(),
            "Berlin".to_string(),
            "Arena".to_string(),
            12_000,
        );
        team.finance = finance;
        team
    }

    fn game_with_team(mut parent: Team) -> Game {
        let mut manager = Manager::new(
            "manager-1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1985-01-01".to_string(),
            "ES".to_string(),
        );
        manager.hire(parent.id.clone());
        parent.manager_id = Some(manager.id.clone());

        Game::new(
            GameClock::new(Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap()),
            manager,
            vec![parent],
            vec![],
            vec![],
            vec![],
        )
    }

    #[test]
    fn acquisition_options_list_existing_domestic_candidate_teams() {
        let game = game_with_team(team("koi", "ES", 1_000_000));

        let response = get_academy_acquisition_options_for_game(&game, "koi").expect("options");

        assert!(response.acquisition_allowed);
        assert_eq!(response.parent_team_id, "koi");
        assert!(response.options.len() >= 5);
        let koi_fenix = response
            .options
            .iter()
            .find(|option| option.name == "Movistar KOI Fénix")
            .expect("koi academy candidate in open pool");
        assert_eq!(koi_fenix.name, "Movistar KOI Fénix");
        assert_eq!(koi_fenix.erl_league_id, "liga-espanola");
        assert_eq!(koi_fenix.assignment_rule, ErlAssignmentRule::Domestic);
        assert_eq!(koi_fenix.fallback_reason, None);
        assert!(koi_fenix.acquisition_cost > 0);
    }

    #[test]
    fn acquisition_options_include_karmine_corp_blue_for_france() {
        let game = game_with_team(team("karmine", "FR", 1_000_000));

        let response = get_academy_acquisition_options_for_game(&game, "karmine").expect("options");

        assert!(response.acquisition_allowed);
        assert!(response
            .options
            .iter()
            .any(|option| option.name == "Karmine Corp Blue"));
    }

    #[test]
    fn acquisition_options_include_cross_country_candidates_in_open_pool() {
        let game = game_with_team(team("swiss-team", "CH", 1_000_000));

        let response =
            get_academy_acquisition_options_for_game(&game, "swiss-team").expect("options");

        assert!(response.acquisition_allowed);
        assert!(response.options.len() >= 5);
        let kcb = response
            .options
            .iter()
            .find(|option| option.name == "Karmine Corp Blue")
            .expect("kcb option available cross-country");
        assert_eq!(kcb.assignment_rule, ErlAssignmentRule::Fallback);
        assert_eq!(kcb.fallback_reason, None);
    }

    #[test]
    fn acquisition_options_exclude_taken_source_teams() {
        let mut game = game_with_team(team("mad", "ES", 1_000_000));
        let mut taken_academy = team("academy-any", "ES", 0);
        taken_academy.team_kind = TeamKind::Academy;
        taken_academy.parent_team_id = Some("other-parent".to_string());
        taken_academy.academy = Some(AcademyMetadata {
            lifecycle: AcademyLifecycle::Active,
            erl_assignment: ErlAssignment {
                erl_league_id: "liga-espanola".to_string(),
                country_rule: ErlAssignmentRule::Domestic,
                fallback_reason: None,
                reputation: 4,
                acquisition_cost: 300_000,
                acquired_at: "2026-01-01T12:00:00+00:00".to_string(),
                creation_cost: 0,
                created_at: String::new(),
            },
            source_team_id: "academy-liga-espanola-team-heretics".to_string(),
            original_name: "Team Heretics".to_string(),
            original_short_name: "TH".to_string(),
            original_logo_url: None,
            current_logo_url: None,
            acquisition_cost: 300_000,
            acquired_at: "2026-01-01T12:00:00+00:00".to_string(),
        });
        game.teams.push(taken_academy);

        let response = get_academy_acquisition_options_for_game(&game, "mad").expect("options");

        assert!(!response
            .options
            .iter()
            .any(|option| option.name == "Team Heretics"));
    }

    #[test]
    fn options_generation_marks_acquisition_blocked_when_team_cannot_afford_any_option() {
        let game = game_with_team(team("broke-team", "ES", 1));

        let response =
            get_academy_acquisition_options_for_game(&game, "broke-team").expect("options");

        assert!(!response.acquisition_allowed);
        assert_eq!(
            response.blocked_reason.as_deref(),
            Some("Insufficient funds for all eligible academy acquisition options")
        );
        assert_eq!(response.options[0].erl_league_id, "liga-espanola");
        assert!(response.options[0].acquisition_cost > 1);
    }

    #[test]
    fn acquire_academy_team_rejects_invalid_candidate_without_mutation() {
        let mut game = game_with_team(team("mad", "ES", 1_000_000));
        let before = game.clone();

        let result = acquire_academy_team_in_game(
            &mut game,
            AcquireAcademyTeamRequest {
                parent_team_id: "mad".to_string(),
                source_team_id: "not-a-real-candidate".to_string(),
                custom_name: None,
                custom_short_name: None,
                custom_logo_url: None,
            },
        );

        assert!(result
            .expect_err("invalid source candidate should be rejected")
            .contains("is not eligible for this team"));
        assert_eq!(game.teams.len(), before.teams.len());
        assert_eq!(game.teams[0].finance, before.teams[0].finance);
        assert_eq!(game.teams[0].academy_team_id, None);
    }

    #[test]
    fn acquire_academy_team_rejects_parent_that_already_has_academy_without_mutation() {
        let mut parent = team("g2", "DE", 1_000_000);
        parent.academy_team_id = Some("g2-academy-existing".to_string());
        let mut game = game_with_team(parent);
        let before_team_count = game.teams.len();
        let before_parent_finance = game
            .teams
            .iter()
            .find(|team| team.id == "g2")
            .unwrap()
            .finance;
        let spandau_source_id = source_id_by_name(&game, "g2", "Eintracht Spandau");

        let result = acquire_academy_team_in_game(
            &mut game,
            AcquireAcademyTeamRequest {
                parent_team_id: "g2".to_string(),
                source_team_id: spandau_source_id,
                custom_name: None,
                custom_short_name: None,
                custom_logo_url: None,
            },
        );

        assert!(result
            .expect_err("existing academy should be rejected")
            .contains("already has academy"));
        assert_eq!(game.teams.len(), before_team_count);
        assert_eq!(
            game.teams
                .iter()
                .find(|team| team.id == "g2")
                .unwrap()
                .finance,
            before_parent_finance
        );
    }

    #[test]
    fn acquire_academy_team_rejects_insufficient_funds_without_mutation() {
        let mut game = game_with_team(team("broke-mad", "ES", 1));
        let before_team_count = game.teams.len();
        let koi_source_id = source_id_by_name(&game, "broke-mad", "Movistar KOI Fénix");

        let result = acquire_academy_team_in_game(
            &mut game,
            AcquireAcademyTeamRequest {
                parent_team_id: "broke-mad".to_string(),
                source_team_id: koi_source_id,
                custom_name: None,
                custom_short_name: None,
                custom_logo_url: None,
            },
        );

        assert!(result
            .expect_err("insufficient funds should be rejected")
            .contains("Insufficient funds"));
        let parent = game
            .teams
            .iter()
            .find(|team| team.id == "broke-mad")
            .unwrap();
        assert_eq!(game.teams.len(), before_team_count);
        assert_eq!(parent.finance, 1);
        assert_eq!(parent.academy_team_id, None);
    }

    #[test]
    fn acquire_academy_team_links_existing_candidate_with_source_metadata_and_expense() {
        let mut game = game_with_team(team("mad", "ES", 1_000_000));
        let koi_source_id = source_id_by_name(&game, "mad", "Movistar KOI Fénix");

        let updated = acquire_academy_team_in_game(
            &mut game,
            AcquireAcademyTeamRequest {
                parent_team_id: "mad".to_string(),
                source_team_id: koi_source_id.clone(),
                custom_name: Some("MAD Academy".to_string()),
                custom_short_name: Some("MADA".to_string()),
                custom_logo_url: Some("logos/mad-academy.svg".to_string()),
            },
        )
        .expect("acquired");

        let parent = updated.teams.iter().find(|team| team.id == "mad").unwrap();
        let academy_id = parent.academy_team_id.as_deref().expect("academy link");
        let academy = updated
            .teams
            .iter()
            .find(|team| team.id == academy_id)
            .expect("academy team exists");

        let metadata = academy.academy.as_ref().unwrap();
        assert_eq!(academy.team_kind, TeamKind::Academy);
        assert_eq!(academy.id, koi_source_id);
        assert_eq!(academy.name, "MAD Academy");
        assert_eq!(academy.short_name, "MADA");
        assert_eq!(academy.parent_team_id.as_deref(), Some("mad"));
        assert_eq!(metadata.source_team_id, academy.id);
        assert_eq!(metadata.original_name, "Movistar KOI Fénix");
        assert!(metadata.original_logo_url.is_some());
        assert_eq!(
            metadata.current_logo_url.as_deref(),
            Some("logos/mad-academy.svg")
        );
        assert_eq!(metadata.acquisition_cost, 300_000);
        assert_eq!(metadata.acquired_at, "2026-01-01T12:00:00+00:00");
        assert_eq!(metadata.erl_assignment.erl_league_id, "liga-espanola");
        assert_eq!(
            metadata.erl_assignment.country_rule,
            ErlAssignmentRule::Domestic
        );
        assert_eq!(parent.finance, 700_000);
        assert_eq!(parent.season_expenses, 300_000);
        assert!(updated
            .messages
            .iter()
            .any(|message| message.id.starts_with("academy-acquired-")));
        assert_eq!(game.teams.len(), updated.teams.len());
        assert_eq!(
            game.teams
                .iter()
                .find(|team| team.id == "mad")
                .and_then(|team| team.academy_team_id.as_deref()),
            parent.academy_team_id.as_deref()
        );
    }
}
