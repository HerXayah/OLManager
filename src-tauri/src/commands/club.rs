use log::info;
use tauri::State;

use ofm_core::game::Game;
use ofm_core::state::StateManager;

#[tauri::command]
pub fn upgrade_facility(state: State<'_, StateManager>, facility: String) -> Result<Game, String> {
    upgrade_facility_internal(&state, &facility)
}

#[tauri::command]
pub fn upgrade_main_facility_module(
    state: State<'_, StateManager>,
    module: String,
) -> Result<Game, String> {
    upgrade_main_facility_module_internal(&state, &module)
}

#[tauri::command]
pub fn expand_main_facility_hub(state: State<'_, StateManager>) -> Result<Game, String> {
    expand_main_facility_hub_internal(&state)
}

fn upgrade_facility_internal(state: &StateManager, facility: &str) -> Result<Game, String> {
    info!("[cmd] upgrade_facility: {}", facility);
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    let facility_type = match facility {
        "Training" => domain::team::FacilityType::Training,
        "Medical" => domain::team::FacilityType::Medical,
        "Scouting" => domain::team::FacilityType::Scouting,
        _ => return Err(format!("Unknown facility type: {}", facility)),
    };

    let team = game
        .teams
        .iter_mut()
        .find(|team| team.id == team_id)
        .ok_or("Managed team not found".to_string())?;

    ofm_core::club::upgrade_facility(team, facility_type)?;

    state.set_game(game.clone());
    Ok(game)
}

fn upgrade_main_facility_module_internal(
    state: &StateManager,
    module: &str,
) -> Result<Game, String> {
    info!("[cmd] upgrade_main_facility_module: {}", module);
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    let module_kind = match module {
        "ScrimsRoom" => domain::team::MainFacilityModuleKind::ScrimsRoom,
        "AnalysisRoom" => domain::team::MainFacilityModuleKind::AnalysisRoom,
        "BootcampArea" => domain::team::MainFacilityModuleKind::BootcampArea,
        "RecoverySuite" => domain::team::MainFacilityModuleKind::RecoverySuite,
        "ContentStudio" => domain::team::MainFacilityModuleKind::ContentStudio,
        "ScoutingLab" => domain::team::MainFacilityModuleKind::ScoutingLab,
        _ => return Err(format!("Unknown facility module: {}", module)),
    };

    let team = game
        .teams
        .iter_mut()
        .find(|team| team.id == team_id)
        .ok_or("Managed team not found".to_string())?;

    ofm_core::club::upgrade_main_facility_module(team, module_kind)?;

    state.set_game(game.clone());
    Ok(game)
}

fn expand_main_facility_hub_internal(state: &StateManager) -> Result<Game, String> {
    info!("[cmd] expand_main_facility_hub");
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned".to_string())?;

    let team = game
        .teams
        .iter_mut()
        .find(|team| team.id == team_id)
        .ok_or("Managed team not found".to_string())?;

    ofm_core::club::expand_main_facility_hub(team)?;

    state.set_game(game.clone());
    Ok(game)
}

#[cfg(test)]
mod tests {
    use super::upgrade_facility_internal;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
    use ofm_core::state::StateManager;

    fn make_team() -> Team {
        let mut team = Team::new(
            "team-1".to_string(),
            "User FC".to_string(),
            "USR".to_string(),
            "England".to_string(),
            "London".to_string(),
            "User Ground".to_string(),
            25_000,
        );
        team.finance = 1_000_000;
        team.manager_id = Some("manager-1".to_string());
        team
    }

    fn make_game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "manager-1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team-1".to_string());

        Game::new(clock, manager, vec![make_team()], vec![], vec![], vec![])
    }

    #[test]
    fn upgrade_facility_internal_updates_state() {
        let state = StateManager::new();
        state.set_game(make_game());

        let response = upgrade_facility_internal(&state, "Medical").expect("response");
        let team = response
            .teams
            .iter()
            .find(|team| team.id == "team-1")
            .unwrap();

        assert_eq!(team.facilities.medical, 2);
        assert_eq!(team.finance, 750_000);

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        let stored_team = stored_game
            .teams
            .iter()
            .find(|team| team.id == "team-1")
            .expect("stored team should exist");
        assert_eq!(stored_team.facilities.medical, 2);
        assert_eq!(stored_team.finance, 750_000);
    }
}
