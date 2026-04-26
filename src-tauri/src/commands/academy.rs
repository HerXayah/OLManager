use chrono::Utc;
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
    let game = state
        .get_game(|game| game.clone())
        .ok_or("No active game session".to_string())?;

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
    let updated = acquire_academy_team_in_game(&mut game, request)?;
    state.set_game(updated.clone());
    Ok(updated)
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
    let options = eligible_academy_acquisition_options(
        &parent.country,
        academy_erl_catalog(),
        academy_candidate_catalog(),
    );
    let blocked_reason = if !parent.is_main() {
        Some("Academy can only be acquired for a main team".to_string())
    } else if parent.academy_team_id.is_some() {
        Some("Parent team already has academy".to_string())
    } else if options.is_empty() {
        Some("No eligible ERL acquisition candidate configured for this team country".to_string())
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
    if game.teams.iter().any(|team| team.id == academy_id) {
        return Err(format!("Academy team id '{}' already exists", academy_id));
    }

    let created_at = game.clock.current_date.with_timezone(&Utc).to_rfc3339();
    let metadata = academy_metadata(&option, created_at.clone(), request.custom_logo_url.clone());
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

    let parent = game
        .teams
        .iter_mut()
        .find(|team| team.id == request.parent_team_id)
        .ok_or("Parent team not found".to_string())?;
    parent.finance -= option.acquisition_cost;
    parent.season_expenses += option.acquisition_cost;
    parent.academy_team_id = Some(academy_id);

    game.teams.push(academy);
    Ok(game.clone())
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
        vec![
            candidate(
                "movistar-koi-fenix",
                "Movistar KOI Fénix",
                "KOIF",
                Some("logos/movistar-koi-fenix.svg"),
                "liga-espanola",
                "ES",
                4,
                2,
            ),
            candidate(
                "los-heretics",
                "Los Heretics",
                "LHT",
                Some("logos/los-heretics.svg"),
                "liga-espanola",
                "ES",
                3,
                2,
            ),
            candidate(
                "eintracht-spandau",
                "Eintracht Spandau",
                "EINS",
                Some("logos/eintracht-spandau.svg"),
                "prime-league",
                "DE",
                5,
                3,
            ),
            candidate(
                "nno-prime",
                "NNO Prime",
                "NNO",
                Some("logos/nno-prime.svg"),
                "prime-league",
                "DE",
                4,
                2,
            ),
        ]
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

fn candidate(
    source_team_id: &str,
    name: &str,
    short_name: &str,
    logo_url: Option<&str>,
    erl_league_id: &str,
    country_code: &str,
    reputation: u8,
    development_level: u8,
) -> ErlAcademyCandidate {
    ErlAcademyCandidate {
        source_team_id: source_team_id.to_string(),
        name: name.to_string(),
        short_name: short_name.to_string(),
        logo_url: logo_url.map(str::to_string),
        erl_league_id: erl_league_id.to_string(),
        country_code: country_code.to_string(),
        reputation,
        development_level,
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
    use domain::team::{ErlAssignmentRule, Team, TeamKind};
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;

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
        assert_eq!(response.options.len(), 2);
        assert_eq!(response.options[0].source_team_id, "movistar-koi-fenix");
        assert_eq!(response.options[0].name, "Movistar KOI Fénix");
        assert_eq!(response.options[0].erl_league_id, "liga-espanola");
        assert_eq!(
            response.options[0].assignment_rule,
            ErlAssignmentRule::Domestic
        );
        assert_eq!(response.options[0].fallback_reason, None);
        assert!(response.options[0].acquisition_cost > 0);
    }

    #[test]
    fn acquisition_options_use_nearby_fallback_and_candidate_costs() {
        let game = game_with_team(team("swiss-team", "CH", 1_000_000));

        let response =
            get_academy_acquisition_options_for_game(&game, "swiss-team").expect("options");

        assert!(response.acquisition_allowed);
        assert_eq!(response.options.len(), 2);
        assert_eq!(response.options[0].erl_league_id, "prime-league");
        assert_eq!(
            response.options[0].assignment_rule,
            ErlAssignmentRule::Fallback
        );
        assert_eq!(
            response.options[0].fallback_reason.as_deref(),
            Some("CH has no domestic ERL; prime-league is configured as nearby")
        );
        assert!(response.options[0].acquisition_cost > response.options[1].acquisition_cost);
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

        let result = acquire_academy_team_in_game(
            &mut game,
            AcquireAcademyTeamRequest {
                parent_team_id: "g2".to_string(),
                source_team_id: "eintracht-spandau".to_string(),
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

        let result = acquire_academy_team_in_game(
            &mut game,
            AcquireAcademyTeamRequest {
                parent_team_id: "broke-mad".to_string(),
                source_team_id: "movistar-koi-fenix".to_string(),
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

        let updated = acquire_academy_team_in_game(
            &mut game,
            AcquireAcademyTeamRequest {
                parent_team_id: "mad".to_string(),
                source_team_id: "movistar-koi-fenix".to_string(),
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
        assert_eq!(academy.id, "movistar-koi-fenix");
        assert_eq!(academy.name, "MAD Academy");
        assert_eq!(academy.short_name, "MADA");
        assert_eq!(academy.parent_team_id.as_deref(), Some("mad"));
        assert_eq!(metadata.source_team_id, "movistar-koi-fenix");
        assert_eq!(metadata.original_name, "Movistar KOI Fénix");
        assert_eq!(
            metadata.original_logo_url.as_deref(),
            Some("logos/movistar-koi-fenix.svg")
        );
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
