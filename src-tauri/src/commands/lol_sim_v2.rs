use tauri::State;

use crate::application::lol_sim_v2::{
    LolSimV2DisposeRequest, LolSimV2DisposeResponse, LolSimV2ResetRequest, LolSimV2StateResponse,
    LolSimV2StoreState, LolSimV2TickRequest, init as init_service, reset as reset_service,
    tick as tick_service,
};

#[tauri::command]
pub fn lol_sim_v2_init(
    state: State<'_, LolSimV2StoreState>,
    request: crate::application::lol_sim_v2::LolSimV2InitRequest,
) -> Result<LolSimV2StateResponse, String> {
    init_service(&state, request)
}

#[tauri::command]
pub fn lol_sim_v2_tick(
    state: State<'_, LolSimV2StoreState>,
    request: LolSimV2TickRequest,
) -> Result<LolSimV2StateResponse, String> {
    tick_service(&state, request)
}

#[tauri::command]
pub fn lol_sim_v2_reset(
    state: State<'_, LolSimV2StoreState>,
    request: LolSimV2ResetRequest,
) -> Result<LolSimV2StateResponse, String> {
    reset_service(&state, request)
}

#[tauri::command]
pub fn lol_sim_v2_dispose(
    state: State<'_, LolSimV2StoreState>,
    request: LolSimV2DisposeRequest,
) -> Result<LolSimV2DisposeResponse, String> {
    crate::application::lol_sim_v2::dispose(&state, request)
}
