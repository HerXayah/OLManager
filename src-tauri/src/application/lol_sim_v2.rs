use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};
use std::fs::{create_dir_all, read_dir, remove_file, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Default)]
pub struct LolSimV2StoreState {
    pub sessions: Mutex<HashMap<String, LolSimV2Session>>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LolSimV2Session {
    pub id: String,
    pub seed: String,
    pub state: Value,
    pub tick_index: u64,
    pub wave_spawn_at: f64,
    pub next_minion_id: u64,
    pub snapshot: Value,
    pub champion_by_player_id: HashMap<String, String>,
    pub champion_profiles_by_id: HashMap<String, LolChampionCombatProfileInput>,
    pub champion_ultimates_by_id: HashMap<String, LolChampionUltimateInput>,
    pub lane_combat_state_by_champion: HashMap<String, LanerCombatStateRuntime>,
    pub ai_mode: SimulatorAiMode,
    pub policy: SimulatorPolicyConfig,
    telemetry: TelemetryRuntime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulatorPolicyConfig {
    #[serde(default = "default_no_dive_hp_min")]
    pub no_dive_hp_min: f64,
    #[serde(default = "default_trade_retreat_hp_ratio")]
    pub trade_retreat_hp_ratio: f64,
    #[serde(default = "default_trade_hp_disadvantage_allowance")]
    pub trade_hp_disadvantage_allowance: f64,
    #[serde(default = "default_lane_chase_leash_radius")]
    pub lane_chase_leash_radius: f64,
    #[serde(default = "default_hybrid_open_trade_confidence_high")]
    pub hybrid_open_trade_confidence_high: f64,
    #[serde(default = "default_hybrid_disengage_confidence_low")]
    pub hybrid_disengage_confidence_low: f64,
}

impl Default for SimulatorPolicyConfig {
    fn default() -> Self {
        Self {
            no_dive_hp_min: default_no_dive_hp_min(),
            trade_retreat_hp_ratio: default_trade_retreat_hp_ratio(),
            trade_hp_disadvantage_allowance: default_trade_hp_disadvantage_allowance(),
            lane_chase_leash_radius: default_lane_chase_leash_radius(),
            hybrid_open_trade_confidence_high: default_hybrid_open_trade_confidence_high(),
            hybrid_disengage_confidence_low: default_hybrid_disengage_confidence_low(),
        }
    }
}

fn default_no_dive_hp_min() -> f64 {
    0.45
}
fn default_trade_retreat_hp_ratio() -> f64 {
    0.36
}
fn default_trade_hp_disadvantage_allowance() -> f64 {
    0.2
}
fn default_lane_chase_leash_radius() -> f64 {
    0.11
}
fn default_hybrid_open_trade_confidence_high() -> f64 {
    0.60
}
fn default_hybrid_disengage_confidence_low() -> f64 {
    0.32
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulatorTelemetryConfig {
    #[serde(default = "default_telemetry_enabled")]
    pub enabled: bool,
    #[serde(default = "default_telemetry_sample_every_ticks")]
    pub sample_every_ticks: u64,
    #[serde(default = "default_telemetry_outcome_window_ticks")]
    pub outcome_window_ticks: u64,
    #[serde(default = "default_telemetry_decision_change_only")]
    pub decision_change_only: bool,
    #[serde(default)]
    pub output_path: Option<String>,
}

impl Default for SimulatorTelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: default_telemetry_enabled(),
            sample_every_ticks: default_telemetry_sample_every_ticks(),
            outcome_window_ticks: default_telemetry_outcome_window_ticks(),
            decision_change_only: true,
            output_path: None,
        }
    }
}

fn default_telemetry_enabled() -> bool {
    false
}

fn default_telemetry_sample_every_ticks() -> u64 {
    20
}

fn default_telemetry_outcome_window_ticks() -> u64 {
    160
}

fn default_telemetry_decision_change_only() -> bool {
    true
}

fn default_visible_stat() -> f64 {
    70.0
}

#[derive(Debug, Clone, Default)]
struct TelemetryRuntime {
    config: SimulatorTelemetryConfig,
    output_path: Option<PathBuf>,
    pending: Vec<PendingTelemetryOutcome>,
    last_tick_by_key: HashMap<String, u64>,
    last_decision_by_key: HashMap<String, bool>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SimulatorAiMode {
    Rules,
    #[default]
    Hybrid,
}

impl SimulatorAiMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Rules => "rules",
            Self::Hybrid => "hybrid",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanerCombatStateRuntime {
    pub last_disengage_at: f64,
    pub reengage_at: f64,
    pub recent_trade_until: f64,
    pub last_ai_debug_at: f64,
}

impl Default for LanerCombatStateRuntime {
    fn default() -> Self {
        Self {
            last_disengage_at: -999.0,
            reengage_at: -999.0,
            recent_trade_until: -999.0,
            last_ai_debug_at: -999.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LolChampionCombatProfileInput {
    pub base_hp: f64,
    pub attack_type: String,
    pub attack_range: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LolChampionUltimateInput {
    pub archetype: String,
    #[serde(default)]
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LolSimV2InitRequest {
    pub session_id: String,
    pub seed: String,
    pub snapshot: Value,
    #[serde(default)]
    pub champion_by_player_id: HashMap<String, String>,
    #[serde(default)]
    pub champion_profiles_by_id: HashMap<String, LolChampionCombatProfileInput>,
    #[serde(default)]
    pub champion_ultimates_by_id: HashMap<String, LolChampionUltimateInput>,
    pub initial_state: Option<Value>,
    #[serde(default)]
    pub ai_mode: SimulatorAiMode,
    #[serde(default)]
    pub policy: SimulatorPolicyConfig,
    #[serde(default)]
    pub telemetry: SimulatorTelemetryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LolSimV2TickRequest {
    pub session_id: String,
    pub dt_sec: f64,
    pub running: bool,
    pub speed: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LolSimV2ResetRequest {
    pub session_id: String,
    pub seed: String,
    pub initial_state: Option<Value>,
    #[serde(default)]
    pub ai_mode: SimulatorAiMode,
    #[serde(default)]
    pub policy: Option<SimulatorPolicyConfig>,
    #[serde(default)]
    pub telemetry: Option<SimulatorTelemetryConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LolSimV2DisposeRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LolSimV2StateResponse {
    pub session_id: String,
    pub state: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LolSimV2DisposeResponse {
    pub session_id: String,
    pub disposed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LolSimV2RunToCompletionRequest {
    pub seed: String,
    pub snapshot: Value,
    #[serde(default)]
    pub champion_by_player_id: HashMap<String, String>,
    #[serde(default)]
    pub champion_profiles_by_id: HashMap<String, LolChampionCombatProfileInput>,
    #[serde(default)]
    pub champion_ultimates_by_id: HashMap<String, LolChampionUltimateInput>,
    #[serde(default)]
    pub ai_mode: SimulatorAiMode,
    #[serde(default)]
    pub policy: SimulatorPolicyConfig,
    #[serde(default)]
    pub telemetry: SimulatorTelemetryConfig,
    #[serde(default = "default_run_to_completion_dt_sec")]
    pub dt_sec: f64,
    #[serde(default = "default_run_to_completion_speed")]
    pub speed: f64,
    #[serde(default = "default_run_to_completion_max_ticks")]
    pub max_ticks: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LolSimV2RunToCompletionResponse {
    pub winner: Option<String>,
    pub ticks: u64,
    pub elapsed_simulated_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LolSimV2SkipToEndRequest {
    pub session_id: String,
    #[serde(default = "default_skip_to_end_dt_sec")]
    pub dt_sec: f64,
    #[serde(default = "default_skip_to_end_speed")]
    pub speed: f64,
    #[serde(default = "default_skip_to_end_max_ticks")]
    pub max_ticks: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LolSimV2SkipToEndResponse {
    pub session_id: String,
    pub state: Value,
    pub winner: Option<String>,
    pub ticks: u64,
    pub elapsed_simulated_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearTelemetryFilesResponse {
    pub directory: String,
    pub deleted_files: usize,
    pub skipped_entries: usize,
    pub existed: bool,
}

pub fn init(
    store: &LolSimV2StoreState,
    request: LolSimV2InitRequest,
) -> Result<LolSimV2StateResponse, String> {
    if request.session_id.trim().is_empty() {
        return Err("sessionId is required".to_string());
    }

    let mut state = create_initial_state(
        &request.seed,
        &request.snapshot,
        &request.champion_by_player_id,
        &request.champion_profiles_by_id,
        &request.champion_ultimates_by_id,
        request.ai_mode,
    );
    ensure_runtime_state_defaults(&mut state);
    let telemetry_output_path = resolve_telemetry_output_path(&request.telemetry, &request.seed);

    let session = LolSimV2Session {
        id: request.session_id.clone(),
        seed: request.seed,
        state: state.clone(),
        tick_index: 0,
        wave_spawn_at: MINION_FIRST_WAVE_AT,
        next_minion_id: 1,
        snapshot: request.snapshot,
        champion_by_player_id: request.champion_by_player_id,
        champion_profiles_by_id: request.champion_profiles_by_id,
        champion_ultimates_by_id: request.champion_ultimates_by_id,
        lane_combat_state_by_champion: HashMap::new(),
        ai_mode: request.ai_mode,
        policy: request.policy,
        telemetry: TelemetryRuntime {
            output_path: telemetry_output_path,
            config: request.telemetry,
            ..TelemetryRuntime::default()
        },
    };

    let mut sessions = store
        .sessions
        .lock()
        .map_err(|_| "lol sim v2 session store lock poisoned".to_string())?;
    sessions.insert(session.id.clone(), session);

    Ok(LolSimV2StateResponse {
        session_id: request.session_id,
        state,
    })
}

pub fn tick(
    store: &LolSimV2StoreState,
    request: LolSimV2TickRequest,
) -> Result<LolSimV2StateResponse, String> {
    let mut sessions = store
        .sessions
        .lock()
        .map_err(|_| "lol sim v2 session store lock poisoned".to_string())?;

    let session = sessions
        .get_mut(&request.session_id)
        .ok_or_else(|| format!("lol sim v2 session not found: {}", request.session_id))?;

    ensure_runtime_state_defaults(&mut session.state);

    let mut runtime: RuntimeState = serde_json::from_value(session.state.clone())
        .map_err(|err| format!("failed to decode lol_sim_v2 runtime state: {err}"))?;
    runtime.lane_combat_state_by_champion = session.lane_combat_state_by_champion.clone();
    runtime.ai_mode = session.ai_mode;
    runtime.policy = session.policy.clone();
    runtime.telemetry_decisions.clear();

    let speed = request.speed.max(0.0);
    runtime.speed = speed;
    if runtime.winner.is_some() {
        runtime.running = false;
    } else {
        runtime.running = request.running;
    }

    if !runtime.running {
        session.lane_combat_state_by_champion = runtime.lane_combat_state_by_champion.clone();
        session.state = serde_json::to_value(runtime)
            .map_err(|err| format!("failed to encode lol_sim_v2 runtime state: {err}"))?;
        return Ok(LolSimV2StateResponse {
            session_id: session.id.clone(),
            state: session.state.clone(),
        });
    }

    let dt = request.dt_sec.clamp(0.0, 0.05) * speed;
    if dt <= 0.0 {
        session.lane_combat_state_by_champion = runtime.lane_combat_state_by_champion.clone();
        session.state = serde_json::to_value(runtime)
            .map_err(|err| format!("failed to encode lol_sim_v2 runtime state: {err}"))?;
        return Ok(LolSimV2StateResponse {
            session_id: session.id.clone(),
            state: session.state.clone(),
        });
    }

    runtime.time_sec += dt;
    session.tick_index = session.tick_index.saturating_add(1);

    spawn_waves_if_due(&mut runtime, session);
    move_champions(&mut runtime, dt);
    maybe_deploy_rift_herald_charge(&mut runtime);
    place_wards(&mut runtime);
    process_sweepers(&mut runtime);
    move_minions(&mut runtime, dt);
    resolve_minion_combat(&mut runtime);
    resolve_champion_combat(&mut runtime);
    resolve_structure_combat(&mut runtime);
    tick_neutral_timers(&mut runtime);
    cleanup_tick(&mut runtime);
    process_telemetry(&mut runtime, session);

    if runtime.winner.is_some() {
        runtime.running = false;
    }

    session.lane_combat_state_by_champion = runtime.lane_combat_state_by_champion.clone();
    session.state = serde_json::to_value(runtime)
        .map_err(|err| format!("failed to encode lol_sim_v2 runtime state: {err}"))?;

    Ok(LolSimV2StateResponse {
        session_id: session.id.clone(),
        state: session.state.clone(),
    })
}

pub fn reset(
    store: &LolSimV2StoreState,
    request: LolSimV2ResetRequest,
) -> Result<LolSimV2StateResponse, String> {
    let mut sessions = store
        .sessions
        .lock()
        .map_err(|_| "lol sim v2 session store lock poisoned".to_string())?;

    let session = sessions
        .get_mut(&request.session_id)
        .ok_or_else(|| format!("lol sim v2 session not found: {}", request.session_id))?;

    session.seed = request.seed;
    session.state = create_initial_state(
        &session.seed,
        &session.snapshot,
        &session.champion_by_player_id,
        &session.champion_profiles_by_id,
        &session.champion_ultimates_by_id,
        request.ai_mode,
    );
    session.ai_mode = request.ai_mode;
    if let Some(policy) = request.policy {
        session.policy = policy;
    }
    session.tick_index = 0;
    if let Some(config) = request.telemetry {
        session.telemetry.config = config;
    }
    session.telemetry.output_path =
        resolve_telemetry_output_path(&session.telemetry.config, &session.seed);
    session.telemetry.pending.clear();
    session.telemetry.last_tick_by_key.clear();
    session.telemetry.last_decision_by_key.clear();
    ensure_runtime_state_defaults(&mut session.state);
    session.wave_spawn_at = MINION_FIRST_WAVE_AT;
    session.next_minion_id = 1;
    session.lane_combat_state_by_champion.clear();

    Ok(LolSimV2StateResponse {
        session_id: session.id.clone(),
        state: session.state.clone(),
    })
}

pub fn dispose(
    store: &LolSimV2StoreState,
    request: LolSimV2DisposeRequest,
) -> Result<LolSimV2DisposeResponse, String> {
    let mut sessions = store
        .sessions
        .lock()
        .map_err(|_| "lol sim v2 session store lock poisoned".to_string())?;
    let removed = sessions.remove(&request.session_id).is_some();

    Ok(LolSimV2DisposeResponse {
        session_id: request.session_id,
        disposed: removed,
    })
}

pub fn run_to_completion(
    _store: &LolSimV2StoreState,
    request: LolSimV2RunToCompletionRequest,
) -> Result<LolSimV2RunToCompletionResponse, String> {
    // Use an isolated in-memory store per run to avoid lock contention
    // between concurrent AFK simulations.
    let local_store = LolSimV2StoreState::default();

    let session_id = format!(
        "lol-sim-v2-afk-{}-{}",
        request.seed,
        next_run_to_completion_suffix()
    );

    let run_result = (|| {
        let mut response = init(
            &local_store,
            LolSimV2InitRequest {
                session_id: session_id.clone(),
                seed: request.seed,
                snapshot: request.snapshot,
                champion_by_player_id: request.champion_by_player_id,
                champion_profiles_by_id: request.champion_profiles_by_id,
                champion_ultimates_by_id: request.champion_ultimates_by_id,
                initial_state: None,
                ai_mode: request.ai_mode,
                policy: request.policy,
                telemetry: request.telemetry,
            },
        )?;

        let mut ticks = 0u64;
        for _ in 0..request.max_ticks {
            if read_winner(&response.state).is_some() {
                break;
            }

            response = tick(
                &local_store,
                LolSimV2TickRequest {
                    session_id: session_id.clone(),
                    dt_sec: request.dt_sec,
                    running: true,
                    speed: request.speed,
                },
            )?;
            ticks = ticks.saturating_add(1);
        }

        Ok(LolSimV2RunToCompletionResponse {
            winner: read_winner(&response.state),
            ticks,
            elapsed_simulated_sec: read_time_sec(&response.state),
        })
    })();

    let _ = dispose(
        &local_store,
        LolSimV2DisposeRequest {
            session_id: session_id.clone(),
        },
    );

    run_result
}

pub fn skip_to_end(
    store: &LolSimV2StoreState,
    request: LolSimV2SkipToEndRequest,
) -> Result<LolSimV2SkipToEndResponse, String> {
    if request.max_ticks == 0 {
        let sessions = store
            .sessions
            .lock()
            .map_err(|_| "lol sim v2 session store lock poisoned".to_string())?;

        let session = sessions
            .get(&request.session_id)
            .ok_or_else(|| format!("lol sim v2 session not found: {}", request.session_id))?;

        return Ok(LolSimV2SkipToEndResponse {
            session_id: request.session_id,
            winner: read_winner(&session.state),
            elapsed_simulated_sec: read_time_sec(&session.state),
            ticks: 0,
            state: session.state.clone(),
        });
    }

    let mut sessions = store
        .sessions
        .lock()
        .map_err(|_| "lol sim v2 session store lock poisoned".to_string())?;

    let session = sessions
        .get_mut(&request.session_id)
        .ok_or_else(|| format!("lol sim v2 session not found: {}", request.session_id))?;

    ensure_runtime_state_defaults(&mut session.state);

    let mut runtime: RuntimeState = serde_json::from_value(session.state.clone())
        .map_err(|err| format!("failed to decode lol_sim_v2 runtime state: {err}"))?;
    runtime.lane_combat_state_by_champion = session.lane_combat_state_by_champion.clone();
    runtime.ai_mode = session.ai_mode;
    runtime.policy = session.policy.clone();
    runtime.telemetry_decisions.clear();
    runtime
        .extra
        .insert(SKIP_FAST_MODE_EXTRA_KEY.to_string(), Value::Bool(true));

    let previous_telemetry_enabled = session.telemetry.config.enabled;
    session.telemetry.config.enabled = false;
    session.telemetry.pending.clear();

    let dt = request.dt_sec.clamp(0.0, 0.05) * request.speed.max(0.0);
    if dt <= 0.0 {
        runtime.extra.remove(SKIP_FAST_MODE_EXTRA_KEY);
        session.telemetry.config.enabled = previous_telemetry_enabled;
        session.lane_combat_state_by_champion = runtime.lane_combat_state_by_champion.clone();
        session.state = serde_json::to_value(runtime)
            .map_err(|err| format!("failed to encode lol_sim_v2 runtime state: {err}"))?;
        return Ok(LolSimV2SkipToEndResponse {
            session_id: request.session_id,
            winner: read_winner(&session.state),
            elapsed_simulated_sec: read_time_sec(&session.state),
            ticks: 0,
            state: session.state.clone(),
        });
    }

    let mut ticks = 0u64;
    while ticks < request.max_ticks {
        if runtime.winner.is_some() {
            break;
        }

        let fast_skip_macro_tick = ticks % 3 == 0;

        runtime.speed = request.speed.max(0.0);
        runtime.running = true;
        runtime.time_sec += dt;
        session.tick_index = session.tick_index.saturating_add(1);

        spawn_waves_if_due(&mut runtime, session);
        move_champions(&mut runtime, dt);
        if fast_skip_macro_tick {
            place_wards(&mut runtime);
            process_sweepers(&mut runtime);
        }
        move_minions(&mut runtime, dt);
        resolve_minion_combat(&mut runtime);
        resolve_champion_combat(&mut runtime);
        resolve_structure_combat(&mut runtime);
        tick_neutral_timers(&mut runtime);
        cleanup_tick(&mut runtime);
        process_telemetry(&mut runtime, session);

        ticks = ticks.saturating_add(1);
        if runtime.winner.is_some() {
            runtime.running = false;
            break;
        }
    }

    runtime.extra.remove(SKIP_FAST_MODE_EXTRA_KEY);
    session.telemetry.config.enabled = previous_telemetry_enabled;
    session.lane_combat_state_by_champion = runtime.lane_combat_state_by_champion.clone();
    session.state = serde_json::to_value(runtime)
        .map_err(|err| format!("failed to encode lol_sim_v2 runtime state: {err}"))?;

    Ok(LolSimV2SkipToEndResponse {
        session_id: request.session_id,
        winner: read_winner(&session.state),
        elapsed_simulated_sec: read_time_sec(&session.state),
        ticks,
        state: session.state.clone(),
    })
}

fn default_run_to_completion_dt_sec() -> f64 {
    0.2
}

fn default_run_to_completion_speed() -> f64 {
    12.0
}

fn default_run_to_completion_max_ticks() -> u64 {
    3600
}

fn default_skip_to_end_dt_sec() -> f64 {
    default_run_to_completion_dt_sec()
}

fn default_skip_to_end_speed() -> f64 {
    default_run_to_completion_speed()
}

fn default_skip_to_end_max_ticks() -> u64 {
    default_run_to_completion_max_ticks()
}

fn next_run_to_completion_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0)
}

fn read_winner(state: &Value) -> Option<String> {
    state
        .get("winner")
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn read_time_sec(state: &Value) -> f64 {
    state.get("timeSec").and_then(Value::as_f64).unwrap_or(0.0)
}

pub fn clear_default_telemetry_files() -> Result<ClearTelemetryFilesResponse, String> {
    let telemetry_dir = default_telemetry_directory();
    let existed = telemetry_dir.exists();

    if !existed {
        return Ok(ClearTelemetryFilesResponse {
            directory: telemetry_dir.to_string_lossy().to_string(),
            deleted_files: 0,
            skipped_entries: 0,
            existed: false,
        });
    }

    let mut deleted_files = 0usize;
    let mut skipped_entries = 0usize;

    for entry in read_dir(&telemetry_dir).map_err(|err| {
        format!(
            "failed to read telemetry directory {}: {}",
            telemetry_dir.to_string_lossy(),
            err
        )
    })? {
        let entry = entry.map_err(|err| format!("failed to inspect telemetry entry: {}", err))?;
        let path = entry.path();
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("jsonl"))
            .unwrap_or(false);

        if !extension || !path.is_file() {
            skipped_entries += 1;
            continue;
        }

        remove_file(&path).map_err(|err| {
            format!(
                "failed to remove telemetry file {}: {}",
                path.to_string_lossy(),
                err
            )
        })?;
        deleted_files += 1;
    }

    Ok(ClearTelemetryFilesResponse {
        directory: telemetry_dir.to_string_lossy().to_string(),
        deleted_files,
        skipped_entries,
        existed: true,
    })
}

fn default_telemetry_directory() -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push("olmanager");
    path.push("lol-sim-telemetry");
    path
}

fn resolve_telemetry_output_path(config: &SimulatorTelemetryConfig, seed: &str) -> Option<PathBuf> {
    if !config.enabled {
        return None;
    }

    if let Some(custom) = config
        .output_path
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        return Some(PathBuf::from(custom));
    }

    let mut path = default_telemetry_directory();
    path.push(format!("{}.jsonl", sanitize_for_file_name(seed)));
    Some(path)
}

fn sanitize_for_file_name(input: &str) -> String {
    let mut sanitized = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }
    if sanitized.is_empty() {
        "default-seed".to_string()
    } else {
        sanitized
    }
}

fn process_telemetry(runtime: &mut RuntimeState, session: &mut LolSimV2Session) {
    if !session.telemetry.config.enabled {
        runtime.telemetry_decisions.clear();
        return;
    }

    let now_tick = session.tick_index;
    let mut pending_to_write: Vec<Value> = Vec::new();
    let mut still_pending = Vec::with_capacity(session.telemetry.pending.len());

    for pending in session.telemetry.pending.drain(..) {
        if now_tick < pending.due_tick {
            still_pending.push(pending);
            continue;
        }

        let Some(champion_now) = runtime
            .champions
            .iter()
            .find(|champion| champion.id == pending.champion_id)
        else {
            continue;
        };

        let enemy_now = runtime
            .champions
            .iter()
            .find(|champion| champion.id == pending.enemy_id);
        let hp_ratio_now = ratio_or_zero(champion_now.hp, champion_now.max_hp);
        let deaths_now = champion_now.deaths;
        let kills_assists_now = champion_now.kills + champion_now.assists;
        let lane_progress_now = lane_progress_for_champion(champion_now);

        let enemy_hp_ratio_now = enemy_now
            .map(|enemy| ratio_or_zero(enemy.hp, enemy.max_hp))
            .unwrap_or(0.0);

        pending_to_write.push(json!({
            "sessionId": pending.session_id,
            "seed": pending.seed,
            "tick": pending.sampled_tick,
            "timeSec": pending.sampled_at_sec,
            "outcomeTick": now_tick,
            "outcomeTimeSec": runtime.time_sec,
            "championId": pending.champion_id,
            "enemyChampionId": pending.enemy_id,
            "team": pending.team,
            "role": pending.role,
            "lane": pending.lane,
            "aiMode": pending.ai_mode,
            "intent": pending.intent,
            "decision": pending.decision,
            "ruleDecision": pending.rule_decision,
            "confidence": pending.confidence,
            "flippedByHybrid": pending.flipped_by_hybrid,
            "featureSnapshot": {
                "hpRatio": pending.hp_ratio,
                "enemyHpRatio": pending.enemy_hp_ratio,
                "allyChampionsLocal": pending.ally_champions_local,
                "enemyChampionsLocal": pending.enemy_champions_local,
                "allyMinionsLocal": pending.ally_minions_local,
                "enemyMinionsLocal": pending.enemy_minions_local,
                "nearestEnemyTowerDistance": pending.nearest_enemy_tower_distance,
                "enemyOverextended": pending.enemy_overextended,
                "objectiveDanger": pending.objective_danger,
                "gold": pending.gold,
                "xp": pending.xp,
                "level": pending.level,
                "resourceRatio": Value::Null
            },
            "guardrails": {
                "forceDisengage": pending.force_disengage_guardrail,
                "laneContext": pending.lane_context_guardrail
            },
            "outcome": {
                "windowTicks": session.telemetry.config.outcome_window_ticks,
                "hpRatioDelta": hp_ratio_now - pending.base_hp_ratio,
                "enemyHpRatioDelta": enemy_hp_ratio_now - pending.enemy_hp_ratio,
                "deathDelta": deaths_now - pending.base_deaths,
                "killParticipationDelta": kills_assists_now - pending.base_kills_assists,
                "laneProgressDelta": lane_progress_now - pending.base_lane_progress,
                "alive": champion_now.alive
            }
        }));
    }

    session.telemetry.pending = still_pending;

    for candidate in runtime.telemetry_decisions.drain(..) {
        let telemetry_key = format!(
            "{}|{}|{}|{}",
            candidate.champion_id, candidate.enemy_id, candidate.intent, session.id
        );

        let sample_every = session.telemetry.config.sample_every_ticks.max(1);
        let last_tick = *session
            .telemetry
            .last_tick_by_key
            .get(&telemetry_key)
            .unwrap_or(&0);
        let enough_gap = now_tick == 0 || now_tick.saturating_sub(last_tick) >= sample_every;

        let last_decision = session
            .telemetry
            .last_decision_by_key
            .get(&telemetry_key)
            .copied();
        let changed = last_decision
            .map(|prev| prev != candidate.decision)
            .unwrap_or(true);
        let should_record = if session.telemetry.config.decision_change_only {
            changed || enough_gap
        } else {
            enough_gap
        };

        if !should_record {
            continue;
        }

        let Some(champion_now) = runtime
            .champions
            .iter()
            .find(|champion| champion.id == candidate.champion_id)
        else {
            continue;
        };

        session
            .telemetry
            .last_tick_by_key
            .insert(telemetry_key.clone(), now_tick);
        session
            .telemetry
            .last_decision_by_key
            .insert(telemetry_key, candidate.decision);

        let pending = PendingTelemetryOutcome {
            due_tick: now_tick.saturating_add(session.telemetry.config.outcome_window_ticks.max(1)),
            session_id: session.id.clone(),
            seed: session.seed.clone(),
            sampled_tick: now_tick,
            sampled_at_sec: runtime.time_sec,
            champion_id: candidate.champion_id,
            enemy_id: candidate.enemy_id,
            team: candidate.team,
            role: candidate.role,
            lane: candidate.lane,
            ai_mode: candidate.ai_mode.as_str().to_string(),
            intent: candidate.intent,
            decision: candidate.decision,
            rule_decision: candidate.rule_decision,
            confidence: candidate.confidence,
            flipped_by_hybrid: candidate.flipped_by_hybrid,
            hp_ratio: candidate.hp_ratio,
            enemy_hp_ratio: candidate.enemy_hp_ratio,
            ally_champions_local: candidate.ally_champions_local,
            enemy_champions_local: candidate.enemy_champions_local,
            ally_minions_local: candidate.ally_minions_local,
            enemy_minions_local: candidate.enemy_minions_local,
            nearest_enemy_tower_distance: candidate.nearest_enemy_tower_distance,
            enemy_overextended: candidate.enemy_overextended,
            objective_danger: candidate.objective_danger,
            gold: candidate.gold,
            xp: candidate.xp,
            level: candidate.level,
            force_disengage_guardrail: candidate.force_disengage_guardrail,
            lane_context_guardrail: candidate.lane_context_guardrail,
            base_hp_ratio: ratio_or_zero(champion_now.hp, champion_now.max_hp),
            base_deaths: champion_now.deaths,
            base_kills_assists: champion_now.kills + champion_now.assists,
            base_lane_progress: lane_progress_for_champion(champion_now),
        };
        session.telemetry.pending.push(pending);
    }

    if pending_to_write.is_empty() {
        return;
    }

    let Some(output_path) = session.telemetry.output_path.as_ref() else {
        return;
    };

    if let Some(parent) = output_path.parent() {
        let _ = create_dir_all(parent);
    }

    let mut file = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_path)
    {
        Ok(file) => file,
        Err(err) => {
            log_event(
                runtime,
                &format!("[telemetry] failed to open output file: {}", err),
                "info",
            );
            return;
        }
    };

    for line in pending_to_write {
        let serialized = match serde_json::to_string(&line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if writeln!(file, "{}", serialized).is_err() {
            log_event(
                runtime,
                "[telemetry] failed to append telemetry line",
                "info",
            );
            return;
        }
    }
}

fn ratio_or_zero(value: f64, max: f64) -> f64 {
    if max <= 0.0 {
        0.0
    } else {
        clamp(value / max, 0.0, 1.0)
    }
}

fn lane_progress_for_champion(champion: &ChampionRuntime) -> f64 {
    let path = lane_path_for(&champion.team, &champion.lane);
    if path.len() < 2 {
        return 0.0;
    }
    let idx = closest_lane_path_index(champion.pos, &path);
    idx as f64 / (path.len().saturating_sub(1)) as f64
}

fn default_runtime_state() -> Value {
    json!({
        "timeSec": 0.0,
        "running": true,
        "speed": 1.0,
        "aiMode": "hybrid",
        "winner": Value::Null,
        "showWalls": false,
        "champions": [],
        "minions": [],
        "structures": [],
        "objectives": {
            "dragon": { "key": "dragon", "pos": { "x": 0.673828125, "y": 0.703125 }, "alive": false, "nextSpawnAt": 300.0 },
            "baron": { "key": "baron", "pos": { "x": 0.3274739583333333, "y": 0.2981770833333333 }, "alive": false, "nextSpawnAt": 1200.0 }
        },
        "neutralTimers": {
            "dragonSoulUnlocked": false,
            "elderUnlocked": false,
            "entities": {}
        },
        "stats": {
            "blue": { "kills": 0, "towers": 0, "dragons": 0, "barons": 0, "gold": 2500 },
            "red": { "kills": 0, "towers": 0, "dragons": 0, "barons": 0, "gold": 2500 }
        },
        "events": [{ "t": 0.0, "text": "Match started", "type": "info" }],
        "teamTactics": {
            "blue": RuntimeTeamTactics::default(),
            "red": RuntimeTeamTactics::default()
        },
        "teamBuffs": {
            "blue": RuntimeTeamBuffState::default(),
            "red": RuntimeTeamBuffState::default()
        }
    })
}

fn ensure_runtime_state_defaults(state: &mut Value) {
    let Ok(root) = as_mut_object(state) else {
        *state = default_runtime_state();
        return;
    };

    if !root.contains_key("timeSec") {
        root.insert("timeSec".to_string(), json!(0.0));
    }
    if !root.contains_key("running") {
        root.insert("running".to_string(), json!(true));
    }
    if !root.contains_key("speed") {
        root.insert("speed".to_string(), json!(1.0));
    }
    if !root.contains_key("aiMode") {
        root.insert("aiMode".to_string(), json!("hybrid"));
    }
    if !root.contains_key("winner") {
        root.insert("winner".to_string(), Value::Null);
    }
    if !root.contains_key("showWalls") {
        root.insert("showWalls".to_string(), json!(false));
    }
    if !root.contains_key("champions") {
        root.insert("champions".to_string(), Value::Array(Vec::new()));
    }
    if !root.contains_key("minions") {
        root.insert("minions".to_string(), Value::Array(Vec::new()));
    }
    if !root.contains_key("structures") {
        root.insert("structures".to_string(), Value::Array(Vec::new()));
    }
    if !root.contains_key("objectives") {
        root.insert(
            "objectives".to_string(),
            json!({
                "dragon": { "key": "dragon", "pos": { "x": 0.673828125, "y": 0.703125 }, "alive": false, "nextSpawnAt": 300.0 },
                "baron": { "key": "baron", "pos": { "x": 0.3274739583333333, "y": 0.2981770833333333 }, "alive": false, "nextSpawnAt": 1200.0 }
            }),
        );
    }
    if !root.contains_key("neutralTimers") {
        root.insert(
            "neutralTimers".to_string(),
            json!({ "dragonSoulUnlocked": false, "elderUnlocked": false, "entities": {} }),
        );
    }
    if !root.contains_key("stats") {
        root.insert(
            "stats".to_string(),
            json!({
                "blue": { "kills": 0, "towers": 0, "dragons": 0, "barons": 0, "gold": 2500 },
                "red": { "kills": 0, "towers": 0, "dragons": 0, "barons": 0, "gold": 2500 }
            }),
        );
    }
    if !root.contains_key("events") {
        root.insert(
            "events".to_string(),
            Value::Array(vec![
                json!({ "t": 0.0, "text": "Match started", "type": "info" }),
            ]),
        );
    }
    if !root.contains_key("teamTactics") {
        root.insert(
            "teamTactics".to_string(),
            json!({
                "blue": RuntimeTeamTactics::default(),
                "red": RuntimeTeamTactics::default(),
            }),
        );
    }
    if !root.contains_key("teamBuffs") {
        root.insert(
            "teamBuffs".to_string(),
            json!({
                "blue": RuntimeTeamBuffState::default(),
                "red": RuntimeTeamBuffState::default(),
            }),
        );
    }
}

#[derive(Clone)]
struct SnapshotPlayer {
    id: String,
    name: String,
    dribbling: f64,
    agility: f64,
    pace: f64,
    composure: f64,
    shooting: f64,
    positioning: f64,
    teamwork: f64,
    stamina: f64,
    decisions: f64,
    vision: f64,
    passing: f64,
    leadership: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Vec2 {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeState {
    time_sec: f64,
    running: bool,
    speed: f64,
    #[serde(default)]
    ai_mode: SimulatorAiMode,
    #[serde(default, skip)]
    policy: SimulatorPolicyConfig,
    winner: Option<String>,
    show_walls: bool,
    champions: Vec<ChampionRuntime>,
    minions: Vec<MinionRuntime>,
    structures: Vec<StructureRuntime>,
    #[serde(default)]
    wards: Vec<WardRuntime>,
    objectives: Value,
    neutral_timers: Value,
    stats: RuntimeStats,
    events: Vec<RuntimeEvent>,
    #[serde(default, skip)]
    lane_combat_state_by_champion: HashMap<String, LanerCombatStateRuntime>,
    #[serde(default, skip)]
    telemetry_decisions: Vec<TelemetryDecisionCandidate>,
    #[serde(default)]
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
struct TelemetryDecisionCandidate {
    champion_id: String,
    team: String,
    role: String,
    lane: String,
    enemy_id: String,
    ai_mode: SimulatorAiMode,
    intent: String,
    decision: bool,
    rule_decision: bool,
    confidence: f64,
    flipped_by_hybrid: bool,
    hp_ratio: f64,
    enemy_hp_ratio: f64,
    ally_champions_local: usize,
    enemy_champions_local: usize,
    ally_minions_local: usize,
    enemy_minions_local: usize,
    nearest_enemy_tower_distance: f64,
    enemy_overextended: bool,
    objective_danger: f64,
    gold: i64,
    xp: i64,
    level: i64,
    force_disengage_guardrail: bool,
    lane_context_guardrail: bool,
}

#[derive(Debug, Clone)]
struct PendingTelemetryOutcome {
    due_tick: u64,
    session_id: String,
    seed: String,
    sampled_tick: u64,
    sampled_at_sec: f64,
    champion_id: String,
    enemy_id: String,
    team: String,
    role: String,
    lane: String,
    ai_mode: String,
    intent: String,
    decision: bool,
    rule_decision: bool,
    confidence: f64,
    flipped_by_hybrid: bool,
    hp_ratio: f64,
    enemy_hp_ratio: f64,
    ally_champions_local: usize,
    enemy_champions_local: usize,
    ally_minions_local: usize,
    enemy_minions_local: usize,
    nearest_enemy_tower_distance: f64,
    enemy_overextended: bool,
    objective_danger: f64,
    gold: i64,
    xp: i64,
    level: i64,
    force_disengage_guardrail: bool,
    lane_context_guardrail: bool,
    base_hp_ratio: f64,
    base_deaths: i64,
    base_kills_assists: i64,
    base_lane_progress: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeStats {
    blue: RuntimeTeamStats,
    red: RuntimeTeamStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeTeamStats {
    kills: i64,
    towers: i64,
    dragons: i64,
    barons: i64,
    gold: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeEvent {
    t: f64,
    text: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WardRuntime {
    id: String,
    team: String,
    owner_champion_id: String,
    pos: Vec2,
    expires_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeSummonerSpellSlot {
    key: String,
    cd_until: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeUltimateSlot {
    archetype: String,
    #[serde(default)]
    icon: String,
    cd_until: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChampionRuntime {
    id: String,
    name: String,
    #[serde(default)]
    champion_id: String,
    team: String,
    role: String,
    lane: String,
    pos: Vec2,
    hp: f64,
    max_hp: f64,
    alive: bool,
    respawn_at: f64,
    attack_cd_until: f64,
    move_speed: f64,
    attack_range: f64,
    attack_type: String,
    attack_damage: f64,
    target_path: Vec<Vec2>,
    target_path_index: usize,
    next_decision_at: f64,
    kills: i64,
    deaths: i64,
    assists: i64,
    gold: i64,
    #[serde(default)]
    spent_gold: i64,
    xp: i64,
    level: i64,
    #[serde(default)]
    cs: i64,
    #[serde(default)]
    has_left_base_once: bool,
    #[serde(default)]
    last_support_cs_at: f64,
    #[serde(default)]
    items: Vec<String>,
    #[serde(default = "default_visible_stat")]
    gameplay_score: f64,
    #[serde(default = "default_visible_stat")]
    iq_score: f64,
    #[serde(default = "default_visible_stat")]
    competitive_score: f64,
    #[serde(default)]
    summoner_spells: Vec<RuntimeSummonerSpellSlot>,
    #[serde(default)]
    ultimate: Option<RuntimeUltimateSlot>,
    #[serde(default)]
    ignite_dot_until: f64,
    #[serde(default)]
    ignite_source_id: Option<String>,
    last_damaged_by_champion_id: Option<String>,
    last_damaged_at: f64,
    state: String,
    recall_anchor: Option<Vec2>,
    recall_channel_until: f64,
    #[serde(default)]
    realm_banished_until: f64,
    #[serde(default)]
    realm_return_pos: Option<Vec2>,
    #[serde(default)]
    ward_cd_until: f64,
    #[serde(default)]
    sweeper_cd_until: f64,
    #[serde(default)]
    sweeper_active_until: f64,
    #[serde(default)]
    trinket_key: String,
    #[serde(default)]
    trinket_swapped: bool,
    #[serde(default)]
    support_roam_uses: i64,
    #[serde(default)]
    support_roam_cd_until: f64,
    #[serde(default)]
    support_last_roam_role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MinionRuntime {
    id: String,
    team: String,
    lane: String,
    pos: Vec2,
    hp: f64,
    max_hp: f64,
    alive: bool,
    kind: String,
    last_hit_by_champion_id: Option<String>,
    #[serde(default)]
    owner_champion_id: Option<String>,
    #[serde(default)]
    summon_kind: Option<String>,
    #[serde(default)]
    summon_expires_at: f64,
    attack_cd_until: f64,
    move_speed: f64,
    attack_range: f64,
    attack_damage: f64,
    path: Vec<Vec2>,
    path_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StructureRuntime {
    id: String,
    team: String,
    lane: String,
    kind: String,
    pos: Vec2,
    hp: f64,
    max_hp: f64,
    alive: bool,
    attack_cd_until: f64,
    #[serde(default)]
    forced_target_champion_id: Option<String>,
    #[serde(default)]
    forced_target_until: f64,
}

#[derive(Clone, Copy)]
struct RoleSeed {
    role: &'static str,
    lane: &'static str,
    offset: Vec2,
}

#[derive(Clone, Copy)]
struct StructureSeed {
    id: &'static str,
    team: &'static str,
    lane: &'static str,
    kind: &'static str,
    pos: Vec2,
}

#[derive(Clone, Copy)]
struct NeutralTimerTemplate {
    key: &'static str,
    label: &'static str,
    first_spawn_at: f64,
    max_hp: f64,
    respawn_delay_sec: Option<f64>,
    one_shot: bool,
    window_close_at: Option<f64>,
    combat_grace_until: Option<f64>,
    unlocked: bool,
    pos: Vec2,
}

#[derive(Clone, Copy)]
struct ItemTemplate {
    key: &'static str,
    cost: i64,
    attack_damage: f64,
    max_hp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeTeamTactics {
    strong_side: String,
    game_timing: String,
    jungle_style: String,
    jungle_pathing: String,
    fight_plan: String,
    support_roaming: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeRoleImpact {
    modifier: f64,
    variance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeTeamBuffState {
    baron_until: f64,
    elder_until: f64,
    infernal_stacks: i64,
    mountain_stacks: i64,
    ocean_stacks: i64,
    cloud_stacks: i64,
    hextech_stacks: i64,
    chemtech_stacks: i64,
    #[serde(default)]
    voidgrub_stacks: i64,
    dragon_stacks: i64,
    #[serde(default)]
    dragon_history: Vec<String>,
    soul_kind: Option<String>,
}

impl Default for RuntimeTeamBuffState {
    fn default() -> Self {
        Self {
            baron_until: 0.0,
            elder_until: 0.0,
            infernal_stacks: 0,
            mountain_stacks: 0,
            ocean_stacks: 0,
            cloud_stacks: 0,
            hextech_stacks: 0,
            chemtech_stacks: 0,
            voidgrub_stacks: 0,
            dragon_stacks: 0,
            dragon_history: Vec::new(),
            soul_kind: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RuntimeBuffState {
    blue: RuntimeTeamBuffState,
    red: RuntimeTeamBuffState,
}

impl Default for RuntimeTeamTactics {
    fn default() -> Self {
        Self {
            strong_side: "Bot".to_string(),
            game_timing: "Mid".to_string(),
            jungle_style: "Enabler".to_string(),
            jungle_pathing: "TopToBot".to_string(),
            fight_plan: "FrontToBack".to_string(),
            support_roaming: "Lane".to_string(),
        }
    }
}

#[derive(Clone, Copy)]
enum ItemBuildCategory {
    Tank,
    Bruiser,
    Colossus,
    AssassinAd,
    AssassinAp,
    ControlMage,
    BattleMage,
    AdcCrit,
    AdcAttackSpeed,
    LethalityMarksman,
    SupportEngage,
    SupportEnchanter,
    SupportDamage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NeutralTimersRuntime {
    dragon_soul_unlocked: bool,
    elder_unlocked: bool,
    entities: HashMap<String, NeutralTimerRuntime>,
    #[serde(default)]
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NeutralTimerRuntime {
    key: String,
    label: String,
    alive: bool,
    hp: f64,
    max_hp: f64,
    next_spawn_at: Option<f64>,
    first_spawn_at: f64,
    respawn_delay_sec: Option<f64>,
    one_shot: bool,
    window_close_at: Option<f64>,
    combat_grace_until: Option<f64>,
    unlocked: bool,
    last_spawn_at: Option<f64>,
    last_taken_at: Option<f64>,
    #[serde(default)]
    times_spawned: i64,
    #[serde(default)]
    times_taken: i64,
    pos: Vec2,
    #[serde(default)]
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct WallFile {
    walls: Vec<WallPolygon>,
}

#[derive(Debug, Clone, Deserialize)]
struct WallPolygon {
    id: String,
    #[serde(default)]
    closed: bool,
    points: Vec<Vec2>,
}

#[derive(Debug, Clone)]
struct NavGrid {
    grid_size: usize,
    blocked: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
struct GridCell {
    cx: usize,
    cy: usize,
}

const BASE_POSITION_BLUE: Vec2 = Vec2 { x: 0.115, y: 0.882 };
const BASE_POSITION_RED: Vec2 = Vec2 { x: 0.891, y: 0.117 };

const ROLE_SEEDS: [RoleSeed; 5] = [
    RoleSeed {
        role: "TOP",
        lane: "top",
        offset: Vec2 {
            x: -0.014,
            y: -0.012,
        },
    },
    RoleSeed {
        role: "JGL",
        lane: "bot",
        offset: Vec2 { x: 0.014, y: -0.01 },
    },
    RoleSeed {
        role: "MID",
        lane: "mid",
        offset: Vec2 { x: 0.011, y: 0.011 },
    },
    RoleSeed {
        role: "ADC",
        lane: "bot",
        offset: Vec2 {
            x: -0.012,
            y: 0.018,
        },
    },
    RoleSeed {
        role: "SUP",
        lane: "bot",
        offset: Vec2 { x: 0.004, y: 0.021 },
    },
];

// TODO(parity-chunk-b): source these from a shared Rust map/layout module instead of duplicated static data.
const STRUCTURE_LAYOUT: [StructureSeed; 30] = [
    StructureSeed {
        id: "blue-top-outer",
        team: "blue",
        lane: "top",
        kind: "tower",
        pos: Vec2 {
            x: 0.072265625,
            y: 0.2838541666666667,
        },
    },
    StructureSeed {
        id: "blue-top-inner",
        team: "blue",
        lane: "top",
        kind: "tower",
        pos: Vec2 {
            x: 0.099609375,
            y: 0.5533854166666666,
        },
    },
    StructureSeed {
        id: "blue-top-inhib-tower",
        team: "blue",
        lane: "top",
        kind: "tower",
        pos: Vec2 {
            x: 0.09049479166666667,
            y: 0.69921875,
        },
    },
    StructureSeed {
        id: "blue-mid-outer",
        team: "blue",
        lane: "mid",
        kind: "tower",
        pos: Vec2 {
            x: 0.4016927083333333,
            y: 0.5755208333333334,
        },
    },
    StructureSeed {
        id: "blue-mid-inner",
        team: "blue",
        lane: "mid",
        kind: "tower",
        pos: Vec2 {
            x: 0.3470052083333333,
            y: 0.6705729166666666,
        },
    },
    StructureSeed {
        id: "blue-mid-inhib-tower",
        team: "blue",
        lane: "mid",
        kind: "tower",
        pos: Vec2 {
            x: 0.2623697916666667,
            y: 0.7408854166666666,
        },
    },
    StructureSeed {
        id: "blue-bot-inner",
        team: "blue",
        lane: "bot",
        kind: "tower",
        pos: Vec2 {
            x: 0.4720052083333333,
            y: 0.8958333333333334,
        },
    },
    StructureSeed {
        id: "blue-bot-outer",
        team: "blue",
        lane: "bot",
        kind: "tower",
        pos: Vec2 {
            x: 0.720703125,
            y: 0.9231770833333334,
        },
    },
    StructureSeed {
        id: "blue-bot-inhib-tower",
        team: "blue",
        lane: "bot",
        kind: "tower",
        pos: Vec2 {
            x: 0.298828125,
            y: 0.9127604166666666,
        },
    },
    StructureSeed {
        id: "blue-inhib-top",
        team: "blue",
        lane: "base",
        kind: "inhib",
        pos: Vec2 {
            x: 0.08658854166666667,
            y: 0.7591145833333334,
        },
    },
    StructureSeed {
        id: "blue-inhib-mid",
        team: "blue",
        lane: "base",
        kind: "inhib",
        pos: Vec2 {
            x: 0.224609375,
            y: 0.7864583333333334,
        },
    },
    StructureSeed {
        id: "blue-inhib-bot",
        team: "blue",
        lane: "base",
        kind: "inhib",
        pos: Vec2 {
            x: 0.24544270833333334,
            y: 0.9114583333333334,
        },
    },
    StructureSeed {
        id: "blue-nexus-top-tower",
        team: "blue",
        lane: "base",
        kind: "tower",
        pos: Vec2 {
            x: 0.126953125,
            y: 0.8372395833333334,
        },
    },
    StructureSeed {
        id: "blue-nexus-bot-tower",
        team: "blue",
        lane: "base",
        kind: "tower",
        pos: Vec2 {
            x: 0.15950520833333334,
            y: 0.875,
        },
    },
    StructureSeed {
        id: "blue-nexus",
        team: "blue",
        lane: "base",
        kind: "nexus",
        pos: Vec2 {
            x: 0.115234375,
            y: 0.8815104166666666,
        },
    },
    StructureSeed {
        id: "red-top-outer",
        team: "red",
        lane: "top",
        kind: "tower",
        pos: Vec2 {
            x: 0.275390625,
            y: 0.07161458333333333,
        },
    },
    StructureSeed {
        id: "red-top-inner",
        team: "red",
        lane: "top",
        kind: "tower",
        pos: Vec2 {
            x: 0.533203125,
            y: 0.08203125,
        },
    },
    StructureSeed {
        id: "red-top-inhib-tower",
        team: "red",
        lane: "top",
        kind: "tower",
        pos: Vec2 {
            x: 0.912109375,
            y: 0.3125,
        },
    },
    StructureSeed {
        id: "red-mid-outer",
        team: "red",
        lane: "mid",
        kind: "tower",
        pos: Vec2 {
            x: 0.595703125,
            y: 0.44140625,
        },
    },
    StructureSeed {
        id: "red-mid-inner",
        team: "red",
        lane: "mid",
        kind: "tower",
        pos: Vec2 {
            x: 0.6569010416666666,
            y: 0.33203125,
        },
    },
    StructureSeed {
        id: "red-mid-inhib-tower",
        team: "red",
        lane: "mid",
        kind: "tower",
        pos: Vec2 {
            x: 0.740234375,
            y: 0.26171875,
        },
    },
    StructureSeed {
        id: "red-bot-inner",
        team: "red",
        lane: "bot",
        kind: "tower",
        pos: Vec2 {
            x: 0.9016927083333334,
            y: 0.44921875,
        },
    },
    StructureSeed {
        id: "red-bot-outer",
        team: "red",
        lane: "bot",
        kind: "tower",
        pos: Vec2 {
            x: 0.9303385416666666,
            y: 0.7057291666666666,
        },
    },
    StructureSeed {
        id: "red-bot-inhib-tower",
        team: "red",
        lane: "bot",
        kind: "tower",
        pos: Vec2 {
            x: 0.7024739583333334,
            y: 0.09375,
        },
    },
    StructureSeed {
        id: "red-inhib-top",
        team: "red",
        lane: "base",
        kind: "inhib",
        pos: Vec2 {
            x: 0.7545572916666666,
            y: 0.09114583333333333,
        },
    },
    StructureSeed {
        id: "red-inhib-mid",
        team: "red",
        lane: "base",
        kind: "inhib",
        pos: Vec2 {
            x: 0.783203125,
            y: 0.22395833333333334,
        },
    },
    StructureSeed {
        id: "red-inhib-bot",
        team: "red",
        lane: "base",
        kind: "inhib",
        pos: Vec2 {
            x: 0.9108072916666666,
            y: 0.24869791666666666,
        },
    },
    StructureSeed {
        id: "red-nexus-top-tower",
        team: "red",
        lane: "base",
        kind: "tower",
        pos: Vec2 {
            x: 0.845703125,
            y: 0.1328125,
        },
    },
    StructureSeed {
        id: "red-nexus-bot-tower",
        team: "red",
        lane: "base",
        kind: "tower",
        pos: Vec2 {
            x: 0.8717447916666666,
            y: 0.1640625,
        },
    },
    StructureSeed {
        id: "red-nexus",
        team: "red",
        lane: "base",
        kind: "nexus",
        pos: Vec2 {
            x: 0.8912760416666666,
            y: 0.1171875,
        },
    },
];

const MINION_FIRST_WAVE_AT: f64 = 30.0;
const LANE_COMBAT_UNLOCK_AT: f64 = MINION_FIRST_WAVE_AT + 8.0;
const FIRST_WAVE_CONTEST_UNTIL: f64 = MINION_FIRST_WAVE_AT + 45.0;
const CHAMPION_DECISION_CADENCE_SEC: f64 = 0.8;
const MINION_DAMAGE_TO_MINION_MULTIPLIER: f64 = 0.52;
const MINION_DAMAGE_TO_CHAMPION_MULTIPLIER: f64 = 0.44;
const CHAMPION_DAMAGE_TO_MINION_MULTIPLIER: f64 = 0.6;
const RECALL_TRIGGER_HP_RATIO: f64 = 0.34;
const RECALL_CHANNEL_SEC: f64 = 6.5;
const RECALL_REACH_BUFFER_SEC: f64 = 0.8;
const RECALL_SAFE_ENEMY_RADIUS: f64 = 0.2;
const LANE_CHAMPION_TRADE_RADIUS: f64 = 0.19;
const LANE_REENGAGE_COOLDOWN_SEC: f64 = 2.8;
const LANE_RECENT_TRADE_LOCK_SEC: f64 = 1.7;
const TRADE_HP_DISADVANTAGE_ALLOWANCE: f64 = 0.2;
const LANE_LOCAL_PRESSURE_RADIUS: f64 = 0.1;
const LANE_MINION_CONTEXT_RADIUS: f64 = 0.105;
const LANE_CHASE_MINION_CONTEXT_RADIUS: f64 = 0.12;
const LOCAL_COMBAT_ENGAGE_RADIUS: f64 = 0.16;
const LOCAL_STRUCTURE_ENGAGE_RADIUS: f64 = 0.12;
const LANE_STRUCTURE_PRESSURE_RADIUS: f64 = 0.12;
const LANE_HEALTHY_RETREAT_HP_RATIO: f64 = 0.6;
const LANE_STRONG_UNFAVORABLE_PRESSURE_DELTA: f64 = 0.7;
const LANE_EMPTY_ANCHOR_PROGRESS_MAX_INDEX: usize = 4;
const HYBRID_TRADE_DEBUG_LOG_COOLDOWN_SEC: f64 = 8.0;
const TRADE_SCORE_WEIGHT_BIAS: f64 = -0.18;
const TRADE_SCORE_WEIGHT_SELF_HP: f64 = 1.55;
const TRADE_SCORE_WEIGHT_ENEMY_HP: f64 = -1.45;
const TRADE_SCORE_WEIGHT_CHAMP_NUMBERS: f64 = 0.62;
const TRADE_SCORE_WEIGHT_MINION_NUMBERS: f64 = 0.38;
const TRADE_SCORE_WEIGHT_TOWER_DISTANCE: f64 = 0.56;
const TRADE_SCORE_WEIGHT_ENEMY_OVEREXTENDED: f64 = 0.74;
const TRADE_SCORE_WEIGHT_FIRST_WAVE: f64 = -0.22;
const ASSIST_RADIUS: f64 = 0.11;
const CHAMPION_KILL_GOLD: i64 = 300;
const CHAMPION_ASSIST_GOLD_TOTAL: i64 = 150;
const CHAMPION_KILL_XP: i64 = 220;
const CHAMPION_KILL_GOLD_MIN: i64 = 170;
const CHAMPION_KILL_GOLD_MAX: i64 = 650;
const CHAMPION_KILL_XP_MIN: i64 = 150;
const CHAMPION_KILL_XP_MAX: i64 = 360;
const CHAMPION_RESPAWN_BASE_SEC: f64 = 14.0;
const CHAMPION_RESPAWN_PER_LEVEL_SEC: f64 = 1.35;
const BARON_BUFF_DURATION_SEC: f64 = 180.0;
const ELDER_BUFF_DURATION_SEC: f64 = 150.0;
const ELDER_EXECUTE_HP_RATIO: f64 = 0.20;
const BARON_MINION_AURA_RADIUS: f64 = 0.12;
const BARON_MINION_DAMAGE_MULTIPLIER: f64 = 1.35;
const BARON_MINION_DAMAGE_REDUCTION: f64 = 0.35;
const CHAMPION_MAX_LEVEL: i64 = 18;
const CHAMPION_LEVEL_UP_HP_GAIN: f64 = 92.0;
const CHAMPION_LEVEL_UP_AD_GAIN: f64 = 3.8;
const TOWER_OUTER_HP: f64 = 5000.0;
const TOWER_INNER_HP: f64 = 3600.0;
const TOWER_INHIB_HP: f64 = 3400.0;
const TOWER_NEXUS_HP: f64 = 2700.0;
const INHIBITOR_HP: f64 = 4000.0;
const NEXUS_HP: f64 = 5500.0;
const EARLY_TOWER_FORTIFICATION_END_AT: f64 = 14.0 * 60.0;
const EARLY_TOWER_DAMAGE_REDUCTION: f64 = 0.45;
const CHAMPION_ATTACK_CADENCE_SEC: f64 = 0.85;
const TOWER_SHOT_DAMAGE: f64 = 32.0;
const TOWER_SHOT_DAMAGE_TO_MINION: f64 = 24.0;
const TOWER_ATTACK_RANGE: f64 = 0.08;
const TOWER_ATTACK_CADENCE_SEC: f64 = 1.0;
const TOWER_AGGRO_LOCK_SEC: f64 = 2.6;
const TOWER_AGGRO_VICTIM_RADIUS: f64 = 0.09;
const TOWER_AGGRO_ATTACKER_RADIUS: f64 = 0.10;
const EVENT_CAP: usize = 200;
const SKIP_FAST_MODE_EXTRA_KEY: &str = "skipFastMode";
const MINION_MELEE_MAX_HP: f64 = 118.0;
const MINION_MELEE_MOVE_SPEED: f64 = 0.068;
const MINION_MELEE_ATTACK_RANGE: f64 = 0.035;
const MINION_MELEE_ATTACK_DAMAGE: f64 = 6.0;
const MINION_MELEE_ATTACK_CADENCE: f64 = 1.05;
const MINION_RANGED_MAX_HP: f64 = 92.0;
const MINION_RANGED_MOVE_SPEED: f64 = 0.071;
const MINION_RANGED_ATTACK_RANGE: f64 = 0.055;
const MINION_RANGED_ATTACK_DAMAGE: f64 = 7.0;
const MINION_RANGED_ATTACK_CADENCE: f64 = 1.14;
const MINION_STRUCTURE_AGGRO_RANGE: f64 = 0.05;
const MINION_CHAMPION_AGGRO_MIN_RANGE: f64 = 0.055;
const JUNGLE_INITIAL_SPAWN_AT: f64 = MINION_FIRST_WAVE_AT;
const SCUTTLE_INITIAL_SPAWN_AT: f64 = 210.0;
const JUNGLE_CAMP_ENGAGE_RADIUS: f64 = 0.09;
const OBJECTIVE_ATTEMPT_RADIUS: f64 = 0.12;
const OBJECTIVE_ASSIST_RADIUS: f64 = 0.24;
const MAJOR_OBJECTIVE_TEAM_ASSIST_RADIUS: f64 = 0.52;
const BASE_DEFENSE_RECALL_DISTANCE: f64 = 0.34;
const NEXUS_DEFENSE_THREAT_RADIUS: f64 = 0.13;
const ALLY_HELP_RADIUS: f64 = 0.17;
const ALLY_HELP_DAMAGE_RECENT_SEC: f64 = 3.2;
const OFFROLE_JUNGLE_REWARD_MULTIPLIER: f64 = 0.65;
const JGL_JUNGLE_GOLD_MULTIPLIER: f64 = 0.78;
const JGL_JUNGLE_XP_MULTIPLIER: f64 = 0.90;
const OBJECTIVE_PATH_MIN_TARGET_DELTA: f64 = 0.014;
const JUNGLE_DISENGAGE_THREAT_AVOID_RADIUS: f64 = 0.1;
const VOIDGRUBS_SOFT_CLOSE_AT: f64 = 14.0 * 60.0 + 45.0;
const VOIDGRUBS_HARD_CLOSE_AT: f64 = 14.0 * 60.0 + 55.0;
const HERALD_SOFT_CLOSE_AT: f64 = 19.0 * 60.0 + 45.0;
const HERALD_HARD_CLOSE_AT: f64 = 19.0 * 60.0 + 55.0;
const DRAGON_SECURE_GOLD: i64 = 55;
const DRAGON_SECURE_XP: i64 = 110;
const BARON_SECURE_GOLD: i64 = 80;
const BARON_SECURE_XP: i64 = 140;
const OBJECTIVE_SECURE_GOLD: i64 = 45;
const OBJECTIVE_SECURE_XP: i64 = 90;
const VOIDGRUB_TOWER_DAMAGE_PER_STACK: f64 = 0.03;
const VOIDGRUB_TOWER_DAMAGE_MAX: f64 = 0.09;
const OBJECTIVE_NEXT_SPAWN_FALLBACK: f64 = 9_999_999.0;
const NAV_GRID_SIZE: usize = 120;
const NAV_PATH_MIN_DIRECT_DIST: f64 = 0.012;
const NAV_PATH_TRIVIAL_NODE_EPSILON: f64 = 0.0095;
const ITEM_COST_MULTIPLIER: f64 = 0.32;
const ITEM_COST_MIN: i64 = 300;
const SUPPORT_CS_MIN_INTERVAL_SEC: f64 = 24.0;
const SUPPORT_ROAM_UNLOCK_AT_SEC: f64 = 15.0 * 60.0;
const SUPPORT_OPEN_ROAM_AT_SEC: f64 = 15.0 * 60.0;
const SUMMONER_FLASH_CD_SEC: f64 = 300.0;
const SUMMONER_IGNITE_CD_SEC: f64 = 180.0;
const SUMMONER_HEAL_CD_SEC: f64 = 240.0;
const SUMMONER_SMITE_CD_SEC: f64 = 90.0;
const SUMMONER_TP_CD_SEC: f64 = 300.0;
const SUMMONER_TP_UNLOCK_AT_SEC: f64 = 6.0 * 60.0;
const SUMMONER_FLASH_RANGE: f64 = 0.085;
const SUMMONER_IGNITE_RANGE: f64 = 0.072;
const SUMMONER_IGNITE_DURATION_SEC: f64 = 5.0;
const SUMMONER_IGNITE_DPS: f64 = 18.0;
const SUMMONER_HEAL_RADIUS: f64 = 0.085;
const SUMMONER_HEAL_SELF_RATIO: f64 = 0.22;
const SUMMONER_HEAL_ALLY_RATIO: f64 = 0.18;
const SUMMONER_SMITE_RANGE: f64 = 0.095;
const SUMMONER_SMITE_DAMAGE: f64 = 600.0;
const ULTIMATE_UNLOCK_LEVEL: i64 = 6;
const ULTIMATE_BASE_CD_SEC: f64 = 120.0;
const ULTIMATE_BURST_RANGE: f64 = 0.085;
const ULTIMATE_GLOBAL_RANGE: f64 = 0.18;
const ULTIMATE_MORDE_REALM_DURATION_SEC: f64 = 30.0;
const ULTIMATE_SUMMON_DAMAGE_RATIO: f64 = 0.45;
const ULTIMATE_SUMMON_HP_RATIO: f64 = 0.5;
const WARD_UNLOCK_AT_SEC: f64 = 90.0;
const WARD_DURATION_SEC: f64 = 95.0;
const WARD_COOLDOWN_SEC: f64 = 120.0;
const WARD_VISION_RADIUS: f64 = 0.18;
const CHAMPION_VISION_RADIUS: f64 = 0.145;
const MINION_VISION_RADIUS: f64 = 0.10;
const STRUCTURE_VISION_RADIUS: f64 = 0.16;
const SWEEPER_COOLDOWN_SEC: f64 = 95.0;
const SWEEPER_DURATION_SEC: f64 = 10.0;
const SWEEPER_CLEAR_RADIUS: f64 = 0.145;
const TRINKET_SWAP_UNLOCK_AT_SEC: f64 = 6.0 * 60.0;
const TRINKET_WARDING_TOTEM: &str = "WardingTotem";
const TRINKET_ORACLE_LENS: &str = "OracleLens";

fn summon_profile(champion_key: &str) -> (&'static str, f64, f64, f64) {
    match champion_key {
        "yorick" => ("maiden", 0.55, 0.50, 45.0),
        "annie" => ("tibbers", 0.50, 0.52, 45.0),
        "ivern" => ("daisy", 0.58, 0.44, 60.0),
        "shaco" => ("clone", 0.45, 0.48, 20.0),
        _ => (
            "summon",
            ULTIMATE_SUMMON_HP_RATIO,
            ULTIMATE_SUMMON_DAMAGE_RATIO,
            35.0,
        ),
    }
}

const LEVEL_XP_THRESHOLDS: [i64; 18] = [
    0, 280, 660, 1080, 1560, 2100, 2700, 3360, 4080, 4860, 5700, 6600, 7560, 8580, 9660, 10800,
    12000, 13260,
];

const TANK_ITEM_PLAN: [ItemTemplate; 6] = [
    ItemTemplate {
        key: "sunfire_aegis",
        cost: 2700,
        attack_damage: 10.0,
        max_hp: 350.0,
    },
    ItemTemplate {
        key: "warmogs_armor",
        cost: 3100,
        attack_damage: 0.0,
        max_hp: 1000.0,
    },
    ItemTemplate {
        key: "iceborn_gauntlet",
        cost: 2900,
        attack_damage: 18.0,
        max_hp: 300.0,
    },
    ItemTemplate {
        key: "randuins_omen",
        cost: 3000,
        attack_damage: 0.0,
        max_hp: 350.0,
    },
    ItemTemplate {
        key: "spirit_visage",
        cost: 2900,
        attack_damage: 0.0,
        max_hp: 450.0,
    },
    ItemTemplate {
        key: "plated_steelcaps",
        cost: 1200,
        attack_damage: 0.0,
        max_hp: 0.0,
    },
];

const BRUISER_ITEM_PLAN: [ItemTemplate; 6] = [
    ItemTemplate {
        key: "sundered_sky",
        cost: 3100,
        attack_damage: 40.0,
        max_hp: 300.0,
    },
    ItemTemplate {
        key: "deaths_dance",
        cost: 3300,
        attack_damage: 55.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "steraks_gage",
        cost: 3200,
        attack_damage: 32.0,
        max_hp: 450.0,
    },
    ItemTemplate {
        key: "titanic_hydra",
        cost: 3300,
        attack_damage: 42.0,
        max_hp: 550.0,
    },
    ItemTemplate {
        key: "maw_of_malmortius",
        cost: 3100,
        attack_damage: 50.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "mercurys_treads",
        cost: 1250,
        attack_damage: 0.0,
        max_hp: 0.0,
    },
];

const COLOSSUS_ITEM_PLAN: [ItemTemplate; 6] = [
    ItemTemplate {
        key: "black_cleaver",
        cost: 3000,
        attack_damage: 40.0,
        max_hp: 400.0,
    },
    ItemTemplate {
        key: "steraks_gage",
        cost: 3200,
        attack_damage: 32.0,
        max_hp: 450.0,
    },
    ItemTemplate {
        key: "hullbreaker",
        cost: 3000,
        attack_damage: 40.0,
        max_hp: 500.0,
    },
    ItemTemplate {
        key: "titanic_hydra",
        cost: 3300,
        attack_damage: 42.0,
        max_hp: 550.0,
    },
    ItemTemplate {
        key: "dead_mans_plate",
        cost: 2900,
        attack_damage: 10.0,
        max_hp: 350.0,
    },
    ItemTemplate {
        key: "plated_steelcaps",
        cost: 1200,
        attack_damage: 0.0,
        max_hp: 0.0,
    },
];

const ASSASSIN_AD_ITEM_PLAN: [ItemTemplate; 6] = [
    ItemTemplate {
        key: "voltaic_cyclosword",
        cost: 2900,
        attack_damage: 55.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "opportunity",
        cost: 2700,
        attack_damage: 55.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "immortal_shieldbow",
        cost: 3000,
        attack_damage: 50.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "seryldas_grudge",
        cost: 3200,
        attack_damage: 45.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "profane_hydra",
        cost: 3300,
        attack_damage: 60.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "boots_of_swiftness",
        cost: 1000,
        attack_damage: 0.0,
        max_hp: 0.0,
    },
];

const ASSASSIN_AP_ITEM_PLAN: [ItemTemplate; 6] = [
    ItemTemplate {
        key: "stormsurge",
        cost: 2900,
        attack_damage: 36.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "lich_bane",
        cost: 3200,
        attack_damage: 32.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "shadowflame",
        cost: 3200,
        attack_damage: 35.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "zhonyas_hourglass",
        cost: 3250,
        attack_damage: 25.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "rabadons_deathcap",
        cost: 3600,
        attack_damage: 45.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "sorcerers_shoes",
        cost: 1100,
        attack_damage: 0.0,
        max_hp: 0.0,
    },
];

const CONTROL_MAGE_ITEM_PLAN: [ItemTemplate; 6] = [
    ItemTemplate {
        key: "ludens_companion",
        cost: 2900,
        attack_damage: 35.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "void_staff",
        cost: 3000,
        attack_damage: 30.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "zhonyas_hourglass",
        cost: 3250,
        attack_damage: 25.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "seraphs_embrace",
        cost: 3000,
        attack_damage: 28.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "rabadons_deathcap",
        cost: 3600,
        attack_damage: 45.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "sorcerers_shoes",
        cost: 1100,
        attack_damage: 0.0,
        max_hp: 0.0,
    },
];

const BATTLE_MAGE_ITEM_PLAN: [ItemTemplate; 6] = [
    ItemTemplate {
        key: "liandrys_torment",
        cost: 3000,
        attack_damage: 33.0,
        max_hp: 300.0,
    },
    ItemTemplate {
        key: "rylais_crystal_scepter",
        cost: 2600,
        attack_damage: 25.0,
        max_hp: 400.0,
    },
    ItemTemplate {
        key: "seraphs_embrace",
        cost: 3000,
        attack_damage: 28.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "cosmic_drive",
        cost: 3000,
        attack_damage: 30.0,
        max_hp: 350.0,
    },
    ItemTemplate {
        key: "zhonyas_hourglass",
        cost: 3250,
        attack_damage: 25.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "mercurys_treads",
        cost: 1250,
        attack_damage: 0.0,
        max_hp: 0.0,
    },
];

const ADC_CRIT_ITEM_PLAN: [ItemTemplate; 6] = [
    ItemTemplate {
        key: "bloodthirster",
        cost: 3400,
        attack_damage: 70.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "infinity_edge",
        cost: 3400,
        attack_damage: 65.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "mortal_reminder",
        cost: 3200,
        attack_damage: 40.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "rapid_firecannon",
        cost: 2600,
        attack_damage: 24.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "phantom_dancer",
        cost: 2600,
        attack_damage: 24.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "berserkers_greaves",
        cost: 1100,
        attack_damage: 0.0,
        max_hp: 0.0,
    },
];

const ADC_ATTACK_SPEED_ITEM_PLAN: [ItemTemplate; 6] = [
    ItemTemplate {
        key: "blade_of_the_ruined_king",
        cost: 3200,
        attack_damage: 42.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "wits_end",
        cost: 2900,
        attack_damage: 34.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "runaans_hurricane",
        cost: 2650,
        attack_damage: 24.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "guinsoos_rageblade",
        cost: 3000,
        attack_damage: 36.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "terminus",
        cost: 3000,
        attack_damage: 35.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "berserkers_greaves",
        cost: 1100,
        attack_damage: 0.0,
        max_hp: 0.0,
    },
];

const LETHALITY_MARKSMAN_ITEM_PLAN: [ItemTemplate; 6] = [
    ItemTemplate {
        key: "the_collector",
        cost: 3100,
        attack_damage: 55.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "opportunity",
        cost: 2700,
        attack_damage: 55.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "seryldas_grudge",
        cost: 3200,
        attack_damage: 45.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "edge_of_night",
        cost: 3000,
        attack_damage: 50.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "profane_hydra",
        cost: 3300,
        attack_damage: 60.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "ionian_boots_of_lucidity",
        cost: 900,
        attack_damage: 0.0,
        max_hp: 0.0,
    },
];

const SUPPORT_ENGAGE_ITEM_PLAN: [ItemTemplate; 6] = [
    ItemTemplate {
        key: "trailblazer",
        cost: 2400,
        attack_damage: 8.0,
        max_hp: 350.0,
    },
    ItemTemplate {
        key: "zekes_convergence",
        cost: 2200,
        attack_damage: 8.0,
        max_hp: 250.0,
    },
    ItemTemplate {
        key: "knights_vow",
        cost: 2300,
        attack_damage: 0.0,
        max_hp: 350.0,
    },
    ItemTemplate {
        key: "locket_of_the_iron_solari",
        cost: 2200,
        attack_damage: 0.0,
        max_hp: 250.0,
    },
    ItemTemplate {
        key: "thornmail",
        cost: 2450,
        attack_damage: 0.0,
        max_hp: 350.0,
    },
    ItemTemplate {
        key: "mobility_boots",
        cost: 1000,
        attack_damage: 0.0,
        max_hp: 0.0,
    },
];

const SUPPORT_ENCHANTER_ITEM_PLAN: [ItemTemplate; 6] = [
    ItemTemplate {
        key: "shurelyas_battlesong",
        cost: 2200,
        attack_damage: 10.0,
        max_hp: 300.0,
    },
    ItemTemplate {
        key: "ardent_censer",
        cost: 2300,
        attack_damage: 18.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "moonstone_renewer",
        cost: 2200,
        attack_damage: 14.0,
        max_hp: 250.0,
    },
    ItemTemplate {
        key: "redemption",
        cost: 2300,
        attack_damage: 12.0,
        max_hp: 250.0,
    },
    ItemTemplate {
        key: "staff_of_flowing_water",
        cost: 2250,
        attack_damage: 18.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "ionian_boots_of_lucidity",
        cost: 900,
        attack_damage: 0.0,
        max_hp: 0.0,
    },
];

const SUPPORT_DAMAGE_ITEM_PLAN: [ItemTemplate; 6] = [
    ItemTemplate {
        key: "rylais_crystal_scepter",
        cost: 2600,
        attack_damage: 25.0,
        max_hp: 400.0,
    },
    ItemTemplate {
        key: "liandrys_torment",
        cost: 3000,
        attack_damage: 33.0,
        max_hp: 300.0,
    },
    ItemTemplate {
        key: "morellonomicon",
        cost: 2950,
        attack_damage: 28.0,
        max_hp: 350.0,
    },
    ItemTemplate {
        key: "zhonyas_hourglass",
        cost: 3250,
        attack_damage: 25.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "cryptbloom",
        cost: 2850,
        attack_damage: 27.0,
        max_hp: 0.0,
    },
    ItemTemplate {
        key: "sorcerers_shoes",
        cost: 1100,
        attack_damage: 0.0,
        max_hp: 0.0,
    },
];

const LANE_PATH_TOP_BLUE: [Vec2; 11] = [
    Vec2 { x: 0.12, y: 0.88 },
    Vec2 { x: 0.109, y: 0.76 },
    Vec2 { x: 0.104, y: 0.67 },
    Vec2 { x: 0.101, y: 0.56 },
    Vec2 { x: 0.099, y: 0.43 },
    Vec2 { x: 0.098, y: 0.31 },
    Vec2 { x: 0.122, y: 0.20 },
    Vec2 { x: 0.2, y: 0.11 },
    Vec2 { x: 0.28, y: 0.08 },
    Vec2 { x: 0.53, y: 0.08 },
    Vec2 { x: 0.89, y: 0.12 },
];

const LANE_PATH_MID_BLUE: [Vec2; 7] = [
    Vec2 { x: 0.12, y: 0.88 },
    Vec2 { x: 0.22, y: 0.78 },
    Vec2 { x: 0.34, y: 0.67 },
    Vec2 { x: 0.46, y: 0.54 },
    Vec2 { x: 0.58, y: 0.42 },
    Vec2 { x: 0.7, y: 0.3 },
    Vec2 { x: 0.89, y: 0.12 },
];

const LANE_PATH_BOT_BLUE: [Vec2; 11] = [
    Vec2 { x: 0.12, y: 0.88 },
    Vec2 { x: 0.24, y: 0.89 },
    Vec2 { x: 0.36, y: 0.9 },
    Vec2 { x: 0.49, y: 0.907 },
    Vec2 { x: 0.62, y: 0.909 },
    Vec2 { x: 0.72, y: 0.912 },
    Vec2 { x: 0.81, y: 0.852 },
    Vec2 { x: 0.89, y: 0.705 },
    Vec2 { x: 0.91, y: 0.58 },
    Vec2 { x: 0.91, y: 0.45 },
    Vec2 { x: 0.89, y: 0.12 },
];

fn create_initial_state(
    seed: &str,
    snapshot: &Value,
    champion_by_player_id: &HashMap<String, String>,
    champion_profiles_by_id: &HashMap<String, LolChampionCombatProfileInput>,
    champion_ultimates_by_id: &HashMap<String, LolChampionUltimateInput>,
    ai_mode: SimulatorAiMode,
) -> Value {
    // TODO(parity-chunk-b): port movement, wave spawn/advance, and combat systems from TS simulation.ts.
    let champions = create_champions(
        seed,
        snapshot,
        champion_by_player_id,
        champion_profiles_by_id,
        champion_ultimates_by_id,
    );
    let structures = create_structures();
    let neutral_timers = build_neutral_timers_state();
    let team_tactics = build_team_tactics_state(snapshot);

    json!({
        "timeSec": 0.0,
        "running": true,
        "speed": 1.0,
        "aiMode": ai_mode.as_str(),
        "winner": Value::Null,
        "champions": champions,
        "minions": [],
            "structures": structures,
            "wards": [],
            "objectives": {
            "dragon": { "key": "dragon", "pos": { "x": 0.673828125, "y": 0.703125 }, "alive": false, "nextSpawnAt": 5.0 * 60.0 },
            "baron": { "key": "baron", "pos": { "x": 0.3274739583333333, "y": 0.2981770833333333 }, "alive": false, "nextSpawnAt": 20.0 * 60.0 }
        },
        "neutralTimers": neutral_timers,
        "stats": {
            "blue": { "kills": 0, "towers": 0, "dragons": 0, "barons": 0, "gold": 2500 },
            "red": { "kills": 0, "towers": 0, "dragons": 0, "barons": 0, "gold": 2500 }
        },
        "events": [{ "t": 0.0, "text": "Match started", "type": "info" }],
        "teamTactics": team_tactics,
        "showWalls": false,
    })
}

fn create_champions(
    seed: &str,
    snapshot: &Value,
    champion_by_player_id: &HashMap<String, String>,
    champion_profiles_by_id: &HashMap<String, LolChampionCombatProfileInput>,
    champion_ultimates_by_id: &HashMap<String, LolChampionUltimateInput>,
) -> Vec<Value> {
    let mut rng = Mulberry32::new(hash_seed(seed));
    let mut champions = Vec::new();

    let home_players = snapshot_team_players(snapshot, "home_team");
    let away_players = snapshot_team_players(snapshot, "away_team");
    let home_tactics = extract_runtime_team_tactics(snapshot, "home", "home_team");
    let away_tactics = extract_runtime_team_tactics(snapshot, "away", "away_team");

    seed_team(
        &mut champions,
        &home_players,
        "home",
        "blue",
        BASE_POSITION_BLUE,
        &home_tactics,
        snapshot,
        champion_by_player_id,
        champion_profiles_by_id,
        champion_ultimates_by_id,
        &mut rng,
    );

    seed_team(
        &mut champions,
        &away_players,
        "away",
        "red",
        BASE_POSITION_RED,
        &away_tactics,
        snapshot,
        champion_by_player_id,
        champion_profiles_by_id,
        champion_ultimates_by_id,
        &mut rng,
    );

    champions
}

fn seed_team(
    champions: &mut Vec<Value>,
    players: &[SnapshotPlayer],
    side_key: &str,
    team: &str,
    base_pos: Vec2,
    team_tactics: &RuntimeTeamTactics,
    snapshot: &Value,
    champion_by_player_id: &HashMap<String, String>,
    champion_profiles_by_id: &HashMap<String, LolChampionCombatProfileInput>,
    champion_ultimates_by_id: &HashMap<String, LolChampionUltimateInput>,
    rng: &mut Mulberry32,
) {
    for (index, player) in players.iter().take(5).enumerate() {
        let Some(role_seed) = ROLE_SEEDS.get(index) else {
            break;
        };

        let champion_id = champion_by_player_id.get(&player.id);
        let profile = champion_id.and_then(|id| champion_profiles_by_id.get(id));
        let attack_type = profile
            .map(|p| normalize_attack_type(&p.attack_type))
            .unwrap_or("melee");
        let max_hp = champion_max_hp_from_base(profile.map(|p| p.base_hp).unwrap_or(560.0));
        let attack_range = profile
            .map(|p| p.attack_range)
            .unwrap_or(if attack_type == "ranged" {
                0.056
            } else {
                0.049
            });
        let role_impact = extract_runtime_role_impact(snapshot, side_key, &player.id);
        let role_modifier = role_impact
            .as_ref()
            .map(|impact| impact.modifier.clamp(-4.0, 4.0))
            .unwrap_or(0.0);
        let role_variance = role_impact
            .as_ref()
            .map(|impact| impact.variance.clamp(0.5, 4.5))
            .unwrap_or(1.0);

        let (
            mechanics,
            laning,
            teamfighting,
            macro_stat,
            consistency,
            shotcalling,
            champion_pool,
            discipline,
            mental_resilience,
        ) = player_visible_stats(player);

        let gameplay_score = (mechanics + laning + teamfighting) / 3.0;
        let iq_score = (macro_stat + consistency + shotcalling) / 3.0;
        let competitive_score = (champion_pool + discipline + mental_resilience) / 3.0;

        let gameplay_delta = stat_delta(gameplay_score);
        let iq_delta = stat_delta(iq_score);
        let competitive_delta = stat_delta(competitive_score);
        let mechanics_delta = stat_delta(mechanics);
        let laning_delta = stat_delta(laning);
        let teamfighting_delta = stat_delta(teamfighting);
        let consistency_delta = stat_delta(consistency);
        let discipline_delta = stat_delta(discipline);
        let champion_pool_delta = stat_delta(champion_pool);

        let max_hp = (max_hp
            * (1.0 + role_modifier * 0.012 + competitive_delta * 0.04 + teamfighting_delta * 0.02))
            .clamp(120.0, 340.0);
        let attack_damage = (14.0 + rng.next_f64() * 5.0)
            * (1.0 + role_modifier * 0.016 + gameplay_delta * 0.06 + mechanics_delta * 0.03);
        let move_speed = (0.043
            + rng.next_f64() * 0.008
            + (role_modifier * 0.00035)
            + iq_delta * 0.001
            + laning_delta * 0.0006)
            .clamp(0.036, 0.062);

        let spawn_pos = Vec2 {
            x: base_pos.x + role_seed.offset.x,
            y: base_pos.y + role_seed.offset.y,
        };

        let jgl_start = if role_seed.role == "JGL" {
            if normalized_team(team) == "blue" {
                if team_tactics.jungle_pathing == "BotToTop" {
                    Vec2 {
                        x: 0.5266927083333334,
                        y: 0.7421875,
                    }
                } else {
                    Vec2 {
                        x: 0.24934895833333334,
                        y: 0.4622395833333333,
                    }
                }
            } else if team_tactics.jungle_pathing == "BotToTop" {
                Vec2 {
                    x: 0.7545572916666666,
                    y: 0.5403645833333334,
                }
            } else {
                Vec2 {
                    x: 0.478515625,
                    y: 0.26171875,
                }
            }
        } else {
            spawn_pos
        };

        let initial_target_path = if role_seed.role == "JGL" {
            vec![json!({ "x": jgl_start.x, "y": jgl_start.y })]
        } else {
            Vec::new()
        };
        let initial_state = if role_seed.role == "JGL" {
            "objective"
        } else {
            "lane"
        };
        let consistency_factor =
            (1.0 - consistency_delta * 0.26 - discipline_delta * 0.12 - champion_pool_delta * 0.08)
                .clamp(0.65, 1.35);
        let decision_jitter =
            (((role_variance - 1.0).max(0.0) * 0.35) + rng.next_f64() * 0.08) * consistency_factor;
        let initial_next_decision_at = if role_seed.role == "JGL" {
            6.0 + decision_jitter
        } else {
            decision_jitter
        };
        let summoner_spells = default_summoner_spells_for_role(role_seed.role);
        let ultimate = champion_id
            .and_then(|id| champion_ultimates_by_id.get(id))
            .map(|slot| {
                json!({
                    "archetype": slot.archetype,
                    "icon": slot.icon,
                    "cdUntil": 0.0,
                })
            })
            .unwrap_or_else(|| {
                json!({
                    "archetype": default_ultimate_archetype_for_role(role_seed.role),
                    "icon": "",
                    "cdUntil": 0.0,
                })
            });

        let mut champion_json = json!({
            "id": player.id,
            "name": player.name,
            "championId": champion_id.cloned().unwrap_or_default(),
            "team": team,
            "role": role_seed.role,
            "lane": role_seed.lane,
            "pos": {
                "x": spawn_pos.x,
                "y": spawn_pos.y,
            },
            "hp": max_hp,
            "maxHp": max_hp,
            "alive": true,
            "respawnAt": 0.0,
            "attackCdUntil": 0.0,
            "moveSpeed": move_speed,
            "attackRange": attack_range,
            "attackType": attack_type,
            "attackDamage": attack_damage,
            "targetPath": initial_target_path,
            "targetPathIndex": 0,
            "nextDecisionAt": initial_next_decision_at,
            "kills": 0,
            "deaths": 0,
            "assists": 0,
            "gold": 500,
            "spentGold": 0,
            "xp": 0,
            "level": 1,
            "cs": 0,
            "hasLeftBaseOnce": false,
            "lastSupportCsAt": -999.0,
            "items": [],
            "gameplayScore": gameplay_score,
            "iqScore": iq_score,
            "competitiveScore": competitive_score,
            "summonerSpells": summoner_spells,
            "igniteDotUntil": 0.0,
            "igniteSourceId": Value::Null,
            "lastDamagedByChampionId": Value::Null,
            "lastDamagedAt": -999.0,
            "state": initial_state,
            "recallAnchor": Value::Null,
            "recallChannelUntil": 0.0,
        });

        if let Some(obj) = champion_json.as_object_mut() {
            obj.insert("ultimate".to_string(), ultimate);
            obj.insert("realmBanishedUntil".to_string(), Value::from(0.0));
            obj.insert("realmReturnPos".to_string(), Value::Null);
            obj.insert("wardCdUntil".to_string(), Value::from(0.0));
            obj.insert("sweeperCdUntil".to_string(), Value::from(0.0));
            obj.insert("sweeperActiveUntil".to_string(), Value::from(0.0));
            obj.insert("trinketKey".to_string(), Value::from(TRINKET_WARDING_TOTEM));
            obj.insert("trinketSwapped".to_string(), Value::from(false));
            obj.insert("supportRoamUses".to_string(), Value::from(0));
            obj.insert("supportRoamCdUntil".to_string(), Value::from(0.0));
            obj.insert("supportLastRoamRole".to_string(), Value::from(""));
        }

        champions.push(champion_json);
    }
}

fn default_summoner_spells_for_role(role: &str) -> Vec<Value> {
    let keys: [&str; 2] = match role {
        "JGL" => ["Smite", "Flash"],
        "TOP" => ["Teleport", "Flash"],
        "MID" => ["Ignite", "Flash"],
        "ADC" => ["Heal", "Flash"],
        _ => ["Ignite", "Flash"],
    };
    keys.iter()
        .map(|key| json!({ "key": key, "cdUntil": 0.0 }))
        .collect()
}

fn default_ultimate_archetype_for_role(role: &str) -> &'static str {
    match role {
        "TOP" => "engage",
        "JGL" => "global",
        "MID" => "burst",
        "ADC" => "execute",
        _ => "utility",
    }
}

fn create_structures() -> Vec<Value> {
    STRUCTURE_LAYOUT
        .iter()
        .map(|s| {
            let hp = structure_base_hp(s);
            json!({
                "id": s.id,
                "team": s.team,
                "lane": s.lane,
                "kind": s.kind,
                "pos": { "x": s.pos.x, "y": s.pos.y },
                "hp": hp,
                "maxHp": hp,
                "alive": true,
                "attackCdUntil": 0.0,
                "forcedTargetChampionId": Value::Null,
                "forcedTargetUntil": 0.0,
            })
        })
        .collect()
}

fn structure_base_hp(seed: &StructureSeed) -> f64 {
    match seed.kind {
        "nexus" => NEXUS_HP,
        "inhib" => INHIBITOR_HP,
        "tower" => {
            if seed.id.contains("nexus") {
                TOWER_NEXUS_HP
            } else if seed.id.contains("inhib") {
                TOWER_INHIB_HP
            } else if seed.id.contains("inner") {
                TOWER_INNER_HP
            } else {
                TOWER_OUTER_HP
            }
        }
        _ => TOWER_OUTER_HP,
    }
}

fn build_neutral_timers_state() -> Value {
    let mut entities = Map::new();

    for timer in neutral_timer_templates() {
        entities.insert(
            timer.key.to_string(),
            json!({
                "key": timer.key,
                "label": timer.label,
                "alive": false,
                "hp": timer.max_hp,
                "maxHp": timer.max_hp,
                "nextSpawnAt": if timer.unlocked { Value::from(timer.first_spawn_at) } else { Value::Null },
                "firstSpawnAt": timer.first_spawn_at,
                "respawnDelaySec": match timer.respawn_delay_sec {
                    Some(value) => Value::from(value),
                    None => Value::Null,
                },
                "oneShot": timer.one_shot,
                "windowCloseAt": match timer.window_close_at {
                    Some(value) => Value::from(value),
                    None => Value::Null,
                },
                "combatGraceUntil": match timer.combat_grace_until {
                    Some(value) => Value::from(value),
                    None => Value::Null,
                },
                "unlocked": timer.unlocked,
                "lastSpawnAt": Value::Null,
                "lastTakenAt": Value::Null,
                "timesSpawned": 0,
                "timesTaken": 0,
                "pos": { "x": timer.pos.x, "y": timer.pos.y },
            }),
        );
    }

    json!({
        "dragonSoulUnlocked": false,
        "elderUnlocked": false,
        "entities": Value::Object(entities),
    })
}

fn neutral_timer_templates() -> Vec<NeutralTimerTemplate> {
    vec![
        NeutralTimerTemplate {
            key: "blue-buff-blue",
            label: "Blue Blue Buff",
            first_spawn_at: JUNGLE_INITIAL_SPAWN_AT,
            max_hp: 470.0,
            respawn_delay_sec: Some(300.0),
            one_shot: false,
            window_close_at: None,
            combat_grace_until: None,
            unlocked: true,
            pos: Vec2 {
                x: 0.24934895833333334,
                y: 0.4622395833333333,
            },
        },
        NeutralTimerTemplate {
            key: "blue-buff-red",
            label: "Red Blue Buff",
            first_spawn_at: JUNGLE_INITIAL_SPAWN_AT,
            max_hp: 470.0,
            respawn_delay_sec: Some(300.0),
            one_shot: false,
            window_close_at: None,
            combat_grace_until: None,
            unlocked: true,
            pos: Vec2 {
                x: 0.478515625,
                y: 0.26171875,
            },
        },
        NeutralTimerTemplate {
            key: "red-buff-blue",
            label: "Blue Red Buff",
            first_spawn_at: JUNGLE_INITIAL_SPAWN_AT,
            max_hp: 500.0,
            respawn_delay_sec: Some(300.0),
            one_shot: false,
            window_close_at: None,
            combat_grace_until: None,
            unlocked: true,
            pos: Vec2 {
                x: 0.5266927083333334,
                y: 0.7421875,
            },
        },
        NeutralTimerTemplate {
            key: "red-buff-red",
            label: "Red Red Buff",
            first_spawn_at: JUNGLE_INITIAL_SPAWN_AT,
            max_hp: 500.0,
            respawn_delay_sec: Some(300.0),
            one_shot: false,
            window_close_at: None,
            combat_grace_until: None,
            unlocked: true,
            pos: Vec2 {
                x: 0.7545572916666666,
                y: 0.5403645833333334,
            },
        },
        NeutralTimerTemplate {
            key: "wolves-blue",
            label: "Blue Wolves",
            first_spawn_at: JUNGLE_INITIAL_SPAWN_AT,
            max_hp: 380.0,
            respawn_delay_sec: Some(135.0),
            one_shot: false,
            window_close_at: None,
            combat_grace_until: None,
            unlocked: true,
            pos: Vec2 {
                x: 0.2584635416666667,
                y: 0.56640625,
            },
        },
        NeutralTimerTemplate {
            key: "wolves-red",
            label: "Red Wolves",
            first_spawn_at: JUNGLE_INITIAL_SPAWN_AT,
            max_hp: 380.0,
            respawn_delay_sec: Some(135.0),
            one_shot: false,
            window_close_at: None,
            combat_grace_until: None,
            unlocked: true,
            pos: Vec2 {
                x: 0.525390625,
                y: 0.3528645833333333,
            },
        },
        NeutralTimerTemplate {
            key: "raptors-blue",
            label: "Blue Raptors",
            first_spawn_at: JUNGLE_INITIAL_SPAWN_AT,
            max_hp: 390.0,
            respawn_delay_sec: Some(135.0),
            one_shot: false,
            window_close_at: None,
            combat_grace_until: None,
            unlocked: true,
            pos: Vec2 {
                x: 0.4759114583333333,
                y: 0.6432291666666666,
            },
        },
        NeutralTimerTemplate {
            key: "raptors-red",
            label: "Red Raptors",
            first_spawn_at: JUNGLE_INITIAL_SPAWN_AT,
            max_hp: 390.0,
            respawn_delay_sec: Some(135.0),
            one_shot: false,
            window_close_at: None,
            combat_grace_until: None,
            unlocked: true,
            pos: Vec2 {
                x: 0.748046875,
                y: 0.4361979166666667,
            },
        },
        NeutralTimerTemplate {
            key: "gromp-blue",
            label: "Blue Gromp",
            first_spawn_at: JUNGLE_INITIAL_SPAWN_AT,
            max_hp: 520.0,
            respawn_delay_sec: Some(135.0),
            one_shot: false,
            window_close_at: None,
            combat_grace_until: None,
            unlocked: true,
            pos: Vec2 {
                x: 0.14908854166666666,
                y: 0.43359375,
            },
        },
        NeutralTimerTemplate {
            key: "gromp-red",
            label: "Red Gromp",
            first_spawn_at: JUNGLE_INITIAL_SPAWN_AT,
            max_hp: 520.0,
            respawn_delay_sec: Some(135.0),
            one_shot: false,
            window_close_at: None,
            combat_grace_until: None,
            unlocked: true,
            pos: Vec2 {
                x: 0.4381510416666667,
                y: 0.16536458333333334,
            },
        },
        NeutralTimerTemplate {
            key: "krugs-blue",
            label: "Blue Krugs",
            first_spawn_at: JUNGLE_INITIAL_SPAWN_AT,
            max_hp: 560.0,
            respawn_delay_sec: Some(135.0),
            one_shot: false,
            window_close_at: None,
            combat_grace_until: None,
            unlocked: true,
            pos: Vec2 {
                x: 0.568359375,
                y: 0.828125,
            },
        },
        NeutralTimerTemplate {
            key: "krugs-red",
            label: "Red Krugs",
            first_spawn_at: JUNGLE_INITIAL_SPAWN_AT,
            max_hp: 560.0,
            respawn_delay_sec: Some(135.0),
            one_shot: false,
            window_close_at: None,
            combat_grace_until: None,
            unlocked: true,
            pos: Vec2 {
                x: 0.8483072916666666,
                y: 0.56640625,
            },
        },
        NeutralTimerTemplate {
            key: "scuttle-top",
            label: "Scuttle Top",
            first_spawn_at: SCUTTLE_INITIAL_SPAWN_AT,
            max_hp: 560.0,
            respawn_delay_sec: Some(150.0),
            one_shot: false,
            window_close_at: None,
            combat_grace_until: None,
            unlocked: true,
            pos: Vec2 {
                x: 0.2845052083333333,
                y: 0.34765625,
            },
        },
        NeutralTimerTemplate {
            key: "scuttle-bot",
            label: "Scuttle Bot",
            first_spawn_at: SCUTTLE_INITIAL_SPAWN_AT,
            max_hp: 560.0,
            respawn_delay_sec: Some(150.0),
            one_shot: false,
            window_close_at: None,
            combat_grace_until: None,
            unlocked: true,
            pos: Vec2 {
                x: 0.6998697916666666,
                y: 0.6419270833333334,
            },
        },
        NeutralTimerTemplate {
            key: "dragon",
            label: "Dragon",
            first_spawn_at: 5.0 * 60.0,
            max_hp: 3600.0,
            respawn_delay_sec: Some(5.0 * 60.0),
            one_shot: false,
            window_close_at: None,
            combat_grace_until: None,
            unlocked: true,
            pos: Vec2 {
                x: 0.673828125,
                y: 0.703125,
            },
        },
        NeutralTimerTemplate {
            key: "voidgrubs",
            label: "Voidgrubs",
            first_spawn_at: 8.0 * 60.0,
            max_hp: 2800.0,
            respawn_delay_sec: None,
            one_shot: true,
            window_close_at: Some(VOIDGRUBS_SOFT_CLOSE_AT),
            combat_grace_until: Some(VOIDGRUBS_HARD_CLOSE_AT),
            unlocked: true,
            pos: Vec2 {
                x: 0.3274739583333333,
                y: 0.2981770833333333,
            },
        },
        NeutralTimerTemplate {
            key: "herald",
            label: "Rift Herald",
            first_spawn_at: 15.0 * 60.0,
            max_hp: 5500.0,
            respawn_delay_sec: None,
            one_shot: true,
            window_close_at: Some(HERALD_SOFT_CLOSE_AT),
            combat_grace_until: Some(HERALD_HARD_CLOSE_AT),
            unlocked: true,
            pos: Vec2 {
                x: 0.3274739583333333,
                y: 0.2981770833333333,
            },
        },
        NeutralTimerTemplate {
            key: "baron",
            label: "Baron",
            first_spawn_at: 20.0 * 60.0,
            max_hp: 9000.0,
            respawn_delay_sec: Some(6.0 * 60.0),
            one_shot: false,
            window_close_at: None,
            combat_grace_until: None,
            unlocked: true,
            pos: Vec2 {
                x: 0.3274739583333333,
                y: 0.2981770833333333,
            },
        },
        NeutralTimerTemplate {
            key: "elder",
            label: "Elder Dragon",
            first_spawn_at: 0.0,
            max_hp: 7200.0,
            respawn_delay_sec: Some(6.0 * 60.0),
            one_shot: false,
            window_close_at: None,
            combat_grace_until: None,
            unlocked: false,
            pos: Vec2 {
                x: 0.673828125,
                y: 0.703125,
            },
        },
    ]
}

fn snapshot_team_players(snapshot: &Value, team_key: &str) -> Vec<SnapshotPlayer> {
    snapshot
        .get(team_key)
        .and_then(Value::as_object)
        .and_then(|team| team.get("players"))
        .and_then(Value::as_array)
        .map(|players| {
            players
                .iter()
                .filter_map(|player| {
                    let id = player.get("id").and_then(Value::as_str)?.to_string();
                    let name = player
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or(&id)
                        .to_string();
                    let stat = |key: &str| {
                        player
                            .get(key)
                            .and_then(Value::as_f64)
                            .unwrap_or(70.0)
                            .clamp(1.0, 99.0)
                    };
                    Some(SnapshotPlayer {
                        id,
                        name,
                        dribbling: stat("dribbling"),
                        agility: stat("agility"),
                        pace: stat("pace"),
                        composure: stat("composure"),
                        shooting: stat("shooting"),
                        positioning: stat("positioning"),
                        teamwork: stat("teamwork"),
                        stamina: stat("stamina"),
                        decisions: stat("decisions"),
                        vision: stat("vision"),
                        passing: stat("passing"),
                        leadership: stat("leadership"),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn avg4(a: f64, b: f64, c: f64, d: f64) -> f64 {
    (a + b + c + d) / 4.0
}

fn player_visible_stats(player: &SnapshotPlayer) -> (f64, f64, f64, f64, f64, f64, f64, f64, f64) {
    let mechanics = avg4(
        player.dribbling,
        player.agility,
        player.pace,
        player.composure,
    );
    let laning = avg4(
        player.shooting,
        player.positioning,
        player.dribbling,
        player.composure,
    );
    let teamfighting = avg4(
        player.teamwork,
        player.stamina,
        player.decisions,
        player.composure,
    );
    let macro_stat = avg4(
        player.vision,
        player.decisions,
        player.positioning,
        player.passing,
    );
    let consistency = avg4(
        player.decisions,
        player.vision,
        player.composure,
        player.teamwork,
    );
    let shotcalling = avg4(
        player.leadership,
        player.teamwork,
        player.vision,
        player.decisions,
    );
    let champion_pool = avg4(
        player.dribbling,
        player.agility,
        player.vision,
        player.passing,
    );
    let discipline = avg4(
        player.decisions,
        player.composure,
        player.teamwork,
        player.leadership,
    );
    let mental_resilience = avg4(
        player.composure,
        player.teamwork,
        player.leadership,
        player.stamina,
    );
    (
        mechanics,
        laning,
        teamfighting,
        macro_stat,
        consistency,
        shotcalling,
        champion_pool,
        discipline,
        mental_resilience,
    )
}

fn stat_delta(score: f64) -> f64 {
    ((score - 70.0) / 30.0).clamp(-1.0, 1.0)
}

fn champion_micro_damage_multiplier(champion: &ChampionRuntime) -> f64 {
    let gameplay = stat_delta(champion.gameplay_score);
    (1.0 + gameplay * 0.09).clamp(0.88, 1.16)
}

fn champion_lane_damage_multiplier(champion: &ChampionRuntime) -> f64 {
    let gameplay = stat_delta(champion.gameplay_score);
    (1.0 + gameplay * 0.11).clamp(0.86, 1.18)
}

fn champion_structure_focus_multiplier(champion: &ChampionRuntime) -> f64 {
    let iq_delta = stat_delta(champion.iq_score);
    (1.0 + iq_delta * 0.08).clamp(0.88, 1.14)
}

fn extract_runtime_team_tactics(
    snapshot: &Value,
    side_key: &str,
    team_key: &str,
) -> RuntimeTeamTactics {
    let from_root = snapshot
        .get("lol_tactics")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get(side_key))
        .cloned();
    let from_team = snapshot
        .get(team_key)
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("lol_tactics"))
        .cloned();

    let payload = from_root.or(from_team);
    payload
        .and_then(|value| serde_json::from_value::<RuntimeTeamTactics>(value).ok())
        .unwrap_or_default()
}

fn build_team_tactics_state(snapshot: &Value) -> Value {
    let blue = extract_runtime_team_tactics(snapshot, "home", "home_team");
    let red = extract_runtime_team_tactics(snapshot, "away", "away_team");
    json!({ "blue": blue, "red": red })
}

fn extract_runtime_role_impact(
    snapshot: &Value,
    side_key: &str,
    player_id: &str,
) -> Option<RuntimeRoleImpact> {
    snapshot
        .get("lol_role_impact_by_player")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get(side_key))
        .and_then(Value::as_object)
        .and_then(|by_player| by_player.get(player_id))
        .cloned()
        .and_then(|value| serde_json::from_value::<RuntimeRoleImpact>(value).ok())
}

fn team_tactics_for_runtime(team_tactics: Option<&Value>, team: &str) -> RuntimeTeamTactics {
    team_tactics
        .and_then(Value::as_object)
        .and_then(|obj| obj.get(normalized_team(team)))
        .cloned()
        .and_then(|value| serde_json::from_value::<RuntimeTeamTactics>(value).ok())
        .unwrap_or_default()
}

fn team_buffs_for_runtime(team_buffs: Option<&Value>, team: &str) -> RuntimeTeamBuffState {
    team_buffs
        .and_then(Value::as_object)
        .and_then(|obj| obj.get(normalized_team(team)))
        .cloned()
        .and_then(|value| serde_json::from_value::<RuntimeTeamBuffState>(value).ok())
        .unwrap_or_default()
}

fn runtime_buffs_from_extra(team_buffs: Option<&Value>) -> RuntimeBuffState {
    team_buffs
        .cloned()
        .and_then(|value| serde_json::from_value::<RuntimeBuffState>(value).ok())
        .unwrap_or_default()
}

fn set_runtime_buffs(runtime: &mut RuntimeState, buffs: &RuntimeBuffState) {
    if let Ok(value) = serde_json::to_value(buffs) {
        runtime.extra.insert("teamBuffs".to_string(), value);
    }
}

fn team_buffs_mut<'a>(buffs: &'a mut RuntimeBuffState, team: &str) -> &'a mut RuntimeTeamBuffState {
    if normalized_team(team) == "red" {
        &mut buffs.red
    } else {
        &mut buffs.blue
    }
}

fn team_buffs_ref<'a>(buffs: &'a RuntimeBuffState, team: &str) -> &'a RuntimeTeamBuffState {
    if normalized_team(team) == "red" {
        &buffs.red
    } else {
        &buffs.blue
    }
}

fn current_dragon_kind(neutral_timers: &NeutralTimersRuntime) -> String {
    let raw = neutral_timers
        .extra
        .get("dragonCurrentKind")
        .and_then(Value::as_str)
        .unwrap_or("infernal")
        .trim()
        .to_lowercase();

    match raw.as_str() {
        "infernal" | "ocean" | "mountain" | "cloud" | "hextech" | "chemtech" => raw,
        _ => "infernal".to_string(),
    }
}

fn set_current_dragon_kind(neutral_timers: &mut NeutralTimersRuntime, kind: &str) {
    neutral_timers
        .extra
        .insert("dragonCurrentKind".to_string(), Value::from(kind));
}

fn choose_different_dragon_kind(base_kind: &str, seed: i64) -> &'static str {
    const KINDS: [&str; 6] = [
        "infernal", "ocean", "mountain", "cloud", "hextech", "chemtech",
    ];
    let mut options: Vec<&str> = KINDS
        .into_iter()
        .filter(|kind| *kind != base_kind)
        .collect();
    if options.is_empty() {
        return "infernal";
    }
    let idx = (seed.unsigned_abs() as usize) % options.len();
    options.swap_remove(idx)
}

fn choose_dragon_kind_excluding(excluded: &[&str], seed: i64) -> &'static str {
    const KINDS: [&str; 6] = [
        "infernal", "ocean", "mountain", "cloud", "hextech", "chemtech",
    ];
    let mut options: Vec<&str> = KINDS
        .into_iter()
        .filter(|kind| !excluded.iter().any(|excluded_kind| excluded_kind == kind))
        .collect();
    if options.is_empty() {
        return "infernal";
    }
    let idx = (seed.unsigned_abs() as usize) % options.len();
    options.swap_remove(idx)
}

fn ensure_dragon_cycle_defaults(runtime: &RuntimeState, neutral_timers: &mut NeutralTimersRuntime) {
    if neutral_timers.extra.get("dragonCurrentKind").is_some() {
        return;
    }
    let seed = runtime.champions.iter().fold(0_i64, |acc, champion| {
        acc + champion.id.bytes().fold(0_i64, |s, b| s + b as i64)
    });
    let first = choose_different_dragon_kind("", seed);
    set_current_dragon_kind(neutral_timers, first);
    neutral_timers
        .extra
        .insert("dragonFirstKind".to_string(), Value::from(""));
    neutral_timers
        .extra
        .insert("dragonSecondKind".to_string(), Value::from(""));
    neutral_timers
        .extra
        .insert("dragonSoulRiftKind".to_string(), Value::from(""));
}

fn normalize_attack_type(raw: &str) -> &'static str {
    if raw.eq_ignore_ascii_case("ranged") {
        "ranged"
    } else {
        "melee"
    }
}

fn champion_max_hp_from_base(base_hp: f64) -> f64 {
    (base_hp / 4.0).round().clamp(120.0, 240.0)
}

fn hash_seed(seed: &str) -> u32 {
    let mut h: u32 = 2_166_136_261;
    for ch in seed.encode_utf16() {
        h ^= ch as u32;
        h = h.wrapping_mul(16_777_619);
    }
    h
}

struct Mulberry32 {
    a: u32,
}

impl Mulberry32 {
    fn new(seed: u32) -> Self {
        Self { a: seed }
    }

    fn next_f64(&mut self) -> f64 {
        self.a = self.a.wrapping_add(0x6d2b79f5);
        let mut t = self.a;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        ((t ^ (t >> 14)) as f64) / 4_294_967_296.0
    }
}

fn wave_interval_sec(at_time_sec: f64) -> f64 {
    if at_time_sec < 14.0 * 60.0 {
        30.0
    } else if at_time_sec < 30.0 * 60.0 {
        25.0
    } else {
        20.0
    }
}

fn normalized_lane(lane: &str) -> &'static str {
    match lane {
        "top" => "top",
        "mid" => "mid",
        "bot" => "bot",
        _ => "mid",
    }
}

fn normalized_team(team: &str) -> &'static str {
    if team == "red" {
        "red"
    } else {
        "blue"
    }
}

fn lane_path_blue(lane: &str) -> &'static [Vec2] {
    match normalized_lane(lane) {
        "top" => &LANE_PATH_TOP_BLUE,
        "bot" => &LANE_PATH_BOT_BLUE,
        _ => &LANE_PATH_MID_BLUE,
    }
}

fn lane_path_for(team: &str, lane: &str) -> Vec<Vec2> {
    let mut path = lane_path_blue(lane).to_vec();
    if normalized_team(team) == "red" {
        path.reverse();
    }
    path
}

fn base_position_for(team: &str) -> Vec2 {
    if normalized_team(team) == "red" {
        BASE_POSITION_RED
    } else {
        BASE_POSITION_BLUE
    }
}

fn active_nav_walls() -> &'static [WallPolygon] {
    static WALLS: OnceLock<Vec<WallPolygon>> = OnceLock::new();
    WALLS
        .get_or_init(|| {
            let raw = include_str!("../../crates/engine/src/live_match/lol_walls.json");
            let Ok(file) = serde_json::from_str::<WallFile>(raw) else {
                return Vec::new();
            };
            file.walls
                .into_iter()
                .filter(|wall| wall.closed && wall.points.len() >= 3 && !wall.id.is_empty())
                .collect()
        })
        .as_slice()
}

fn nav_grid() -> &'static NavGrid {
    static NAV: OnceLock<NavGrid> = OnceLock::new();
    NAV.get_or_init(|| NavGrid::new(active_nav_walls(), NAV_GRID_SIZE))
}

impl NavGrid {
    fn new(walls: &[WallPolygon], grid_size: usize) -> Self {
        let mut blocked = vec![0u8; grid_size * grid_size];
        for y in 0..grid_size {
            for x in 0..grid_size {
                let p = Vec2 {
                    x: Self::to_norm_with_size(x, grid_size),
                    y: Self::to_norm_with_size(y, grid_size),
                };
                let is_blocked = walls.iter().any(|w| point_in_polygon(p, &w.points));
                blocked[y * grid_size + x] = if is_blocked { 1 } else { 0 };
            }
        }

        Self { grid_size, blocked }
    }

    fn idx(&self, cx: usize, cy: usize) -> usize {
        cy * self.grid_size + cx
    }

    fn in_bounds(&self, cx: isize, cy: isize) -> bool {
        cx >= 0 && cy >= 0 && cx < self.grid_size as isize && cy < self.grid_size as isize
    }

    fn is_blocked_cell(&self, cx: usize, cy: usize) -> bool {
        self.blocked[self.idx(cx, cy)] == 1
    }

    fn to_cell_with_size(v: f64, grid_size: usize) -> usize {
        let scaled = (v * grid_size as f64).floor();
        clamp(scaled, 0.0, grid_size.saturating_sub(1) as f64) as usize
    }

    fn to_cell(&self, v: f64) -> usize {
        Self::to_cell_with_size(v, self.grid_size)
    }

    fn to_norm_with_size(c: usize, grid_size: usize) -> f64 {
        (c as f64 + 0.5) / grid_size as f64
    }

    fn to_norm(&self, c: usize) -> f64 {
        Self::to_norm_with_size(c, self.grid_size)
    }

    fn nearest_free_cell(&self, cx: usize, cy: usize) -> GridCell {
        if !self.is_blocked_cell(cx, cy) {
            return GridCell { cx, cy };
        }

        let mut queue = VecDeque::new();
        let mut seen = vec![false; self.grid_size * self.grid_size];
        let start_idx = self.idx(cx, cy);
        queue.push_back(GridCell { cx, cy });
        seen[start_idx] = true;

        let dirs: [(isize, isize); 8] = [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ];

        while let Some(cur) = queue.pop_front() {
            if !self.is_blocked_cell(cur.cx, cur.cy) {
                return cur;
            }

            for (dx, dy) in dirs {
                let nx = cur.cx as isize + dx;
                let ny = cur.cy as isize + dy;
                if !self.in_bounds(nx, ny) {
                    continue;
                }
                let nx = nx as usize;
                let ny = ny as usize;
                let idx = self.idx(nx, ny);
                if seen[idx] {
                    continue;
                }
                seen[idx] = true;
                queue.push_back(GridCell { cx: nx, cy: ny });
            }
        }

        GridCell { cx, cy }
    }

    fn has_line_of_sight(&self, a: Vec2, b: Vec2) -> bool {
        let ax = self.to_cell(a.x);
        let ay = self.to_cell(a.y);
        let bx = self.to_cell(b.x);
        let by = self.to_cell(b.y);
        let cell_distance =
            ((bx as f64 - ax as f64).powi(2) + (by as f64 - ay as f64).powi(2)).sqrt();
        let steps = (cell_distance * 2.0).ceil().max(6.0) as usize;
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let p = Vec2 {
                x: a.x + (b.x - a.x) * t,
                y: a.y + (b.y - a.y) * t,
            };
            let cx = self.to_cell(p.x);
            let cy = self.to_cell(p.y);
            if self.is_blocked_cell(cx, cy) {
                return false;
            }
        }
        true
    }

    fn smooth_path(&self, path: Vec<Vec2>) -> Vec<Vec2> {
        if path.len() <= 2 {
            return path;
        }

        let mut out = vec![path[0]];
        let mut i = 0usize;
        while i < path.len().saturating_sub(1) {
            let mut j = path.len().saturating_sub(1);
            while j > i + 1 {
                if self.has_line_of_sight(path[i], path[j]) {
                    break;
                }
                j = j.saturating_sub(1);
            }
            out.push(path[j]);
            i = j;
        }
        out
    }

    fn find_path(&self, start: Vec2, end: Vec2) -> Vec<Vec2> {
        let s = self.nearest_free_cell(self.to_cell(start.x), self.to_cell(start.y));
        let e = self.nearest_free_cell(self.to_cell(end.x), self.to_cell(end.y));

        let total = self.grid_size * self.grid_size;
        let mut g_score = vec![f64::INFINITY; total];
        let mut parent = vec![usize::MAX; total];
        let mut closed = vec![false; total];
        let mut in_open = vec![false; total];
        let mut open: Vec<usize> = Vec::new();

        let start_idx = self.idx(s.cx, s.cy);
        let end_idx = self.idx(e.cx, e.cy);

        g_score[start_idx] = 0.0;
        open.push(start_idx);
        in_open[start_idx] = true;

        let heuristic = |idx: usize| -> f64 {
            let cx = idx % self.grid_size;
            let cy = idx / self.grid_size;
            ((e.cx as f64 - cx as f64).powi(2) + (e.cy as f64 - cy as f64).powi(2)).sqrt()
        };

        let dirs: [(isize, isize, f64); 8] = [
            (1, 0, 1.0),
            (-1, 0, 1.0),
            (0, 1, 1.0),
            (0, -1, 1.0),
            (1, 1, 1.414),
            (-1, -1, 1.414),
            (1, -1, 1.414),
            (-1, 1, 1.414),
        ];

        while !open.is_empty() {
            open.sort_by(|a, b| {
                let f_a = g_score[*a] + heuristic(*a);
                let f_b = g_score[*b] + heuristic(*b);
                f_a.partial_cmp(&f_b)
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| {
                        heuristic(*a)
                            .partial_cmp(&heuristic(*b))
                            .unwrap_or(Ordering::Equal)
                    })
                    .then_with(|| a.cmp(b))
            });

            let current = open.remove(0);
            in_open[current] = false;
            if current == end_idx {
                let mut cell_path = Vec::new();
                let mut at = current;
                loop {
                    let cx = at % self.grid_size;
                    let cy = at / self.grid_size;
                    cell_path.push(Vec2 {
                        x: self.to_norm(cx),
                        y: self.to_norm(cy),
                    });
                    let p = parent[at];
                    if p == usize::MAX {
                        break;
                    }
                    at = p;
                }
                cell_path.reverse();
                return self.smooth_path(cell_path);
            }

            closed[current] = true;
            let cur_x = current % self.grid_size;
            let cur_y = current / self.grid_size;

            for (dx, dy, step_cost) in dirs {
                let nx = cur_x as isize + dx;
                let ny = cur_y as isize + dy;
                if !self.in_bounds(nx, ny) {
                    continue;
                }
                let nx = nx as usize;
                let ny = ny as usize;

                let is_diagonal = dx != 0 && dy != 0;
                if is_diagonal {
                    let side_x = self.is_blocked_cell((cur_x as isize + dx) as usize, cur_y);
                    let side_y = self.is_blocked_cell(cur_x, (cur_y as isize + dy) as usize);
                    if side_x || side_y {
                        continue;
                    }
                }

                if self.is_blocked_cell(nx, ny) {
                    continue;
                }

                let neighbor_idx = self.idx(nx, ny);
                if closed[neighbor_idx] {
                    continue;
                }

                let tentative_g = g_score[current] + step_cost;
                if tentative_g < g_score[neighbor_idx] {
                    g_score[neighbor_idx] = tentative_g;
                    parent[neighbor_idx] = current;
                    if !in_open[neighbor_idx] {
                        in_open[neighbor_idx] = true;
                        open.push(neighbor_idx);
                    }
                }
            }
        }

        vec![start, end]
    }
}

fn point_in_polygon(point: Vec2, polygon: &[Vec2]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        let xi = polygon[i].x;
        let yi = polygon[i].y;
        let xj = polygon[j].x;
        let yj = polygon[j].y;
        let intersects = ((yi > point.y) != (yj > point.y))
            && (point.x < (xj - xi) * (point.y - yi) / (yj - yi + 1e-9) + xi);
        if intersects {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn set_champion_direct_path(champion: &mut ChampionRuntime, target: Vec2) {
    let mut path = nav_grid().find_path(champion.pos, target);

    while path.len() > 1 && dist(path[0], champion.pos) < NAV_PATH_TRIVIAL_NODE_EPSILON {
        path.remove(0);
    }

    if path.len() <= 1 && dist(champion.pos, target) > NAV_PATH_MIN_DIRECT_DIST {
        champion.target_path = vec![target];
    } else {
        champion.target_path = path;
    }
    champion.target_path_index = 0;
}

fn current_champion_path_target(champion: &ChampionRuntime) -> Option<Vec2> {
    champion
        .target_path
        .get(champion.target_path_index)
        .copied()
        .or_else(|| champion.target_path.last().copied())
}

fn set_champion_direct_path_hysteresis(
    champion: &mut ChampionRuntime,
    target: Vec2,
    min_target_delta: f64,
) {
    if let Some(current_target) = current_champion_path_target(champion) {
        if dist(current_target, target) <= min_target_delta {
            return;
        }
    }
    set_champion_direct_path(champion, target);
}

fn jungle_disengage_fallback_order_for_team(team: &str, jungle_pathing: &str) -> Vec<&'static str> {
    let (own_top, own_bot) = if normalized_team(team) == "red" {
        (
            ["gromp-red", "blue-buff-red", "wolves-red"],
            ["raptors-red", "red-buff-red", "krugs-red"],
        )
    } else {
        (
            ["gromp-blue", "blue-buff-blue", "wolves-blue"],
            ["raptors-blue", "red-buff-blue", "krugs-blue"],
        )
    };

    if jungle_pathing == "BotToTop" {
        vec![
            own_bot[0],
            own_bot[1],
            own_bot[2],
            "scuttle-bot",
            own_top[0],
            own_top[1],
            own_top[2],
            "scuttle-top",
        ]
    } else {
        vec![
            own_top[0],
            own_top[1],
            own_top[2],
            "scuttle-top",
            own_bot[0],
            own_bot[1],
            own_bot[2],
            "scuttle-bot",
        ]
    }
}

fn pick_jungle_farm_fallback_pos(
    champion: &ChampionRuntime,
    neutral_timers: &NeutralTimersRuntime,
    jungle_pathing: &str,
    threat_pos: Option<Vec2>,
) -> Option<Vec2> {
    let mut first_alive_fallback: Option<Vec2> = None;

    for key in jungle_disengage_fallback_order_for_team(&champion.team, jungle_pathing) {
        let Some(timer) = neutral_timers.entities.get(key) else {
            continue;
        };
        if !(timer.alive && timer.unlocked && is_jungle_camp_key(&timer.key)) {
            continue;
        }
        if first_alive_fallback.is_none() {
            first_alive_fallback = Some(timer.pos);
        }

        if let Some(threat) = threat_pos {
            if dist(timer.pos, threat) <= JUNGLE_DISENGAGE_THREAT_AVOID_RADIUS {
                continue;
            }
        }

        return Some(timer.pos);
    }

    first_alive_fallback
}

fn jgl_disengage_fallback_pos(
    runtime: &RuntimeState,
    champion: &ChampionRuntime,
    threat_pos: Vec2,
) -> Vec2 {
    let neutral_timers = decode_neutral_timers_state(&runtime.neutral_timers)
        .unwrap_or_else(|| neutral_timers_default_runtime_state());
    let team_tactics = team_tactics_for_runtime(runtime.extra.get("teamTactics"), &champion.team);
    if let Some(camp_pos) = pick_jungle_farm_fallback_pos(
        champion,
        &neutral_timers,
        &team_tactics.jungle_pathing,
        Some(threat_pos),
    ) {
        return camp_pos;
    }
    recall_fallback_toward_base(champion, None)
}

fn closest_lane_path_index(pos: Vec2, path: &[Vec2]) -> usize {
    path.iter()
        .enumerate()
        .min_by(|(idx_a, a), (idx_b, b)| {
            dist(**a, pos)
                .partial_cmp(&dist(**b, pos))
                .unwrap_or(Ordering::Equal)
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

fn lane_fallback_pos_from_tower(
    champion: &ChampionRuntime,
    tower_pos: Vec2,
    toward_base: bool,
) -> Vec2 {
    let lane_path = lane_path_for(&champion.team, &champion.lane);
    let tower_idx = closest_lane_path_index(tower_pos, &lane_path);
    let lane_target = if toward_base {
        lane_path
            .get(tower_idx.saturating_sub(1))
            .copied()
            .unwrap_or(base_position_for(&champion.team))
    } else {
        lane_path
            .get((tower_idx + 1).min(lane_path.len().saturating_sub(1)))
            .copied()
            .unwrap_or(tower_pos)
    };

    let dir = normalize(Vec2 {
        x: lane_target.x - tower_pos.x,
        y: lane_target.y - tower_pos.y,
    });
    let offset = if toward_base { 0.019 } else { 0.024 };
    Vec2 {
        x: clamp(tower_pos.x + dir.x * offset, 0.01, 0.99),
        y: clamp(tower_pos.y + dir.y * offset, 0.01, 0.99),
    }
}

fn lane_pre_wave_hold_pos(champion: &ChampionRuntime, structures: &[StructureRuntime]) -> Vec2 {
    let lane_path = lane_path_for(&champion.team, &champion.lane);
    let allied_lane_tower = structures
        .iter()
        .filter(|s| {
            s.alive
                && s.kind == "tower"
                && normalized_team(&s.team) == normalized_team(&champion.team)
                && normalized_lane(&s.lane) == normalized_lane(&champion.lane)
        })
        .max_by(|a, b| {
            let idx_a = closest_lane_path_index(a.pos, &lane_path);
            let idx_b = closest_lane_path_index(b.pos, &lane_path);
            idx_a.cmp(&idx_b)
        });

    if let Some(tower) = allied_lane_tower {
        return lane_fallback_pos_from_tower(champion, tower.pos, false);
    }

    lane_path
        .get(2.min(lane_path.len().saturating_sub(1)))
        .copied()
        .unwrap_or(base_position_for(&champion.team))
}

fn lane_farm_anchor_pos(champion: &ChampionRuntime) -> Vec2 {
    let lane_path = lane_path_for(&champion.team, &champion.lane);
    lane_path
        .get(3.min(lane_path.len().saturating_sub(1)))
        .copied()
        .unwrap_or(base_position_for(&champion.team))
}

#[derive(Clone, Copy)]
struct LaneRoleProfile {
    chase_leash: f64,
    approach_leash: f64,
    retreat_hp: f64,
    outnumber_tolerance: f64,
}

#[derive(Clone, Copy)]
struct LanePressure {
    ally_champions: usize,
    enemy_champions: usize,
    ally_lane_minions: usize,
    enemy_lane_minions: usize,
    ally_score: f64,
    enemy_score: f64,
}

fn lane_role_profile(champion: &ChampionRuntime) -> Option<LaneRoleProfile> {
    if champion.role == "JGL" {
        return None;
    }
    match champion.role.as_str() {
        "TOP" => Some(LaneRoleProfile {
            chase_leash: 0.11,
            approach_leash: 0.062,
            retreat_hp: 0.34,
            outnumber_tolerance: 0.25,
        }),
        "MID" => Some(LaneRoleProfile {
            chase_leash: 0.10,
            approach_leash: 0.058,
            retreat_hp: 0.36,
            outnumber_tolerance: 0.20,
        }),
        "ADC" => Some(LaneRoleProfile {
            chase_leash: 0.095,
            approach_leash: 0.058,
            retreat_hp: 0.44,
            outnumber_tolerance: 0.08,
        }),
        _ => Some(LaneRoleProfile {
            chase_leash: 0.09,
            approach_leash: 0.055,
            retreat_hp: 0.41,
            outnumber_tolerance: 0.08,
        }),
    }
}

fn is_first_wave_contest_active(champion: &ChampionRuntime, now: f64) -> bool {
    if champion.role == "JGL" {
        return false;
    }
    now >= MINION_FIRST_WAVE_AT && now <= FIRST_WAVE_CONTEST_UNTIL
}

fn choose_lane_anchor_index(
    champion: &ChampionRuntime,
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
) -> usize {
    let lane_path = lane_path_for(&champion.team, &champion.lane);
    if lane_path.is_empty() {
        return 0;
    }

    let lane_last_idx = lane_path.len().saturating_sub(1);
    if lane_last_idx == 0 {
        return 0;
    }

    let allied_front = minions
        .iter()
        .filter(|m| {
            m.alive
                && normalized_team(&m.team) == normalized_team(&champion.team)
                && normalized_lane(&m.lane) == normalized_lane(&champion.lane)
        })
        .max_by(|a, b| a.path_index.cmp(&b.path_index));

    if let Some(front) = allied_front {
        return front.path_index.saturating_sub(1).clamp(1, lane_last_idx);
    }

    let nearest_enemy_lane_minion = minions
        .iter()
        .filter(|m| {
            m.alive
                && normalized_team(&m.team) != normalized_team(&champion.team)
                && normalized_lane(&m.lane) == normalized_lane(&champion.lane)
        })
        .min_by(|a, b| {
            dist(a.pos, champion.pos)
                .partial_cmp(&dist(b.pos, champion.pos))
                .unwrap_or(Ordering::Equal)
        });

    if let Some(enemy_unit) = nearest_enemy_lane_minion {
        let enemy_idx = closest_lane_path_index(enemy_unit.pos, &lane_path);
        let allied_lane_tower = structures
            .iter()
            .filter(|s| {
                s.alive
                    && s.kind == "tower"
                    && normalized_team(&s.team) == normalized_team(&champion.team)
                    && normalized_lane(&s.lane) == normalized_lane(&champion.lane)
            })
            .min_by(|a, b| {
                dist(a.pos, champion.pos)
                    .partial_cmp(&dist(b.pos, champion.pos))
                    .unwrap_or(Ordering::Equal)
            });
        let wave_at_own_tower = allied_lane_tower
            .map(|tower| dist(enemy_unit.pos, tower.pos) <= 0.11)
            .unwrap_or(false);
        let offset = if wave_at_own_tower { 0 } else { 1 };
        return enemy_idx.saturating_sub(offset).clamp(1, lane_last_idx);
    }

    let current_index = closest_lane_path_index(champion.pos, &lane_path);
    let capped_current = current_index.min(LANE_EMPTY_ANCHOR_PROGRESS_MAX_INDEX);
    capped_current.clamp(1, lane_last_idx)
}

fn lane_anchor_pos(
    champion: &ChampionRuntime,
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
) -> Vec2 {
    let lane_path = lane_path_for(&champion.team, &champion.lane);
    let idx = choose_lane_anchor_index(champion, minions, structures);
    lane_path
        .get(idx)
        .copied()
        .unwrap_or(base_position_for(&champion.team))
}

fn lane_wave_front_pos(
    champion: &ChampionRuntime,
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
) -> Vec2 {
    let mut allied: Vec<&MinionRuntime> = minions
        .iter()
        .filter(|m| {
            m.alive
                && normalized_team(&m.team) == normalized_team(&champion.team)
                && normalized_lane(&m.lane) == normalized_lane(&champion.lane)
        })
        .collect();
    allied.sort_by(|a, b| b.path_index.cmp(&a.path_index));
    allied.truncate(3);

    let mut enemy: Vec<&MinionRuntime> = minions
        .iter()
        .filter(|m| {
            m.alive
                && normalized_team(&m.team) != normalized_team(&champion.team)
                && normalized_lane(&m.lane) == normalized_lane(&champion.lane)
        })
        .collect();
    enemy.sort_by(|a, b| b.path_index.cmp(&a.path_index));
    enemy.truncate(3);

    let allied_wave = if allied.is_empty() {
        None
    } else {
        let sum = allied.iter().fold(Vec2 { x: 0.0, y: 0.0 }, |acc, m| Vec2 {
            x: acc.x + m.pos.x,
            y: acc.y + m.pos.y,
        });
        Some(Vec2 {
            x: sum.x / allied.len() as f64,
            y: sum.y / allied.len() as f64,
        })
    };

    let enemy_wave = if enemy.is_empty() {
        None
    } else {
        let sum = enemy.iter().fold(Vec2 { x: 0.0, y: 0.0 }, |acc, m| Vec2 {
            x: acc.x + m.pos.x,
            y: acc.y + m.pos.y,
        });
        Some(Vec2 {
            x: sum.x / enemy.len() as f64,
            y: sum.y / enemy.len() as f64,
        })
    };

    match (allied_wave, enemy_wave) {
        (Some(a), Some(e)) => Vec2 {
            x: (a.x + e.x) * 0.5,
            y: (a.y + e.y) * 0.5,
        },
        (Some(a), None) => a,
        (None, Some(e)) => e,
        (None, None) => lane_anchor_pos(champion, minions, structures),
    }
}

fn lane_pressure_at(
    champion: &ChampionRuntime,
    pos: Vec2,
    champions: &[ChampionRuntime],
    minions: &[MinionRuntime],
    radius: f64,
) -> LanePressure {
    let ally_champions = champions
        .iter()
        .filter(|u| {
            u.alive
                && normalized_team(&u.team) == normalized_team(&champion.team)
                && dist(u.pos, pos) <= radius
        })
        .count();
    let enemy_champions = champions
        .iter()
        .filter(|u| {
            u.alive
                && normalized_team(&u.team) != normalized_team(&champion.team)
                && dist(u.pos, pos) <= radius
        })
        .count();
    let ally_lane_minions = minions
        .iter()
        .filter(|m| {
            m.alive
                && normalized_team(&m.team) == normalized_team(&champion.team)
                && normalized_lane(&m.lane) == normalized_lane(&champion.lane)
                && dist(m.pos, pos) <= radius
        })
        .count();
    let enemy_lane_minions = minions
        .iter()
        .filter(|m| {
            m.alive
                && normalized_team(&m.team) != normalized_team(&champion.team)
                && normalized_lane(&m.lane) == normalized_lane(&champion.lane)
                && dist(m.pos, pos) <= radius
        })
        .count();

    let ally_score = ally_champions as f64 * 1.25 + ally_lane_minions as f64 * 0.48;
    let enemy_score = enemy_champions as f64 * 1.25 + enemy_lane_minions as f64 * 0.48;

    LanePressure {
        ally_champions,
        enemy_champions,
        ally_lane_minions,
        enemy_lane_minions,
        ally_score,
        enemy_score,
    }
}

fn lane_minion_context_distance(
    champion: &ChampionRuntime,
    pos: Vec2,
    minions: &[MinionRuntime],
) -> f64 {
    minions
        .iter()
        .filter(|m| m.alive && normalized_lane(&m.lane) == normalized_lane(&champion.lane))
        .map(|m| dist(pos, m.pos))
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
        .unwrap_or(f64::INFINITY)
}

fn in_lane_trade_context(
    champion: &ChampionRuntime,
    pos: Vec2,
    for_chase: bool,
    champions: &[ChampionRuntime],
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
) -> bool {
    if champion.role == "JGL" {
        return true;
    }
    let Some(profile) = lane_role_profile(champion) else {
        return true;
    };

    let lane_anchor = lane_anchor_pos(champion, minions, structures);
    let wave_front = lane_wave_front_pos(champion, minions, structures);
    let _local_pressure = lane_pressure_at(
        champion,
        pos,
        champions,
        minions,
        LANE_LOCAL_PRESSURE_RADIUS,
    );

    let anchor_budget = profile.chase_leash * if for_chase { 1.05 } else { 0.92 };
    let wave_budget = profile.chase_leash * if for_chase { 1.15 } else { 1.0 };
    let minion_budget = if for_chase {
        LANE_CHASE_MINION_CONTEXT_RADIUS
    } else {
        LANE_MINION_CONTEXT_RADIUS
    };

    if dist(pos, lane_anchor) > anchor_budget {
        return false;
    }
    if dist(pos, wave_front) > wave_budget {
        return false;
    }
    if lane_minion_context_distance(champion, pos, minions) > minion_budget {
        return false;
    }
    true
}

fn is_deep_enemy_tower_zone(
    champion: &ChampionRuntime,
    target_pos: Vec2,
    structures: &[StructureRuntime],
    minions: &[MinionRuntime],
) -> bool {
    let enemy_tower = structures.iter().find(|s| {
        s.alive
            && s.kind == "tower"
            && normalized_team(&s.team) != normalized_team(&champion.team)
            && normalized_lane(&s.lane) == normalized_lane(&champion.lane)
            && dist(s.pos, target_pos) <= 0.1
    });

    let Some(tower) = enemy_tower else {
        return false;
    };

    let allied_wave_near_tower = minions
        .iter()
        .filter(|m| {
            m.alive
                && normalized_team(&m.team) == normalized_team(&champion.team)
                && normalized_lane(&m.lane) == normalized_lane(&champion.lane)
                && dist(m.pos, tower.pos) <= 0.085
        })
        .count();
    allied_wave_near_tower < 2
}

fn is_inside_laner_trade_leash(
    champion: &ChampionRuntime,
    target_pos: Vec2,
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
) -> bool {
    let Some(profile) = lane_role_profile(champion) else {
        return true;
    };
    let lane_anchor = lane_anchor_pos(champion, minions, structures);
    let wave_front = lane_wave_front_pos(champion, minions, structures);
    dist(target_pos, lane_anchor) <= profile.chase_leash
        && dist(target_pos, wave_front) <= profile.chase_leash * 1.15
}

fn should_force_laner_disengage(
    champion: &ChampionRuntime,
    target_pos: Vec2,
    enemy: Option<&ChampionRuntime>,
    champions: &[ChampionRuntime],
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
) -> bool {
    if champion.role == "JGL" {
        return false;
    }
    let Some(profile) = lane_role_profile(champion) else {
        return false;
    };

    let hp_ratio = if champion.max_hp <= 0.0 {
        1.0
    } else {
        champion.hp / champion.max_hp
    };
    if hp_ratio <= profile.retreat_hp {
        return true;
    }
    if !is_inside_laner_trade_leash(champion, target_pos, minions, structures) {
        return true;
    }
    if is_deep_enemy_tower_zone(champion, target_pos, structures, minions) {
        return true;
    }

    let pressure = lane_pressure_at(
        champion,
        target_pos,
        champions,
        minions,
        LANE_LOCAL_PRESSURE_RADIUS,
    );
    if pressure.enemy_score > pressure.ally_score + profile.outnumber_tolerance {
        return true;
    }

    if let Some(enemy_champion) = enemy {
        let enemy_hp_ratio = if enemy_champion.max_hp <= 0.0 {
            1.0
        } else {
            enemy_champion.hp / enemy_champion.max_hp
        };
        if hp_ratio + TRADE_HP_DISADVANTAGE_ALLOWANCE < enemy_hp_ratio {
            return true;
        }
    }

    false
}

fn lane_combat_state_mut<'a>(
    lane_combat_state_by_champion: &'a mut HashMap<String, LanerCombatStateRuntime>,
    champion_id: &str,
) -> &'a mut LanerCombatStateRuntime {
    lane_combat_state_by_champion
        .entry(champion_id.to_string())
        .or_default()
}

fn mark_lane_disengage(
    champion: &ChampionRuntime,
    now: f64,
    lane_combat_state_by_champion: &mut HashMap<String, LanerCombatStateRuntime>,
) {
    if champion.role == "JGL" {
        return;
    }
    let state = lane_combat_state_mut(lane_combat_state_by_champion, &champion.id);
    state.last_disengage_at = now;
    state.reengage_at = f64::max(state.reengage_at, now + LANE_REENGAGE_COOLDOWN_SEC);
    state.recent_trade_until = f64::max(state.recent_trade_until, now + LANE_RECENT_TRADE_LOCK_SEC);
}

fn mark_lane_trade_hit(
    champion: &ChampionRuntime,
    now: f64,
    lane_combat_state_by_champion: &mut HashMap<String, LanerCombatStateRuntime>,
) {
    if champion.role == "JGL" {
        return;
    }
    let state = lane_combat_state_mut(lane_combat_state_by_champion, &champion.id);
    state.recent_trade_until = f64::max(state.recent_trade_until, now + LANE_RECENT_TRADE_LOCK_SEC);
}

fn lane_trade_cooldown_active(
    champion: &ChampionRuntime,
    now: f64,
    lane_combat_state_by_champion: &HashMap<String, LanerCombatStateRuntime>,
) -> bool {
    if champion.role == "JGL" {
        return false;
    }
    lane_combat_state_by_champion
        .get(&champion.id)
        .map(|state| now < state.reengage_at)
        .unwrap_or(false)
}

fn lane_recent_trade_lock_active(
    champion: &ChampionRuntime,
    now: f64,
    lane_combat_state_by_champion: &HashMap<String, LanerCombatStateRuntime>,
) -> bool {
    if champion.role == "JGL" {
        return false;
    }
    lane_combat_state_by_champion
        .get(&champion.id)
        .map(|state| now < state.recent_trade_until)
        .unwrap_or(false)
}

#[derive(Debug, Clone, Copy)]
struct TradeConfidenceFeatures {
    self_hp_ratio: f64,
    enemy_hp_ratio: f64,
    ally_champions_local: usize,
    enemy_champions_local: usize,
    ally_minions_local: usize,
    enemy_minions_local: usize,
    nearest_enemy_tower_distance: f64,
    enemy_overextended: bool,
    first_wave_window: bool,
}

#[derive(Debug, Clone, Copy)]
struct TradeDecisionEvaluation {
    decision: bool,
    rule_decision: bool,
    confidence: f64,
    flipped_by_hybrid: bool,
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

fn clamp_ratio_01(value: f64) -> f64 {
    clamp(value, 0.0, 1.0)
}

fn nearest_enemy_lane_tower_distance(
    champion: &ChampionRuntime,
    target_pos: Vec2,
    structures: &[StructureRuntime],
) -> f64 {
    structures
        .iter()
        .filter(|structure| {
            structure.alive
                && structure.kind == "tower"
                && normalized_team(&structure.team) != normalized_team(&champion.team)
                && normalized_lane(&structure.lane) == normalized_lane(&champion.lane)
        })
        .map(|tower| dist(tower.pos, target_pos))
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
        .unwrap_or(0.4)
}

fn enemy_overextended_in_lane(champion: &ChampionRuntime, enemy: &ChampionRuntime) -> bool {
    let lane_path = lane_path_for(&champion.team, &champion.lane);
    if lane_path.len() < 2 {
        return false;
    }
    let enemy_idx = closest_lane_path_index(enemy.pos, &lane_path);
    let overextended_max_idx = lane_path.len().saturating_sub(1).min(2);
    enemy_idx <= overextended_max_idx
}

fn trade_confidence_score(features: TradeConfidenceFeatures) -> f64 {
    let champion_numbers = clamp_ratio_01(
        (features.ally_champions_local as f64 - features.enemy_champions_local as f64 + 2.0) / 4.0,
    );
    let minion_numbers = clamp_ratio_01(
        (features.ally_minions_local as f64 - features.enemy_minions_local as f64 + 5.0) / 10.0,
    );
    let enemy_tower_distance_norm = clamp_ratio_01(features.nearest_enemy_tower_distance / 0.18);
    let enemy_overextended = if features.enemy_overextended {
        1.0
    } else {
        0.0
    };
    let first_wave_window = if features.first_wave_window { 1.0 } else { 0.0 };

    let logit = TRADE_SCORE_WEIGHT_BIAS
        + TRADE_SCORE_WEIGHT_SELF_HP * clamp_ratio_01(features.self_hp_ratio)
        + TRADE_SCORE_WEIGHT_ENEMY_HP * clamp_ratio_01(features.enemy_hp_ratio)
        + TRADE_SCORE_WEIGHT_CHAMP_NUMBERS * champion_numbers
        + TRADE_SCORE_WEIGHT_MINION_NUMBERS * minion_numbers
        + TRADE_SCORE_WEIGHT_TOWER_DISTANCE * enemy_tower_distance_norm
        + TRADE_SCORE_WEIGHT_ENEMY_OVEREXTENDED * enemy_overextended
        + TRADE_SCORE_WEIGHT_FIRST_WAVE * first_wave_window;

    clamp_ratio_01(sigmoid(logit))
}

fn calibrate_trade_confidence(raw_confidence: f64) -> f64 {
    let raw = clamp_ratio_01(raw_confidence);

    // Telemetry-driven reliability correction:
    // very high raw confidence (>= 0.8) was over-optimistic in AFK runs.
    if raw <= 0.7 {
        return raw;
    }

    // Compress the high-confidence tail while keeping ordering stable.
    0.7 + (raw - 0.7) * 0.35
}

fn trade_confidence_features(
    champion: &ChampionRuntime,
    enemy: &ChampionRuntime,
    now: f64,
    champions: &[ChampionRuntime],
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
) -> TradeConfidenceFeatures {
    let self_hp_ratio = if champion.max_hp <= 0.0 {
        1.0
    } else {
        champion.hp / champion.max_hp
    };
    let enemy_hp_ratio = if enemy.max_hp <= 0.0 {
        1.0
    } else {
        enemy.hp / enemy.max_hp
    };

    let pressure = lane_pressure_at(
        champion,
        enemy.pos,
        champions,
        minions,
        LANE_LOCAL_PRESSURE_RADIUS,
    );
    let nearest_enemy_tower_distance =
        nearest_enemy_lane_tower_distance(champion, enemy.pos, structures);

    TradeConfidenceFeatures {
        self_hp_ratio,
        enemy_hp_ratio,
        ally_champions_local: pressure.ally_champions,
        enemy_champions_local: pressure.enemy_champions,
        ally_minions_local: pressure.ally_lane_minions,
        enemy_minions_local: pressure.enemy_lane_minions,
        nearest_enemy_tower_distance,
        enemy_overextended: enemy_overextended_in_lane(champion, enemy),
        first_wave_window: is_first_wave_contest_active(champion, now),
    }
}

fn maybe_log_hybrid_trade_flip(
    runtime: &mut RuntimeState,
    champion: &ChampionRuntime,
    decision_kind: &str,
    confidence: f64,
    rule_decision: bool,
    hybrid_decision: bool,
) {
    if runtime.ai_mode != SimulatorAiMode::Hybrid || rule_decision == hybrid_decision {
        return;
    }

    let state = lane_combat_state_mut(&mut runtime.lane_combat_state_by_champion, &champion.id);
    if runtime.time_sec < state.last_ai_debug_at + HYBRID_TRADE_DEBUG_LOG_COOLDOWN_SEC {
        return;
    }
    state.last_ai_debug_at = runtime.time_sec;

    log_event(
        runtime,
        &format!(
            "[ai-hybrid] {} {} flip: {} -> {} (score={:.2})",
            champion.name,
            decision_kind,
            if rule_decision {
                "rules-open"
            } else {
                "rules-close"
            },
            if hybrid_decision {
                "hybrid-open"
            } else {
                "hybrid-close"
            },
            confidence
        ),
        "info",
    );
}

fn capture_trade_decision_candidate(
    runtime: &mut RuntimeState,
    champion: &ChampionRuntime,
    enemy: &ChampionRuntime,
    eval: TradeDecisionEvaluation,
    intent: &str,
) {
    let pressure = lane_pressure_at(
        champion,
        enemy.pos,
        &runtime.champions,
        &runtime.minions,
        LANE_LOCAL_PRESSURE_RADIUS,
    );
    let self_hp_ratio = ratio_or_zero(champion.hp, champion.max_hp);
    let enemy_hp_ratio = ratio_or_zero(enemy.hp, enemy.max_hp);
    let nearest_enemy_tower_distance =
        nearest_enemy_lane_tower_distance(champion, enemy.pos, &runtime.structures);
    let objective_danger = 1.0 - clamp_ratio_01(nearest_enemy_tower_distance / 0.18);
    let force_disengage_guardrail = should_force_laner_disengage(
        champion,
        enemy.pos,
        Some(enemy),
        &runtime.champions,
        &runtime.minions,
        &runtime.structures,
    );
    let lane_context_guardrail = in_lane_trade_context(
        champion,
        enemy.pos,
        true,
        &runtime.champions,
        &runtime.minions,
        &runtime.structures,
    ) && in_lane_trade_context(
        champion,
        champion.pos,
        false,
        &runtime.champions,
        &runtime.minions,
        &runtime.structures,
    );

    runtime
        .telemetry_decisions
        .push(TelemetryDecisionCandidate {
            champion_id: champion.id.clone(),
            enemy_id: enemy.id.clone(),
            team: champion.team.clone(),
            role: champion.role.clone(),
            lane: champion.lane.clone(),
            ai_mode: runtime.ai_mode,
            intent: intent.to_string(),
            decision: eval.decision,
            rule_decision: eval.rule_decision,
            confidence: eval.confidence,
            flipped_by_hybrid: eval.flipped_by_hybrid,
            hp_ratio: self_hp_ratio,
            enemy_hp_ratio,
            ally_champions_local: pressure.ally_champions,
            enemy_champions_local: pressure.enemy_champions,
            ally_minions_local: pressure.ally_lane_minions,
            enemy_minions_local: pressure.enemy_lane_minions,
            nearest_enemy_tower_distance,
            enemy_overextended: enemy_overextended_in_lane(champion, enemy),
            objective_danger,
            gold: champion.gold,
            xp: champion.xp,
            level: champion.level,
            force_disengage_guardrail,
            lane_context_guardrail,
        });
}

fn should_commit_all_in_trade(
    champion: &ChampionRuntime,
    enemy: &ChampionRuntime,
    champions: &[ChampionRuntime],
    minions: &[MinionRuntime],
) -> bool {
    if champion.role == "JGL" {
        return true;
    }

    let self_hp = if champion.max_hp <= 0.0 {
        1.0
    } else {
        champion.hp / champion.max_hp
    };
    let enemy_hp = if enemy.max_hp <= 0.0 {
        1.0
    } else {
        enemy.hp / enemy.max_hp
    };

    if enemy_hp <= 0.2 && self_hp >= 0.25 {
        return true;
    }

    let pressure = lane_pressure_at(
        champion,
        enemy.pos,
        champions,
        minions,
        LANE_LOCAL_PRESSURE_RADIUS,
    );
    if pressure.ally_champions > pressure.enemy_champions && self_hp >= 0.32 {
        return true;
    }

    pressure.ally_score >= pressure.enemy_score + 0.9 && self_hp >= enemy_hp
}

fn evaluate_open_trade_window(
    champion: &ChampionRuntime,
    enemy: &ChampionRuntime,
    now: f64,
    champions: &[ChampionRuntime],
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
    lane_combat_state_by_champion: &HashMap<String, LanerCombatStateRuntime>,
    ai_mode: SimulatorAiMode,
    policy: &SimulatorPolicyConfig,
) -> TradeDecisionEvaluation {
    if champion.role == "JGL" {
        return TradeDecisionEvaluation {
            decision: true,
            rule_decision: true,
            confidence: 1.0,
            flipped_by_hybrid: false,
        };
    }
    if dist(champion.pos, enemy.pos) > LANE_CHAMPION_TRADE_RADIUS {
        return TradeDecisionEvaluation {
            decision: false,
            rule_decision: false,
            confidence: 0.0,
            flipped_by_hybrid: false,
        };
    }
    if !in_lane_trade_context(
        champion,
        champion.pos,
        false,
        champions,
        minions,
        structures,
    ) {
        return TradeDecisionEvaluation {
            decision: false,
            rule_decision: false,
            confidence: 0.0,
            flipped_by_hybrid: false,
        };
    }
    if !in_lane_trade_context(champion, enemy.pos, true, champions, minions, structures) {
        return TradeDecisionEvaluation {
            decision: false,
            rule_decision: false,
            confidence: 0.0,
            flipped_by_hybrid: false,
        };
    }
    if should_force_laner_disengage(
        champion,
        enemy.pos,
        Some(enemy),
        champions,
        minions,
        structures,
    ) {
        return TradeDecisionEvaluation {
            decision: false,
            rule_decision: false,
            confidence: 0.0,
            flipped_by_hybrid: false,
        };
    }
    let clear_win_condition = should_commit_all_in_trade(champion, enemy, champions, minions);
    if (lane_trade_cooldown_active(champion, now, lane_combat_state_by_champion)
        || lane_recent_trade_lock_active(champion, now, lane_combat_state_by_champion))
        && !clear_win_condition
    {
        return TradeDecisionEvaluation {
            decision: false,
            rule_decision: false,
            confidence: 0.0,
            flipped_by_hybrid: false,
        };
    }

    let hp_ratio = if champion.max_hp <= 0.0 {
        1.0
    } else {
        champion.hp / champion.max_hp
    };
    let enemy_hp_ratio = if enemy.max_hp <= 0.0 {
        1.0
    } else {
        enemy.hp / enemy.max_hp
    };

    let pressure = lane_pressure_at(
        champion,
        enemy.pos,
        champions,
        minions,
        LANE_LOCAL_PRESSURE_RADIUS,
    );
    let numbers_advantage = pressure.ally_champions > pressure.enemy_champions;
    if numbers_advantage && hp_ratio + 0.02 >= enemy_hp_ratio && hp_ratio >= 0.32 {
        return TradeDecisionEvaluation {
            decision: true,
            rule_decision: true,
            confidence: 1.0,
            flipped_by_hybrid: false,
        };
    }

    let ally_minions_near_fight = minions
        .iter()
        .filter(|m| {
            m.alive
                && normalized_team(&m.team) == normalized_team(&champion.team)
                && normalized_lane(&m.lane) == normalized_lane(&champion.lane)
                && dist(m.pos, enemy.pos) <= 0.1
        })
        .count();
    let enemy_minions_near_fight = minions
        .iter()
        .filter(|m| {
            m.alive
                && normalized_team(&m.team) != normalized_team(&champion.team)
                && normalized_lane(&m.lane) == normalized_lane(&champion.lane)
                && dist(m.pos, enemy.pos) <= 0.1
        })
        .count();

    let total_wave_context = ally_minions_near_fight + enemy_minions_near_fight;
    if total_wave_context < 1 {
        return TradeDecisionEvaluation {
            decision: false,
            rule_decision: false,
            confidence: 0.0,
            flipped_by_hybrid: false,
        };
    }
    if is_first_wave_contest_active(champion, now)
        && (ally_minions_near_fight < 2 || enemy_minions_near_fight < 2)
    {
        return TradeDecisionEvaluation {
            decision: false,
            rule_decision: false,
            confidence: 0.0,
            flipped_by_hybrid: false,
        };
    }
    if ally_minions_near_fight == 0 {
        let low_enemy_window = enemy_hp_ratio <= 0.34;
        let hp_safe_to_trade = hp_ratio >= 0.5;
        if !(low_enemy_window && hp_safe_to_trade) {
            return TradeDecisionEvaluation {
                decision: false,
                rule_decision: false,
                confidence: 0.0,
                flipped_by_hybrid: false,
            };
        }
    }

    let hp_advantage = hp_ratio + 0.08 >= enemy_hp_ratio;
    let wave_pressure = pressure.ally_lane_minions >= pressure.enemy_lane_minions;
    let score_pressure = pressure.ally_score >= pressure.enemy_score - 0.05;
    let rule_decision = hp_advantage && wave_pressure && score_pressure;

    if ai_mode != SimulatorAiMode::Hybrid {
        return TradeDecisionEvaluation {
            decision: rule_decision,
            rule_decision,
            confidence: 0.0,
            flipped_by_hybrid: false,
        };
    }

    let features = trade_confidence_features(champion, enemy, now, champions, minions, structures);
    let confidence = calibrate_trade_confidence(trade_confidence_score(features));
    let hp_gap = enemy_hp_ratio - (hp_ratio + 0.08);
    let wave_gap = pressure.enemy_lane_minions as i64 - pressure.ally_lane_minions as i64;
    let score_gap = pressure.enemy_score - (pressure.ally_score + 0.05);
    let borderline_reject = !rule_decision && hp_gap <= 0.08 && wave_gap <= 2 && score_gap <= 0.35;
    let hybrid_decision = rule_decision
        || (borderline_reject && confidence >= policy.hybrid_open_trade_confidence_high);

    TradeDecisionEvaluation {
        decision: hybrid_decision,
        rule_decision,
        confidence,
        flipped_by_hybrid: hybrid_decision != rule_decision,
    }
}

fn can_open_trade_window(
    champion: &ChampionRuntime,
    enemy: &ChampionRuntime,
    now: f64,
    champions: &[ChampionRuntime],
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
    lane_combat_state_by_champion: &HashMap<String, LanerCombatStateRuntime>,
    ai_mode: SimulatorAiMode,
    policy: &SimulatorPolicyConfig,
) -> bool {
    evaluate_open_trade_window(
        champion,
        enemy,
        now,
        champions,
        minions,
        structures,
        lane_combat_state_by_champion,
        ai_mode,
        policy,
    )
    .decision
}

fn evaluate_disengage_champion_trade(
    champion: &ChampionRuntime,
    enemy: &ChampionRuntime,
    now: f64,
    champions: &[ChampionRuntime],
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
    ai_mode: SimulatorAiMode,
    policy: &SimulatorPolicyConfig,
) -> TradeDecisionEvaluation {
    if champion.role == "JGL" {
        return TradeDecisionEvaluation {
            decision: false,
            rule_decision: false,
            confidence: 1.0,
            flipped_by_hybrid: false,
        };
    }

    if should_force_laner_disengage(
        champion,
        enemy.pos,
        Some(enemy),
        champions,
        minions,
        structures,
    ) {
        return TradeDecisionEvaluation {
            decision: true,
            rule_decision: true,
            confidence: 0.0,
            flipped_by_hybrid: false,
        };
    }

    let self_hp_ratio = if champion.max_hp <= 0.0 {
        1.0
    } else {
        champion.hp / champion.max_hp
    };
    let enemy_hp_ratio = if enemy.max_hp <= 0.0 {
        1.0
    } else {
        enemy.hp / enemy.max_hp
    };
    if self_hp_ratio < policy.trade_retreat_hp_ratio {
        return TradeDecisionEvaluation {
            decision: true,
            rule_decision: true,
            confidence: 0.0,
            flipped_by_hybrid: false,
        };
    }
    if self_hp_ratio + policy.trade_hp_disadvantage_allowance < enemy_hp_ratio {
        return TradeDecisionEvaluation {
            decision: true,
            rule_decision: true,
            confidence: 0.0,
            flipped_by_hybrid: false,
        };
    }

    let ally_champions = champions
        .iter()
        .filter(|u| {
            u.alive
                && normalized_team(&u.team) == normalized_team(&champion.team)
                && dist(u.pos, enemy.pos) <= 0.11
        })
        .count();
    let enemy_champions = champions
        .iter()
        .filter(|u| {
            u.alive
                && normalized_team(&u.team) != normalized_team(&champion.team)
                && dist(u.pos, enemy.pos) <= 0.11
        })
        .count();
    let ally_lane_minions = minions
        .iter()
        .filter(|m| {
            m.alive
                && normalized_team(&m.team) == normalized_team(&champion.team)
                && normalized_lane(&m.lane) == normalized_lane(&champion.lane)
                && dist(m.pos, enemy.pos) <= 0.085
        })
        .count();
    let enemy_lane_minions = minions
        .iter()
        .filter(|m| {
            m.alive
                && normalized_team(&m.team) != normalized_team(&champion.team)
                && normalized_lane(&m.lane) == normalized_lane(&champion.lane)
                && dist(m.pos, enemy.pos) <= 0.085
        })
        .count();

    let allied_pressure = ally_champions as f64 + ally_lane_minions as f64 * 0.5;
    let enemy_pressure = enemy_champions as f64 + enemy_lane_minions as f64 * 0.5;
    if enemy_pressure > allied_pressure + 0.7 {
        return TradeDecisionEvaluation {
            decision: true,
            rule_decision: true,
            confidence: 0.0,
            flipped_by_hybrid: false,
        };
    }

    let lane_anchor = lane_anchor_pos(champion, minions, structures);
    let rule_decision = dist(enemy.pos, lane_anchor) > policy.lane_chase_leash_radius
        && enemy_pressure >= allied_pressure;
    if ai_mode != SimulatorAiMode::Hybrid {
        return TradeDecisionEvaluation {
            decision: rule_decision,
            rule_decision,
            confidence: 0.0,
            flipped_by_hybrid: false,
        };
    }

    let features = trade_confidence_features(champion, enemy, now, champions, minions, structures);
    let confidence = calibrate_trade_confidence(trade_confidence_score(features));
    let pressure_margin = enemy_pressure - (allied_pressure + 0.7);
    let hp_margin = (self_hp_ratio + policy.trade_hp_disadvantage_allowance) - enemy_hp_ratio;
    let leash_margin = dist(enemy.pos, lane_anchor) - policy.lane_chase_leash_radius;
    let borderline_risk = !rule_decision
        && (pressure_margin > -0.2 || hp_margin < 0.04 || leash_margin > -0.008)
        && (self_hp_ratio < policy.trade_retreat_hp_ratio + 0.08);
    let hybrid_decision =
        rule_decision || (borderline_risk && confidence <= policy.hybrid_disengage_confidence_low);

    TradeDecisionEvaluation {
        decision: hybrid_decision,
        rule_decision,
        confidence,
        flipped_by_hybrid: hybrid_decision != rule_decision,
    }
}

fn should_disengage_champion_trade(
    champion: &ChampionRuntime,
    enemy: &ChampionRuntime,
    now: f64,
    champions: &[ChampionRuntime],
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
    ai_mode: SimulatorAiMode,
    policy: &SimulatorPolicyConfig,
) -> bool {
    evaluate_disengage_champion_trade(
        champion, enemy, now, champions, minions, structures, ai_mode, policy,
    )
    .decision
}

fn lane_farm_anchor_pos_v2(
    champion: &ChampionRuntime,
    now: f64,
    champions: &[ChampionRuntime],
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
) -> Vec2 {
    let lane_anchor = lane_anchor_pos(champion, minions, structures);
    let wave_front = lane_wave_front_pos(champion, minions, structures);

    if is_first_wave_contest_active(champion, now) {
        let to_wave = normalize(Vec2 {
            x: wave_front.x - lane_anchor.x,
            y: wave_front.y - lane_anchor.y,
        });
        let approach = lane_role_profile(champion)
            .map(|profile| profile.approach_leash)
            .unwrap_or(0.058);
        let contest_advance = f64::max(
            0.014,
            f64::min(approach * 0.95, dist(lane_anchor, wave_front) * 0.6),
        );
        return Vec2 {
            x: clamp(lane_anchor.x + to_wave.x * contest_advance, 0.01, 0.99),
            y: clamp(lane_anchor.y + to_wave.y * contest_advance, 0.01, 0.99),
        };
    }

    if champion.role == "SUP" {
        let allied_adc = champions
            .iter()
            .filter(|ally| {
                ally.alive
                    && ally.id != champion.id
                    && normalized_team(&ally.team) == normalized_team(&champion.team)
                    && ally.role == "ADC"
            })
            .min_by(|a, b| {
                dist(champion.pos, a.pos)
                    .partial_cmp(&dist(champion.pos, b.pos))
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| a.id.cmp(&b.id))
            });

        if let Some(adc) = allied_adc {
            let to_wave_from_adc = normalize(Vec2 {
                x: wave_front.x - adc.pos.x,
                y: wave_front.y - adc.pos.y,
            });
            let tethered = Vec2 {
                x: adc.pos.x - to_wave_from_adc.x * 0.012,
                y: adc.pos.y - to_wave_from_adc.y * 0.012,
            };
            if dist(tethered, wave_front) <= 0.14 {
                return Vec2 {
                    x: clamp(tethered.x, 0.01, 0.99),
                    y: clamp(tethered.y, 0.01, 0.99),
                };
            }
        }
    }

    let to_wave = normalize(Vec2 {
        x: wave_front.x - lane_anchor.x,
        y: wave_front.y - lane_anchor.y,
    });
    let role_leash = lane_role_profile(champion)
        .map(|profile| profile.approach_leash)
        .unwrap_or(0.058);

    let allied_lane_tower = structures
        .iter()
        .filter(|s| {
            s.alive
                && s.kind == "tower"
                && normalized_team(&s.team) == normalized_team(&champion.team)
                && normalized_lane(&s.lane) == normalized_lane(&champion.lane)
        })
        .min_by(|a, b| {
            dist(champion.pos, a.pos)
                .partial_cmp(&dist(champion.pos, b.pos))
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
    let wave_at_own_tower = allied_lane_tower
        .map(|tower| dist(wave_front, tower.pos) <= 0.11)
        .unwrap_or(false);

    if wave_at_own_tower && champion.role != "SUP" {
        if let Some(tower) = allied_lane_tower {
            let to_wave_from_tower = normalize(Vec2 {
                x: wave_front.x - tower.pos.x,
                y: wave_front.y - tower.pos.y,
            });
            let front_offset = clamp(champion.attack_range * 0.7, 0.02, 0.034);
            return Vec2 {
                x: clamp(
                    tower.pos.x + to_wave_from_tower.x * front_offset,
                    0.01,
                    0.99,
                ),
                y: clamp(
                    tower.pos.y + to_wave_from_tower.y * front_offset,
                    0.01,
                    0.99,
                ),
            };
        }
    }

    let emergency_farm_boost = if wave_at_own_tower { 1.55 } else { 1.0 };
    let advance = f64::min(
        role_leash * emergency_farm_boost,
        f64::max(0.01, dist(lane_anchor, wave_front) * 0.7),
    );

    Vec2 {
        x: clamp(lane_anchor.x + to_wave.x * advance, 0.01, 0.99),
        y: clamp(lane_anchor.y + to_wave.y * advance, 0.01, 0.99),
    }
}

fn lane_trade_approach_pos(
    champion: &ChampionRuntime,
    enemy: &ChampionRuntime,
    now: f64,
    champions: &[ChampionRuntime],
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
) -> Vec2 {
    let anchor = lane_farm_anchor_pos_v2(champion, now, champions, minions, structures);
    let leash = lane_role_profile(champion)
        .map(|profile| profile.approach_leash)
        .unwrap_or(0.058);
    let enemy_from_anchor = normalize(Vec2 {
        x: enemy.pos.x - anchor.x,
        y: enemy.pos.y - anchor.y,
    });
    let desired_spacing = f64::max(0.025, champion.attack_range * 0.9);

    let ideal = Vec2 {
        x: enemy.pos.x - enemy_from_anchor.x * desired_spacing,
        y: enemy.pos.y - enemy_from_anchor.y * desired_spacing,
    };

    let delta = Vec2 {
        x: ideal.x - anchor.x,
        y: ideal.y - anchor.y,
    };
    let dist_from_anchor = dist(ideal, anchor);
    if dist_from_anchor <= leash {
        return Vec2 {
            x: clamp(ideal.x, 0.01, 0.99),
            y: clamp(ideal.y, 0.01, 0.99),
        };
    }

    let capped = normalize(delta);
    Vec2 {
        x: clamp(anchor.x + capped.x * leash, 0.01, 0.99),
        y: clamp(anchor.y + capped.y * leash, 0.01, 0.99),
    }
}

fn lane_retreat_anchor_pos(
    champion: &ChampionRuntime,
    threat_pos: Vec2,
    now: f64,
    champions: &[ChampionRuntime],
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
) -> Vec2 {
    if champion.role == "JGL" {
        return base_position_for(&champion.team);
    }

    let hp_ratio = if champion.max_hp <= 0.0 {
        1.0
    } else {
        champion.hp / champion.max_hp
    };
    if is_first_wave_contest_active(champion, now) && hp_ratio >= 0.45 {
        return lane_farm_anchor_pos_v2(champion, now, champions, minions, structures);
    }

    let farm_anchor = lane_farm_anchor_pos_v2(champion, now, champions, minions, structures);
    let emergency = should_allow_emergency_retreat(champion, threat_pos, champions, minions);
    let Some(tower_idx) =
        pick_allied_lane_fallback_tower(champion, threat_pos, emergency, structures)
    else {
        return farm_anchor;
    };
    let tower = &structures[tower_idx];

    let tower_fallback = lane_fallback_pos_from_tower(champion, tower.pos, emergency);
    if emergency {
        return tower_fallback;
    }

    let lane_path = lane_path_for(&champion.team, &champion.lane);

    let farm_idx = closest_lane_path_index(farm_anchor, &lane_path);
    let tower_idx = closest_lane_path_index(tower_fallback, &lane_path);
    if tower_idx < farm_idx {
        farm_anchor
    } else {
        tower_fallback
    }
}

fn should_allow_emergency_retreat(
    champion: &ChampionRuntime,
    threat_pos: Vec2,
    champions: &[ChampionRuntime],
    minions: &[MinionRuntime],
) -> bool {
    if champion.role == "JGL" {
        return false;
    }
    let Some(profile) = lane_role_profile(champion) else {
        return false;
    };

    let hp_ratio = if champion.max_hp <= 0.0 {
        1.0
    } else {
        champion.hp / champion.max_hp
    };
    if hp_ratio <= profile.retreat_hp {
        return true;
    }

    let pressure = lane_pressure_at(
        champion,
        threat_pos,
        champions,
        minions,
        LANE_LOCAL_PRESSURE_RADIUS,
    );
    let strongly_unfavorable = pressure.enemy_score
        >= pressure.ally_score
            + profile.outnumber_tolerance
            + LANE_STRONG_UNFAVORABLE_PRESSURE_DELTA
        || pressure.enemy_champions >= pressure.ally_champions + 1;
    if !strongly_unfavorable {
        return false;
    }

    hp_ratio < LANE_HEALTHY_RETREAT_HP_RATIO
        || pressure.enemy_champions >= pressure.ally_champions + 2
}

fn pick_allied_lane_fallback_tower(
    champion: &ChampionRuntime,
    threat_pos: Vec2,
    allow_emergency_retreat: bool,
    structures: &[StructureRuntime],
) -> Option<usize> {
    let lane_path = lane_path_for(&champion.team, &champion.lane);
    let mut towers: Vec<(usize, usize)> = structures
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            s.alive
                && s.kind == "tower"
                && normalized_team(&s.team) == normalized_team(&champion.team)
                && normalized_lane(&s.lane) == normalized_lane(&champion.lane)
        })
        .map(|(idx, tower)| (idx, closest_lane_path_index(tower.pos, &lane_path)))
        .collect();

    towers.sort_by(|(idx_a, path_a), (idx_b, path_b)| {
        path_a.cmp(path_b).then_with(|| idx_a.cmp(idx_b))
    });
    if towers.is_empty() {
        return None;
    }

    let threat_index = closest_lane_path_index(threat_pos, &lane_path);
    let mut selected = towers
        .iter()
        .filter(|(_, path_index)| *path_index <= threat_index + 1)
        .max_by(|(idx_a, path_a), (idx_b, path_b)| {
            path_a.cmp(path_b).then_with(|| idx_a.cmp(idx_b))
        })
        .copied();

    if selected.is_none() {
        selected = towers
            .iter()
            .min_by(|(idx_a, path_a), (idx_b, path_b)| {
                dist(threat_pos, structures[*idx_a].pos)
                    .partial_cmp(&dist(threat_pos, structures[*idx_b].pos))
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| path_b.cmp(path_a))
                    .then_with(|| idx_a.cmp(idx_b))
            })
            .copied();
    }

    let Some(selected_entry) = selected else {
        return None;
    };
    if allow_emergency_retreat || towers.len() < 2 {
        return Some(selected_entry.0);
    }

    let mut lane_defense_band = towers.clone();
    lane_defense_band.sort_by(|(idx_a, path_a), (idx_b, path_b)| {
        path_b.cmp(path_a).then_with(|| idx_a.cmp(idx_b))
    });
    lane_defense_band.truncate(2);
    let min_safe_band_index = lane_defense_band
        .iter()
        .map(|(_, path_index)| *path_index)
        .min()
        .unwrap_or(selected_entry.1);

    if selected_entry.1 >= min_safe_band_index {
        return Some(selected_entry.0);
    }

    towers
        .iter()
        .filter(|(_, path_index)| *path_index >= min_safe_band_index)
        .min_by(|(idx_a, path_a), (idx_b, path_b)| {
            path_a
                .abs_diff(min_safe_band_index)
                .cmp(&path_b.abs_diff(min_safe_band_index))
                .then_with(|| path_b.cmp(path_a))
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| *idx)
        .or(Some(selected_entry.0))
}

fn issue_lane_disengage(runtime: &mut RuntimeState, champion_idx: usize, threat_pos: Vec2) {
    if champion_idx >= runtime.champions.len() {
        return;
    }

    let now = runtime.time_sec;
    let champion_snapshot = runtime.champions[champion_idx].clone();
    let fallback = if champion_snapshot.role == "JGL" {
        jgl_disengage_fallback_pos(runtime, &champion_snapshot, threat_pos)
    } else {
        lane_retreat_anchor_pos(
            &champion_snapshot,
            threat_pos,
            now,
            &runtime.champions,
            &runtime.minions,
            &runtime.structures,
        )
    };

    let champion = &mut runtime.champions[champion_idx];
    if champion.role != "JGL" {
        mark_lane_disengage(champion, now, &mut runtime.lane_combat_state_by_champion);
    }
    champion.state = "lane".to_string();
    set_champion_direct_path(champion, fallback);
}

fn nearest_enemy_champion_snapshot<'a>(
    champion: &ChampionRuntime,
    champions: &'a [ChampionRuntime],
    radius: f64,
) -> Option<&'a ChampionRuntime> {
    champions
        .iter()
        .filter(|enemy| {
            enemy.alive
                && enemy.id != champion.id
                && normalized_team(&enemy.team) != normalized_team(&champion.team)
                && dist(enemy.pos, champion.pos) <= radius
        })
        .min_by(|a, b| {
            dist(a.pos, champion.pos)
                .partial_cmp(&dist(b.pos, champion.pos))
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        })
}

fn should_recall_in_place(champion: &ChampionRuntime, champions: &[ChampionRuntime]) -> bool {
    let nearest = nearest_enemy_champion_snapshot(champion, champions, RECALL_SAFE_ENEMY_RADIUS);
    let Some(enemy) = nearest else {
        return true;
    };
    let d = dist(champion.pos, enemy.pos);
    let enemy_reach_time = d / enemy.move_speed.max(0.01);
    enemy_reach_time > RECALL_CHANNEL_SEC + RECALL_REACH_BUFFER_SEC
}

fn recall_fallback_toward_base(
    champion: &ChampionRuntime,
    threat: Option<&ChampionRuntime>,
) -> Vec2 {
    let base = base_position_for(&champion.team);

    let direction = if let Some(enemy) = threat {
        let away = normalize(Vec2 {
            x: champion.pos.x - enemy.pos.x,
            y: champion.pos.y - enemy.pos.y,
        });
        let toward_base = normalize(Vec2 {
            x: base.x - champion.pos.x,
            y: base.y - champion.pos.y,
        });
        normalize(Vec2 {
            x: away.x * 0.8 + toward_base.x * 0.2,
            y: away.y * 0.8 + toward_base.y * 0.2,
        })
    } else {
        normalize(Vec2 {
            x: base.x - champion.pos.x,
            y: base.y - champion.pos.y,
        })
    };

    let step = if champion.role == "JGL" { 0.05 } else { 0.04 };
    Vec2 {
        x: clamp(champion.pos.x + direction.x * step, 0.01, 0.99),
        y: clamp(champion.pos.y + direction.y * step, 0.01, 0.99),
    }
}

fn start_recall(
    champion: &mut ChampionRuntime,
    now: f64,
    champions: &[ChampionRuntime],
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
) {
    if champion.state == "recall" {
        return;
    }

    champion.state = "recall".to_string();
    champion.recall_channel_until = 0.0;
    champion.target_path.clear();
    champion.target_path_index = 0;
    champion.recall_anchor = if should_recall_in_place(champion, champions) {
        Some(champion.pos)
    } else {
        let nearest =
            nearest_enemy_champion_snapshot(champion, champions, RECALL_SAFE_ENEMY_RADIUS)
                .or_else(|| nearest_enemy_champion_snapshot(champion, champions, f64::INFINITY));
        if let Some(threat) = nearest {
            if champion.role == "JGL" {
                Some(recall_fallback_toward_base(champion, Some(threat)))
            } else {
                Some(lane_retreat_anchor_pos(
                    champion, threat.pos, now, champions, minions, structures,
                ))
            }
        } else {
            if champion.role == "JGL" {
                Some(base_position_for(&champion.team))
            } else {
                Some(lane_retreat_anchor_pos(
                    champion,
                    champion.pos,
                    now,
                    champions,
                    minions,
                    structures,
                ))
            }
        }
    };
}

fn cancel_recall(champion: &mut ChampionRuntime, now: f64, events: &mut Vec<RuntimeEvent>) {
    if champion.state != "recall" {
        return;
    }

    let was_channeling = champion.recall_channel_until > now;
    champion.state = "lane".to_string();
    champion.recall_anchor = None;
    champion.recall_channel_until = 0.0;

    if was_channeling {
        push_event(
            events,
            now,
            &format!("{} recall interrupted", champion.name),
            "recall",
        );
    }
}

fn tick_recall(
    champion: &mut ChampionRuntime,
    now: f64,
    champions: &[ChampionRuntime],
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
    events: &mut Vec<RuntimeEvent>,
) -> bool {
    if champion.state != "recall" {
        return false;
    }

    if champion.recall_channel_until > 0.0 && now >= champion.recall_channel_until {
        champion.pos = base_position_for(&champion.team);
        champion.hp = champion.max_hp;
        maybe_upgrade_trinket_to_oracle(champion, now);
        champion.state = "lane".to_string();
        champion.recall_anchor = None;
        champion.recall_channel_until = 0.0;
        champion.target_path.clear();
        champion.target_path_index = 0;
        champion.next_decision_at = now;
        push_event(
            events,
            now,
            &format!("{} recalled", champion.name),
            "recall",
        );
        return false;
    }

    if champion.recall_channel_until > now {
        return true;
    }

    let anchor = champion.recall_anchor.unwrap_or(champion.pos);
    if dist(champion.pos, anchor) > 0.012 {
        set_champion_direct_path(champion, anchor);
        return true;
    }

    if !should_recall_in_place(champion, champions) {
        let threat = nearest_enemy_champion_snapshot(champion, champions, RECALL_SAFE_ENEMY_RADIUS)
            .or_else(|| nearest_enemy_champion_snapshot(champion, champions, f64::INFINITY));
        let fallback_anchor = if champion.role == "JGL" {
            recall_fallback_toward_base(champion, threat)
        } else {
            let threat_pos = threat.map(|enemy| enemy.pos).unwrap_or(champion.pos);
            lane_retreat_anchor_pos(champion, threat_pos, now, champions, minions, structures)
        };
        champion.recall_anchor = Some(fallback_anchor);
        set_champion_direct_path(champion, fallback_anchor);
        return true;
    }

    champion.recall_channel_until = now + RECALL_CHANNEL_SEC;
    champion.target_path.clear();
    champion.target_path_index = 0;
    push_event(
        events,
        now,
        &format!("{} started recall", champion.name),
        "recall",
    );
    true
}

fn decide_champion_state(
    champion: &mut ChampionRuntime,
    now: f64,
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
    champions: &[ChampionRuntime],
    neutral_timers: Option<&NeutralTimersRuntime>,
    team_tactics: &RuntimeTeamTactics,
    team_buffs: &RuntimeTeamBuffState,
) {
    if champion.state == "recall" {
        return;
    }

    let hp_ratio = if champion.max_hp <= 0.0 {
        1.0
    } else {
        champion.hp / champion.max_hp
    };

    if hp_ratio <= RECALL_TRIGGER_HP_RATIO {
        start_recall(champion, now, champions, minions, structures);
        return;
    }

    if let Some(defense_pos) =
        allied_nexus_under_threat_pos(champion, champions, minions, structures)
    {
        if dist(champion.pos, defense_pos) > BASE_DEFENSE_RECALL_DISTANCE {
            start_recall(champion, now, champions, minions, structures);
        } else {
            champion.state = "objective".to_string();
            set_champion_direct_path_hysteresis(
                champion,
                defense_pos,
                OBJECTIVE_PATH_MIN_TARGET_DELTA,
            );
        }
        return;
    }

    if team_buffs.baron_until > now {
        if let Some(lane) = weakest_enemy_lane_for_team(structures, &champion.team) {
            if let Some(push_target) = baron_push_target_for_lane(structures, &champion.team, lane)
            {
                champion.state = "objective".to_string();
                set_champion_direct_path_hysteresis(
                    champion,
                    push_target,
                    OBJECTIVE_PATH_MIN_TARGET_DELTA,
                );
                return;
            }
        }
    }

    if let Some(timers) = neutral_timers {
        let contested_dragon = contested_dragon_attempt_for_team(&champion.team, champions, timers);
        if should_hard_assist_contested_dragon(champion, contested_dragon) {
            if let Some(dragon) = contested_dragon {
                champion.state = "objective".to_string();
                set_champion_direct_path_hysteresis(
                    champion,
                    dragon.pos,
                    OBJECTIVE_PATH_MIN_TARGET_DELTA,
                );
                return;
            }
        }

        if should_assist_objective_attempt(champion, champions, timers) {
            if let Some(attempt) =
                active_objective_attempt_for_team(&champion.team, champions, timers)
            {
                champion.state = "objective".to_string();
                set_champion_direct_path_hysteresis(
                    champion,
                    attempt.pos,
                    OBJECTIVE_PATH_MIN_TARGET_DELTA,
                );
                return;
            }
        }

        if champion.role == "JGL" {
            if let Some(objective_pos) =
                pick_macro_objective_pos(champion, timers, now, team_tactics)
            {
                champion.state = "objective".to_string();
                set_champion_direct_path_hysteresis(
                    champion,
                    objective_pos,
                    OBJECTIVE_PATH_MIN_TARGET_DELTA,
                );
                return;
            }
        }

        if champion.role == "SUP" && now >= SUPPORT_ROAM_UNLOCK_AT_SEC {
            if now < SUPPORT_OPEN_ROAM_AT_SEC {
                let roam_target_role = match team_tactics.support_roaming.as_str() {
                    "RoamMid" => Some("MID"),
                    "RoamTop" => Some("TOP"),
                    _ => None,
                };
                if let Some(target_role) = roam_target_role {
                    if champion.support_roam_uses < 2 && now >= champion.support_roam_cd_until {
                        let ally_target = champions.iter().find(|ally| {
                            ally.alive
                                && ally.id != champion.id
                                && normalized_team(&ally.team) == normalized_team(&champion.team)
                                && ally.role == target_role
                        });
                        if let Some(ally_target) = ally_target {
                            champion.state = "objective".to_string();
                            champion.support_roam_uses += 1;
                            champion.support_roam_cd_until = now + 85.0;
                            champion.support_last_roam_role = target_role.to_string();
                            set_champion_direct_path_hysteresis(
                                champion,
                                ally_target.pos,
                                OBJECTIVE_PATH_MIN_TARGET_DELTA,
                            );
                            return;
                        }
                    }
                }
            } else if now >= champion.support_roam_cd_until {
                let ally_target = champions
                    .iter()
                    .filter(|ally| {
                        ally.alive
                            && ally.id != champion.id
                            && normalized_team(&ally.team) == normalized_team(&champion.team)
                            && (ally.role == "TOP" || ally.role == "MID" || ally.role == "ADC")
                    })
                    .min_by(|a, b| {
                        let a_ratio = if a.max_hp <= 0.0 {
                            1.0
                        } else {
                            a.hp / a.max_hp
                        };
                        let b_ratio = if b.max_hp <= 0.0 {
                            1.0
                        } else {
                            b.hp / b.max_hp
                        };
                        let a_repeat_penalty = if !champion.support_last_roam_role.is_empty()
                            && a.role
                                .eq_ignore_ascii_case(&champion.support_last_roam_role)
                        {
                            1
                        } else {
                            0
                        };
                        let b_repeat_penalty = if !champion.support_last_roam_role.is_empty()
                            && b.role
                                .eq_ignore_ascii_case(&champion.support_last_roam_role)
                        {
                            1
                        } else {
                            0
                        };

                        a_repeat_penalty
                            .cmp(&b_repeat_penalty)
                            .then_with(|| a_ratio.partial_cmp(&b_ratio).unwrap_or(Ordering::Equal))
                            .then_with(|| {
                                dist(champion.pos, a.pos)
                                    .partial_cmp(&dist(champion.pos, b.pos))
                                    .unwrap_or(Ordering::Equal)
                            })
                    });

                if let Some(ally_target) = ally_target {
                    champion.state = "objective".to_string();
                    champion.support_roam_cd_until = now + 55.0;
                    champion.support_last_roam_role = ally_target.role.clone();
                    set_champion_direct_path_hysteresis(
                        champion,
                        ally_target.pos,
                        OBJECTIVE_PATH_MIN_TARGET_DELTA,
                    );
                    return;
                }
            }
        }
    }

    champion.state = "lane".to_string();
    let target = if now < LANE_COMBAT_UNLOCK_AT {
        lane_pre_wave_hold_pos(champion, structures)
    } else {
        lane_farm_anchor_pos_v2(champion, now, champions, minions, structures)
    };
    set_champion_direct_path(champion, target);
}

fn is_objective_neutral_key(key: &str) -> bool {
    matches!(key, "dragon" | "baron" | "herald" | "voidgrubs" | "elder")
}

fn objective_adjacent_lanes(key: &str) -> &'static [&'static str] {
    if key == "dragon" || key == "elder" || key == "scuttle-bot" {
        &["mid", "bot"]
    } else {
        &["mid", "top"]
    }
}

fn is_jungle_camp_key(key: &str) -> bool {
    matches!(
        key,
        "blue-buff-blue"
            | "blue-buff-red"
            | "red-buff-blue"
            | "red-buff-red"
            | "wolves-blue"
            | "wolves-red"
            | "raptors-blue"
            | "raptors-red"
            | "gromp-blue"
            | "gromp-red"
            | "krugs-blue"
            | "krugs-red"
            | "scuttle-top"
            | "scuttle-bot"
    )
}

fn is_enemy_jungle_camp_key_for_team(key: &str, team: &str) -> bool {
    if !is_jungle_camp_key(key) {
        return false;
    }
    let own_suffix = if normalized_team(team) == "blue" {
        "-blue"
    } else {
        "-red"
    };
    (key.ends_with("-blue") || key.ends_with("-red")) && !key.ends_with(own_suffix)
}

fn contested_dragon_attempt_for_team<'a>(
    team: &str,
    champions: &[ChampionRuntime],
    neutral_timers: &'a NeutralTimersRuntime,
) -> Option<&'a NeutralTimerRuntime> {
    let dragon = neutral_timers.entities.get("dragon")?;
    if !dragon.alive {
        return None;
    }

    let allied_jungler = champions.iter().find(|champion| {
        champion.alive
            && normalized_team(&champion.team) == normalized_team(team)
            && champion.role == "JGL"
    })?;

    if dist(allied_jungler.pos, dragon.pos) > OBJECTIVE_ASSIST_RADIUS {
        return None;
    }

    let enemy_team = if normalized_team(team) == "blue" {
        "red"
    } else {
        "blue"
    };

    let enemy_contestants = champions
        .iter()
        .filter(|enemy| {
            enemy.alive
                && normalized_team(&enemy.team) == enemy_team
                && dist(enemy.pos, dragon.pos) <= OBJECTIVE_ASSIST_RADIUS
        })
        .count();
    if enemy_contestants == 0 {
        return None;
    }

    let dragon_being_done = dragon.hp <= dragon.max_hp * 0.97
        || dist(allied_jungler.pos, dragon.pos) <= OBJECTIVE_ATTEMPT_RADIUS;
    if !dragon_being_done {
        return None;
    }

    Some(dragon)
}

fn nearby_neutral_objective_key(
    champion: &ChampionRuntime,
    neutral_timers: &NeutralTimersRuntime,
) -> Option<String> {
    neutral_timers
        .entities
        .values()
        .filter(|timer| timer.alive && is_objective_neutral_key(&timer.key))
        .filter(|timer| dist(champion.pos, timer.pos) <= OBJECTIVE_ATTEMPT_RADIUS)
        .min_by(|a, b| {
            dist(champion.pos, a.pos)
                .partial_cmp(&dist(champion.pos, b.pos))
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.key.cmp(&b.key))
        })
        .map(|timer| timer.key.clone())
}

fn active_objective_attempt_for_team<'a>(
    team: &str,
    champions: &[ChampionRuntime],
    neutral_timers: &'a NeutralTimersRuntime,
) -> Option<&'a NeutralTimerRuntime> {
    let allied_jungler = champions.iter().find(|champion| {
        champion.alive
            && normalized_team(&champion.team) == normalized_team(team)
            && champion.role == "JGL"
    })?;

    let enemy_team = if normalized_team(team) == "blue" {
        "red"
    } else {
        "blue"
    };

    neutral_timers
        .entities
        .values()
        .filter(|timer| timer.alive && is_objective_neutral_key(&timer.key))
        .filter_map(|timer| {
            let d = dist(allied_jungler.pos, timer.pos);
            if d > OBJECTIVE_ASSIST_RADIUS {
                return None;
            }

            let enemy_contest = champions.iter().any(|enemy| {
                enemy.alive
                    && normalized_team(&enemy.team) == enemy_team
                    && dist(enemy.pos, timer.pos) <= OBJECTIVE_ASSIST_RADIUS
            });
            let is_damaged = timer.hp <= timer.max_hp * 0.9;
            if !(enemy_contest || is_damaged) {
                return None;
            }

            Some((timer, d))
        })
        .min_by(|(a, d_a), (b, d_b)| {
            d_a.partial_cmp(d_b)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.key.cmp(&b.key))
        })
        .map(|(timer, _)| timer)
}

fn should_assist_objective_attempt(
    champion: &ChampionRuntime,
    champions: &[ChampionRuntime],
    neutral_timers: &NeutralTimersRuntime,
) -> bool {
    if champion.role == "JGL" {
        return false;
    }

    let Some(attempt) =
        active_objective_attempt_for_team(&champion.team, champions, neutral_timers)
    else {
        return false;
    };

    let iq_delta = stat_delta(champion.iq_score);
    let discipline_delta = stat_delta(champion.competitive_score);
    let proactive_rotation = iq_delta > -0.2;

    if is_major_teamfight_objective(attempt, neutral_timers) {
        return dist(champion.pos, attempt.pos) <= MAJOR_OBJECTIVE_TEAM_ASSIST_RADIUS
            && can_rotate_without_suicide(champion, attempt.pos, champions);
    }

    let lane = normalized_lane(&champion.lane);
    let role = champion.role.as_str();
    let role_priority = match attempt.key.as_str() {
        "voidgrubs" | "herald" | "baron" => role == "TOP" || role == "MID",
        "dragon" | "elder" => role == "ADC" || role == "SUP" || role == "MID",
        _ => role == "MID",
    };
    if role_priority
        && proactive_rotation
        && can_rotate_without_suicide(champion, attempt.pos, champions)
    {
        return true;
    }

    if !objective_adjacent_lanes(&attempt.key)
        .iter()
        .any(|adj| *adj == lane)
    {
        return false;
    }

    let enemy_team = if normalized_team(&champion.team) == "blue" {
        "red"
    } else {
        "blue"
    };
    let nearby_contestants = champions
        .iter()
        .filter(|enemy| {
            enemy.alive
                && normalized_team(&enemy.team) == enemy_team
                && dist(enemy.pos, attempt.pos) <= OBJECTIVE_ASSIST_RADIUS
        })
        .count();

    let patience_gate = (0.82 - iq_delta * 0.06 - discipline_delta * 0.03).clamp(0.70, 0.90);
    if nearby_contestants == 0 && attempt.hp > attempt.max_hp * patience_gate {
        return false;
    }

    true
}

fn should_hard_assist_contested_dragon(
    champion: &ChampionRuntime,
    contested_dragon: Option<&NeutralTimerRuntime>,
) -> bool {
    if champion.role != "ADC" && champion.role != "SUP" {
        return false;
    }
    if normalized_lane(&champion.lane) != "bot" {
        return false;
    }
    contested_dragon.is_some()
}

fn is_major_teamfight_objective(
    attempt: &NeutralTimerRuntime,
    neutral_timers: &NeutralTimersRuntime,
) -> bool {
    attempt.key == "elder"
        || attempt.key == "baron"
        || (attempt.key == "dragon" && neutral_timers.dragon_soul_unlocked)
}

fn can_rotate_without_suicide(
    champion: &ChampionRuntime,
    objective_pos: Vec2,
    champions: &[ChampionRuntime],
) -> bool {
    let hp_ratio = ratio_or_zero(champion.hp, champion.max_hp);
    let iq_delta = stat_delta(champion.iq_score);
    let hp_floor = (0.38 - iq_delta * 0.06).clamp(0.28, 0.46);
    if hp_ratio < hp_floor {
        return false;
    }

    let ally_nearby = champions
        .iter()
        .filter(|ally| {
            ally.alive
                && normalized_team(&ally.team) == normalized_team(&champion.team)
                && dist(ally.pos, objective_pos) <= OBJECTIVE_ASSIST_RADIUS
        })
        .count();
    let enemy_nearby = champions
        .iter()
        .filter(|enemy| {
            enemy.alive
                && normalized_team(&enemy.team) != normalized_team(&champion.team)
                && dist(enemy.pos, objective_pos) <= OBJECTIVE_ASSIST_RADIUS
        })
        .count();

    let sync_bonus = if champion.iq_score >= 74.0 { 1 } else { 0 };
    ally_nearby + 1 + sync_bonus >= enemy_nearby
}

fn allied_nexus_under_threat_pos(
    champion: &ChampionRuntime,
    champions: &[ChampionRuntime],
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
) -> Option<Vec2> {
    let allied_nexus_towers: Vec<&StructureRuntime> = structures
        .iter()
        .filter(|structure| {
            structure.alive
                && structure.kind == "tower"
                && structure.id.contains("nexus")
                && normalized_team(&structure.team) == normalized_team(&champion.team)
        })
        .collect();

    if allied_nexus_towers.is_empty() {
        return None;
    }

    for tower in allied_nexus_towers {
        let champion_threat = champions.iter().any(|enemy| {
            enemy.alive
                && normalized_team(&enemy.team) != normalized_team(&champion.team)
                && dist(enemy.pos, tower.pos) <= NEXUS_DEFENSE_THREAT_RADIUS
        });
        let minion_threat = minions.iter().any(|enemy| {
            enemy.alive
                && normalized_team(&enemy.team) != normalized_team(&champion.team)
                && dist(enemy.pos, tower.pos) <= NEXUS_DEFENSE_THREAT_RADIUS
        });
        if champion_threat || minion_threat {
            return Some(tower.pos);
        }
    }

    None
}

fn pick_macro_objective_pos(
    champion: &ChampionRuntime,
    neutral_timers: &NeutralTimersRuntime,
    now: f64,
    team_tactics: &RuntimeTeamTactics,
) -> Option<Vec2> {
    if champion.role != "JGL" {
        return None;
    }

    let objective_lead_time = match team_tactics.game_timing.as_str() {
        "Early" => 50.0,
        "Late" => 22.0,
        _ => 35.0,
    };

    for key in ["elder", "baron"] {
        let Some(timer) = neutral_timers.entities.get(key) else {
            continue;
        };
        if !timer.unlocked {
            continue;
        }
        if timer.alive {
            return Some(timer.pos);
        }
        if let Some(next_spawn_at) = timer.next_spawn_at {
            if next_spawn_at >= now && next_spawn_at - now <= objective_lead_time {
                return Some(timer.pos);
            }
        }
    }

    let side_objective_order: [&str; 5] = match team_tactics.strong_side.as_str() {
        "Top" => [
            "herald",
            "voidgrubs",
            "dragon",
            "scuttle-top",
            "scuttle-bot",
        ],
        "Mid" => [
            "dragon",
            "herald",
            "voidgrubs",
            "scuttle-bot",
            "scuttle-top",
        ],
        _ => [
            "dragon",
            "scuttle-bot",
            "herald",
            "voidgrubs",
            "scuttle-top",
        ],
    };

    let can_hard_invade = team_tactics.jungle_style == "Invader"
        || (now >= 14.0 * 60.0 && champion.kills >= champion.deaths + 2);

    if team_tactics.jungle_style == "Farmer" {
        for key in
            jungler_macro_jungle_priority_for_team(&champion.team, &team_tactics.jungle_pathing)
        {
            if is_enemy_jungle_camp_key_for_team(key, &champion.team) && !can_hard_invade {
                continue;
            }
            let Some(timer) = neutral_timers.entities.get(key) else {
                continue;
            };
            if !timer.unlocked {
                continue;
            }
            if timer.alive {
                return Some(timer.pos);
            }
            if let Some(next_spawn_at) = timer.next_spawn_at {
                if next_spawn_at >= now && next_spawn_at - now <= objective_lead_time {
                    return Some(timer.pos);
                }
            }
        }
    }

    for key in side_objective_order {
        let Some(timer) = neutral_timers.entities.get(key) else {
            continue;
        };
        if !timer.unlocked {
            continue;
        }
        if timer.alive {
            return Some(timer.pos);
        }
        if let Some(next_spawn_at) = timer.next_spawn_at {
            if next_spawn_at >= now && next_spawn_at - now <= objective_lead_time {
                return Some(timer.pos);
            }
        }
    }

    for key in jungler_macro_jungle_priority_for_team(&champion.team, &team_tactics.jungle_pathing)
    {
        if is_enemy_jungle_camp_key_for_team(key, &champion.team) && !can_hard_invade {
            continue;
        }
        let Some(timer) = neutral_timers.entities.get(key) else {
            continue;
        };
        if !timer.unlocked {
            continue;
        }
        if timer.alive {
            return Some(timer.pos);
        }
        if let Some(next_spawn_at) = timer.next_spawn_at {
            if next_spawn_at >= now && next_spawn_at - now <= objective_lead_time {
                return Some(timer.pos);
            }
        }
    }

    None
}

fn jungler_macro_jungle_priority_for_team(team: &str, jungle_pathing: &str) -> Vec<&'static str> {
    let (own_top, own_bot, enemy_top, enemy_bot): ([&str; 3], [&str; 3], [&str; 3], [&str; 3]) =
        if normalized_team(team) == "red" {
            (
                ["blue-buff-red", "wolves-red", "gromp-red"],
                ["red-buff-red", "raptors-red", "krugs-red"],
                ["blue-buff-blue", "wolves-blue", "gromp-blue"],
                ["red-buff-blue", "raptors-blue", "krugs-blue"],
            )
        } else {
            (
                ["blue-buff-blue", "wolves-blue", "gromp-blue"],
                ["red-buff-blue", "raptors-blue", "krugs-blue"],
                ["blue-buff-red", "wolves-red", "gromp-red"],
                ["red-buff-red", "raptors-red", "krugs-red"],
            )
        };

    if jungle_pathing == "BotToTop" {
        vec![
            own_bot[0],
            own_bot[1],
            own_bot[2],
            "scuttle-bot",
            own_top[0],
            own_top[1],
            own_top[2],
            "scuttle-top",
            enemy_top[0],
            enemy_top[1],
            enemy_top[2],
            enemy_bot[0],
            enemy_bot[1],
            enemy_bot[2],
        ]
    } else {
        vec![
            own_top[0],
            own_top[1],
            own_top[2],
            "scuttle-top",
            own_bot[0],
            own_bot[1],
            own_bot[2],
            "scuttle-bot",
            enemy_bot[0],
            enemy_bot[1],
            enemy_bot[2],
            enemy_top[0],
            enemy_top[1],
            enemy_top[2],
        ]
    }
}

fn minion_stats(kind: &str) -> (f64, f64, f64, f64) {
    if kind == "ranged" {
        (
            MINION_RANGED_MOVE_SPEED,
            MINION_RANGED_ATTACK_RANGE,
            MINION_RANGED_ATTACK_DAMAGE,
            MINION_RANGED_ATTACK_CADENCE,
        )
    } else {
        (
            MINION_MELEE_MOVE_SPEED,
            MINION_MELEE_ATTACK_RANGE,
            MINION_MELEE_ATTACK_DAMAGE,
            MINION_MELEE_ATTACK_CADENCE,
        )
    }
}

fn spawn_waves_if_due(runtime: &mut RuntimeState, session: &mut LolSimV2Session) {
    while runtime.time_sec >= session.wave_spawn_at {
        spawn_wave(runtime, session);
        session.wave_spawn_at += wave_interval_sec(session.wave_spawn_at);
    }
}

fn spawn_wave(runtime: &mut RuntimeState, session: &mut LolSimV2Session) {
    for lane in ["top", "mid", "bot"] {
        for i in 0..3 {
            runtime
                .minions
                .push(build_minion(session, "blue", lane, "melee", i));
            runtime
                .minions
                .push(build_minion(session, "red", lane, "melee", i));
        }
        for i in 0..3 {
            runtime
                .minions
                .push(build_minion(session, "blue", lane, "ranged", i));
            runtime
                .minions
                .push(build_minion(session, "red", lane, "ranged", i));
        }
    }

    log_event(runtime, "Minion wave spawned", "spawn");
}

fn build_minion(
    session: &mut LolSimV2Session,
    team: &str,
    lane: &str,
    kind: &str,
    slot: i32,
) -> MinionRuntime {
    let path = lane_path_for(team, lane);
    let (move_speed, attack_range, attack_damage, _) = minion_stats(kind);
    let max_hp = if kind == "ranged" {
        MINION_RANGED_MAX_HP
    } else {
        MINION_MELEE_MAX_HP
    };

    let id = format!("m-{}", session.next_minion_id);
    session.next_minion_id += 1;

    MinionRuntime {
        id,
        team: team.to_string(),
        lane: normalized_lane(lane).to_string(),
        pos: spawn_formation_position(&path, kind, slot),
        hp: max_hp,
        max_hp,
        alive: true,
        kind: kind.to_string(),
        last_hit_by_champion_id: None,
        owner_champion_id: None,
        summon_kind: None,
        summon_expires_at: 0.0,
        attack_cd_until: 0.0,
        move_speed,
        attack_range,
        attack_damage,
        path,
        path_index: 1,
    }
}

fn spawn_formation_position(path: &[Vec2], kind: &str, slot: i32) -> Vec2 {
    let origin = path.first().copied().unwrap_or(Vec2 { x: 0.5, y: 0.5 });
    let next = path.get(1).copied().unwrap_or(origin);
    let direction = normalize(Vec2 {
        x: next.x - origin.x,
        y: next.y - origin.y,
    });
    let perpendicular = Vec2 {
        x: -direction.y,
        y: direction.x,
    };
    let row = if kind == "melee" { 0.0 } else { 1.0 };
    let column = f64::from(slot) - 1.0;
    let depth = row * 0.0105 + column.abs() * 0.002;
    let lateral = column * 0.0048;

    Vec2 {
        x: clamp(
            origin.x - direction.x * depth + perpendicular.x * lateral,
            0.01,
            0.99,
        ),
        y: clamp(
            origin.y - direction.y * depth + perpendicular.y * lateral,
            0.01,
            0.99,
        ),
    }
}

fn move_champions(runtime: &mut RuntimeState, dt: f64) {
    let now = runtime.time_sec;
    let champion_snapshot = runtime.champions.clone();
    let neutral_timers_snapshot = decode_neutral_timers_state(&runtime.neutral_timers);
    let team_tactics_snapshot = runtime.extra.get("teamTactics").cloned();
    let team_buffs_snapshot = runtime.extra.get("teamBuffs").cloned();

    for champion in &mut runtime.champions {
        if champion.realm_banished_until > 0.0 {
            if now >= champion.realm_banished_until {
                champion.realm_banished_until = 0.0;
                if let Some(return_pos) = champion.realm_return_pos {
                    champion.pos = return_pos;
                }
                champion.realm_return_pos = None;
                champion.target_path.clear();
                champion.target_path_index = 0;
                champion.next_decision_at = now;
                continue;
            } else {
                continue;
            }
        }

        if !champion.alive {
            if now >= champion.respawn_at {
                champion.alive = true;
                champion.hp = champion.max_hp;
                champion.pos = base_position_for(&champion.team);
                maybe_upgrade_trinket_to_oracle(champion, now);
                champion.attack_cd_until = now;
                champion.state = "lane".to_string();
                champion.recall_anchor = None;
                champion.recall_channel_until = 0.0;
                champion.target_path.clear();
                champion.target_path_index = 0;
                champion.next_decision_at = now;
            } else {
                continue;
            }
        }

        if now >= champion.next_decision_at {
            decide_champion_state(
                champion,
                now,
                &runtime.minions,
                &runtime.structures,
                &champion_snapshot,
                neutral_timers_snapshot.as_ref(),
                &team_tactics_for_runtime(team_tactics_snapshot.as_ref(), &champion.team),
                &team_buffs_for_runtime(team_buffs_snapshot.as_ref(), &champion.team),
            );
            champion.next_decision_at = now + CHAMPION_DECISION_CADENCE_SEC;
        }

        if champion.state == "recall" {
            tick_recall(
                champion,
                now,
                &champion_snapshot,
                &runtime.minions,
                &runtime.structures,
                &mut runtime.events,
            );
            if champion.state == "recall" && champion.recall_channel_until > now {
                continue;
            }
        }

        if champion.target_path.is_empty() {
            champion.target_path = lane_path_for(&champion.team, &champion.lane);
            champion.target_path_index = 1;
        }

        if champion.target_path_index >= champion.target_path.len() {
            champion.target_path_index = champion.target_path.len().saturating_sub(1);
        }

        if let Some(target) = champion
            .target_path
            .get(champion.target_path_index)
            .copied()
        {
            let buffs = team_buffs_for_runtime(team_buffs_snapshot.as_ref(), &champion.team);
            let mut speed_multiplier =
                1.0 + buffs.cloud_stacks as f64 * 0.015 + buffs.hextech_stacks as f64 * 0.01;
            if buffs.soul_kind.as_deref() == Some("cloud") {
                speed_multiplier += 0.08;
            }
            if buffs.soul_kind.as_deref() == Some("hextech") {
                speed_multiplier += 0.04;
            }
            move_entity(
                &mut champion.pos,
                target,
                champion.move_speed * speed_multiplier,
                dt,
            );
            if dist(champion.pos, target) < 0.01
                && champion.target_path_index < champion.target_path.len().saturating_sub(1)
            {
                champion.target_path_index += 1;
            }
        }

        let buffs = team_buffs_for_runtime(team_buffs_snapshot.as_ref(), &champion.team);
        let mut ocean_regen = buffs.ocean_stacks as f64 * 0.45;
        if buffs.soul_kind.as_deref() == Some("ocean") {
            ocean_regen += 1.2;
        }
        if ocean_regen > 0.0 && (now - champion.last_damaged_at) >= 5.0 {
            champion.hp = (champion.hp + ocean_regen * dt).min(champion.max_hp);
        }

        champion.pos.x = clamp(champion.pos.x, 0.01, 0.99);
        champion.pos.y = clamp(champion.pos.y, 0.01, 0.99);

        if champion.state == "recall" {
            tick_recall(
                champion,
                now,
                &champion_snapshot,
                &runtime.minions,
                &runtime.structures,
                &mut runtime.events,
            );
        }
    }
}

fn minion_has_lane_combat_target(
    minion: &MinionRuntime,
    minions: &[MinionRuntime],
    champions: &[ChampionRuntime],
    structures: &[StructureRuntime],
) -> bool {
    let structure_range = minion.attack_range.max(MINION_STRUCTURE_AGGRO_RANGE);
    if nearest_enemy_structure_index(
        structures,
        &minion.team,
        &minion.lane,
        minion.pos,
        structure_range,
    )
    .is_some()
    {
        return true;
    }

    let minion_range = minion.attack_range.max(0.05);
    let nearby_enemy_minion = minions.iter().any(|enemy| {
        enemy.alive
            && enemy.id != minion.id
            && normalized_team(&enemy.team) != normalized_team(&minion.team)
            && normalized_lane(&enemy.lane) == normalized_lane(&minion.lane)
            && dist(enemy.pos, minion.pos) <= minion_range
    });
    if nearby_enemy_minion {
        return true;
    }

    let champion_range = minion.attack_range.max(MINION_CHAMPION_AGGRO_MIN_RANGE);
    nearest_enemy_champion_for_minion(
        champions,
        &minion.team,
        &minion.lane,
        &minion.kind,
        minion.pos,
        champion_range,
    )
    .is_some()
}

fn move_minions(runtime: &mut RuntimeState, dt: f64) {
    for i in 0..runtime.minions.len() {
        if !runtime.minions[i].alive {
            continue;
        }

        if runtime.minions[i].kind == "summon" {
            if runtime.minions[i].summon_expires_at > 0.0
                && runtime.time_sec >= runtime.minions[i].summon_expires_at
            {
                runtime.minions[i].alive = false;
                continue;
            }
            let lane_push_summon = runtime.minions[i].summon_kind.as_deref() == Some("herald");
            if lane_push_summon {
                // Herald acts as a lane pusher summon, not an owner-orbit pet.
            } else {
                let owner_id = runtime.minions[i].owner_champion_id.clone();
                let owner = owner_id.as_ref().and_then(|id| {
                    runtime
                        .champions
                        .iter()
                        .find(|champion| champion.id == *id && champion.alive)
                });
                if let Some(owner) = owner {
                    let seed = runtime.minions[i]
                        .id
                        .bytes()
                        .fold(0u64, |acc, b| acc.wrapping_mul(131).wrapping_add(b as u64));
                    let phase = (seed % 628) as f64 / 100.0;
                    let angle = runtime.time_sec * 1.9 + phase;
                    let orbit = 0.018 + ((seed % 7) as f64) * 0.001;
                    let follow_target = Vec2 {
                        x: clamp(owner.pos.x + angle.cos() * orbit, 0.01, 0.99),
                        y: clamp(owner.pos.y + angle.sin() * orbit, 0.01, 0.99),
                    };
                    let speed = runtime.minions[i].move_speed.max(owner.move_speed * 0.85);
                    move_entity(&mut runtime.minions[i].pos, follow_target, speed, dt);
                } else {
                    runtime.minions[i].alive = false;
                    continue;
                }
                runtime.minions[i].pos.x = clamp(runtime.minions[i].pos.x, 0.01, 0.99);
                runtime.minions[i].pos.y = clamp(runtime.minions[i].pos.y, 0.01, 0.99);
                continue;
            }
        }

        let snapshot = runtime.minions[i].clone();
        if minion_has_lane_combat_target(
            &snapshot,
            &runtime.minions,
            &runtime.champions,
            &runtime.structures,
        ) {
            continue;
        }

        let minion = &mut runtime.minions[i];

        if minion.path_index >= minion.path.len() {
            minion.path_index = minion.path.len().saturating_sub(1);
        }

        if let Some(target) = minion.path.get(minion.path_index).copied() {
            move_entity(&mut minion.pos, target, minion.move_speed, dt);
            if dist(minion.pos, target) < 0.01
                && minion.path_index < minion.path.len().saturating_sub(1)
            {
                minion.path_index += 1;
            }
        }

        minion.pos.x = clamp(minion.pos.x, 0.01, 0.99);
        minion.pos.y = clamp(minion.pos.y, 0.01, 0.99);
    }
}

fn resolve_minion_combat(runtime: &mut RuntimeState) {
    let now = runtime.time_sec;

    for i in 0..runtime.minions.len() {
        if !runtime.minions[i].alive || now < runtime.minions[i].attack_cd_until {
            continue;
        }

        let attacker_empowered = minion_is_baron_empowered(runtime, &runtime.minions[i]);

        let cadence = minion_stats(&runtime.minions[i].kind).3;
        let enemy_minion = nearest_enemy_minion_index(
            &runtime.minions,
            i,
            runtime.minions[i].attack_range.max(0.05),
        );

        if let Some(enemy_idx) = enemy_minion {
            let attacker_damage = runtime.minions[i].attack_damage
                * if attacker_empowered {
                    BARON_MINION_DAMAGE_MULTIPLIER
                } else {
                    1.0
                };
            let defender_empowered =
                minion_is_baron_empowered(runtime, &runtime.minions[enemy_idx]);
            let damage = attacker_damage
                * MINION_DAMAGE_TO_MINION_MULTIPLIER
                * if defender_empowered {
                    1.0 - BARON_MINION_DAMAGE_REDUCTION
                } else {
                    1.0
                };
            if i < enemy_idx {
                let (left, right) = runtime.minions.split_at_mut(enemy_idx);
                let attacker = &mut left[i];
                let defender = &mut right[0];
                defender.hp -= damage;
                attacker.attack_cd_until = now + cadence;
            } else if enemy_idx < i {
                let (left, right) = runtime.minions.split_at_mut(i);
                let defender = &mut left[enemy_idx];
                let attacker = &mut right[0];
                defender.hp -= damage;
                attacker.attack_cd_until = now + cadence;
            }

            if runtime.minions[enemy_idx].hp <= 0.0 {
                runtime.minions[enemy_idx].alive = false;
            }
            continue;
        }

        let structure_range = runtime.minions[i]
            .attack_range
            .max(MINION_STRUCTURE_AGGRO_RANGE);
        let enemy_structure = nearest_enemy_structure_index(
            &runtime.structures,
            &runtime.minions[i].team,
            &runtime.minions[i].lane,
            runtime.minions[i].pos,
            structure_range,
        );

        if let Some(structure_idx) = enemy_structure {
            if !runtime.structures[structure_idx].alive
                || !is_structure_targetable(
                    &runtime.structures,
                    &runtime.minions[i].team,
                    &runtime.structures[structure_idx],
                )
            {
                continue;
            }

            let attacker_team = runtime.minions[i].team.clone();
            let damage = runtime.minions[i].attack_damage
                * if attacker_empowered {
                    BARON_MINION_DAMAGE_MULTIPLIER
                } else {
                    1.0
                };
            apply_damage_to_structure(runtime, structure_idx, damage, &attacker_team);
            runtime.minions[i].attack_cd_until = now + cadence;
            continue;
        }

        let attacker_team = runtime.minions[i].team.clone();
        let attacker_lane = runtime.minions[i].lane.clone();
        let attacker_pos = runtime.minions[i].pos;
        let attacker_damage = runtime.minions[i].attack_damage
            * if attacker_empowered {
                BARON_MINION_DAMAGE_MULTIPLIER
            } else {
                1.0
            };
        let attacker_range = runtime.minions[i]
            .attack_range
            .max(MINION_CHAMPION_AGGRO_MIN_RANGE);

        let enemy_champion = nearest_enemy_champion_for_minion(
            &runtime.champions,
            &attacker_team,
            &attacker_lane,
            &runtime.minions[i].kind,
            attacker_pos,
            attacker_range,
        );

        if let Some(champion_idx) = enemy_champion {
            let defender_mult =
                team_damage_reduction_multiplier(runtime, &runtime.champions[champion_idx].team);
            runtime.champions[champion_idx].hp -=
                attacker_damage * MINION_DAMAGE_TO_CHAMPION_MULTIPLIER * defender_mult;
            runtime.champions[champion_idx].last_damaged_by_champion_id = None;
            runtime.champions[champion_idx].last_damaged_at = now;
            cancel_recall(
                &mut runtime.champions[champion_idx],
                now,
                &mut runtime.events,
            );
            runtime.minions[i].attack_cd_until = now + cadence;

            if runtime.champions[champion_idx].hp <= 0.0 && runtime.champions[champion_idx].alive {
                runtime.champions[champion_idx].alive = false;
                runtime.champions[champion_idx].deaths += 1;
                let respawn = champion_respawn_seconds(runtime.champions[champion_idx].level, now);
                runtime.champions[champion_idx].respawn_at = now + respawn;
            }
            continue;
        }
    }
}

#[derive(Clone)]
enum CombatTarget {
    Champion(usize),
    Minion(usize),
    Structure(usize),
    Neutral(String),
}

fn laner_farm_search_radius(champion: &ChampionRuntime) -> f64 {
    if champion.role == "JGL" {
        return 0.13;
    }
    match champion.role.as_str() {
        "TOP" => 0.14,
        "MID" => 0.15,
        "ADC" => 0.145,
        _ => 0.12,
    }
}

fn has_local_numbers_advantage(
    champion: &ChampionRuntime,
    pos: Vec2,
    champions: &[ChampionRuntime],
    radius: f64,
) -> bool {
    let ally = champions
        .iter()
        .filter(|u| {
            u.alive
                && normalized_team(&u.team) == normalized_team(&champion.team)
                && dist(u.pos, pos) <= radius
        })
        .count();
    let enemy = champions
        .iter()
        .filter(|u| {
            u.alive
                && normalized_team(&u.team) != normalized_team(&champion.team)
                && dist(u.pos, pos) <= radius
        })
        .count();
    ally > enemy
}

fn enemy_pressuring_allied_tower_idx(
    champion: &ChampionRuntime,
    champions: &[ChampionRuntime],
    structures: &[StructureRuntime],
) -> Option<usize> {
    let allied_towers: Vec<&StructureRuntime> = structures
        .iter()
        .filter(|s| {
            s.alive
                && s.kind == "tower"
                && normalized_team(&s.team) == normalized_team(&champion.team)
        })
        .collect();
    if allied_towers.is_empty() {
        return None;
    }

    champions
        .iter()
        .enumerate()
        .filter(|(_, enemy)| {
            enemy.alive
                && normalized_team(&enemy.team) != normalized_team(&champion.team)
                && dist(champion.pos, enemy.pos) <= LANE_CHAMPION_TRADE_RADIUS
                && allied_towers.iter().any(|tower| {
                    normalized_lane(&tower.lane) == normalized_lane(&enemy.lane)
                        && dist(enemy.pos, tower.pos) <= 0.095
                })
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            a.hp.partial_cmp(&b.hp)
                .unwrap_or(Ordering::Equal)
                .then_with(|| {
                    dist(champion.pos, a.pos)
                        .partial_cmp(&dist(champion.pos, b.pos))
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx)
}

fn has_credible_kill_chance(
    runtime: &RuntimeState,
    champion_idx: usize,
    enemy_idx: usize,
    now: f64,
) -> bool {
    if champion_idx >= runtime.champions.len() || enemy_idx >= runtime.champions.len() {
        return false;
    }
    let champion = &runtime.champions[champion_idx];
    let enemy = &runtime.champions[enemy_idx];
    if !enemy.alive || normalized_team(&enemy.team) == normalized_team(&champion.team) {
        return false;
    }

    let range_gate = if champion.role == "JGL" {
        0.11
    } else {
        LANE_CHAMPION_TRADE_RADIUS
    };
    if dist(champion.pos, enemy.pos) > range_gate {
        return false;
    }

    let hp_ratio = if champion.max_hp <= 0.0 {
        1.0
    } else {
        champion.hp / champion.max_hp
    };
    if hp_ratio <= 0.24 {
        return false;
    }

    if champion.role != "JGL"
        && !can_open_trade_window(
            champion,
            enemy,
            now,
            &runtime.champions,
            &runtime.minions,
            &runtime.structures,
            &runtime.lane_combat_state_by_champion,
            runtime.ai_mode,
            &runtime.policy,
        )
    {
        return false;
    }

    let ally_pressure = runtime
        .champions
        .iter()
        .filter(|u| {
            u.alive
                && normalized_team(&u.team) == normalized_team(&champion.team)
                && dist(u.pos, enemy.pos) <= 0.12
        })
        .count() as f64;
    let enemy_pressure = runtime
        .champions
        .iter()
        .filter(|u| {
            u.alive
                && normalized_team(&u.team) == normalized_team(&enemy.team)
                && dist(u.pos, enemy.pos) <= 0.12
        })
        .count() as f64;

    let ttk_enemy = enemy.hp / champion.attack_damage.max(1.0);
    let ttk_self = champion.hp / enemy.attack_damage.max(1.0);
    let enemy_hp_ratio = if enemy.max_hp <= 0.0 {
        1.0
    } else {
        enemy.hp / enemy.max_hp
    };
    let low_enemy = enemy_hp_ratio <= 0.48;

    (ttk_enemy <= ttk_self * 0.95 || low_enemy) && ally_pressure + 0.5 >= enemy_pressure
}

fn is_backline_champion(champion: &ChampionRuntime) -> bool {
    champion.attack_range >= 0.05
}

fn target_priority_rank_for_fight_plan(fight_plan: &str, enemy: &ChampionRuntime) -> u8 {
    let enemy_is_backline = is_backline_champion(enemy);
    match fight_plan {
        "FrontToBack" => {
            if enemy_is_backline {
                1
            } else {
                0
            }
        }
        "Dive" | "Pick" => {
            if enemy_is_backline {
                0
            } else {
                1
            }
        }
        _ => 0,
    }
}

fn pick_combat_target(
    runtime: &RuntimeState,
    champion_idx: usize,
    now: f64,
    neutral_timers: &NeutralTimersRuntime,
) -> Option<CombatTarget> {
    if champion_idx >= runtime.champions.len() {
        return None;
    }
    let champion = &runtime.champions[champion_idx];
    let team_tactics = team_tactics_for_runtime(runtime.extra.get("teamTactics"), &champion.team);
    let fight_plan = team_tactics.fight_plan.as_str();
    let enemy_team = if normalized_team(&champion.team) == "blue" {
        "red"
    } else {
        "blue"
    };

    let kill_window_enemy = runtime
        .champions
        .iter()
        .enumerate()
        .filter(|(idx, enemy)| {
            *idx != champion_idx
                && enemy.alive
                && normalized_team(&enemy.team) == enemy_team
                && team_has_vision_at(runtime, &champion.team, enemy.pos)
                && has_credible_kill_chance(runtime, champion_idx, *idx, now)
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            target_priority_rank_for_fight_plan(fight_plan, a)
                .cmp(&target_priority_rank_for_fight_plan(fight_plan, b))
                .then_with(|| a.hp.partial_cmp(&b.hp).unwrap_or(Ordering::Equal))
                .then_with(|| {
                    dist(champion.pos, a.pos)
                        .partial_cmp(&dist(champion.pos, b.pos))
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx);
    if let Some(enemy_idx) = kill_window_enemy {
        return Some(CombatTarget::Champion(enemy_idx));
    }

    if champion.role == "JGL" {
        let nearby_enemy = runtime
            .champions
            .iter()
            .enumerate()
            .filter(|(_, enemy)| {
                enemy.alive
                    && normalized_team(&enemy.team) == enemy_team
                    && team_has_vision_at(runtime, &champion.team, enemy.pos)
                    && dist(champion.pos, enemy.pos) <= 0.13
            })
            .min_by(|(idx_a, a), (idx_b, b)| {
                target_priority_rank_for_fight_plan(fight_plan, a)
                    .cmp(&target_priority_rank_for_fight_plan(fight_plan, b))
                    .then_with(|| {
                        dist(champion.pos, a.pos)
                            .partial_cmp(&dist(champion.pos, b.pos))
                            .unwrap_or(Ordering::Equal)
                    })
                    .then_with(|| idx_a.cmp(idx_b))
            })
            .map(|(idx, _)| idx);
        if let Some(enemy_idx) = nearby_enemy {
            return Some(CombatTarget::Champion(enemy_idx));
        }

        if let Some(neutral_key) = nearest_attackable_neutral_key(
            champion,
            neutral_timers,
            JUNGLE_CAMP_ENGAGE_RADIUS,
            OBJECTIVE_ATTEMPT_RADIUS,
        ) {
            return Some(CombatTarget::Neutral(neutral_key));
        }
        return None;
    }

    if now < LANE_COMBAT_UNLOCK_AT {
        let early_lane_minion = runtime
            .minions
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                m.alive
                    && normalized_team(&m.team) == enemy_team
                    && normalized_lane(&m.lane) == normalized_lane(&champion.lane)
                    && team_has_vision_at(runtime, &champion.team, m.pos)
                    && dist(champion.pos, m.pos) <= 0.12
            })
            .min_by(|(idx_a, a), (idx_b, b)| {
                a.hp.partial_cmp(&b.hp)
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| {
                        dist(champion.pos, a.pos)
                            .partial_cmp(&dist(champion.pos, b.pos))
                            .unwrap_or(Ordering::Equal)
                    })
                    .then_with(|| idx_a.cmp(idx_b))
            })
            .map(|(idx, _)| idx);
        return early_lane_minion.map(CombatTarget::Minion);
    }

    let recalling_enemy = runtime
        .champions
        .iter()
        .enumerate()
        .filter(|(_, enemy)| {
            enemy.alive
                && normalized_team(&enemy.team) == enemy_team
                && team_has_vision_at(runtime, &champion.team, enemy.pos)
                && enemy.state == "recall"
                && dist(champion.pos, enemy.pos) <= LOCAL_COMBAT_ENGAGE_RADIUS
                && in_lane_trade_context(
                    champion,
                    enemy.pos,
                    true,
                    &runtime.champions,
                    &runtime.minions,
                    &runtime.structures,
                )
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            target_priority_rank_for_fight_plan(fight_plan, a)
                .cmp(&target_priority_rank_for_fight_plan(fight_plan, b))
                .then_with(|| {
                    dist(champion.pos, a.pos)
                        .partial_cmp(&dist(champion.pos, b.pos))
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx);
    if let Some(enemy_idx) = recalling_enemy {
        return Some(CombatTarget::Champion(enemy_idx));
    }

    let threatening_enemy = runtime
        .champions
        .iter()
        .enumerate()
        .filter(|(_, enemy)| {
            enemy.alive
                && normalized_team(&enemy.team) == enemy_team
                && team_has_vision_at(runtime, &champion.team, enemy.pos)
                && dist(champion.pos, enemy.pos) <= 0.12
                && runtime.champions.iter().any(|ally| {
                    ally.alive
                        && normalized_team(&ally.team) == normalized_team(&champion.team)
                        && (dist(ally.pos, champion.pos) <= ALLY_HELP_RADIUS
                            || dist(ally.pos, enemy.pos) <= ALLY_HELP_RADIUS)
                        && ally
                            .last_damaged_by_champion_id
                            .as_ref()
                            .map(|id| id == &enemy.id)
                            .unwrap_or(false)
                        && now - ally.last_damaged_at <= ALLY_HELP_DAMAGE_RECENT_SEC
                })
                && (can_open_trade_window(
                    champion,
                    enemy,
                    now,
                    &runtime.champions,
                    &runtime.minions,
                    &runtime.structures,
                    &runtime.lane_combat_state_by_champion,
                    runtime.ai_mode,
                    &runtime.policy,
                ) || has_local_numbers_advantage(champion, enemy.pos, &runtime.champions, 0.12))
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            target_priority_rank_for_fight_plan(fight_plan, a)
                .cmp(&target_priority_rank_for_fight_plan(fight_plan, b))
                .then_with(|| {
                    dist(champion.pos, a.pos)
                        .partial_cmp(&dist(champion.pos, b.pos))
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx);
    if let Some(enemy_idx) = threatening_enemy {
        return Some(CombatTarget::Champion(enemy_idx));
    }

    if let Some(enemy_idx) =
        enemy_pressuring_allied_tower_idx(champion, &runtime.champions, &runtime.structures)
    {
        return Some(CombatTarget::Champion(enemy_idx));
    }

    let numbers_advantage_enemy = runtime
        .champions
        .iter()
        .enumerate()
        .filter(|(_, enemy)| {
            enemy.alive
                && normalized_team(&enemy.team) == enemy_team
                && normalized_lane(&enemy.lane) == normalized_lane(&champion.lane)
                && team_has_vision_at(runtime, &champion.team, enemy.pos)
                && dist(champion.pos, enemy.pos) <= LANE_CHAMPION_TRADE_RADIUS
                && has_local_numbers_advantage(champion, enemy.pos, &runtime.champions, 0.11)
                && can_open_trade_window(
                    champion,
                    enemy,
                    now,
                    &runtime.champions,
                    &runtime.minions,
                    &runtime.structures,
                    &runtime.lane_combat_state_by_champion,
                    runtime.ai_mode,
                    &runtime.policy,
                )
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            target_priority_rank_for_fight_plan(fight_plan, a)
                .cmp(&target_priority_rank_for_fight_plan(fight_plan, b))
                .then_with(|| a.hp.partial_cmp(&b.hp).unwrap_or(Ordering::Equal))
                .then_with(|| {
                    dist(champion.pos, a.pos)
                        .partial_cmp(&dist(champion.pos, b.pos))
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx);
    if let Some(enemy_idx) = numbers_advantage_enemy {
        return Some(CombatTarget::Champion(enemy_idx));
    }

    let objective_assist_active =
        should_assist_objective_attempt(champion, &runtime.champions, neutral_timers);
    if objective_assist_active {
        if let Some(neutral_key) = nearby_neutral_objective_key(champion, neutral_timers) {
            return Some(CombatTarget::Neutral(neutral_key));
        }
    }

    let last_hit_minion = runtime
        .minions
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            if objective_assist_active {
                return false;
            }
            m.alive
                && normalized_team(&m.team) == enemy_team
                && normalized_lane(&m.lane) == normalized_lane(&champion.lane)
                && team_has_vision_at(runtime, &champion.team, m.pos)
                && dist(champion.pos, m.pos) <= laner_farm_search_radius(champion)
                && m.hp <= champion.attack_damage * CHAMPION_DAMAGE_TO_MINION_MULTIPLIER * 1.4
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            a.hp.partial_cmp(&b.hp)
                .unwrap_or(Ordering::Equal)
                .then_with(|| {
                    dist(champion.pos, a.pos)
                        .partial_cmp(&dist(champion.pos, b.pos))
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx);
    if let Some(minion_idx) = last_hit_minion {
        return Some(CombatTarget::Minion(minion_idx));
    }

    let lane_skirmish_enemy = runtime
        .champions
        .iter()
        .enumerate()
        .filter(|(_, enemy)| {
            enemy.alive
                && normalized_team(&enemy.team) == enemy_team
                && normalized_lane(&enemy.lane) == normalized_lane(&champion.lane)
                && team_has_vision_at(runtime, &champion.team, enemy.pos)
                && dist(champion.pos, enemy.pos) <= LANE_CHAMPION_TRADE_RADIUS
                && can_open_trade_window(
                    champion,
                    enemy,
                    now,
                    &runtime.champions,
                    &runtime.minions,
                    &runtime.structures,
                    &runtime.lane_combat_state_by_champion,
                    runtime.ai_mode,
                    &runtime.policy,
                )
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            target_priority_rank_for_fight_plan(fight_plan, a)
                .cmp(&target_priority_rank_for_fight_plan(fight_plan, b))
                .then_with(|| {
                    dist(champion.pos, a.pos)
                        .partial_cmp(&dist(champion.pos, b.pos))
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx);
    if let Some(enemy_idx) = lane_skirmish_enemy {
        return Some(CombatTarget::Champion(enemy_idx));
    }

    let wave_front = lane_wave_front_pos(champion, &runtime.minions, &runtime.structures);
    let farming_minion = runtime
        .minions
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            if objective_assist_active {
                return false;
            }
            m.alive
                && normalized_team(&m.team) == enemy_team
                && normalized_lane(&m.lane) == normalized_lane(&champion.lane)
                && team_has_vision_at(runtime, &champion.team, m.pos)
                && dist(champion.pos, m.pos) <= laner_farm_search_radius(champion)
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            dist(wave_front, a.pos)
                .partial_cmp(&dist(wave_front, b.pos))
                .unwrap_or(Ordering::Equal)
                .then_with(|| {
                    dist(champion.pos, a.pos)
                        .partial_cmp(&dist(champion.pos, b.pos))
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx);
    if let Some(minion_idx) = farming_minion {
        return Some(CombatTarget::Minion(minion_idx));
    }

    let pressure_structure = runtime
        .structures
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            if !(s.alive
                && normalized_team(&s.team) == enemy_team
                && (normalized_lane(&s.lane) == normalized_lane(&champion.lane)
                    || s.kind == "nexus")
                && dist(champion.pos, s.pos) <= LANE_STRUCTURE_PRESSURE_RADIUS
                && is_structure_targetable(&runtime.structures, &champion.team, s))
            {
                return false;
            }

            if champion.role == "JGL" {
                return true;
            }

            let has_allied_wave_at_structure = runtime.minions.iter().any(|m| {
                m.alive
                    && normalized_team(&m.team) == normalized_team(&champion.team)
                    && normalized_lane(&m.lane) == normalized_lane(&champion.lane)
                    && dist(m.pos, s.pos) <= 0.1
            });
            if !has_allied_wave_at_structure {
                return false;
            }

            let enemy_wave_at_structure = runtime
                .minions
                .iter()
                .filter(|m| {
                    m.alive
                        && normalized_team(&m.team) != normalized_team(&champion.team)
                        && normalized_lane(&m.lane) == normalized_lane(&champion.lane)
                        && dist(m.pos, s.pos) <= 0.08
                })
                .count();
            enemy_wave_at_structure < 2
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            dist(champion.pos, a.pos)
                .partial_cmp(&dist(champion.pos, b.pos))
                .unwrap_or(Ordering::Equal)
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx);

    if let Some(structure_idx) = pressure_structure {
        return Some(CombatTarget::Structure(structure_idx));
    }

    let nearest_structure = runtime
        .structures
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            if !s.alive
                || normalized_team(&s.team) != enemy_team
                || !(normalized_lane(&s.lane) == normalized_lane(&champion.lane)
                    || s.kind == "nexus")
                || !is_structure_targetable(&runtime.structures, &champion.team, s)
            {
                return false;
            }
            if champion.role != "JGL" {
                if dist(champion.pos, s.pos) > LANE_STRUCTURE_PRESSURE_RADIUS {
                    return false;
                }
                let has_allied_wave = runtime.minions.iter().any(|m| {
                    m.alive
                        && normalized_team(&m.team) == normalized_team(&champion.team)
                        && normalized_lane(&m.lane) == normalized_lane(&champion.lane)
                        && dist(m.pos, s.pos) <= 0.09
                });
                if !has_allied_wave {
                    return false;
                }
                let enemy_wave_at_structure = runtime
                    .minions
                    .iter()
                    .filter(|m| {
                        m.alive
                            && normalized_team(&m.team) != normalized_team(&champion.team)
                            && normalized_lane(&m.lane) == normalized_lane(&champion.lane)
                            && dist(m.pos, s.pos) <= 0.08
                    })
                    .count();
                if enemy_wave_at_structure >= 2 {
                    return false;
                }
            }
            true
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            dist(champion.pos, a.pos)
                .partial_cmp(&dist(champion.pos, b.pos))
                .unwrap_or(Ordering::Equal)
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx);

    let nearest_minion = runtime
        .minions
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            m.alive
                && normalized_team(&m.team) == enemy_team
                && normalized_lane(&m.lane) == normalized_lane(&champion.lane)
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            dist(champion.pos, a.pos)
                .partial_cmp(&dist(champion.pos, b.pos))
                .unwrap_or(Ordering::Equal)
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx);

    let nearest_enemy_champion = runtime
        .champions
        .iter()
        .enumerate()
        .filter(|(idx, enemy)| {
            *idx != champion_idx
                && enemy.alive
                && normalized_team(&enemy.team) == enemy_team
                && normalized_lane(&enemy.lane) == normalized_lane(&champion.lane)
                && can_open_trade_window(
                    champion,
                    enemy,
                    now,
                    &runtime.champions,
                    &runtime.minions,
                    &runtime.structures,
                    &runtime.lane_combat_state_by_champion,
                    runtime.ai_mode,
                    &runtime.policy,
                )
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            dist(champion.pos, a.pos)
                .partial_cmp(&dist(champion.pos, b.pos))
                .unwrap_or(Ordering::Equal)
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx);

    let nearby_neutral =
        nearest_attackable_neutral_key(champion, neutral_timers, JUNGLE_CAMP_ENGAGE_RADIUS, 0.0)
            .filter(|key| is_jungle_camp_key(key));

    #[derive(Clone)]
    struct FallbackCandidate {
        target: CombatTarget,
        score: f64,
        distance: f64,
        kind_rank: u8,
        stable_key: String,
    }

    let mut fallback_candidates: Vec<FallbackCandidate> = Vec::new();

    if let Some(key) = nearby_neutral {
        if let Some(timer) = neutral_timers.entities.get(&key) {
            let d = dist(champion.pos, timer.pos);
            fallback_candidates.push(FallbackCandidate {
                target: CombatTarget::Neutral(key.clone()),
                score: d + 0.03,
                distance: d,
                kind_rank: 4,
                stable_key: key,
            });
        }
    }

    if let Some(enemy_idx) = nearest_enemy_champion {
        let enemy = &runtime.champions[enemy_idx];
        let d = dist(champion.pos, enemy.pos);
        fallback_candidates.push(FallbackCandidate {
            target: CombatTarget::Champion(enemy_idx),
            score: d,
            distance: d,
            kind_rank: 1,
            stable_key: enemy.id.clone(),
        });
    }

    if let Some(minion_idx) = nearest_minion {
        let minion = &runtime.minions[minion_idx];
        let distance_to_champion = dist(champion.pos, minion.pos);
        let distance_to_wave = dist(wave_front, minion.pos);
        let mut score = distance_to_champion * 0.88 + distance_to_wave * 0.12;
        if distance_to_champion <= champion.attack_range.max(0.04) + 0.008 {
            score -= 0.004;
        }
        fallback_candidates.push(FallbackCandidate {
            target: CombatTarget::Minion(minion_idx),
            score,
            distance: distance_to_champion,
            kind_rank: 2,
            stable_key: minion.id.clone(),
        });
    }

    if let Some(structure_idx) = nearest_structure {
        let structure = &runtime.structures[structure_idx];
        let d = dist(champion.pos, structure.pos);
        let mut score = d;
        if d <= champion.attack_range.max(0.04) + 0.008 {
            score -= 0.004;
        }
        fallback_candidates.push(FallbackCandidate {
            target: CombatTarget::Structure(structure_idx),
            score,
            distance: d,
            kind_rank: 3,
            stable_key: structure.id.clone(),
        });
    }

    fallback_candidates.sort_by(|a, b| {
        a.score
            .partial_cmp(&b.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                a.distance
                    .partial_cmp(&b.distance)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| a.kind_rank.cmp(&b.kind_rank))
            .then_with(|| a.stable_key.cmp(&b.stable_key))
    });

    fallback_candidates
        .first()
        .map(|candidate| candidate.target.clone())
}

fn combat_target_pos(runtime: &RuntimeState, target: &CombatTarget) -> Option<Vec2> {
    match target {
        CombatTarget::Champion(idx) => runtime.champions.get(*idx).map(|c| c.pos),
        CombatTarget::Minion(idx) => runtime.minions.get(*idx).map(|m| m.pos),
        CombatTarget::Structure(idx) => runtime.structures.get(*idx).map(|s| s.pos),
        CombatTarget::Neutral(key) => decode_neutral_timers_state(&runtime.neutral_timers)
            .and_then(|timers| timers.entities.get(key).cloned())
            .map(|timer| timer.pos),
    }
}

fn is_local_combat_target(
    runtime: &RuntimeState,
    champion_idx: usize,
    target: &CombatTarget,
) -> bool {
    if champion_idx >= runtime.champions.len() {
        return false;
    }
    let champion = &runtime.champions[champion_idx];
    let Some(target_pos) = combat_target_pos(runtime, target) else {
        return false;
    };

    let target_distance = dist(champion.pos, target_pos);
    if target_distance > LOCAL_COMBAT_ENGAGE_RADIUS {
        return false;
    }
    if matches!(target, CombatTarget::Structure(_))
        && target_distance > LOCAL_STRUCTURE_ENGAGE_RADIUS
    {
        return false;
    }
    if let CombatTarget::Neutral(key) = target {
        let max_range = if is_objective_neutral_key(key) {
            OBJECTIVE_ATTEMPT_RADIUS
        } else {
            JUNGLE_CAMP_ENGAGE_RADIUS
        };
        if target_distance > max_range {
            return false;
        }
    }

    true
}

fn resolve_champion_combat(runtime: &mut RuntimeState) {
    let now = runtime.time_sec;
    let mut neutral_timers = decode_neutral_timers_state(&runtime.neutral_timers)
        .unwrap_or_else(|| neutral_timers_default_runtime_state());

    tick_ignite_dot_effects(runtime, now);

    for idx in 0..runtime.champions.len() {
        if !runtime.champions[idx].alive
            || champion_is_banished(&runtime.champions[idx])
            || runtime.champions[idx].state == "recall"
            || now < runtime.champions[idx].attack_cd_until
        {
            continue;
        }

        let team = normalized_team(&runtime.champions[idx].team).to_string();
        let attack_range = runtime.champions[idx].attack_range.max(0.04);

        if try_cast_ultimate(runtime, idx, now) {
            continue;
        }

        if try_cast_summoner_spells(runtime, &mut neutral_timers, idx, now) {
            continue;
        }

        let is_hard_assist = {
            let contested =
                contested_dragon_attempt_for_team(&team, &runtime.champions, &neutral_timers);
            should_hard_assist_contested_dragon(&runtime.champions[idx], contested)
        };

        if is_hard_assist {
            if let Some(dragon) =
                contested_dragon_attempt_for_team(&team, &runtime.champions, &neutral_timers)
            {
                let dragon_key = dragon.key.clone();
                let dragon_pos = dragon.pos;
                if let Some(champion_idx) = nearest_enemy_champion_contesting_objective(
                    &runtime.champions,
                    &runtime.champions[idx],
                    dragon_pos,
                    attack_range,
                ) {
                    if should_engage_enemy_champion(runtime, idx, champion_idx) {
                        attack_enemy_champion(runtime, idx, champion_idx);
                        continue;
                    }
                }

                if attack_neutral_if_in_range(runtime, &mut neutral_timers, idx, &dragon_key) {
                    continue;
                }
            }

            // Hard assist parity: skip regular wave-farm lock while dragon is contested.
            continue;
        }

        let is_objective_assist = should_assist_objective_attempt(
            &runtime.champions[idx],
            &runtime.champions,
            &neutral_timers,
        );
        if is_objective_assist && runtime.champions[idx].state == "objective" {
            if let Some(attempt) =
                active_objective_attempt_for_team(&team, &runtime.champions, &neutral_timers)
            {
                let objective_key = attempt.key.clone();
                let objective_pos = attempt.pos;

                if let Some(champion_idx) = nearest_enemy_champion_contesting_objective(
                    &runtime.champions,
                    &runtime.champions[idx],
                    objective_pos,
                    attack_range,
                ) {
                    if should_engage_enemy_champion(runtime, idx, champion_idx) {
                        attack_enemy_champion(runtime, idx, champion_idx);
                        continue;
                    }
                }

                if attack_neutral_if_in_range(runtime, &mut neutral_timers, idx, &objective_key) {
                    continue;
                }

                // Objective assist parity: skip regular farm lock while rotating to attempt.
                continue;
            }
        }

        let Some(target) = pick_combat_target(runtime, idx, now, &neutral_timers) else {
            continue;
        };
        if !is_local_combat_target(runtime, idx, &target) {
            continue;
        }

        let attacker_snapshot = runtime.champions[idx].clone();
        let Some(target_pos) = combat_target_pos(runtime, &target) else {
            continue;
        };

        if dist(attacker_snapshot.pos, target_pos) > attack_range {
            if let CombatTarget::Champion(enemy_idx) = &target {
                let target_snapshot = runtime.champions[*enemy_idx].clone();
                if attacker_snapshot.role != "JGL" {
                    if should_force_laner_disengage(
                        &attacker_snapshot,
                        target_snapshot.pos,
                        Some(&target_snapshot),
                        &runtime.champions,
                        &runtime.minions,
                        &runtime.structures,
                    ) || !in_lane_trade_context(
                        &attacker_snapshot,
                        target_snapshot.pos,
                        true,
                        &runtime.champions,
                        &runtime.minions,
                        &runtime.structures,
                    ) {
                        issue_lane_disengage(runtime, idx, target_snapshot.pos);
                        continue;
                    }

                    let approach = lane_trade_approach_pos(
                        &attacker_snapshot,
                        &target_snapshot,
                        now,
                        &runtime.champions,
                        &runtime.minions,
                        &runtime.structures,
                    );
                    set_champion_direct_path(&mut runtime.champions[idx], approach);
                    continue;
                }
            }

            if runtime.champions[idx].state == "objective" {
                set_champion_direct_path_hysteresis(
                    &mut runtime.champions[idx],
                    target_pos,
                    OBJECTIVE_PATH_MIN_TARGET_DELTA,
                );
            } else {
                set_champion_direct_path(&mut runtime.champions[idx], target_pos);
            }
            continue;
        }

        match target {
            CombatTarget::Champion(champion_idx) => {
                let target_snapshot = runtime.champions[champion_idx].clone();

                if attacker_snapshot.role != "JGL" {
                    let open_eval = evaluate_open_trade_window(
                        &attacker_snapshot,
                        &target_snapshot,
                        now,
                        &runtime.champions,
                        &runtime.minions,
                        &runtime.structures,
                        &runtime.lane_combat_state_by_champion,
                        runtime.ai_mode,
                        &runtime.policy,
                    );
                    capture_trade_decision_candidate(
                        runtime,
                        &attacker_snapshot,
                        &target_snapshot,
                        open_eval,
                        "open-trade",
                    );
                    if open_eval.flipped_by_hybrid {
                        maybe_log_hybrid_trade_flip(
                            runtime,
                            &attacker_snapshot,
                            "open-trade",
                            open_eval.confidence,
                            open_eval.rule_decision,
                            open_eval.decision,
                        );
                    }
                    if !open_eval.decision {
                        issue_lane_disengage(runtime, idx, target_snapshot.pos);
                        continue;
                    }
                }

                let disengage_eval = evaluate_disengage_champion_trade(
                    &attacker_snapshot,
                    &target_snapshot,
                    now,
                    &runtime.champions,
                    &runtime.minions,
                    &runtime.structures,
                    runtime.ai_mode,
                    &runtime.policy,
                );
                capture_trade_decision_candidate(
                    runtime,
                    &attacker_snapshot,
                    &target_snapshot,
                    disengage_eval,
                    "disengage",
                );
                if disengage_eval.flipped_by_hybrid {
                    maybe_log_hybrid_trade_flip(
                        runtime,
                        &attacker_snapshot,
                        "disengage",
                        disengage_eval.confidence,
                        disengage_eval.rule_decision,
                        disengage_eval.decision,
                    );
                }
                if disengage_eval.decision {
                    issue_lane_disengage(runtime, idx, target_snapshot.pos);
                    continue;
                }

                if !should_engage_enemy_champion(runtime, idx, champion_idx) {
                    if attacker_snapshot.role != "JGL" {
                        issue_lane_disengage(runtime, idx, target_snapshot.pos);
                    }
                    continue;
                }

                attack_enemy_champion(runtime, idx, champion_idx);

                let attacker_after = runtime.champions[idx].clone();
                if attacker_after.role != "JGL"
                    && champion_idx < runtime.champions.len()
                    && runtime.champions[champion_idx].alive
                    && !should_commit_all_in_trade(
                        &attacker_after,
                        &runtime.champions[champion_idx],
                        &runtime.champions,
                        &runtime.minions,
                    )
                {
                    let enemy_pos = runtime.champions[champion_idx].pos;
                    issue_lane_disengage(runtime, idx, enemy_pos);
                }
                continue;
            }
            CombatTarget::Minion(minion_idx) => {
                if minion_idx >= runtime.minions.len() || !runtime.minions[minion_idx].alive {
                    continue;
                }
                let lane_mult = champion_lane_damage_multiplier(&runtime.champions[idx]);
                let damage = runtime.champions[idx].attack_damage
                    * CHAMPION_DAMAGE_TO_MINION_MULTIPLIER
                    * lane_mult;
                runtime.minions[minion_idx].hp -= damage;
                runtime.minions[minion_idx].last_hit_by_champion_id =
                    Some(runtime.champions[idx].id.clone());
                runtime.champions[idx].attack_cd_until = now + 0.75;
                if runtime.minions[minion_idx].hp <= 0.0 {
                    register_minion_death(runtime, minion_idx);
                }
                continue;
            }
            CombatTarget::Structure(structure_idx) => {
                if structure_idx >= runtime.structures.len()
                    || !runtime.structures[structure_idx].alive
                    || !is_structure_targetable(
                        &runtime.structures,
                        &team,
                        &runtime.structures[structure_idx],
                    )
                {
                    continue;
                }
                let structure_mult = champion_structure_focus_multiplier(&runtime.champions[idx]);
                apply_damage_to_structure(
                    runtime,
                    structure_idx,
                    runtime.champions[idx].attack_damage * structure_mult,
                    &team,
                );
                runtime.champions[idx].attack_cd_until = now + 0.9;
            }
            CombatTarget::Neutral(neutral_key) => {
                if attack_neutral_if_in_range(runtime, &mut neutral_timers, idx, &neutral_key) {
                    continue;
                }
            }
        }
    }

    if let Ok(value) = serde_json::to_value(&neutral_timers) {
        runtime.neutral_timers = value;
    }
    sync_objectives_from_neutral_timers(runtime, &neutral_timers);
}

fn champion_has_spell(champion: &ChampionRuntime, key: &str) -> bool {
    champion
        .summoner_spells
        .iter()
        .any(|spell| spell.key.eq_ignore_ascii_case(key))
}

fn spell_ready(champion: &ChampionRuntime, key: &str, now: f64) -> bool {
    champion
        .summoner_spells
        .iter()
        .find(|spell| spell.key.eq_ignore_ascii_case(key))
        .map(|spell| now >= spell.cd_until)
        .unwrap_or(false)
}

fn set_spell_cd(champion: &mut ChampionRuntime, key: &str, now: f64, cooldown_sec: f64) -> bool {
    let Some(spell) = champion
        .summoner_spells
        .iter_mut()
        .find(|spell| spell.key.eq_ignore_ascii_case(key))
    else {
        return false;
    };
    spell.cd_until = now + cooldown_sec;
    true
}

fn champion_is_banished(champion: &ChampionRuntime) -> bool {
    champion.realm_banished_until > 0.0
}

fn team_has_vision_at(runtime: &RuntimeState, team: &str, pos: Vec2) -> bool {
    if runtime.champions.iter().any(|champion| {
        champion.alive
            && !champion_is_banished(champion)
            && normalized_team(&champion.team) == normalized_team(team)
            && dist(champion.pos, pos) <= CHAMPION_VISION_RADIUS
    }) {
        return true;
    }

    if runtime.minions.iter().any(|minion| {
        minion.alive
            && normalized_team(&minion.team) == normalized_team(team)
            && dist(minion.pos, pos) <= MINION_VISION_RADIUS
    }) {
        return true;
    }

    if runtime.structures.iter().any(|structure| {
        structure.alive
            && normalized_team(&structure.team) == normalized_team(team)
            && dist(structure.pos, pos) <= STRUCTURE_VISION_RADIUS
    }) {
        return true;
    }

    runtime.wards.iter().any(|ward| {
        normalized_team(&ward.team) == normalized_team(team)
            && ward.expires_at > runtime.time_sec
            && dist(ward.pos, pos) <= WARD_VISION_RADIUS
    })
}

fn strategic_ward_points_for_team(team: &str) -> &'static [Vec2] {
    if normalized_team(team) == "blue" {
        &[
            Vec2 { x: 0.615, y: 0.61 },  // river bot bush
            Vec2 { x: 0.565, y: 0.455 }, // river mid bot side
            Vec2 { x: 0.49, y: 0.525 },  // mid river center
            Vec2 { x: 0.412, y: 0.39 },  // river top side
            Vec2 { x: 0.675, y: 0.705 }, // dragon pit edge
            Vec2 { x: 0.328, y: 0.302 }, // baron pit edge
            Vec2 { x: 0.725, y: 0.548 }, // enemy raptor entrance
            Vec2 { x: 0.73, y: 0.37 },   // enemy blue-side entrance
        ]
    } else {
        &[
            Vec2 { x: 0.385, y: 0.39 },  // river bot bush (red perspective)
            Vec2 { x: 0.435, y: 0.545 }, // river mid bot side
            Vec2 { x: 0.51, y: 0.475 },  // mid river center
            Vec2 { x: 0.588, y: 0.61 },  // river top side
            Vec2 { x: 0.675, y: 0.705 }, // dragon pit edge
            Vec2 { x: 0.328, y: 0.302 }, // baron pit edge
            Vec2 { x: 0.272, y: 0.46 },  // enemy raptor entrance
            Vec2 { x: 0.272, y: 0.63 },  // enemy blue-side entrance
        ]
    }
}

fn pick_ward_placement_pos(
    runtime: &RuntimeState,
    champion: &ChampionRuntime,
    now: f64,
) -> Option<Vec2> {
    let points = strategic_ward_points_for_team(&champion.team);
    let max_place_dist = if champion.role == "JGL" || champion.role == "SUP" {
        0.24
    } else {
        0.18
    };

    points
        .iter()
        .copied()
        .filter(|point| dist(champion.pos, *point) <= max_place_dist)
        .filter(|point| {
            !runtime.wards.iter().any(|ward| {
                normalized_team(&ward.team) == normalized_team(&champion.team)
                    && ward.expires_at > now
                    && dist(ward.pos, *point) <= 0.095
            })
        })
        .min_by(|a, b| {
            let da = dist(champion.pos, *a);
            let db = dist(champion.pos, *b);
            da.partial_cmp(&db).unwrap_or(Ordering::Equal)
        })
}

fn place_wards(runtime: &mut RuntimeState) {
    let now = runtime.time_sec;
    if now < WARD_UNLOCK_AT_SEC {
        return;
    }

    let mut placements: Vec<WardRuntime> = Vec::new();

    for idx in 0..runtime.champions.len() {
        let champion = runtime.champions[idx].clone();
        if !champion.alive
            || champion_is_banished(&champion)
            || champion.state == "recall"
            || now < champion.ward_cd_until
            || !champion
                .trinket_key
                .eq_ignore_ascii_case(TRINKET_WARDING_TOTEM)
        {
            continue;
        }

        let Some(place_pos) = pick_ward_placement_pos(runtime, &champion, now) else {
            continue;
        };

        runtime.champions[idx].ward_cd_until = now + WARD_COOLDOWN_SEC;
        placements.push(WardRuntime {
            id: format!("ward-{}-{:.0}", champion.id, now * 10.0),
            team: champion.team.clone(),
            owner_champion_id: champion.id.clone(),
            pos: place_pos,
            expires_at: now + WARD_DURATION_SEC,
        });
    }

    if placements.is_empty() {
        return;
    }

    for ward in placements {
        let owner_id = ward.owner_champion_id.clone();
        let mut owner_wards: Vec<usize> = runtime
            .wards
            .iter()
            .enumerate()
            .filter(|(_, w)| w.owner_champion_id == owner_id && w.expires_at > now)
            .map(|(idx, _)| idx)
            .collect();
        if owner_wards.len() >= 2 {
            owner_wards.sort_by(|a, b| {
                runtime.wards[*a]
                    .expires_at
                    .partial_cmp(&runtime.wards[*b].expires_at)
                    .unwrap_or(Ordering::Equal)
            });
            if let Some(drop_idx) = owner_wards.first().copied() {
                runtime.wards.remove(drop_idx);
            }
        }
        runtime.wards.push(ward);
    }
}

fn maybe_upgrade_trinket_to_oracle(champion: &mut ChampionRuntime, now: f64) {
    if champion.trinket_swapped || now < TRINKET_SWAP_UNLOCK_AT_SEC {
        return;
    }
    if champion.role != "JGL" && champion.role != "SUP" {
        return;
    }
    champion.trinket_key = TRINKET_ORACLE_LENS.to_string();
    champion.trinket_swapped = true;
}

fn process_sweepers(runtime: &mut RuntimeState) {
    let now = runtime.time_sec;
    let mut activated_by: Vec<String> = Vec::new();

    for champion in &mut runtime.champions {
        if !champion.alive || champion_is_banished(champion) {
            continue;
        }
        if champion.role != "JGL" && champion.role != "SUP" {
            continue;
        }
        if !champion
            .trinket_key
            .eq_ignore_ascii_case(TRINKET_ORACLE_LENS)
        {
            continue;
        }

        if now >= champion.sweeper_active_until
            && now >= champion.sweeper_cd_until
            && runtime.wards.iter().any(|ward| {
                normalized_team(&ward.team) != normalized_team(&champion.team)
                    && ward.expires_at > now
                    && dist(ward.pos, champion.pos) <= SWEEPER_CLEAR_RADIUS
            })
        {
            champion.sweeper_active_until = now + SWEEPER_DURATION_SEC;
            champion.sweeper_cd_until = now + SWEEPER_COOLDOWN_SEC;
            activated_by.push(champion.name.clone());
        }
    }

    for name in activated_by {
        log_event(runtime, &format!("{} activated Sweeper", name), "info");
    }

    let mut should_clear = Vec::new();
    for (idx, ward) in runtime.wards.iter().enumerate() {
        let cleared = runtime.champions.iter().any(|champion| {
            champion.alive
                && !champion_is_banished(champion)
                && (champion.role == "JGL" || champion.role == "SUP")
                && champion.sweeper_active_until > now
                && normalized_team(&champion.team) != normalized_team(&ward.team)
                && dist(champion.pos, ward.pos) <= SWEEPER_CLEAR_RADIUS
        });
        if cleared {
            should_clear.push(idx);
        }
    }

    for idx in should_clear.into_iter().rev() {
        runtime.wards.remove(idx);
    }
}

fn ultimate_ready(champion: &ChampionRuntime, now: f64) -> bool {
    champion
        .ultimate
        .as_ref()
        .map(|ultimate| now >= ultimate.cd_until)
        .unwrap_or(false)
}

fn set_ultimate_cd(champion: &mut ChampionRuntime, now: f64, cooldown_sec: f64) -> bool {
    let Some(ultimate) = champion.ultimate.as_mut() else {
        return false;
    };
    ultimate.cd_until = now + cooldown_sec;
    true
}

fn nearest_enemy_in_range(
    runtime: &RuntimeState,
    champion_idx: usize,
    range: f64,
) -> Option<usize> {
    if champion_idx >= runtime.champions.len() {
        return None;
    }
    let champion = &runtime.champions[champion_idx];
    runtime
        .champions
        .iter()
        .enumerate()
        .filter(|(idx, enemy)| {
            *idx != champion_idx
                && enemy.alive
                && !champion_is_banished(enemy)
                && normalized_team(&enemy.team) != normalized_team(&champion.team)
                && team_has_vision_at(runtime, &champion.team, enemy.pos)
                && dist(enemy.pos, champion.pos) <= range
        })
        .min_by(|(_, a), (_, b)| {
            dist(a.pos, champion.pos)
                .partial_cmp(&dist(b.pos, champion.pos))
                .unwrap_or(Ordering::Equal)
        })
        .map(|(idx, _)| idx)
}

fn next_summon_id(runtime: &mut RuntimeState) -> String {
    let next = runtime
        .extra
        .get("nextSummonId")
        .and_then(Value::as_i64)
        .unwrap_or(1)
        .max(1);
    runtime
        .extra
        .insert("nextSummonId".to_string(), Value::from(next + 1));
    format!("summon-{next}")
}

fn set_rift_herald_charge(runtime: &mut RuntimeState, killer_team: &str, killer_id: &str) {
    runtime
        .extra
        .insert("heraldReady".to_string(), Value::from(true));
    runtime.extra.insert(
        "heraldTeam".to_string(),
        Value::from(normalized_team(killer_team)),
    );
    runtime
        .extra
        .insert("heraldCarrierId".to_string(), Value::from(killer_id));
}

fn clear_rift_herald_charge(runtime: &mut RuntimeState) {
    runtime
        .extra
        .insert("heraldReady".to_string(), Value::from(false));
    runtime.extra.remove("heraldTeam");
    runtime.extra.remove("heraldCarrierId");
}

fn maybe_deploy_rift_herald_charge(runtime: &mut RuntimeState) {
    let ready = runtime
        .extra
        .get("heraldReady")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !ready {
        return;
    }

    let carrier_id = runtime
        .extra
        .get("heraldCarrierId")
        .and_then(Value::as_str)
        .map(|value| value.to_string());
    let herald_team = runtime
        .extra
        .get("heraldTeam")
        .and_then(Value::as_str)
        .map(normalized_team)
        .unwrap_or("blue")
        .to_string();

    let carrier_idx = if let Some(carrier_id) = carrier_id {
        runtime
            .champions
            .iter()
            .position(|champion| champion.alive && champion.id == carrier_id)
    } else {
        runtime.champions.iter().position(|champion| {
            champion.alive
                && normalized_team(&champion.team) == normalized_team(&herald_team)
                && champion.role == "JGL"
        })
    };

    let Some(carrier_idx) = carrier_idx else {
        return;
    };

    let carrier = runtime.champions[carrier_idx].clone();
    let enemy_tower_idx = runtime
        .structures
        .iter()
        .enumerate()
        .filter(|(_, structure)| {
            structure.alive
                && structure.kind == "tower"
                && normalized_lane(&structure.lane) == normalized_lane(&carrier.lane)
                && is_structure_targetable(&runtime.structures, &carrier.team, structure)
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            dist(a.pos, carrier.pos)
                .partial_cmp(&dist(b.pos, carrier.pos))
                .unwrap_or(Ordering::Equal)
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx);

    let Some(enemy_tower_idx) = enemy_tower_idx else {
        clear_rift_herald_charge(runtime);
        return;
    };

    let enemy_tower_pos = runtime.structures[enemy_tower_idx].pos;
    if dist(carrier.pos, enemy_tower_pos) > 0.12 {
        return;
    }

    let summon = MinionRuntime {
        id: format!("herald-{}", next_summon_id(runtime)),
        team: carrier.team.clone(),
        lane: carrier.lane.clone(),
        pos: Vec2 {
            x: clamp(carrier.pos.x + 0.012, 0.01, 0.99),
            y: clamp(carrier.pos.y + 0.012, 0.01, 0.99),
        },
        hp: 420.0,
        max_hp: 420.0,
        alive: true,
        kind: "summon".to_string(),
        last_hit_by_champion_id: None,
        owner_champion_id: None,
        summon_kind: Some("herald".to_string()),
        summon_expires_at: runtime.time_sec + 55.0,
        attack_cd_until: runtime.time_sec,
        move_speed: 0.058,
        attack_range: 0.065,
        attack_damage: 34.0,
        path: lane_path_for(&carrier.team, &carrier.lane),
        path_index: 0,
    };

    runtime.minions.push(summon);
    log_event(
        runtime,
        &format!(
            "{} deployed rift herald",
            normalized_team(&carrier.team).to_uppercase()
        ),
        "info",
    );
    clear_rift_herald_charge(runtime);
}

fn try_cast_special_ultimate(
    runtime: &mut RuntimeState,
    champion_idx: usize,
    now: f64,
) -> Option<bool> {
    let champion = runtime.champions.get(champion_idx)?.clone();
    let key = champion.champion_id.to_lowercase();

    if ["yorick", "annie", "ivern", "shaco"].contains(&key.as_str()) {
        let (summon_kind, hp_ratio, damage_ratio, duration_sec) = summon_profile(&key);

        let already_alive = runtime.minions.iter().any(|minion| {
            minion.alive
                && minion.kind == "summon"
                && minion.owner_champion_id.as_deref() == Some(champion.id.as_str())
        });
        if already_alive {
            return Some(false);
        }

        let summon = MinionRuntime {
            id: format!("{}-{}", summon_kind, next_summon_id(runtime)),
            team: champion.team.clone(),
            lane: champion.lane.clone(),
            pos: Vec2 {
                x: clamp(champion.pos.x + 0.014, 0.01, 0.99),
                y: clamp(champion.pos.y + 0.01, 0.01, 0.99),
            },
            hp: (champion.max_hp * hp_ratio).max(35.0),
            max_hp: (champion.max_hp * hp_ratio).max(35.0),
            alive: true,
            kind: "summon".to_string(),
            last_hit_by_champion_id: None,
            owner_champion_id: Some(champion.id.clone()),
            summon_kind: Some(summon_kind.to_string()),
            summon_expires_at: now + duration_sec,
            attack_cd_until: now,
            move_speed: (champion.move_speed * 0.95).max(0.038),
            attack_range: champion.attack_range.max(0.045),
            attack_damage: (champion.attack_damage * damage_ratio).max(4.0),
            path: vec![champion.pos],
            path_index: 0,
        };

        runtime.minions.push(summon);
        log_event(
            runtime,
            &format!("{} summoned {}", champion.name, summon_kind),
            "info",
        );
        return Some(true);
    }

    if key == "shen" {
        let ally_idx = runtime
            .champions
            .iter()
            .enumerate()
            .filter(|(idx, ally)| {
                *idx != champion_idx
                    && ally.alive
                    && !champion_is_banished(ally)
                    && normalized_team(&ally.team) == normalized_team(&champion.team)
            })
            .min_by(|(idx_a, a), (idx_b, b)| {
                let ratio_a = if a.max_hp <= 0.0 {
                    1.0
                } else {
                    a.hp / a.max_hp
                };
                let ratio_b = if b.max_hp <= 0.0 {
                    1.0
                } else {
                    b.hp / b.max_hp
                };
                ratio_a
                    .partial_cmp(&ratio_b)
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| idx_a.cmp(idx_b))
            })
            .map(|(idx, _)| idx);

        let Some(ally_idx) = ally_idx else {
            return Some(false);
        };

        let shield = runtime.champions[ally_idx].max_hp * 0.30;
        let ally_pos = runtime.champions[ally_idx].pos;
        runtime.champions[ally_idx].hp =
            (runtime.champions[ally_idx].hp + shield).min(runtime.champions[ally_idx].max_hp);
        runtime.champions[champion_idx].pos = ally_pos;
        runtime.champions[champion_idx].target_path.clear();
        runtime.champions[champion_idx].target_path_index = 0;
        runtime.champions[champion_idx].next_decision_at = now;
        log_event(
            runtime,
            &format!("{} cast Stand United", champion.name),
            "info",
        );
        return Some(true);
    }

    if key == "mordekaiser" {
        let Some(target_idx) =
            nearest_enemy_in_range(runtime, champion_idx, ULTIMATE_BURST_RANGE + 0.03)
        else {
            return Some(false);
        };
        let caster_pos = runtime.champions[champion_idx].pos;
        let target_pos = runtime.champions[target_idx].pos;

        runtime.champions[champion_idx].realm_banished_until =
            now + ULTIMATE_MORDE_REALM_DURATION_SEC;
        runtime.champions[champion_idx].realm_return_pos = Some(caster_pos);
        runtime.champions[target_idx].realm_banished_until =
            now + ULTIMATE_MORDE_REALM_DURATION_SEC;
        runtime.champions[target_idx].realm_return_pos = Some(target_pos);

        runtime.champions[champion_idx].pos = Vec2 { x: -5.0, y: -5.0 };
        runtime.champions[target_idx].pos = Vec2 { x: -6.0, y: -6.0 };
        runtime.champions[champion_idx].target_path.clear();
        runtime.champions[target_idx].target_path.clear();
        runtime.champions[champion_idx].target_path_index = 0;
        runtime.champions[target_idx].target_path_index = 0;

        log_event(
            runtime,
            &format!("{} cast Realm of Death", champion.name),
            "info",
        );
        return Some(true);
    }

    None
}

fn try_cast_ultimate(runtime: &mut RuntimeState, champion_idx: usize, now: f64) -> bool {
    if champion_idx >= runtime.champions.len() || !runtime.champions[champion_idx].alive {
        return false;
    }

    let champion_snapshot = runtime.champions[champion_idx].clone();
    if champion_snapshot.level < ULTIMATE_UNLOCK_LEVEL || !ultimate_ready(&champion_snapshot, now) {
        return false;
    }

    if let Some(casted_special) = try_cast_special_ultimate(runtime, champion_idx, now) {
        if casted_special {
            if set_ultimate_cd(
                &mut runtime.champions[champion_idx],
                now,
                ULTIMATE_BASE_CD_SEC,
            ) {
                return true;
            }
        }
        return false;
    }

    let archetype = champion_snapshot
        .ultimate
        .as_ref()
        .map(|ultimate| ultimate.archetype.to_lowercase())
        .unwrap_or_else(|| {
            default_ultimate_archetype_for_role(&champion_snapshot.role).to_string()
        });

    let casted = match archetype.as_str() {
        "execute" => {
            let Some(target_idx) =
                nearest_enemy_in_range(runtime, champion_idx, ULTIMATE_BURST_RANGE)
            else {
                return false;
            };
            let hp_ratio = if runtime.champions[target_idx].max_hp <= 0.0 {
                1.0
            } else {
                runtime.champions[target_idx].hp / runtime.champions[target_idx].max_hp
            };
            if hp_ratio > 0.38 {
                return false;
            }
            attack_enemy_champion(runtime, champion_idx, target_idx);
            attack_enemy_champion(runtime, champion_idx, target_idx);
            true
        }
        "engage" => {
            let Some(target_idx) =
                nearest_enemy_in_range(runtime, champion_idx, ULTIMATE_GLOBAL_RANGE)
            else {
                return false;
            };
            let target = runtime.champions[target_idx].pos;
            runtime.champions[champion_idx].pos = target;
            runtime.champions[champion_idx].target_path.clear();
            runtime.champions[champion_idx].target_path_index = 0;
            attack_enemy_champion(runtime, champion_idx, target_idx);
            true
        }
        "utility" | "sustain" | "defensive" => {
            if champion_snapshot.max_hp <= 0.0 {
                return false;
            }
            let hp_ratio = champion_snapshot.hp / champion_snapshot.max_hp;
            if hp_ratio > 0.55 {
                return false;
            }
            let heal_amount = champion_snapshot.max_hp * 0.26;
            runtime.champions[champion_idx].hp = (runtime.champions[champion_idx].hp + heal_amount)
                .min(runtime.champions[champion_idx].max_hp);
            true
        }
        "global" | "zone" => {
            let Some(target_idx) =
                nearest_enemy_in_range(runtime, champion_idx, ULTIMATE_GLOBAL_RANGE)
            else {
                return false;
            };
            attack_enemy_champion(runtime, champion_idx, target_idx);
            true
        }
        _ => {
            let Some(target_idx) =
                nearest_enemy_in_range(runtime, champion_idx, ULTIMATE_BURST_RANGE)
            else {
                return false;
            };
            attack_enemy_champion(runtime, champion_idx, target_idx);
            true
        }
    };

    if !casted {
        return false;
    }

    if set_ultimate_cd(
        &mut runtime.champions[champion_idx],
        now,
        ULTIMATE_BASE_CD_SEC,
    ) {
        log_event(
            runtime,
            &format!("{} cast Ultimate ({})", champion_snapshot.name, archetype),
            "info",
        );
        return true;
    }
    false
}

fn tick_ignite_dot_effects(runtime: &mut RuntimeState, now: f64) {
    for idx in 0..runtime.champions.len() {
        if !runtime.champions[idx].alive {
            runtime.champions[idx].ignite_dot_until = 0.0;
            runtime.champions[idx].ignite_source_id = None;
            continue;
        }
        if runtime.champions[idx].ignite_dot_until <= now {
            runtime.champions[idx].ignite_source_id = None;
            continue;
        }

        runtime.champions[idx].hp -= SUMMONER_IGNITE_DPS * 0.2;
        runtime.champions[idx].last_damaged_at = now;

        if runtime.champions[idx].hp > 0.0 {
            continue;
        }

        runtime.champions[idx].hp = 0.0;
        runtime.champions[idx].alive = false;
        runtime.champions[idx].deaths += 1;
        runtime.champions[idx].respawn_at =
            now + champion_respawn_seconds(runtime.champions[idx].level, now);

        let victim_name = runtime.champions[idx].name.clone();
        let killer_id = runtime.champions[idx].ignite_source_id.clone();
        runtime.champions[idx].ignite_dot_until = 0.0;
        runtime.champions[idx].ignite_source_id = None;

        if let Some(killer_id) = killer_id {
            if let Some(killer_idx) = runtime
                .champions
                .iter()
                .position(|champion| champion.id == killer_id)
            {
                if runtime.champions[killer_idx].alive {
                    runtime.champions[killer_idx].kills += 1;
                    let killer_team = runtime.champions[killer_idx].team.clone();
                    team_stats_mut(&mut runtime.stats, &killer_team).kills += 1;
                    add_gold_xp_to_champion(
                        runtime,
                        &killer_id,
                        CHAMPION_KILL_GOLD,
                        CHAMPION_KILL_XP,
                    );
                    log_event(
                        runtime,
                        &format!(
                            "{} ignited {}",
                            runtime.champions[killer_idx].name, victim_name
                        ),
                        "kill",
                    );
                    continue;
                }
            }
        }
    }
}

fn best_lane_tp_target(
    champion: &ChampionRuntime,
    structures: &[StructureRuntime],
    minions: &[MinionRuntime],
) -> Option<Vec2> {
    let lane_path = lane_path_for(&champion.team, &champion.lane);
    let max_idx = lane_path.len().saturating_sub(1).max(1);

    let tower_target = structures
        .iter()
        .filter(|structure| {
            structure.alive
                && normalized_team(&structure.team) == normalized_team(&champion.team)
                && structure.kind == "tower"
                && normalized_lane(&structure.lane) == normalized_lane(&champion.lane)
        })
        .max_by(|a, b| {
            let a_idx = closest_lane_path_index(a.pos, &lane_path) as f64 / max_idx as f64;
            let b_idx = closest_lane_path_index(b.pos, &lane_path) as f64 / max_idx as f64;
            a_idx
                .partial_cmp(&b_idx)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        })
        .map(|structure| {
            let progress =
                closest_lane_path_index(structure.pos, &lane_path) as f64 / max_idx as f64;
            (structure.pos, progress)
        });

    let minion_target = minions
        .iter()
        .filter(|minion| {
            minion.alive
                && normalized_team(&minion.team) == normalized_team(&champion.team)
                && normalized_lane(&minion.lane) == normalized_lane(&champion.lane)
        })
        .max_by(|a, b| {
            let a_idx = closest_lane_path_index(a.pos, &lane_path) as f64 / max_idx as f64;
            let b_idx = closest_lane_path_index(b.pos, &lane_path) as f64 / max_idx as f64;
            a_idx
                .partial_cmp(&b_idx)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        })
        .map(|minion| {
            let progress = closest_lane_path_index(minion.pos, &lane_path) as f64 / max_idx as f64;
            (minion.pos, progress)
        });

    match (tower_target, minion_target) {
        (Some((tower_pos, tower_progress)), Some((minion_pos, minion_progress))) => {
            if minion_progress > tower_progress {
                Some(minion_pos)
            } else {
                Some(tower_pos)
            }
        }
        (Some((tower_pos, _)), None) => Some(tower_pos),
        (None, Some((minion_pos, _))) => Some(minion_pos),
        (None, None) => None,
    }
}

fn try_cast_summoner_spells(
    runtime: &mut RuntimeState,
    neutral_timers: &mut NeutralTimersRuntime,
    champion_idx: usize,
    now: f64,
) -> bool {
    if champion_idx >= runtime.champions.len() || !runtime.champions[champion_idx].alive {
        return false;
    }

    if try_cast_heal(runtime, champion_idx, now) {
        return true;
    }
    if try_cast_flash(runtime, champion_idx, now) {
        return true;
    }
    if try_cast_ignite(runtime, champion_idx, now) {
        return true;
    }
    if try_cast_smite(runtime, neutral_timers, champion_idx, now) {
        return true;
    }
    if try_cast_teleport(runtime, champion_idx, now) {
        return true;
    }

    false
}

fn try_cast_heal(runtime: &mut RuntimeState, champion_idx: usize, now: f64) -> bool {
    let champion_snapshot = runtime.champions[champion_idx].clone();
    if !champion_has_spell(&champion_snapshot, "Heal")
        || !spell_ready(&champion_snapshot, "Heal", now)
    {
        return false;
    }

    let self_ratio = if champion_snapshot.max_hp <= 0.0 {
        1.0
    } else {
        champion_snapshot.hp / champion_snapshot.max_hp
    };

    let low_ally_exists = runtime.champions.iter().any(|ally| {
        ally.alive
            && ally.id != champion_snapshot.id
            && normalized_team(&ally.team) == normalized_team(&champion_snapshot.team)
            && dist(ally.pos, champion_snapshot.pos) <= SUMMONER_HEAL_RADIUS
            && ally.max_hp > 0.0
            && (ally.hp / ally.max_hp) <= 0.35
    });

    if self_ratio > 0.34 && !low_ally_exists {
        return false;
    }

    for ally in runtime.champions.iter_mut() {
        if !ally.alive || normalized_team(&ally.team) != normalized_team(&champion_snapshot.team) {
            continue;
        }
        if ally.id != champion_snapshot.id
            && dist(ally.pos, champion_snapshot.pos) > SUMMONER_HEAL_RADIUS
        {
            continue;
        }
        let ratio = if ally.id == champion_snapshot.id {
            SUMMONER_HEAL_SELF_RATIO
        } else {
            SUMMONER_HEAL_ALLY_RATIO
        };
        ally.hp = (ally.hp + ally.max_hp * ratio).min(ally.max_hp);
    }

    if set_spell_cd(
        &mut runtime.champions[champion_idx],
        "Heal",
        now,
        SUMMONER_HEAL_CD_SEC,
    ) {
        log_event(
            runtime,
            &format!("{} cast Heal", champion_snapshot.name),
            "info",
        );
        return true;
    }
    false
}

fn try_cast_flash(runtime: &mut RuntimeState, champion_idx: usize, now: f64) -> bool {
    let champion_snapshot = runtime.champions[champion_idx].clone();
    if !champion_has_spell(&champion_snapshot, "Flash")
        || !spell_ready(&champion_snapshot, "Flash", now)
    {
        return false;
    }

    let self_ratio = if champion_snapshot.max_hp <= 0.0 {
        1.0
    } else {
        champion_snapshot.hp / champion_snapshot.max_hp
    };
    let nearest_enemy = runtime
        .champions
        .iter()
        .filter(|enemy| {
            enemy.alive
                && normalized_team(&enemy.team) != normalized_team(&champion_snapshot.team)
                && dist(enemy.pos, champion_snapshot.pos) <= 0.10
        })
        .min_by(|a, b| {
            dist(a.pos, champion_snapshot.pos)
                .partial_cmp(&dist(b.pos, champion_snapshot.pos))
                .unwrap_or(Ordering::Equal)
        });

    if self_ratio > 0.28 || nearest_enemy.is_none() {
        return false;
    }

    let base = base_position_for(&champion_snapshot.team);
    let to_base = Vec2 {
        x: base.x - champion_snapshot.pos.x,
        y: base.y - champion_snapshot.pos.y,
    };
    let len = (to_base.x * to_base.x + to_base.y * to_base.y)
        .sqrt()
        .max(1e-6);
    let target = Vec2 {
        x: clamp(
            champion_snapshot.pos.x + (to_base.x / len) * SUMMONER_FLASH_RANGE,
            0.01,
            0.99,
        ),
        y: clamp(
            champion_snapshot.pos.y + (to_base.y / len) * SUMMONER_FLASH_RANGE,
            0.01,
            0.99,
        ),
    };

    runtime.champions[champion_idx].pos = target;
    runtime.champions[champion_idx].target_path.clear();
    runtime.champions[champion_idx].target_path_index = 0;

    if set_spell_cd(
        &mut runtime.champions[champion_idx],
        "Flash",
        now,
        SUMMONER_FLASH_CD_SEC,
    ) {
        log_event(
            runtime,
            &format!("{} flashed", champion_snapshot.name),
            "info",
        );
        return true;
    }
    false
}

fn try_cast_ignite(runtime: &mut RuntimeState, champion_idx: usize, now: f64) -> bool {
    let champion_snapshot = runtime.champions[champion_idx].clone();
    if !champion_has_spell(&champion_snapshot, "Ignite")
        || !spell_ready(&champion_snapshot, "Ignite", now)
    {
        return false;
    }

    let target_idx = runtime
        .champions
        .iter()
        .enumerate()
        .filter(|(_, enemy)| {
            enemy.alive
                && normalized_team(&enemy.team) != normalized_team(&champion_snapshot.team)
                && dist(enemy.pos, champion_snapshot.pos) <= SUMMONER_IGNITE_RANGE
                && enemy.ignite_dot_until <= now
                && enemy.max_hp > 0.0
                && (enemy.hp / enemy.max_hp) <= 0.42
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            (a.hp / a.max_hp)
                .partial_cmp(&(b.hp / b.max_hp))
                .unwrap_or(Ordering::Equal)
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx);

    let Some(target_idx) = target_idx else {
        return false;
    };

    let target_name = runtime.champions[target_idx].name.clone();
    runtime.champions[target_idx].ignite_dot_until = now + SUMMONER_IGNITE_DURATION_SEC;
    runtime.champions[target_idx].ignite_source_id = Some(champion_snapshot.id.clone());
    runtime.champions[target_idx].last_damaged_by_champion_id = Some(champion_snapshot.id.clone());
    runtime.champions[target_idx].last_damaged_at = now;

    if set_spell_cd(
        &mut runtime.champions[champion_idx],
        "Ignite",
        now,
        SUMMONER_IGNITE_CD_SEC,
    ) {
        log_event(
            runtime,
            &format!("{} ignited {}", champion_snapshot.name, target_name),
            "info",
        );
        return true;
    }
    false
}

fn try_cast_smite(
    runtime: &mut RuntimeState,
    neutral_timers: &mut NeutralTimersRuntime,
    champion_idx: usize,
    now: f64,
) -> bool {
    let champion_snapshot = runtime.champions[champion_idx].clone();
    if !champion_has_spell(&champion_snapshot, "Smite")
        || !spell_ready(&champion_snapshot, "Smite", now)
    {
        return false;
    }
    if champion_snapshot.role != "JGL" {
        return false;
    }

    let neutral_key = nearest_attackable_neutral_key(
        &champion_snapshot,
        neutral_timers,
        SUMMONER_SMITE_RANGE,
        SUMMONER_SMITE_RANGE,
    );
    let Some(neutral_key) = neutral_key else {
        return false;
    };

    let Some(timer) = neutral_timers.entities.get(&neutral_key) else {
        return false;
    };
    if !timer.alive || timer.hp > SUMMONER_SMITE_DAMAGE {
        return false;
    }

    if let Some(timer_mut) = neutral_timers.entities.get_mut(&neutral_key) {
        timer_mut.hp = 0.0;
    }
    mark_neutral_taken(runtime, neutral_timers, &neutral_key, Some(champion_idx));

    if set_spell_cd(
        &mut runtime.champions[champion_idx],
        "Smite",
        now,
        SUMMONER_SMITE_CD_SEC,
    ) {
        log_event(
            runtime,
            &format!("{} cast Smite", champion_snapshot.name),
            "info",
        );
        return true;
    }
    false
}

fn try_cast_teleport(runtime: &mut RuntimeState, champion_idx: usize, now: f64) -> bool {
    let champion_snapshot = runtime.champions[champion_idx].clone();
    if !champion_has_spell(&champion_snapshot, "Teleport")
        || !spell_ready(&champion_snapshot, "Teleport", now)
    {
        return false;
    }
    if now < SUMMONER_TP_UNLOCK_AT_SEC {
        return false;
    }

    let base = base_position_for(&champion_snapshot.team);
    let at_base = dist(champion_snapshot.pos, base) <= 0.22;
    if !at_base {
        return false;
    }

    let nearby_enemy = runtime.champions.iter().any(|enemy| {
        enemy.alive
            && normalized_team(&enemy.team) != normalized_team(&champion_snapshot.team)
            && dist(enemy.pos, champion_snapshot.pos) <= 0.14
    });
    if nearby_enemy {
        return false;
    }

    let Some(target) =
        best_lane_tp_target(&champion_snapshot, &runtime.structures, &runtime.minions)
    else {
        return false;
    };

    runtime.champions[champion_idx].pos = target;
    runtime.champions[champion_idx].target_path.clear();
    runtime.champions[champion_idx].target_path_index = 0;
    runtime.champions[champion_idx].next_decision_at = now;

    if set_spell_cd(
        &mut runtime.champions[champion_idx],
        "Teleport",
        now,
        SUMMONER_TP_CD_SEC,
    ) {
        log_event(
            runtime,
            &format!("{} cast Teleport", champion_snapshot.name),
            "recall",
        );
        return true;
    }
    false
}

fn resolve_structure_combat(runtime: &mut RuntimeState) {
    let now = runtime.time_sec;

    for idx in 0..runtime.structures.len() {
        if !runtime.structures[idx].alive
            || runtime.structures[idx].kind != "tower"
            || now < runtime.structures[idx].attack_cd_until
        {
            continue;
        }

        let structure_team = runtime.structures[idx].team.clone();
        let structure_pos = runtime.structures[idx].pos;

        let forced_target_id = runtime.structures[idx].forced_target_champion_id.clone();
        let forced_target_until = runtime.structures[idx].forced_target_until;
        if let Some(target_id) = forced_target_id {
            let lock_active = now <= forced_target_until;
            if lock_active {
                let forced_target_idx = runtime
                    .champions
                    .iter()
                    .enumerate()
                    .find(|(_, champion)| {
                        champion.alive
                            && !champion_is_banished(champion)
                            && champion.id == target_id
                            && normalized_team(&champion.team) != normalized_team(&structure_team)
                            && dist(champion.pos, structure_pos) <= TOWER_ATTACK_RANGE
                    })
                    .map(|(champion_idx, _)| champion_idx);

                if let Some(champion_idx) = forced_target_idx {
                    apply_tower_shot_to_champion(runtime, idx, champion_idx);
                    continue;
                }
            }

            runtime.structures[idx].forced_target_champion_id = None;
            runtime.structures[idx].forced_target_until = 0.0;
        }

        if let Some(minion_idx) = nearest_enemy_minion_for_structure(
            &runtime.minions,
            &structure_team,
            structure_pos,
            TOWER_ATTACK_RANGE,
        ) {
            let incoming = if minion_is_baron_empowered(runtime, &runtime.minions[minion_idx]) {
                TOWER_SHOT_DAMAGE_TO_MINION * (1.0 - BARON_MINION_DAMAGE_REDUCTION)
            } else {
                TOWER_SHOT_DAMAGE_TO_MINION
            };
            runtime.minions[minion_idx].hp -= incoming;
            runtime.structures[idx].attack_cd_until = now + TOWER_ATTACK_CADENCE_SEC;
            if runtime.minions[minion_idx].hp <= 0.0 {
                register_minion_death(runtime, minion_idx);
            }
            continue;
        }

        if let Some(champion_idx) = nearest_enemy_champion_for_structure(
            &runtime.champions,
            &structure_team,
            structure_pos,
            TOWER_ATTACK_RANGE,
        ) {
            apply_tower_shot_to_champion(runtime, idx, champion_idx);
        }
    }
}

fn neutral_timers_default_runtime_state() -> NeutralTimersRuntime {
    serde_json::from_value(build_neutral_timers_state()).unwrap_or(NeutralTimersRuntime {
        dragon_soul_unlocked: false,
        elder_unlocked: false,
        entities: HashMap::new(),
        extra: HashMap::new(),
    })
}

fn decode_neutral_timers_state(value: &Value) -> Option<NeutralTimersRuntime> {
    serde_json::from_value(value.clone()).ok()
}

fn nearest_enemy_champion_contesting_objective(
    champions: &[ChampionRuntime],
    attacker: &ChampionRuntime,
    objective_pos: Vec2,
    range: f64,
) -> Option<usize> {
    champions
        .iter()
        .enumerate()
        .filter(|(_, enemy)| {
            enemy.alive
                && enemy.id != attacker.id
                && normalized_team(&enemy.team) != normalized_team(&attacker.team)
                && dist(enemy.pos, objective_pos) <= OBJECTIVE_ASSIST_RADIUS
                && dist(enemy.pos, attacker.pos) <= range
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            dist(a.pos, attacker.pos)
                .partial_cmp(&dist(b.pos, attacker.pos))
                .unwrap_or(Ordering::Equal)
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx)
}

fn nearest_attackable_neutral_key(
    champion: &ChampionRuntime,
    neutral_timers: &NeutralTimersRuntime,
    camp_radius: f64,
    objective_radius: f64,
) -> Option<String> {
    let mut candidates: Vec<&NeutralTimerRuntime> = neutral_timers
        .entities
        .values()
        .filter(|timer| timer.alive && timer.unlocked)
        .filter(|timer| {
            let max_range = if is_objective_neutral_key(&timer.key) {
                objective_radius
            } else {
                camp_radius
            };
            dist(champion.pos, timer.pos) <= max_range
        })
        .collect();

    candidates.sort_by(|a, b| {
        dist(champion.pos, a.pos)
            .partial_cmp(&dist(champion.pos, b.pos))
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.key.cmp(&b.key))
    });

    candidates.first().map(|timer| timer.key.clone())
}

fn jungle_camp_reward(key: &str) -> Option<(i64, i64)> {
    match key {
        "blue-buff-blue" | "blue-buff-red" => Some((95, 150)),
        "red-buff-blue" | "red-buff-red" => Some((95, 155)),
        "wolves-blue" | "wolves-red" => Some((70, 110)),
        "raptors-blue" | "raptors-red" => Some((72, 115)),
        "gromp-blue" | "gromp-red" => Some((82, 128)),
        "krugs-blue" | "krugs-red" => Some((86, 132)),
        "scuttle-top" | "scuttle-bot" => Some((70, 110)),
        _ => None,
    }
}

fn jungle_camp_cs_reward(key: &str) -> Option<i64> {
    match key {
        "blue-buff-blue" | "blue-buff-red" => Some(2),
        "red-buff-blue" | "red-buff-red" => Some(2),
        "wolves-blue" | "wolves-red" => Some(3),
        "raptors-blue" | "raptors-red" => Some(6),
        "gromp-blue" | "gromp-red" => Some(1),
        "krugs-blue" | "krugs-red" => Some(10),
        "scuttle-top" | "scuttle-bot" => Some(1),
        _ => None,
    }
}

fn mark_neutral_taken(
    runtime: &mut RuntimeState,
    neutral_timers: &mut NeutralTimersRuntime,
    key: &str,
    killer_idx: Option<usize>,
) {
    let timer_label = {
        let Some(timer) = neutral_timers.entities.get_mut(key) else {
            return;
        };
        if !timer.alive {
            return;
        }

        timer.alive = false;
        timer.hp = 0.0;
        timer.last_taken_at = Some(runtime.time_sec);
        timer.times_taken += 1;
        timer.next_spawn_at = if timer.one_shot || timer.respawn_delay_sec.is_none() {
            None
        } else {
            Some(runtime.time_sec + timer.respawn_delay_sec.unwrap_or(0.0))
        };
        timer.label.clone()
    };

    let Some(champion_idx) = killer_idx else {
        return;
    };
    if champion_idx >= runtime.champions.len() {
        return;
    }

    let killer_id = runtime.champions[champion_idx].id.clone();
    let killer_name = runtime.champions[champion_idx].name.clone();
    let killer_team = runtime.champions[champion_idx].team.clone();
    let killer_role = runtime.champions[champion_idx].role.clone();

    if is_jungle_camp_key(key) {
        if killer_role == "SUP" && runtime.time_sec >= SUPPORT_OPEN_ROAM_AT_SEC {
            log_event(
                runtime,
                &format!("{} skipped {}", killer_name, timer_label),
                "info",
            );
            return;
        }
        if let Some((gold, xp)) = jungle_camp_reward(key) {
            let (award_gold, award_xp) = if killer_role == "JGL" {
                (
                    ((gold as f64) * JGL_JUNGLE_GOLD_MULTIPLIER).round() as i64,
                    ((xp as f64) * JGL_JUNGLE_XP_MULTIPLIER).round() as i64,
                )
            } else {
                (
                    ((gold as f64) * OFFROLE_JUNGLE_REWARD_MULTIPLIER).round() as i64,
                    ((xp as f64) * OFFROLE_JUNGLE_REWARD_MULTIPLIER).round() as i64,
                )
            };
            add_gold_xp_to_champion(runtime, &killer_id, award_gold, award_xp);
        }
        if let Some(base_cs) = jungle_camp_cs_reward(key) {
            let award_cs = if killer_role == "JGL" {
                base_cs
            } else {
                ((base_cs as f64) * OFFROLE_JUNGLE_REWARD_MULTIPLIER).round() as i64
            }
            .max(1);
            add_cs_to_champion(runtime, &killer_id, award_cs);
        }
        log_event(
            runtime,
            &format!("{} cleared {}", killer_name, timer_label),
            "info",
        );
        return;
    }

    if key == "dragon" {
        team_stats_mut(&mut runtime.stats, &killer_team).dragons += 1;
        add_gold_xp_to_champion(runtime, &killer_id, DRAGON_SECURE_GOLD, DRAGON_SECURE_XP);
        let dragon_kind = process_dragon_capture(runtime, neutral_timers, &killer_team);
        log_event(
            runtime,
            &format!(
                "{} secured {} dragon",
                normalized_team(&killer_team).to_uppercase(),
                dragon_kind.to_uppercase()
            ),
            "dragon",
        );
        return;
    }

    if key == "baron" {
        team_stats_mut(&mut runtime.stats, &killer_team).barons += 1;
        add_gold_xp_to_champion(runtime, &killer_id, BARON_SECURE_GOLD, BARON_SECURE_XP);
        let mut buffs = runtime_buffs_from_extra(runtime.extra.get("teamBuffs"));
        team_buffs_mut(&mut buffs, &killer_team).baron_until =
            runtime.time_sec + BARON_BUFF_DURATION_SEC;
        set_runtime_buffs(runtime, &buffs);
        log_event(
            runtime,
            &format!(
                "{} secured baron",
                normalized_team(&killer_team).to_uppercase()
            ),
            "baron",
        );
        return;
    }

    if key == "elder" {
        add_gold_xp_to_champion(
            runtime,
            &killer_id,
            OBJECTIVE_SECURE_GOLD + 35,
            OBJECTIVE_SECURE_XP + 55,
        );
        let mut buffs = runtime_buffs_from_extra(runtime.extra.get("teamBuffs"));
        team_buffs_mut(&mut buffs, &killer_team).elder_until =
            runtime.time_sec + ELDER_BUFF_DURATION_SEC;
        set_runtime_buffs(runtime, &buffs);
        log_event(
            runtime,
            &format!(
                "{} secured elder",
                normalized_team(&killer_team).to_uppercase()
            ),
            "dragon",
        );
        return;
    }

    if key == "herald" {
        add_gold_xp_to_champion(
            runtime,
            &killer_id,
            OBJECTIVE_SECURE_GOLD + 20,
            OBJECTIVE_SECURE_XP + 30,
        );
        set_rift_herald_charge(runtime, &killer_team, &killer_id);
        log_event(
            runtime,
            &format!(
                "{} secured rift herald",
                normalized_team(&killer_team).to_uppercase()
            ),
            "info",
        );
        return;
    }

    if key == "voidgrubs" {
        // Voidgrubs stacks are awarded incrementally while damaging the camp.
        // At kill time we only grant completion rewards.
        add_gold_xp_to_champion(
            runtime,
            &killer_id,
            OBJECTIVE_SECURE_GOLD,
            OBJECTIVE_SECURE_XP,
        );
        log_event(
            runtime,
            &format!(
                "{} cleared voidgrub camp",
                normalized_team(&killer_team).to_uppercase()
            ),
            "info",
        );
        return;
    }

    if is_objective_neutral_key(key) {
        add_gold_xp_to_champion(
            runtime,
            &killer_id,
            OBJECTIVE_SECURE_GOLD,
            OBJECTIVE_SECURE_XP,
        );
        log_event(
            runtime,
            &format!(
                "{} secured {}",
                normalized_team(&killer_team).to_uppercase(),
                timer_label
            ),
            "info",
        );
    }
}

fn attack_neutral_if_in_range(
    runtime: &mut RuntimeState,
    neutral_timers: &mut NeutralTimersRuntime,
    champion_idx: usize,
    key: &str,
) -> bool {
    let Some(timer) = neutral_timers.entities.get(key) else {
        return false;
    };
    if !timer.alive {
        return false;
    }
    if champion_idx >= runtime.champions.len() || !runtime.champions[champion_idx].alive {
        return false;
    }
    if runtime.champions[champion_idx].role == "SUP"
        && runtime.time_sec >= SUPPORT_OPEN_ROAM_AT_SEC
        && is_jungle_camp_key(key)
    {
        return false;
    }

    let distance = dist(runtime.champions[champion_idx].pos, timer.pos);
    let max_range = if is_objective_neutral_key(key) {
        OBJECTIVE_ATTEMPT_RADIUS
    } else {
        JUNGLE_CAMP_ENGAGE_RADIUS
    };
    if distance > max_range {
        return false;
    }

    let damage = runtime.champions[champion_idx].attack_damage * 1.08;
    runtime.champions[champion_idx].attack_cd_until = runtime.time_sec + 0.78;

    let mut killed = false;
    let mut voidgrub_segments_gained: i64 = 0;
    if let Some(timer_mut) = neutral_timers.entities.get_mut(key) {
        let prev_hp = timer_mut.hp;
        timer_mut.hp -= damage;
        killed = timer_mut.hp <= 0.0;

        if key == "voidgrubs" {
            let prev_ratio = (prev_hp / timer_mut.max_hp).clamp(0.0, 1.0);
            let next_ratio = (timer_mut.hp.max(0.0) / timer_mut.max_hp).clamp(0.0, 1.0);
            let prev_segments_cleared = ((1.0 - prev_ratio) * 3.0).floor() as i64;
            let next_segments_cleared = ((1.0 - next_ratio) * 3.0).floor() as i64;
            voidgrub_segments_gained = (next_segments_cleared - prev_segments_cleared).max(0);
        }
    }

    if key == "voidgrubs" && voidgrub_segments_gained > 0 {
        let killer_team = runtime.champions[champion_idx].team.clone();
        let mut buffs = runtime_buffs_from_extra(runtime.extra.get("teamBuffs"));
        {
            let team_buffs = team_buffs_mut(&mut buffs, &killer_team);
            team_buffs.voidgrub_stacks =
                (team_buffs.voidgrub_stacks + voidgrub_segments_gained).clamp(0, 3);
        }
        set_runtime_buffs(runtime, &buffs);

        let killer_id = runtime.champions[champion_idx].id.clone();
        add_gold_xp_to_champion(
            runtime,
            &killer_id,
            (OBJECTIVE_SECURE_GOLD / 3) * voidgrub_segments_gained,
            (OBJECTIVE_SECURE_XP / 3) * voidgrub_segments_gained,
        );

        for _ in 0..voidgrub_segments_gained {
            log_event(
                runtime,
                &format!(
                    "{} secured voidgrub",
                    normalized_team(&killer_team).to_uppercase()
                ),
                "info",
            );
        }
    }
    if killed {
        mark_neutral_taken(runtime, neutral_timers, key, Some(champion_idx));
    }

    true
}

fn sync_objectives_from_neutral_timers(
    runtime: &mut RuntimeState,
    neutral_timers: &NeutralTimersRuntime,
) {
    let Some(objectives) = runtime.objectives.as_object_mut() else {
        return;
    };

    let buffs = runtime_buffs_from_extra(runtime.extra.get("teamBuffs"));

    if let Some(dragon_timer) = neutral_timers.entities.get("dragon") {
        if let Some(dragon_obj) = objectives.get_mut("dragon").and_then(Value::as_object_mut) {
            dragon_obj.insert("alive".to_string(), Value::from(dragon_timer.alive));
            dragon_obj.insert(
                "nextSpawnAt".to_string(),
                Value::from(
                    dragon_timer
                        .next_spawn_at
                        .unwrap_or(OBJECTIVE_NEXT_SPAWN_FALLBACK),
                ),
            );
            dragon_obj.insert(
                "currentKind".to_string(),
                Value::from(current_dragon_kind(neutral_timers)),
            );
            dragon_obj.insert(
                "firstKind".to_string(),
                neutral_timers
                    .extra
                    .get("dragonFirstKind")
                    .cloned()
                    .unwrap_or(Value::from("")),
            );
            dragon_obj.insert(
                "secondKind".to_string(),
                neutral_timers
                    .extra
                    .get("dragonSecondKind")
                    .cloned()
                    .unwrap_or(Value::from("")),
            );
            dragon_obj.insert(
                "soulRiftKind".to_string(),
                neutral_timers
                    .extra
                    .get("dragonSoulRiftKind")
                    .cloned()
                    .unwrap_or(Value::from("")),
            );
            dragon_obj.insert(
                "homeStacks".to_string(),
                Value::from(buffs.blue.dragon_stacks),
            );
            dragon_obj.insert(
                "awayStacks".to_string(),
                Value::from(buffs.red.dragon_stacks),
            );
            dragon_obj.insert(
                "soulClaimedBy".to_string(),
                if buffs.blue.soul_kind.is_some() {
                    Value::from("Home")
                } else if buffs.red.soul_kind.is_some() {
                    Value::from("Away")
                } else {
                    Value::Null
                },
            );
        }
    }

    if let Some(baron_timer) = neutral_timers.entities.get("baron") {
        if let Some(baron_obj) = objectives.get_mut("baron").and_then(Value::as_object_mut) {
            baron_obj.insert("alive".to_string(), Value::from(baron_timer.alive));
            baron_obj.insert(
                "nextSpawnAt".to_string(),
                Value::from(
                    baron_timer
                        .next_spawn_at
                        .unwrap_or(OBJECTIVE_NEXT_SPAWN_FALLBACK),
                ),
            );
        }
    }
}

fn tick_neutral_timers(runtime: &mut RuntimeState) {
    let mut neutral_timers = decode_neutral_timers_state(&runtime.neutral_timers)
        .unwrap_or_else(|| neutral_timers_default_runtime_state());
    let now = runtime.time_sec;

    ensure_dragon_cycle_defaults(runtime, &mut neutral_timers);

    let dragon_kind = current_dragon_kind(&neutral_timers);
    if let Some(dragon_timer) = neutral_timers.entities.get_mut("dragon") {
        dragon_timer
            .extra
            .insert("dragonCurrentKind".to_string(), Value::from(dragon_kind));
    }

    if neutral_timers.elder_unlocked {
        if let Some(elder) = neutral_timers.entities.get_mut("elder") {
            if !elder.unlocked {
                elder.unlocked = true;
                elder.next_spawn_at = Some(now + 6.0 * 60.0);
            }
        }
    }

    let mut keys: Vec<String> = neutral_timers.entities.keys().cloned().collect();
    keys.sort();

    for key in keys {
        let mut spawn_text: Option<String> = None;
        let mut despawn_text: Option<String> = None;

        if let Some(timer) = neutral_timers.entities.get_mut(&key) {
            let can_spawn = timer.unlocked
                && !timer.alive
                && timer.next_spawn_at.is_some()
                && now >= timer.next_spawn_at.unwrap_or(f64::INFINITY);
            if can_spawn {
                timer.alive = true;
                timer.hp = timer.max_hp;
                timer.last_spawn_at = timer.next_spawn_at;
                timer.times_spawned += 1;
                spawn_text = Some(format!("{} spawned", timer.label));
            }

            if timer.alive {
                if let Some(grace_until) = timer.combat_grace_until {
                    if now >= grace_until {
                        let had_remaining_hp = timer.hp > 0.0;
                        timer.alive = false;
                        timer.hp = 0.0;
                        timer.next_spawn_at = None;
                        despawn_text = Some(format!("{} despawned", timer.label));

                        if key == "voidgrubs" && had_remaining_hp {
                            let mut buffs =
                                runtime_buffs_from_extra(runtime.extra.get("teamBuffs"));
                            let total = (buffs.blue.voidgrub_stacks + buffs.red.voidgrub_stacks)
                                .clamp(0, 3);
                            let remaining = (3 - total).max(0);
                            if remaining > 0 {
                                let winner_team =
                                    if buffs.red.voidgrub_stacks > buffs.blue.voidgrub_stacks {
                                        "red"
                                    } else {
                                        "blue"
                                    };
                                let target = team_buffs_mut(&mut buffs, winner_team);
                                target.voidgrub_stacks =
                                    (target.voidgrub_stacks + remaining).clamp(0, 3);
                                set_runtime_buffs(runtime, &buffs);
                            }
                        }
                    }
                }
            }
        }

        if let Some(text) = spawn_text {
            log_event(runtime, &text, "spawn");
        }
        if let Some(text) = despawn_text {
            log_event(runtime, &text, "info");
        }
    }

    sync_objectives_from_neutral_timers(runtime, &neutral_timers);
    if let Ok(value) = serde_json::to_value(&neutral_timers) {
        runtime.neutral_timers = value;
    }
}

fn should_engage_enemy_champion(
    runtime: &RuntimeState,
    attacker_idx: usize,
    target_idx: usize,
) -> bool {
    if attacker_idx >= runtime.champions.len() || target_idx >= runtime.champions.len() {
        return false;
    }

    let attacker = &runtime.champions[attacker_idx];
    let target = &runtime.champions[target_idx];
    if !attacker.alive
        || !target.alive
        || normalized_team(&attacker.team) == normalized_team(&target.team)
    {
        return false;
    }

    let hp_ratio = if attacker.max_hp <= 0.0 {
        1.0
    } else {
        attacker.hp / attacker.max_hp
    };
    let enemy_hp_ratio = if target.max_hp <= 0.0 {
        1.0
    } else {
        target.hp / target.max_hp
    };

    let team_tactics = team_tactics_for_runtime(runtime.extra.get("teamTactics"), &attacker.team);
    let fight_plan = team_tactics.fight_plan.as_str();
    let risk_tolerance = stat_delta(attacker.competitive_score).clamp(-1.0, 1.0);
    let dynamic_retreat_hp_ratio =
        (runtime.policy.trade_retreat_hp_ratio - risk_tolerance * 0.05).clamp(0.24, 0.60);

    let ally_nearby = runtime
        .champions
        .iter()
        .filter(|champion| {
            champion.alive
                && normalized_team(&champion.team) == normalized_team(&attacker.team)
                && dist(champion.pos, target.pos) <= 0.12
        })
        .count();
    let enemy_nearby = runtime
        .champions
        .iter()
        .filter(|champion| {
            champion.alive
                && normalized_team(&champion.team) == normalized_team(&target.team)
                && dist(champion.pos, target.pos) <= 0.12
        })
        .count();

    if attacker.role == "JGL" {
        if hp_ratio <= 0.35 {
            return false;
        }
        if enemy_nearby > ally_nearby && hp_ratio < 0.75 {
            return false;
        }
    }

    let attacker_is_backline = attacker.attack_range >= 0.05;
    let attacker_is_frontline = !attacker_is_backline;

    if fight_plan == "FrontToBack" && attacker_is_backline && ally_nearby < enemy_nearby {
        return false;
    }

    if fight_plan == "Siege"
        && attacker.role != "JGL"
        && (enemy_hp_ratio > 0.45 || enemy_nearby > ally_nearby)
    {
        return false;
    }

    let target_under_defending_tower = runtime.structures.iter().any(|structure| {
        structure.alive
            && structure.kind == "tower"
            && normalized_team(&structure.team) == normalized_team(&target.team)
            && dist(structure.pos, target.pos) <= TOWER_AGGRO_VICTIM_RADIUS
            && dist(structure.pos, attacker.pos) <= TOWER_AGGRO_ATTACKER_RADIUS
    });

    let pick_force_open = fight_plan == "Pick"
        && (attacker.role == "MID" || attacker.role == "JGL" || attacker.role == "SUP")
        && enemy_nearby <= 1
        && hp_ratio + 0.06 >= dynamic_retreat_hp_ratio;
    let dive_force_open = fight_plan == "Dive"
        && attacker_is_frontline
        && target_under_defending_tower
        && enemy_hp_ratio <= 0.55
        && hp_ratio + 0.05 >= dynamic_retreat_hp_ratio;

    if attacker.role != "JGL" && hp_ratio <= dynamic_retreat_hp_ratio {
        return false;
    }

    if attacker.role != "JGL"
        && !pick_force_open
        && !dive_force_open
        && !can_open_trade_window(
            attacker,
            target,
            runtime.time_sec,
            &runtime.champions,
            &runtime.minions,
            &runtime.structures,
            &runtime.lane_combat_state_by_champion,
            runtime.ai_mode,
            &runtime.policy,
        )
    {
        return false;
    }

    if !pick_force_open
        && !dive_force_open
        && should_disengage_champion_trade(
            attacker,
            target,
            runtime.time_sec,
            &runtime.champions,
            &runtime.minions,
            &runtime.structures,
            runtime.ai_mode,
            &runtime.policy,
        )
    {
        return false;
    }

    can_champion_tower_dive(runtime, attacker, target)
}

fn can_champion_tower_dive(
    runtime: &RuntimeState,
    attacker: &ChampionRuntime,
    target: &ChampionRuntime,
) -> bool {
    let defending_tower = runtime.structures.iter().find(|structure| {
        structure.alive
            && structure.kind == "tower"
            && normalized_team(&structure.team) == normalized_team(&target.team)
            && dist(structure.pos, target.pos) <= TOWER_AGGRO_VICTIM_RADIUS
            && dist(structure.pos, attacker.pos) <= TOWER_AGGRO_ATTACKER_RADIUS
    });

    let Some(tower) = defending_tower else {
        return true;
    };

    let attacker_hp_ratio = if attacker.max_hp <= 0.0 {
        1.0
    } else {
        attacker.hp / attacker.max_hp
    };
    let attacker_is_backline = attacker.attack_range >= 0.05;
    let team_tactics = team_tactics_for_runtime(runtime.extra.get("teamTactics"), &attacker.team);
    let dive_plan = team_tactics.fight_plan == "Dive";
    let front_to_back_plan = team_tactics.fight_plan == "FrontToBack";
    let no_dive_hp_min = (runtime.policy.no_dive_hp_min
        + if dive_plan {
            -0.08
        } else if front_to_back_plan {
            0.04
        } else {
            0.0
        })
    .clamp(0.2, 0.95);
    let no_dive_hp_min = if attacker_is_backline {
        (no_dive_hp_min + 0.05).clamp(0.2, 0.95)
    } else {
        no_dive_hp_min
    };

    if attacker_hp_ratio < no_dive_hp_min {
        return false;
    }

    let allied_minions_near_tower = runtime
        .minions
        .iter()
        .filter(|minion| {
            minion.alive
                && normalized_team(&minion.team) == normalized_team(&attacker.team)
                && dist(minion.pos, tower.pos) <= 0.085
        })
        .count();

    let ally_nearby = runtime
        .champions
        .iter()
        .filter(|champion| {
            champion.alive
                && normalized_team(&champion.team) == normalized_team(&attacker.team)
                && dist(champion.pos, target.pos) <= 0.12
        })
        .count();
    let frontline_ally_nearby = runtime
        .champions
        .iter()
        .filter(|champion| {
            champion.alive
                && champion.id != attacker.id
                && normalized_team(&champion.team) == normalized_team(&attacker.team)
                && champion.attack_range < 0.05
                && dist(champion.pos, target.pos) <= 0.12
        })
        .count();
    let enemy_nearby = runtime
        .champions
        .iter()
        .filter(|champion| {
            champion.alive
                && normalized_team(&champion.team) == normalized_team(&target.team)
                && dist(champion.pos, target.pos) <= 0.12
        })
        .count();

    if front_to_back_plan && attacker_is_backline && frontline_ally_nearby == 0 {
        return false;
    }

    let min_hp_without_wave = if dive_plan { 0.58 } else { 0.65 };
    if allied_minions_near_tower == 0 && attacker_hp_ratio < min_hp_without_wave {
        return false;
    }

    let mut required_allies = if dive_plan {
        enemy_nearby.saturating_sub(1)
    } else {
        enemy_nearby
    };

    if attacker_is_backline {
        required_allies = required_allies.saturating_add(1);
    }

    ally_nearby >= required_allies
}

fn attack_enemy_champion(runtime: &mut RuntimeState, attacker_idx: usize, target_idx: usize) {
    if attacker_idx == target_idx
        || attacker_idx >= runtime.champions.len()
        || target_idx >= runtime.champions.len()
    {
        return;
    }

    let now = runtime.time_sec;
    let attacker_snapshot = runtime.champions[attacker_idx].clone();
    let target_snapshot = runtime.champions[target_idx].clone();
    mark_tower_aggro_on_champion_attack(runtime, &attacker_snapshot, &target_snapshot, now);

    let attacker_has_elder = team_has_active_elder_buff(runtime, &attacker_snapshot.team);
    let attacker_micro_mult = champion_micro_damage_multiplier(&attacker_snapshot);
    let defender_hp_ratio = if target_snapshot.max_hp <= 0.0 {
        1.0
    } else {
        target_snapshot.hp / target_snapshot.max_hp
    };
    let attack_damage_multiplier =
        team_damage_multiplier(runtime, &attacker_snapshot.team, defender_hp_ratio)
            * team_damage_reduction_multiplier(runtime, &target_snapshot.team);

    let mut kill_happened = false;
    let mut victim_pos = Vec2 { x: 0.5, y: 0.5 };
    let mut victim_name = String::new();
    let mut killer_id = String::new();
    let mut killer_name = String::new();
    let mut killer_team = String::new();

    if attacker_idx < target_idx {
        let (left, right) = runtime.champions.split_at_mut(target_idx);
        let attacker = &mut left[attacker_idx];
        let defender = &mut right[0];

        let outgoing = attacker.attack_damage * attack_damage_multiplier * attacker_micro_mult;
        defender.hp -= outgoing;
        defender.last_damaged_by_champion_id = Some(attacker.id.clone());
        defender.last_damaged_at = now;
        cancel_recall(defender, now, &mut runtime.events);
        attacker.attack_cd_until = now + CHAMPION_ATTACK_CADENCE_SEC;

        if attacker_has_elder
            && defender.max_hp > 0.0
            && defender.hp > 0.0
            && (defender.hp / defender.max_hp) <= ELDER_EXECUTE_HP_RATIO
        {
            defender.hp = 0.0;
        }

        if defender.hp <= 0.0 && defender.alive {
            defender.alive = false;
            defender.hp = 0.0;
            defender.deaths += 1;
            defender.respawn_at = now + champion_respawn_seconds(defender.level, now);
            attacker.kills += 1;
            kill_happened = true;
            victim_pos = defender.pos;
            victim_name = defender.name.clone();
            killer_id = attacker.id.clone();
            killer_name = attacker.name.clone();
            killer_team = attacker.team.clone();
        }
    } else {
        let (left, right) = runtime.champions.split_at_mut(attacker_idx);
        let defender = &mut left[target_idx];
        let attacker = &mut right[0];

        let outgoing = attacker.attack_damage * attack_damage_multiplier * attacker_micro_mult;
        defender.hp -= outgoing;
        defender.last_damaged_by_champion_id = Some(attacker.id.clone());
        defender.last_damaged_at = now;
        cancel_recall(defender, now, &mut runtime.events);
        attacker.attack_cd_until = now + CHAMPION_ATTACK_CADENCE_SEC;

        if attacker_has_elder
            && defender.max_hp > 0.0
            && defender.hp > 0.0
            && (defender.hp / defender.max_hp) <= ELDER_EXECUTE_HP_RATIO
        {
            defender.hp = 0.0;
        }

        if defender.hp <= 0.0 && defender.alive {
            defender.alive = false;
            defender.hp = 0.0;
            defender.deaths += 1;
            defender.respawn_at = now + champion_respawn_seconds(defender.level, now);
            attacker.kills += 1;
            kill_happened = true;
            victim_pos = defender.pos;
            victim_name = defender.name.clone();
            killer_id = attacker.id.clone();
            killer_name = attacker.name.clone();
            killer_team = attacker.team.clone();
        }
    }

    if attacker_idx < runtime.champions.len() {
        let attacker_after_hit = runtime.champions[attacker_idx].clone();
        mark_lane_trade_hit(
            &attacker_after_hit,
            now,
            &mut runtime.lane_combat_state_by_champion,
        );
    }

    if !kill_happened {
        return;
    }

    let (kill_gold, kill_xp) = champion_kill_rewards(&attacker_snapshot, &target_snapshot);

    let killer_team_stats = team_stats_mut(&mut runtime.stats, &killer_team);
    killer_team_stats.kills += 1;
    add_gold_xp_to_champion(runtime, &killer_id, kill_gold, kill_xp);

    let assisters: Vec<String> = runtime
        .champions
        .iter()
        .filter(|champion| {
            champion.alive
                && normalized_team(&champion.team) == normalized_team(&killer_team)
                && champion.id != killer_id
                && dist(champion.pos, victim_pos) <= ASSIST_RADIUS
        })
        .map(|champion| champion.id.clone())
        .collect();

    if !assisters.is_empty() {
        let shared_gold = CHAMPION_ASSIST_GOLD_TOTAL / assisters.len() as i64;
        let shared_xp = (kill_xp / 2) / assisters.len() as i64;
        for assist_id in assisters {
            if let Some(champion) = runtime
                .champions
                .iter_mut()
                .find(|champion| champion.id == assist_id)
            {
                champion.assists += 1;
            }
            add_gold_xp_to_champion(runtime, &assist_id, shared_gold, shared_xp);
        }
    }

    log_event(
        runtime,
        &format!("{} killed {}", killer_name, victim_name),
        "kill",
    );
}

fn mark_tower_aggro_on_champion_attack(
    runtime: &mut RuntimeState,
    attacker: &ChampionRuntime,
    victim: &ChampionRuntime,
    now: f64,
) {
    for tower in &mut runtime.structures {
        if !tower.alive
            || tower.kind != "tower"
            || normalized_team(&tower.team) != normalized_team(&victim.team)
        {
            continue;
        }
        if dist(tower.pos, victim.pos) > TOWER_AGGRO_VICTIM_RADIUS {
            continue;
        }
        if dist(tower.pos, attacker.pos) > TOWER_AGGRO_ATTACKER_RADIUS {
            continue;
        }

        tower.forced_target_champion_id = Some(attacker.id.clone());
        tower.forced_target_until = now + TOWER_AGGRO_LOCK_SEC;
    }
}

fn apply_tower_shot_to_champion(
    runtime: &mut RuntimeState,
    structure_idx: usize,
    champion_idx: usize,
) {
    let now = runtime.time_sec;
    runtime.champions[champion_idx].hp -= TOWER_SHOT_DAMAGE;
    runtime.champions[champion_idx].last_damaged_by_champion_id = None;
    runtime.champions[champion_idx].last_damaged_at = now;
    cancel_recall(
        &mut runtime.champions[champion_idx],
        now,
        &mut runtime.events,
    );
    runtime.structures[structure_idx].attack_cd_until = now + TOWER_ATTACK_CADENCE_SEC;
    if runtime.champions[champion_idx].hp <= 0.0 && runtime.champions[champion_idx].alive {
        runtime.champions[champion_idx].alive = false;
        runtime.champions[champion_idx].hp = 0.0;
        runtime.champions[champion_idx].deaths += 1;
        let respawn = champion_respawn_seconds(runtime.champions[champion_idx].level, now);
        runtime.champions[champion_idx].respawn_at = now + respawn;
    }
}

fn champion_level_from_xp(xp: i64) -> i64 {
    let mut level = 1_i64;
    for (idx, threshold) in LEVEL_XP_THRESHOLDS.iter().enumerate() {
        if xp >= *threshold {
            level = (idx + 1) as i64;
        } else {
            break;
        }
    }
    level.clamp(1, CHAMPION_MAX_LEVEL)
}

fn apply_level_scaling(champion: &mut ChampionRuntime) {
    let target_level = champion_level_from_xp(champion.xp);
    if target_level <= champion.level {
        return;
    }

    let level_delta = target_level - champion.level;
    champion.max_hp += CHAMPION_LEVEL_UP_HP_GAIN * level_delta as f64;
    champion.attack_damage += CHAMPION_LEVEL_UP_AD_GAIN * level_delta as f64;
    champion.hp =
        (champion.hp + CHAMPION_LEVEL_UP_HP_GAIN * level_delta as f64).min(champion.max_hp);
    champion.level = target_level;
}

fn champion_respawn_seconds(level: i64, now_sec: f64) -> f64 {
    let time_factor = if now_sec >= 30.0 * 60.0 {
        1.15
    } else if now_sec >= 20.0 * 60.0 {
        1.08
    } else {
        1.0
    };
    ((CHAMPION_RESPAWN_BASE_SEC + (level.max(1) - 1) as f64 * CHAMPION_RESPAWN_PER_LEVEL_SEC)
        * time_factor)
        .clamp(10.0, 42.0)
}

fn champion_kill_rewards(killer: &ChampionRuntime, victim: &ChampionRuntime) -> (i64, i64) {
    let level_gap = victim.level - killer.level;
    let victim_streak = (victim.kills as i64 - victim.deaths as i64).max(0);
    let killer_ahead = (killer.kills as i64 - killer.deaths as i64).max(0);

    let mut gold = CHAMPION_KILL_GOLD + level_gap * 18 + victim_streak * 35;
    if killer_ahead >= 4 {
        gold -= ((killer_ahead - 3) * 24).min(150);
    }

    let mut xp = CHAMPION_KILL_XP + level_gap * 12 + victim_streak * 10;
    if killer_ahead >= 4 {
        xp -= ((killer_ahead - 3) * 10).min(60);
    }

    (
        gold.clamp(CHAMPION_KILL_GOLD_MIN, CHAMPION_KILL_GOLD_MAX),
        xp.clamp(CHAMPION_KILL_XP_MIN, CHAMPION_KILL_XP_MAX),
    )
}

fn team_has_alive_nexus_towers(structures: &[StructureRuntime], team: &str) -> bool {
    structures.iter().any(|structure| {
        structure.alive
            && normalized_team(&structure.team) == normalized_team(team)
            && structure.kind == "tower"
            && structure.id.contains("nexus")
    })
}

fn lane_tag_from_structure_id(id: &str) -> Option<&'static str> {
    if id.contains("-top") {
        Some("top")
    } else if id.contains("-mid") {
        Some("mid")
    } else if id.contains("-bot") {
        Some("bot")
    } else {
        None
    }
}

fn inhib_tower_alive_for_lane(
    structures: &[StructureRuntime],
    defending_team: &str,
    lane: &str,
) -> bool {
    structures.iter().any(|candidate| {
        candidate.alive
            && candidate.kind == "tower"
            && normalized_team(&candidate.team) == normalized_team(defending_team)
            && candidate.id.contains("inhib-tower")
            && candidate.id.contains(lane)
    })
}

fn weakest_enemy_lane_for_team(
    structures: &[StructureRuntime],
    team: &str,
) -> Option<&'static str> {
    let enemy = if normalized_team(team) == "blue" {
        "red"
    } else {
        "blue"
    };
    let lane_count = |lane: &str| -> usize {
        structures
            .iter()
            .filter(|structure| {
                structure.alive
                    && structure.kind == "tower"
                    && normalized_team(&structure.team) == enemy
                    && normalized_lane(&structure.lane) == lane
            })
            .count()
    };

    let top = lane_count("top");
    let mid = lane_count("mid");
    let bot = lane_count("bot");

    if top <= mid && top <= bot {
        Some("top")
    } else if mid <= top && mid <= bot {
        Some("mid")
    } else {
        Some("bot")
    }
}

fn add_dragon_stack_for_kind(team_buffs: &mut RuntimeTeamBuffState, kind: &str) {
    match kind {
        "infernal" => team_buffs.infernal_stacks += 1,
        "mountain" => team_buffs.mountain_stacks += 1,
        "ocean" => team_buffs.ocean_stacks += 1,
        "cloud" => team_buffs.cloud_stacks += 1,
        "hextech" => team_buffs.hextech_stacks += 1,
        "chemtech" => team_buffs.chemtech_stacks += 1,
        _ => {}
    }
    team_buffs.dragon_stacks += 1;
}

fn process_dragon_capture(
    runtime: &mut RuntimeState,
    neutral_timers: &mut NeutralTimersRuntime,
    killer_team: &str,
) -> String {
    ensure_dragon_cycle_defaults(runtime, neutral_timers);
    let dragon_kind = current_dragon_kind(neutral_timers);

    let mut buffs = runtime_buffs_from_extra(runtime.extra.get("teamBuffs"));
    {
        let team_buffs = team_buffs_mut(&mut buffs, killer_team);
        add_dragon_stack_for_kind(team_buffs, &dragon_kind);
        if team_buffs.dragon_history.len() >= 8 {
            team_buffs.dragon_history.remove(0);
        }
        team_buffs.dragon_history.push(dragon_kind.clone());
    }

    let total_dragons = buffs.blue.dragon_stacks + buffs.red.dragon_stacks;

    if total_dragons == 1 {
        neutral_timers.extra.insert(
            "dragonFirstKind".to_string(),
            Value::from(dragon_kind.as_str()),
        );
        let second_kind = choose_different_dragon_kind(
            &dragon_kind,
            runtime.time_sec as i64 + runtime.events.len() as i64,
        );
        set_current_dragon_kind(neutral_timers, second_kind);
    } else if total_dragons == 2 {
        let first_kind = neutral_timers
            .extra
            .get("dragonFirstKind")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("")
            .to_string();
        neutral_timers.extra.insert(
            "dragonSecondKind".to_string(),
            Value::from(dragon_kind.as_str()),
        );
        let rift_kind = choose_dragon_kind_excluding(
            &[first_kind.as_str(), dragon_kind.as_str()],
            runtime.time_sec as i64 + runtime.events.len() as i64 + 37,
        );
        neutral_timers
            .extra
            .insert("dragonSoulRiftKind".to_string(), Value::from(rift_kind));
        set_current_dragon_kind(neutral_timers, rift_kind);
    }

    let soul_rift_kind = neutral_timers
        .extra
        .get("dragonSoulRiftKind")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(dragon_kind.as_str())
        .to_string();

    let team_dragons = team_buffs_ref(&buffs, killer_team).dragon_stacks;
    let soul_missing = team_buffs_ref(&buffs, killer_team).soul_kind.is_none();

    if team_dragons >= 4 && soul_missing {
        team_buffs_mut(&mut buffs, killer_team).soul_kind = Some(soul_rift_kind.clone());
        neutral_timers.dragon_soul_unlocked = true;
        neutral_timers.elder_unlocked = true;

        if let Some(dragon) = neutral_timers.entities.get_mut("dragon") {
            dragon.alive = false;
            dragon.hp = 0.0;
            dragon.unlocked = false;
            dragon.next_spawn_at = None;
        }
        if let Some(elder) = neutral_timers.entities.get_mut("elder") {
            elder.unlocked = true;
            elder.next_spawn_at = Some(runtime.time_sec + 6.0 * 60.0);
        }
    } else if total_dragons != 1 {
        set_current_dragon_kind(neutral_timers, &soul_rift_kind);
    }

    set_runtime_buffs(runtime, &buffs);
    dragon_kind
}

fn baron_push_target_for_lane(
    structures: &[StructureRuntime],
    team: &str,
    lane: &str,
) -> Option<Vec2> {
    let enemy = if normalized_team(team) == "blue" {
        "red"
    } else {
        "blue"
    };
    let lane_tower = structures
        .iter()
        .filter(|structure| {
            structure.alive
                && structure.kind == "tower"
                && normalized_team(&structure.team) == enemy
                && normalized_lane(&structure.lane) == lane
        })
        .min_by(|a, b| a.id.cmp(&b.id));

    if let Some(tower) = lane_tower {
        return Some(tower.pos);
    }

    structures
        .iter()
        .find(|structure| {
            structure.alive
                && normalized_team(&structure.team) == enemy
                && structure.kind == "nexus"
        })
        .map(|nexus| nexus.pos)
}

fn structure_alive_by_id(structures: &[StructureRuntime], id: &str) -> bool {
    structures
        .iter()
        .any(|structure| structure.alive && structure.id == id)
}

fn prerequisite_tower_alive(structures: &[StructureRuntime], structure_id: &str) -> Option<bool> {
    if structure_id.contains("-inner") {
        let prerequisite = structure_id.replace("-inner", "-outer");
        return Some(structure_alive_by_id(structures, &prerequisite));
    }
    if structure_id.contains("-inhib-tower") {
        let prerequisite = structure_id.replace("-inhib-tower", "-inner");
        return Some(structure_alive_by_id(structures, &prerequisite));
    }
    if structure_id.contains("-nexus-top-tower") {
        let prerequisite = structure_id.replace("-nexus-top-tower", "-inhib-top");
        return Some(structure_alive_by_id(structures, &prerequisite));
    }
    if structure_id.contains("-nexus-bot-tower") {
        let prerequisite = structure_id.replace("-nexus-bot-tower", "-inhib-bot");
        return Some(structure_alive_by_id(structures, &prerequisite));
    }
    None
}

fn is_structure_targetable(
    structures: &[StructureRuntime],
    attacker_team: &str,
    structure: &StructureRuntime,
) -> bool {
    if !structure.alive || normalized_team(&structure.team) == normalized_team(attacker_team) {
        return false;
    }

    if structure.kind == "nexus" {
        return !team_has_alive_nexus_towers(structures, &structure.team);
    }

    if structure.kind == "tower" {
        if let Some(prereq_alive) = prerequisite_tower_alive(structures, &structure.id) {
            if prereq_alive {
                return false;
            }
        }
    }

    if structure.kind == "inhib" {
        if let Some(lane) = lane_tag_from_structure_id(&structure.id) {
            return !inhib_tower_alive_for_lane(structures, &structure.team, lane);
        }
    }

    true
}

fn tower_damage_multiplier(at_time_sec: f64, structure: &StructureRuntime) -> f64 {
    if structure.kind == "tower" && at_time_sec < EARLY_TOWER_FORTIFICATION_END_AT {
        1.0 - EARLY_TOWER_DAMAGE_REDUCTION
    } else {
        1.0
    }
}

fn apply_damage_to_structure(
    runtime: &mut RuntimeState,
    structure_idx: usize,
    raw_damage: f64,
    attacker_team: &str,
) {
    if structure_idx >= runtime.structures.len() {
        return;
    }
    if !is_structure_targetable(
        &runtime.structures,
        attacker_team,
        &runtime.structures[structure_idx],
    ) {
        return;
    }

    let multiplier = tower_damage_multiplier(runtime.time_sec, &runtime.structures[structure_idx]);
    let mut damage = raw_damage.max(0.0) * multiplier;
    if runtime.structures[structure_idx].kind == "tower" {
        let buffs = team_buffs_for_runtime(runtime.extra.get("teamBuffs"), attacker_team);
        let voidgrub_bonus = (buffs.voidgrub_stacks as f64 * VOIDGRUB_TOWER_DAMAGE_PER_STACK)
            .min(VOIDGRUB_TOWER_DAMAGE_MAX)
            .max(0.0);
        damage *= 1.0 + voidgrub_bonus;
    }
    if damage <= 0.0 {
        return;
    }

    runtime.structures[structure_idx].hp -= damage;
    if runtime.structures[structure_idx].hp <= 0.0 {
        destroy_structure(runtime, structure_idx, attacker_team);
    }
}

fn add_gold_xp_to_champion(runtime: &mut RuntimeState, champion_id: &str, gold: i64, xp: i64) {
    if let Some(champion) = runtime
        .champions
        .iter_mut()
        .find(|champion| champion.id == champion_id)
    {
        champion.gold += gold;
        champion.xp += xp;
        apply_level_scaling(champion);
        let team_stats = team_stats_mut(&mut runtime.stats, &champion.team);
        team_stats.gold += gold;
    }
}

fn add_cs_to_champion(runtime: &mut RuntimeState, champion_id: &str, cs: i64) {
    if cs <= 0 {
        return;
    }
    if let Some(champion) = runtime
        .champions
        .iter_mut()
        .find(|champion| champion.id == champion_id)
    {
        if champion.role == "SUP" && runtime.time_sec >= SUPPORT_OPEN_ROAM_AT_SEC {
            return;
        }
        champion.cs += cs;
    }
}

fn register_minion_death(runtime: &mut RuntimeState, minion_idx: usize) {
    if !runtime.minions[minion_idx].alive {
        return;
    }

    runtime.minions[minion_idx].alive = false;
    if runtime.minions[minion_idx].kind == "summon" {
        return;
    }
    let last_hit = runtime.minions[minion_idx].last_hit_by_champion_id.clone();
    let gold = if runtime.minions[minion_idx].kind == "ranged" {
        16
    } else {
        22
    };
    let xp = if runtime.minions[minion_idx].kind == "ranged" {
        32
    } else {
        58
    };

    if let Some(champion_id) = last_hit {
        let now = runtime.time_sec;
        if let Some(champion) = runtime
            .champions
            .iter_mut()
            .find(|champion| champion.id == champion_id)
        {
            if champion.role == "SUP"
                && (now - champion.last_support_cs_at) < SUPPORT_CS_MIN_INTERVAL_SEC
            {
                return;
            }
            champion.gold += gold;
            champion.xp += xp;
            champion.cs += 1;
            if champion.role == "SUP" {
                champion.last_support_cs_at = now;
            }
            apply_level_scaling(champion);
            let team_stats = team_stats_mut(&mut runtime.stats, &champion.team);
            team_stats.gold += gold;
        }
    }
}

fn destroy_structure(runtime: &mut RuntimeState, structure_idx: usize, attacker_team: &str) {
    if !runtime.structures[structure_idx].alive {
        return;
    }

    runtime.structures[structure_idx].alive = false;
    runtime.structures[structure_idx].hp = 0.0;

    if runtime.structures[structure_idx].kind == "tower" {
        let team_stats = team_stats_mut(&mut runtime.stats, attacker_team);
        team_stats.towers += 1;
    }

    let event_type = if runtime.structures[structure_idx].kind == "nexus" {
        runtime.winner = Some(normalized_team(attacker_team).to_string());
        runtime.running = false;
        "nexus"
    } else {
        "tower"
    };

    log_event(
        runtime,
        &format!(
            "{} destroyed {}",
            normalized_team(attacker_team).to_uppercase(),
            runtime.structures[structure_idx].id
        ),
        event_type,
    );
}

fn nearest_enemy_minion_for_champion(
    minions: &[MinionRuntime],
    team: &str,
    lane: &str,
    from: Vec2,
    range: f64,
) -> Option<usize> {
    minions
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            m.alive
                && normalized_team(&m.team) != normalized_team(team)
                && normalized_lane(&m.lane) == normalized_lane(lane)
                && dist(m.pos, from) <= range
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            dist(a.pos, from)
                .partial_cmp(&dist(b.pos, from))
                .unwrap_or(Ordering::Equal)
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx)
}

fn nearest_enemy_champion_for_champion(
    champions: &[ChampionRuntime],
    attacker: &ChampionRuntime,
    range: f64,
) -> Option<usize> {
    champions
        .iter()
        .enumerate()
        .filter(|(_, enemy)| {
            enemy.alive
                && !champion_is_banished(enemy)
                && enemy.id != attacker.id
                && normalized_team(&enemy.team) != normalized_team(&attacker.team)
                && dist(enemy.pos, attacker.pos) <= range
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            let lane_penalty_a = if normalized_lane(&a.lane) == normalized_lane(&attacker.lane) {
                0
            } else {
                1
            };
            let lane_penalty_b = if normalized_lane(&b.lane) == normalized_lane(&attacker.lane) {
                0
            } else {
                1
            };

            lane_penalty_a
                .cmp(&lane_penalty_b)
                .then_with(|| {
                    dist(a.pos, attacker.pos)
                        .partial_cmp(&dist(b.pos, attacker.pos))
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| a.hp.partial_cmp(&b.hp).unwrap_or(Ordering::Equal))
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx)
}

fn nearest_enemy_champion_for_minion(
    champions: &[ChampionRuntime],
    attacker_team: &str,
    attacker_lane: &str,
    attacker_kind: &str,
    from: Vec2,
    range: f64,
) -> Option<usize> {
    champions
        .iter()
        .enumerate()
        .filter(|(_, enemy)| {
            enemy.alive
                && !champion_is_banished(enemy)
                && normalized_team(&enemy.team) != normalized_team(attacker_team)
                && (attacker_kind == "summon"
                    || normalized_lane(&enemy.lane) == normalized_lane(attacker_lane))
                && dist(enemy.pos, from) <= range
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            a.hp.partial_cmp(&b.hp)
                .unwrap_or(Ordering::Equal)
                .then_with(|| {
                    dist(a.pos, from)
                        .partial_cmp(&dist(b.pos, from))
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx)
}

fn nearest_enemy_minion_index(
    minions: &[MinionRuntime],
    source_idx: usize,
    range: f64,
) -> Option<usize> {
    let source = &minions[source_idx];
    minions
        .iter()
        .enumerate()
        .filter(|(idx, candidate)| {
            *idx != source_idx
                && candidate.alive
                && normalized_team(&candidate.team) != normalized_team(&source.team)
                && normalized_lane(&candidate.lane) == normalized_lane(&source.lane)
                && dist(candidate.pos, source.pos) <= range
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            dist(a.pos, source.pos)
                .partial_cmp(&dist(b.pos, source.pos))
                .unwrap_or(Ordering::Equal)
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx)
}

fn nearest_enemy_structure_index(
    structures: &[StructureRuntime],
    team: &str,
    lane: &str,
    from: Vec2,
    range: f64,
) -> Option<usize> {
    structures
        .iter()
        .enumerate()
        .filter(|(_, structure)| {
            structure.alive
                && normalized_team(&structure.team) != normalized_team(team)
                && (normalized_lane(&structure.lane) == normalized_lane(lane)
                    || structure.lane == "base")
                && is_structure_targetable(structures, team, structure)
                && dist(structure.pos, from) <= range
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            dist(a.pos, from)
                .partial_cmp(&dist(b.pos, from))
                .unwrap_or(Ordering::Equal)
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx)
}

fn nearest_enemy_minion_for_structure(
    minions: &[MinionRuntime],
    structure_team: &str,
    from: Vec2,
    range: f64,
) -> Option<usize> {
    minions
        .iter()
        .enumerate()
        .filter(|(_, minion)| {
            minion.alive
                && normalized_team(&minion.team) != normalized_team(structure_team)
                && dist(minion.pos, from) <= range
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            dist(a.pos, from)
                .partial_cmp(&dist(b.pos, from))
                .unwrap_or(Ordering::Equal)
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx)
}

fn nearest_enemy_champion_for_structure(
    champions: &[ChampionRuntime],
    structure_team: &str,
    from: Vec2,
    range: f64,
) -> Option<usize> {
    champions
        .iter()
        .enumerate()
        .filter(|(_, champion)| {
            champion.alive
                && !champion_is_banished(champion)
                && normalized_team(&champion.team) != normalized_team(structure_team)
                && dist(champion.pos, from) <= range
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            dist(a.pos, from)
                .partial_cmp(&dist(b.pos, from))
                .unwrap_or(Ordering::Equal)
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx)
}

fn normalize_champion_key(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn category_plan(category: ItemBuildCategory) -> &'static [ItemTemplate; 6] {
    match category {
        ItemBuildCategory::Tank => &TANK_ITEM_PLAN,
        ItemBuildCategory::Bruiser => &BRUISER_ITEM_PLAN,
        ItemBuildCategory::Colossus => &COLOSSUS_ITEM_PLAN,
        ItemBuildCategory::AssassinAd => &ASSASSIN_AD_ITEM_PLAN,
        ItemBuildCategory::AssassinAp => &ASSASSIN_AP_ITEM_PLAN,
        ItemBuildCategory::ControlMage => &CONTROL_MAGE_ITEM_PLAN,
        ItemBuildCategory::BattleMage => &BATTLE_MAGE_ITEM_PLAN,
        ItemBuildCategory::AdcCrit => &ADC_CRIT_ITEM_PLAN,
        ItemBuildCategory::AdcAttackSpeed => &ADC_ATTACK_SPEED_ITEM_PLAN,
        ItemBuildCategory::LethalityMarksman => &LETHALITY_MARKSMAN_ITEM_PLAN,
        ItemBuildCategory::SupportEngage => &SUPPORT_ENGAGE_ITEM_PLAN,
        ItemBuildCategory::SupportEnchanter => &SUPPORT_ENCHANTER_ITEM_PLAN,
        ItemBuildCategory::SupportDamage => &SUPPORT_DAMAGE_ITEM_PLAN,
    }
}

fn classify_item_build(role: &str, champion_id: &str) -> ItemBuildCategory {
    let champion = normalize_champion_key(champion_id);
    let c = champion.as_str();

    if role == "SUP" {
        if matches!(c, "brand" | "velkoz" | "zyra" | "xerath" | "lux") {
            return ItemBuildCategory::SupportDamage;
        }
        if matches!(
            c,
            "bard"
                | "ivern"
                | "janna"
                | "karma"
                | "lulu"
                | "milio"
                | "morgana"
                | "nami"
                | "renataglasc"
                | "seraphine"
                | "sona"
                | "soraka"
                | "yuumi"
        ) {
            return ItemBuildCategory::SupportEnchanter;
        }
        if matches!(
            c,
            "alistar"
                | "blitzcrank"
                | "braum"
                | "leona"
                | "nautilus"
                | "pyke"
                | "rakan"
                | "rell"
                | "thresh"
        ) {
            return ItemBuildCategory::SupportEngage;
        }
    }

    if role == "ADC" {
        if matches!(
            c,
            "kaisa" | "kalista" | "kogmaw" | "masteryi" | "twitch" | "varus" | "vayne" | "yunara"
        ) {
            return ItemBuildCategory::AdcAttackSpeed;
        }
        if matches!(
            c,
            "graves" | "jhin" | "kindred" | "missfortune" | "quinn" | "senna" | "smolder"
        ) {
            return ItemBuildCategory::LethalityMarksman;
        }
    }

    if matches!(
        c,
        "alistar"
            | "amumu"
            | "braum"
            | "chogath"
            | "galio"
            | "ksante"
            | "leona"
            | "malphite"
            | "maokai"
            | "nautilus"
            | "ornn"
            | "poppy"
            | "rammus"
            | "rell"
            | "sejuani"
            | "shen"
            | "sion"
            | "tahmkench"
            | "taric"
            | "zac"
    ) {
        return ItemBuildCategory::Tank;
    }

    if matches!(
        c,
        "darius"
            | "drmundo"
            | "garen"
            | "illaoi"
            | "mordekaiser"
            | "nasus"
            | "sett"
            | "shyvana"
            | "trundle"
            | "udyr"
            | "urgot"
            | "yorick"
    ) {
        return ItemBuildCategory::Colossus;
    }

    if matches!(
        c,
        "akshan"
            | "khazix"
            | "naafiri"
            | "nocturne"
            | "pyke"
            | "qiyana"
            | "rengar"
            | "shaco"
            | "talon"
            | "zed"
            | "kayn"
    ) {
        return ItemBuildCategory::AssassinAd;
    }

    if matches!(
        c,
        "akali" | "ekko" | "evelynn" | "fizz" | "kassadin" | "katarina" | "leblanc" | "nidalee"
    ) {
        return ItemBuildCategory::AssassinAp;
    }

    if matches!(
        c,
        "anivia"
            | "aurelionsol"
            | "azir"
            | "heimerdinger"
            | "hwei"
            | "lissandra"
            | "lux"
            | "malzahar"
            | "mel"
            | "neeko"
            | "orianna"
            | "ryze"
            | "syndra"
            | "taliyah"
            | "vex"
            | "viktor"
            | "xerath"
            | "ziggs"
            | "zoe"
    ) {
        return ItemBuildCategory::ControlMage;
    }

    if matches!(
        c,
        "cassiopeia"
            | "karthus"
            | "vladimir"
            | "swain"
            | "rumble"
            | "singed"
            | "sylas"
            | "gwen"
            | "lillia"
            | "morgana"
    ) {
        return ItemBuildCategory::BattleMage;
    }

    if matches!(
        c,
        "aatrox"
            | "ambessa"
            | "briar"
            | "camille"
            | "diana"
            | "ekko"
            | "elise"
            | "fiora"
            | "gnar"
            | "hecarim"
            | "irelia"
            | "jarvaniv"
            | "jax"
            | "kled"
            | "leesin"
            | "olaf"
            | "pantheon"
            | "reksai"
            | "renekton"
            | "riven"
            | "skarner"
            | "vi"
            | "volibear"
            | "warwick"
            | "wukong"
            | "xinzhao"
            | "yasuo"
            | "yone"
            | "belveth"
            | "zaahen"
    ) {
        return ItemBuildCategory::Bruiser;
    }

    if matches!(
        c,
        "aphelios"
            | "ashe"
            | "caitlyn"
            | "draven"
            | "jinx"
            | "lucian"
            | "nilah"
            | "samira"
            | "sivir"
            | "tristana"
            | "xayah"
            | "tryndamere"
    ) {
        return ItemBuildCategory::AdcCrit;
    }

    if matches!(
        c,
        "graves" | "jhin" | "kindred" | "missfortune" | "quinn" | "senna" | "smolder"
    ) {
        return ItemBuildCategory::LethalityMarksman;
    }

    match role {
        "TOP" | "JGL" => ItemBuildCategory::Bruiser,
        "MID" => ItemBuildCategory::ControlMage,
        "ADC" => ItemBuildCategory::AdcCrit,
        "SUP" => ItemBuildCategory::SupportEnchanter,
        _ => ItemBuildCategory::Bruiser,
    }
}

fn champion_item_plan(role: &str, champion_id: &str) -> &'static [ItemTemplate; 6] {
    category_plan(classify_item_build(role, champion_id))
}

fn effective_item_cost(base_cost: i64) -> i64 {
    ((base_cost as f64) * ITEM_COST_MULTIPLIER)
        .round()
        .max(ITEM_COST_MIN as f64) as i64
}

fn is_boots_item_key(key: &str) -> bool {
    matches!(
        key,
        "plated_steelcaps"
            | "mercurys_treads"
            | "boots_of_swiftness"
            | "sorcerers_shoes"
            | "berserkers_greaves"
            | "ionian_boots_of_lucidity"
            | "mobility_boots"
    )
}

fn try_auto_buy_items(runtime: &mut RuntimeState) {
    for idx in 0..runtime.champions.len() {
        {
            let champion = &mut runtime.champions[idx];
            let base_pos = base_position_for(&champion.team);
            if dist(champion.pos, base_pos) > 0.12 {
                champion.has_left_base_once = true;
            }
        }

        let (
            alive,
            role,
            champion_id,
            at_base,
            item_count,
            gold,
            name,
            owned_items,
            has_left_base_once,
        ) = {
            let champion = &runtime.champions[idx];
            (
                champion.alive,
                champion.role.clone(),
                champion.champion_id.clone(),
                dist(champion.pos, base_position_for(&champion.team)) <= 0.075,
                champion.items.len(),
                champion.gold,
                champion.name.clone(),
                champion.items.clone(),
                champion.has_left_base_once,
            )
        };

        if !alive || !at_base || item_count >= 6 || !has_left_base_once {
            continue;
        }

        let plan = champion_item_plan(&role, &champion_id);
        let has_boots = owned_items.iter().any(|item| is_boots_item_key(item));

        let next_item = if !has_boots {
            plan.iter()
                .find(|candidate| is_boots_item_key(candidate.key))
        } else {
            plan.iter()
                .find(|candidate| !owned_items.iter().any(|owned| owned == candidate.key))
        };

        let Some(next_item) = next_item else {
            continue;
        };

        let buy_cost = effective_item_cost(next_item.cost);

        if gold < buy_cost {
            continue;
        }

        let champion = &mut runtime.champions[idx];
        champion.gold -= buy_cost;
        champion.spent_gold += buy_cost;
        champion.items.push(next_item.key.to_string());
        champion.attack_damage += next_item.attack_damage;
        if next_item.max_hp > 0.0 {
            champion.max_hp += next_item.max_hp;
            champion.hp = (champion.hp + next_item.max_hp).min(champion.max_hp);
        }

        log_event(
            runtime,
            &format!("{} bought {}", name, next_item.key),
            "info",
        );
    }
}

fn push_event(events: &mut Vec<RuntimeEvent>, at: f64, text: &str, kind: &str) {
    events.push(RuntimeEvent {
        t: at,
        text: text.to_string(),
        kind: kind.to_string(),
    });

    if events.len() > EVENT_CAP {
        let drain = events.len() - EVENT_CAP;
        events.drain(0..drain);
    }
}

fn log_event(runtime: &mut RuntimeState, text: &str, kind: &str) {
    if runtime_is_skip_fast_mode(runtime)
        && !matches!(kind, "kill" | "tower" | "dragon" | "baron" | "nexus")
    {
        return;
    }
    push_event(&mut runtime.events, runtime.time_sec, text, kind);
}

fn runtime_is_skip_fast_mode(runtime: &RuntimeState) -> bool {
    runtime
        .extra
        .get(SKIP_FAST_MODE_EXTRA_KEY)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn team_stats_mut<'a>(stats: &'a mut RuntimeStats, team: &str) -> &'a mut RuntimeTeamStats {
    if normalized_team(team) == "red" {
        &mut stats.red
    } else {
        &mut stats.blue
    }
}

fn team_has_active_baron_buff(runtime: &RuntimeState, team: &str) -> bool {
    let buffs = team_buffs_for_runtime(runtime.extra.get("teamBuffs"), team);
    buffs.baron_until > runtime.time_sec
}

fn team_has_active_elder_buff(runtime: &RuntimeState, team: &str) -> bool {
    let buffs = team_buffs_for_runtime(runtime.extra.get("teamBuffs"), team);
    buffs.elder_until > runtime.time_sec
}

fn team_damage_multiplier(runtime: &RuntimeState, team: &str, target_hp_ratio: f64) -> f64 {
    let buffs = team_buffs_for_runtime(runtime.extra.get("teamBuffs"), team);
    let mut mult = 1.0 + buffs.infernal_stacks as f64 * 0.02;
    mult += buffs.hextech_stacks as f64 * 0.01;
    if target_hp_ratio <= 0.5 {
        mult += buffs.chemtech_stacks as f64 * 0.012;
    }
    if let Some(soul) = buffs.soul_kind.as_deref() {
        match soul {
            "infernal" => mult += 0.08,
            "hextech" => mult += 0.05,
            "chemtech" if target_hp_ratio <= 0.5 => mult += 0.06,
            _ => {}
        }
    }
    mult
}

fn team_damage_reduction_multiplier(runtime: &RuntimeState, team: &str) -> f64 {
    let buffs = team_buffs_for_runtime(runtime.extra.get("teamBuffs"), team);
    let mut reduction = (buffs.mountain_stacks as f64 * 0.02).min(0.12);
    if buffs.soul_kind.as_deref() == Some("mountain") {
        reduction += 0.08;
    }
    (1.0 - reduction).clamp(0.72, 1.0)
}

fn minion_is_baron_empowered(runtime: &RuntimeState, minion: &MinionRuntime) -> bool {
    if !team_has_active_baron_buff(runtime, &minion.team) {
        return false;
    }
    runtime.champions.iter().any(|champion| {
        champion.alive
            && normalized_team(&champion.team) == normalized_team(&minion.team)
            && dist(champion.pos, minion.pos) <= BARON_MINION_AURA_RADIUS
    })
}

fn cleanup_tick(runtime: &mut RuntimeState) {
    runtime
        .minions
        .retain(|minion| minion.alive && minion.path_index < minion.path.len());
    runtime
        .wards
        .retain(|ward| ward.expires_at > runtime.time_sec);

    try_auto_buy_items(runtime);

    if runtime.events.len() > EVENT_CAP {
        let drain = runtime.events.len() - EVENT_CAP;
        runtime.events.drain(0..drain);
    }
}

fn dist(a: Vec2, b: Vec2) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

fn normalize(v: Vec2) -> Vec2 {
    let len = (v.x * v.x + v.y * v.y).sqrt();
    if len <= 1e-9 {
        Vec2 { x: 0.0, y: 0.0 }
    } else {
        Vec2 {
            x: v.x / len,
            y: v.y / len,
        }
    }
}

fn move_entity(pos: &mut Vec2, target: Vec2, speed: f64, dt: f64) {
    let dd = dist(*pos, target);
    if dd <= 1e-6 {
        return;
    }
    let step = (speed * dt).min(dd);
    pos.x += ((target.x - pos.x) / dd) * step;
    pos.y += ((target.y - pos.y) / dd) * step;
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

fn as_mut_object(value: &mut Value) -> Result<&mut Map<String, Value>, String> {
    value
        .as_object_mut()
        .ok_or_else(|| "runtime state must be a JSON object".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_champion(id: &str, team: &str, role: &str, lane: &str, pos: Vec2) -> ChampionRuntime {
        ChampionRuntime {
            id: id.to_string(),
            name: id.to_string(),
            champion_id: String::new(),
            team: team.to_string(),
            role: role.to_string(),
            lane: lane.to_string(),
            pos,
            hp: 100.0,
            max_hp: 100.0,
            alive: true,
            respawn_at: 0.0,
            attack_cd_until: 0.0,
            move_speed: 0.07,
            attack_range: 0.055,
            attack_type: "ranged".to_string(),
            attack_damage: 10.0,
            target_path: Vec::new(),
            target_path_index: 0,
            next_decision_at: 0.0,
            kills: 0,
            deaths: 0,
            assists: 0,
            gold: 0,
            spent_gold: 0,
            xp: 0,
            level: 1,
            cs: 0,
            has_left_base_once: false,
            last_support_cs_at: -999.0,
            items: Vec::new(),
            gameplay_score: 70.0,
            iq_score: 70.0,
            competitive_score: 70.0,
            summoner_spells: vec![
                RuntimeSummonerSpellSlot {
                    key: "Flash".to_string(),
                    cd_until: 0.0,
                },
                RuntimeSummonerSpellSlot {
                    key: "Ignite".to_string(),
                    cd_until: 0.0,
                },
            ],
            ultimate: Some(RuntimeUltimateSlot {
                archetype: "burst".to_string(),
                icon: String::new(),
                cd_until: 0.0,
            }),
            ignite_dot_until: 0.0,
            ignite_source_id: None,
            last_damaged_by_champion_id: None,
            last_damaged_at: -999.0,
            state: "lane".to_string(),
            recall_anchor: None,
            recall_channel_until: 0.0,
            realm_banished_until: 0.0,
            realm_return_pos: None,
            ward_cd_until: 0.0,
            sweeper_cd_until: 0.0,
            sweeper_active_until: 0.0,
            trinket_key: TRINKET_WARDING_TOTEM.to_string(),
            trinket_swapped: false,
            support_roam_uses: 0,
            support_roam_cd_until: 0.0,
            support_last_roam_role: String::new(),
        }
    }

    fn test_minion(id: &str, team: &str, lane: &str, pos: Vec2) -> MinionRuntime {
        MinionRuntime {
            id: id.to_string(),
            team: team.to_string(),
            lane: lane.to_string(),
            pos,
            hp: 20.0,
            max_hp: 20.0,
            alive: true,
            kind: "melee".to_string(),
            last_hit_by_champion_id: None,
            owner_champion_id: None,
            summon_kind: None,
            summon_expires_at: 0.0,
            attack_cd_until: 0.0,
            move_speed: 0.06,
            attack_range: 0.04,
            attack_damage: 6.0,
            path: vec![pos],
            path_index: 0,
        }
    }

    fn test_structure(id: &str, team: &str, lane: &str, pos: Vec2) -> StructureRuntime {
        StructureRuntime {
            id: id.to_string(),
            team: team.to_string(),
            lane: lane.to_string(),
            kind: "tower".to_string(),
            pos,
            hp: 1000.0,
            max_hp: 1000.0,
            alive: true,
            attack_cd_until: 0.0,
            forced_target_champion_id: None,
            forced_target_until: 0.0,
        }
    }

    fn test_runtime(
        champions: Vec<ChampionRuntime>,
        minions: Vec<MinionRuntime>,
        structures: Vec<StructureRuntime>,
        neutral_timers: NeutralTimersRuntime,
    ) -> RuntimeState {
        RuntimeState {
            time_sec: LANE_COMBAT_UNLOCK_AT + 1.0,
            running: true,
            speed: 1.0,
            ai_mode: SimulatorAiMode::Rules,
            policy: SimulatorPolicyConfig::default(),
            winner: None,
            show_walls: false,
            champions,
            minions,
            structures,
            wards: Vec::new(),
            objectives: json!({}),
            neutral_timers: serde_json::to_value(neutral_timers).unwrap_or(json!({})),
            stats: RuntimeStats {
                blue: RuntimeTeamStats {
                    kills: 0,
                    towers: 0,
                    dragons: 0,
                    barons: 0,
                    gold: 0,
                },
                red: RuntimeTeamStats {
                    kills: 0,
                    towers: 0,
                    dragons: 0,
                    barons: 0,
                    gold: 0,
                },
            },
            events: Vec::new(),
            lane_combat_state_by_champion: HashMap::new(),
            telemetry_decisions: Vec::new(),
            extra: HashMap::new(),
        }
    }

    fn test_neutral_timer(key: &str, pos: Vec2, alive: bool) -> NeutralTimerRuntime {
        NeutralTimerRuntime {
            key: key.to_string(),
            label: key.to_string(),
            alive,
            hp: 1000.0,
            max_hp: 1000.0,
            next_spawn_at: None,
            first_spawn_at: 0.0,
            respawn_delay_sec: Some(120.0),
            one_shot: false,
            window_close_at: None,
            combat_grace_until: None,
            unlocked: true,
            last_spawn_at: Some(0.0),
            last_taken_at: None,
            times_spawned: 1,
            times_taken: 0,
            pos,
            extra: HashMap::new(),
        }
    }

    #[test]
    fn nav_grid_routes_around_walls_for_champion_paths() {
        let start = Vec2 { x: 0.60, y: 0.70 };
        let end = Vec2 { x: 0.74, y: 0.70 };

        let path = nav_grid().find_path(start, end);

        assert!(path.len() > 1, "expected non-trivial path around wall");
        assert!(
            path.iter().all(|p| !active_nav_walls()
                .iter()
                .any(|w| point_in_polygon(*p, &w.points))),
            "path should not contain blocked wall nodes"
        );
    }

    #[test]
    fn minion_holds_position_when_enemy_lane_combat_is_nearby() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut blue = test_minion("m-blue-1", "blue", "mid", Vec2 { x: 0.5, y: 0.5 });
        blue.path = vec![Vec2 { x: 0.5, y: 0.5 }, Vec2 { x: 0.7, y: 0.5 }];
        blue.path_index = 1;

        let mut red = test_minion("m-red-1", "red", "mid", Vec2 { x: 0.54, y: 0.5 });
        red.path = vec![Vec2 { x: 0.54, y: 0.5 }, Vec2 { x: 0.3, y: 0.5 }];
        red.path_index = 1;

        let start_pos = blue.pos;
        let mut runtime = test_runtime(vec![], vec![blue, red], vec![], neutral);

        move_minions(&mut runtime, 0.05);

        assert!(dist(runtime.minions[0].pos, start_pos) < 1e-6);
    }

    #[test]
    fn minion_prioritizes_minion_over_structure_when_both_in_range() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut blue = test_minion("m-blue-1", "blue", "mid", Vec2 { x: 0.5, y: 0.5 });
        blue.attack_damage = 10.0;
        blue.attack_range = 0.06;

        let red_minion = test_minion("m-red-1", "red", "mid", Vec2 { x: 0.53, y: 0.5 });
        let mut red_tower =
            test_structure("red-mid-outer", "red", "mid", Vec2 { x: 0.535, y: 0.5 });
        red_tower.hp = 100.0;

        let mut runtime = test_runtime(vec![], vec![blue, red_minion], vec![red_tower], neutral);

        let tower_hp_before = runtime.structures[0].hp;
        let minion_hp_before = runtime.minions[1].hp;
        resolve_minion_combat(&mut runtime);

        assert_eq!(runtime.structures[0].hp, tower_hp_before);
        assert!(runtime.minions[1].hp < minion_hp_before);
    }

    #[test]
    fn minion_cannot_target_inhib_while_inhib_tower_alive() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut blue = test_minion("m-blue-1", "blue", "mid", Vec2 { x: 0.79, y: 0.22 });
        blue.attack_damage = 10.0;
        blue.attack_range = 0.06;

        let mut red_inhib = test_structure(
            "red-inhib-mid",
            "red",
            "base",
            Vec2 {
                x: 0.7832,
                y: 0.2240,
            },
        );
        red_inhib.kind = "inhib".to_string();
        red_inhib.hp = 200.0;
        let red_inhib_tower = test_structure(
            "red-mid-inhib-tower",
            "red",
            "mid",
            Vec2 {
                x: 0.740234375,
                y: 0.26171875,
            },
        );

        let mut runtime = test_runtime(
            vec![],
            vec![blue],
            vec![red_inhib, red_inhib_tower],
            neutral,
        );
        let hp_before = runtime.structures[0].hp;

        resolve_minion_combat(&mut runtime);

        assert_eq!(runtime.structures[0].hp, hp_before);
    }

    #[test]
    fn minion_can_target_inhib_after_inhib_tower_is_down() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut blue = test_minion("m-blue-1", "blue", "mid", Vec2 { x: 0.79, y: 0.22 });
        blue.attack_damage = 10.0;
        blue.attack_range = 0.06;

        let mut red_inhib = test_structure(
            "red-inhib-mid",
            "red",
            "base",
            Vec2 {
                x: 0.7832,
                y: 0.2240,
            },
        );
        red_inhib.kind = "inhib".to_string();
        red_inhib.hp = 200.0;

        let mut runtime = test_runtime(vec![], vec![blue], vec![red_inhib], neutral);
        let hp_before = runtime.structures[0].hp;

        resolve_minion_combat(&mut runtime);

        assert!(runtime.structures[0].hp < hp_before);
    }

    #[test]
    fn minion_cannot_target_nexus_tower_while_lane_inhib_alive() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut blue = test_minion("m-blue-1", "blue", "top", Vec2 { x: 0.846, y: 0.133 });
        blue.attack_damage = 10.0;
        blue.attack_range = 0.06;

        let mut red_nexus_top_tower = test_structure(
            "red-nexus-top-tower",
            "red",
            "base",
            Vec2 {
                x: 0.845703125,
                y: 0.1328125,
            },
        );
        red_nexus_top_tower.hp = 200.0;
        let red_inhib_top = test_structure(
            "red-inhib-top",
            "red",
            "base",
            Vec2 {
                x: 0.7545572916666666,
                y: 0.09114583333333333,
            },
        );

        let mut runtime = test_runtime(
            vec![],
            vec![blue],
            vec![red_nexus_top_tower, red_inhib_top],
            neutral,
        );
        let hp_before = runtime.structures[0].hp;

        resolve_minion_combat(&mut runtime);

        assert_eq!(runtime.structures[0].hp, hp_before);
    }

    #[test]
    fn minion_can_target_nexus_tower_after_lane_inhib_is_down() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut blue = test_minion("m-blue-1", "blue", "top", Vec2 { x: 0.846, y: 0.133 });
        blue.attack_damage = 10.0;
        blue.attack_range = 0.06;

        let mut red_nexus_top_tower = test_structure(
            "red-nexus-top-tower",
            "red",
            "base",
            Vec2 {
                x: 0.845703125,
                y: 0.1328125,
            },
        );
        red_nexus_top_tower.hp = 200.0;

        let mut runtime = test_runtime(vec![], vec![blue], vec![red_nexus_top_tower], neutral);
        let hp_before = runtime.structures[0].hp;

        resolve_minion_combat(&mut runtime);

        assert!(runtime.structures[0].hp < hp_before);
    }

    #[test]
    fn jgl_disengage_prefers_jungle_camp_fallback() {
        let jungler = test_champion("jgl-blue", "blue", "JGL", "bot", Vec2 { x: 0.46, y: 0.61 });
        let mut entities = HashMap::new();
        entities.insert(
            "gromp-blue".to_string(),
            test_neutral_timer("gromp-blue", Vec2 { x: 0.16, y: 0.43 }, true),
        );
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities,
            extra: HashMap::new(),
        };
        let mut runtime = test_runtime(vec![jungler], vec![], vec![], neutral);

        issue_lane_disengage(&mut runtime, 0, Vec2 { x: 0.52, y: 0.65 });

        let target = runtime.champions[0].target_path.last().copied();
        assert!(target.is_some());
        let p = target.unwrap_or(Vec2 { x: 0.0, y: 0.0 });
        assert!(dist(p, Vec2 { x: 0.16, y: 0.43 }) <= 0.02);
    }

    #[test]
    fn objective_assist_prioritizes_objective_over_farm_lock() {
        let adc = test_champion("adc-blue", "blue", "ADC", "bot", Vec2 { x: 0.62, y: 0.73 });
        let jungler = test_champion("jgl-blue", "blue", "JGL", "bot", Vec2 { x: 0.64, y: 0.71 });
        let mut enemy = test_champion("mid-red", "red", "MID", "mid", Vec2 { x: 0.82, y: 0.70 });
        enemy.attack_damage = 1.0;

        let minion = test_minion("m-red-1", "red", "bot", Vec2 { x: 0.625, y: 0.735 });

        let mut entities = HashMap::new();
        entities.insert(
            "dragon".to_string(),
            test_neutral_timer("dragon", Vec2 { x: 0.67, y: 0.70 }, true),
        );
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities,
            extra: HashMap::new(),
        };

        let runtime = test_runtime(
            vec![adc, jungler, enemy],
            vec![minion],
            vec![],
            neutral.clone(),
        );

        let target = pick_combat_target(&runtime, 0, runtime.time_sec, &neutral);
        assert!(matches!(target, Some(CombatTarget::Neutral(ref key)) if key == "dragon"));
    }

    #[test]
    fn structure_pressure_blocked_with_two_enemy_minions_near_tower() {
        let laner = test_champion("top-blue", "blue", "TOP", "top", Vec2 { x: 0.28, y: 0.09 });
        let tower = test_structure(
            "red-top-outer",
            "red",
            "top",
            Vec2 {
                x: 0.275390625,
                y: 0.07161458333333333,
            },
        );

        let allied_wave = test_minion("m-blue-1", "blue", "top", Vec2 { x: 0.29, y: 0.08 });
        let enemy_wave_1 = test_minion("m-red-1", "red", "top", Vec2 { x: 0.27, y: 0.074 });
        let enemy_wave_2 = test_minion("m-red-2", "red", "top", Vec2 { x: 0.271, y: 0.073 });

        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let runtime = test_runtime(
            vec![laner],
            vec![allied_wave, enemy_wave_1, enemy_wave_2],
            vec![tower],
            neutral.clone(),
        );

        let target = pick_combat_target(&runtime, 0, runtime.time_sec, &neutral);
        assert!(!matches!(target, Some(CombatTarget::Structure(_))));
    }

    #[test]
    fn red_jungler_macro_prefers_own_side_buffs_first() {
        let red_jgl = test_champion("jgl-red", "red", "JGL", "bot", Vec2 { x: 0.75, y: 0.55 });
        let blue_jgl = test_champion("jgl-blue", "blue", "JGL", "bot", Vec2 { x: 0.25, y: 0.46 });

        let mut entities = HashMap::new();
        entities.insert(
            "blue-buff-blue".to_string(),
            test_neutral_timer("blue-buff-blue", Vec2 { x: 0.25, y: 0.46 }, true),
        );
        entities.insert(
            "blue-buff-red".to_string(),
            test_neutral_timer("blue-buff-red", Vec2 { x: 0.48, y: 0.26 }, true),
        );

        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities,
            extra: HashMap::new(),
        };

        let default_tactics = RuntimeTeamTactics::default();
        let red_pick = pick_macro_objective_pos(&red_jgl, &neutral, 120.0, &default_tactics);
        let blue_pick = pick_macro_objective_pos(&blue_jgl, &neutral, 120.0, &default_tactics);

        assert_eq!(red_pick.map(|p| (p.x, p.y)), Some((0.48, 0.26)));
        assert_eq!(blue_pick.map(|p| (p.x, p.y)), Some((0.25, 0.46)));
    }

    #[test]
    fn jungle_pathing_bot_to_top_invades_enemy_top_side_first_for_both_teams() {
        let blue_order = jungler_macro_jungle_priority_for_team("blue", "BotToTop");
        let red_order = jungler_macro_jungle_priority_for_team("red", "BotToTop");

        assert_eq!(blue_order[8], "blue-buff-red");
        assert_eq!(red_order[8], "blue-buff-blue");
    }

    #[test]
    fn jungle_pathing_top_to_bot_invades_enemy_bot_side_first_for_both_teams() {
        let blue_order = jungler_macro_jungle_priority_for_team("blue", "TopToBot");
        let red_order = jungler_macro_jungle_priority_for_team("red", "TopToBot");

        assert_eq!(blue_order[8], "red-buff-red");
        assert_eq!(red_order[8], "red-buff-blue");
    }

    #[test]
    fn jungle_disengage_fallback_honors_pathing_start_side_for_blue_and_red() {
        let blue_bot_to_top = jungle_disengage_fallback_order_for_team("blue", "BotToTop");
        let blue_top_to_bot = jungle_disengage_fallback_order_for_team("blue", "TopToBot");
        let red_bot_to_top = jungle_disengage_fallback_order_for_team("red", "BotToTop");
        let red_top_to_bot = jungle_disengage_fallback_order_for_team("red", "TopToBot");

        assert_eq!(blue_bot_to_top[0], "raptors-blue");
        assert_eq!(blue_top_to_bot[0], "gromp-blue");
        assert_eq!(red_bot_to_top[0], "raptors-red");
        assert_eq!(red_top_to_bot[0], "gromp-red");
    }

    #[test]
    fn kill_rewards_reduce_when_ahead_killer_farms_behind_target() {
        let mut killer = test_champion("jgl-blue", "blue", "JGL", "bot", Vec2 { x: 0.5, y: 0.5 });
        killer.kills = 10;
        killer.deaths = 1;
        killer.level = 13;

        let mut victim = test_champion("jgl-red", "red", "JGL", "bot", Vec2 { x: 0.52, y: 0.5 });
        victim.kills = 1;
        victim.deaths = 8;
        victim.level = 10;

        let (gold, xp) = champion_kill_rewards(&killer, &victim);
        assert!(gold < CHAMPION_KILL_GOLD);
        assert!(xp < CHAMPION_KILL_XP);
    }

    #[test]
    fn kill_rewards_increase_for_shutdown() {
        let mut killer = test_champion("top-blue", "blue", "TOP", "top", Vec2 { x: 0.5, y: 0.5 });
        killer.kills = 1;
        killer.deaths = 4;
        killer.level = 9;

        let mut victim = test_champion("mid-red", "red", "MID", "mid", Vec2 { x: 0.52, y: 0.5 });
        victim.kills = 9;
        victim.deaths = 1;
        victim.level = 13;

        let (gold, xp) = champion_kill_rewards(&killer, &victim);
        assert!(gold > CHAMPION_KILL_GOLD);
        assert!(xp > CHAMPION_KILL_XP);
    }

    #[test]
    fn respawn_scales_with_level_and_time() {
        let early_low = champion_respawn_seconds(3, 12.0 * 60.0);
        let late_high = champion_respawn_seconds(15, 33.0 * 60.0);
        assert!(late_high > early_low);
        assert!(late_high <= 42.0);
    }

    #[test]
    fn heal_spell_casts_when_self_is_low_hp() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut champion =
            test_champion("adc-blue", "blue", "ADC", "bot", Vec2 { x: 0.50, y: 0.50 });
        champion.hp = 20.0;
        champion.summoner_spells = vec![RuntimeSummonerSpellSlot {
            key: "Heal".to_string(),
            cd_until: 0.0,
        }];

        let mut runtime = test_runtime(vec![champion], vec![], vec![], neutral);
        let hp_before = runtime.champions[0].hp;

        resolve_champion_combat(&mut runtime);

        assert!(runtime.champions[0].hp > hp_before);
        let heal_cd = runtime.champions[0]
            .summoner_spells
            .iter()
            .find(|spell| spell.key == "Heal")
            .map(|spell| spell.cd_until)
            .unwrap_or(0.0);
        assert!(heal_cd > runtime.time_sec);
    }

    #[test]
    fn smite_executes_low_hp_dragon_for_jungler() {
        let mut entities = HashMap::new();
        let mut dragon = test_neutral_timer(
            "dragon",
            Vec2 {
                x: 0.6738,
                y: 0.7031,
            },
            true,
        );
        dragon.hp = 520.0;
        dragon.max_hp = 3600.0;
        entities.insert("dragon".to_string(), dragon);

        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities,
            extra: HashMap::new(),
        };

        let mut jgl = test_champion("jgl-blue", "blue", "JGL", "bot", Vec2 { x: 0.67, y: 0.70 });
        jgl.summoner_spells = vec![RuntimeSummonerSpellSlot {
            key: "Smite".to_string(),
            cd_until: 0.0,
        }];

        let mut runtime = test_runtime(vec![jgl], vec![], vec![], neutral);

        resolve_champion_combat(&mut runtime);

        assert_eq!(runtime.stats.blue.dragons, 1);
        let decoded = decode_neutral_timers_state(&runtime.neutral_timers)
            .unwrap_or_else(|| panic!("failed to decode timers"));
        let dragon_after = decoded
            .entities
            .get("dragon")
            .unwrap_or_else(|| panic!("dragon missing"));
        assert!(!dragon_after.alive);
    }

    #[test]
    fn ultimate_burst_casts_when_level_six_enemy_nearby() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut caster = test_champion("mid-blue", "blue", "MID", "mid", Vec2 { x: 0.50, y: 0.50 });
        caster.level = 6;
        caster.ultimate = Some(RuntimeUltimateSlot {
            archetype: "burst".to_string(),
            icon: String::new(),
            cd_until: 0.0,
        });

        let target = test_champion("mid-red", "red", "MID", "mid", Vec2 { x: 0.55, y: 0.50 });
        let mut runtime = test_runtime(vec![caster, target], vec![], vec![], neutral);
        let hp_before = runtime.champions[1].hp;

        resolve_champion_combat(&mut runtime);

        assert!(runtime.champions[1].hp < hp_before);
        let cd = runtime.champions[0]
            .ultimate
            .as_ref()
            .map(|ultimate| ultimate.cd_until)
            .unwrap_or(0.0);
        assert!(cd > runtime.time_sec);
    }

    #[test]
    fn execute_ultimate_requires_low_hp_target() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut adc = test_champion("adc-blue", "blue", "ADC", "bot", Vec2 { x: 0.50, y: 0.50 });
        adc.level = 7;
        adc.ultimate = Some(RuntimeUltimateSlot {
            archetype: "execute".to_string(),
            icon: String::new(),
            cd_until: 0.0,
        });

        let mut target = test_champion("adc-red", "red", "ADC", "bot", Vec2 { x: 0.55, y: 0.50 });
        target.hp = 90.0;
        let mut runtime = test_runtime(vec![adc, target], vec![], vec![], neutral);

        resolve_champion_combat(&mut runtime);

        let cd = runtime.champions[0]
            .ultimate
            .as_ref()
            .map(|ultimate| ultimate.cd_until)
            .unwrap_or(0.0);
        assert_eq!(cd, 0.0);
    }

    #[test]
    fn annie_ultimate_summons_tibbers_with_scaled_stats() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut annie = test_champion("mid-blue", "blue", "MID", "mid", Vec2 { x: 0.50, y: 0.50 });
        annie.champion_id = "Annie".to_string();
        annie.level = 6;
        annie.ultimate = Some(RuntimeUltimateSlot {
            archetype: "burst".to_string(),
            icon: String::new(),
            cd_until: 0.0,
        });

        let mut runtime = test_runtime(vec![annie], vec![], vec![], neutral);
        resolve_champion_combat(&mut runtime);

        let summon = runtime.minions.iter().find(|minion| {
            minion.id.contains("tibbers") && minion.owner_champion_id.as_deref() == Some("mid-blue")
        });
        assert!(summon.is_some());
    }

    #[test]
    fn shen_ultimate_shields_ally_and_teleports() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut shen = test_champion("top-blue", "blue", "TOP", "top", Vec2 { x: 0.30, y: 0.30 });
        shen.champion_id = "Shen".to_string();
        shen.level = 6;
        shen.ultimate = Some(RuntimeUltimateSlot {
            archetype: "defensive".to_string(),
            icon: String::new(),
            cd_until: 0.0,
        });

        let mut ally = test_champion("adc-blue", "blue", "ADC", "bot", Vec2 { x: 0.72, y: 0.78 });
        ally.hp = 25.0;

        let mut runtime = test_runtime(vec![shen, ally], vec![], vec![], neutral);
        let hp_before = runtime.champions[1].hp;
        let ally_pos = runtime.champions[1].pos;

        resolve_champion_combat(&mut runtime);

        assert!(runtime.champions[1].hp > hp_before);
        assert!(dist(runtime.champions[0].pos, ally_pos) < 0.0001);
    }

    #[test]
    fn mordekaiser_ultimate_banishes_both_champions_temporarily() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut morde = test_champion("top-blue", "blue", "TOP", "top", Vec2 { x: 0.50, y: 0.50 });
        morde.champion_id = "Mordekaiser".to_string();
        morde.level = 6;
        morde.ultimate = Some(RuntimeUltimateSlot {
            archetype: "burst".to_string(),
            icon: String::new(),
            cd_until: 0.0,
        });

        let enemy = test_champion("top-red", "red", "TOP", "top", Vec2 { x: 0.54, y: 0.50 });
        let mut runtime = test_runtime(vec![morde, enemy], vec![], vec![], neutral);

        resolve_champion_combat(&mut runtime);

        assert!(runtime.champions[0].realm_banished_until > runtime.time_sec);
        assert!(runtime.champions[1].realm_banished_until > runtime.time_sec);
    }

    #[test]
    fn summon_expires_after_configured_duration() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut annie = test_champion("mid-blue", "blue", "MID", "mid", Vec2 { x: 0.50, y: 0.50 });
        annie.champion_id = "Annie".to_string();
        annie.level = 6;
        annie.ultimate = Some(RuntimeUltimateSlot {
            archetype: "burst".to_string(),
            icon: String::new(),
            cd_until: 0.0,
        });

        let mut runtime = test_runtime(vec![annie], vec![], vec![], neutral);
        resolve_champion_combat(&mut runtime);
        assert!(runtime
            .minions
            .iter()
            .any(|minion| minion.alive && minion.kind == "summon"));

        runtime.time_sec += 46.0;
        move_minions(&mut runtime, 0.1);

        assert!(!runtime
            .minions
            .iter()
            .any(|minion| minion.alive && minion.kind == "summon"));
    }

    #[test]
    fn mordekaiser_realm_returns_positions_after_duration() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut morde = test_champion("top-blue", "blue", "TOP", "top", Vec2 { x: 0.50, y: 0.50 });
        morde.champion_id = "Mordekaiser".to_string();
        morde.level = 6;
        morde.ultimate = Some(RuntimeUltimateSlot {
            archetype: "burst".to_string(),
            icon: String::new(),
            cd_until: 0.0,
        });

        let enemy = test_champion("top-red", "red", "TOP", "top", Vec2 { x: 0.54, y: 0.50 });
        let mut runtime = test_runtime(vec![morde, enemy], vec![], vec![], neutral);
        let morde_pos = runtime.champions[0].pos;
        let enemy_pos = runtime.champions[1].pos;

        resolve_champion_combat(&mut runtime);
        runtime.time_sec += ULTIMATE_MORDE_REALM_DURATION_SEC + 0.5;
        move_champions(&mut runtime, 0.1);

        assert!(dist(runtime.champions[0].pos, morde_pos) < 0.0001);
        assert!(dist(runtime.champions[1].pos, enemy_pos) < 0.0001);
    }

    #[test]
    fn global_ultimate_requires_team_vision() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut caster = test_champion("jgl-blue", "blue", "JGL", "mid", Vec2 { x: 0.40, y: 0.40 });
        caster.level = 8;
        caster.ultimate = Some(RuntimeUltimateSlot {
            archetype: "global".to_string(),
            icon: String::new(),
            cd_until: 0.0,
        });
        let target = test_champion("mid-red", "red", "MID", "mid", Vec2 { x: 0.56, y: 0.40 });

        let mut runtime = test_runtime(
            vec![caster.clone(), target.clone()],
            vec![],
            vec![],
            neutral.clone(),
        );
        resolve_champion_combat(&mut runtime);
        let cd_without_vision = runtime.champions[0]
            .ultimate
            .as_ref()
            .map(|u| u.cd_until)
            .unwrap_or(0.0);
        assert_eq!(cd_without_vision, 0.0);

        let mut runtime_with_ward = test_runtime(vec![caster, target], vec![], vec![], neutral);
        runtime_with_ward.wards.push(WardRuntime {
            id: "w1".to_string(),
            team: "blue".to_string(),
            owner_champion_id: "jgl-blue".to_string(),
            pos: Vec2 { x: 0.56, y: 0.40 },
            expires_at: runtime_with_ward.time_sec + 30.0,
        });
        resolve_champion_combat(&mut runtime_with_ward);
        let cd_with_vision = runtime_with_ward.champions[0]
            .ultimate
            .as_ref()
            .map(|u| u.cd_until)
            .unwrap_or(0.0);
        assert!(cd_with_vision > runtime_with_ward.time_sec);
    }

    #[test]
    fn sweeper_is_jgl_sup_only_and_clears_enemy_wards() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut jgl = test_champion("jgl-blue", "blue", "JGL", "mid", Vec2 { x: 0.50, y: 0.50 });
        jgl.sweeper_cd_until = 0.0;
        jgl.trinket_key = TRINKET_ORACLE_LENS.to_string();
        let mut top = test_champion("top-blue", "blue", "TOP", "top", Vec2 { x: 0.50, y: 0.50 });
        top.sweeper_cd_until = 0.0;

        let mut runtime = test_runtime(vec![jgl, top], vec![], vec![], neutral);
        runtime.wards.push(WardRuntime {
            id: "w-red".to_string(),
            team: "red".to_string(),
            owner_champion_id: "mid-red".to_string(),
            pos: Vec2 { x: 0.51, y: 0.50 },
            expires_at: runtime.time_sec + 60.0,
        });

        process_sweepers(&mut runtime);

        assert!(runtime.wards.is_empty());
        assert!(runtime.champions[0].sweeper_active_until > runtime.time_sec);
        assert_eq!(runtime.champions[1].sweeper_active_until, 0.0);
    }

    #[test]
    fn jgl_swaps_to_oracle_on_first_recall_after_minute_six() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut jgl = test_champion("jgl-blue", "blue", "JGL", "mid", Vec2 { x: 0.50, y: 0.50 });
        jgl.state = "recall".to_string();
        jgl.recall_channel_until = TRINKET_SWAP_UNLOCK_AT_SEC + 1.0;

        let mut runtime = test_runtime(vec![jgl], vec![], vec![], neutral);
        runtime.time_sec = TRINKET_SWAP_UNLOCK_AT_SEC + 1.0;

        move_champions(&mut runtime, 0.1);

        assert_eq!(runtime.champions[0].trinket_key, TRINKET_ORACLE_LENS);
        assert!(runtime.champions[0].trinket_swapped);
    }

    #[test]
    fn jgl_no_longer_places_wards_after_oracle_swap() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut jgl = test_champion("jgl-blue", "blue", "JGL", "mid", Vec2 { x: 0.52, y: 0.52 });
        jgl.trinket_key = TRINKET_ORACLE_LENS.to_string();
        jgl.trinket_swapped = true;
        jgl.ward_cd_until = 0.0;

        let mut runtime = test_runtime(vec![jgl], vec![], vec![], neutral);
        runtime.time_sec = TRINKET_SWAP_UNLOCK_AT_SEC + 60.0;

        place_wards(&mut runtime);

        assert!(runtime.wards.is_empty());
    }

    #[test]
    fn wards_use_strategic_points_not_raw_champion_position() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut sup = test_champion("sup-blue", "blue", "SUP", "bot", Vec2 { x: 0.60, y: 0.61 });
        sup.ward_cd_until = 0.0;
        sup.trinket_key = TRINKET_WARDING_TOTEM.to_string();

        let mut runtime = test_runtime(vec![sup], vec![], vec![], neutral);
        runtime.time_sec = WARD_UNLOCK_AT_SEC + 30.0;

        place_wards(&mut runtime);
        assert_eq!(runtime.wards.len(), 1);
        let ward_pos = runtime.wards[0].pos;
        assert!(
            dist(ward_pos, Vec2 { x: 0.615, y: 0.61 }) < 0.03
                || dist(ward_pos, Vec2 { x: 0.565, y: 0.455 }) < 0.03
        );
    }

    #[test]
    fn support_roam_after_minute_ten_rotates_not_same_lane_forever() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut sup = test_champion("sup-blue", "blue", "SUP", "bot", Vec2 { x: 0.52, y: 0.70 });
        sup.support_last_roam_role = "MID".to_string();
        sup.support_roam_cd_until = 0.0;

        let mut top = test_champion("top-blue", "blue", "TOP", "top", Vec2 { x: 0.20, y: 0.32 });
        top.hp = 40.0;
        let mut mid = test_champion("mid-blue", "blue", "MID", "mid", Vec2 { x: 0.52, y: 0.52 });
        mid.hp = 35.0;
        let mut adc = test_champion("adc-blue", "blue", "ADC", "bot", Vec2 { x: 0.72, y: 0.80 });
        adc.hp = 85.0;

        let mut runtime = test_runtime(vec![sup.clone(), top, mid, adc], vec![], vec![], neutral);
        runtime.time_sec = SUPPORT_OPEN_ROAM_AT_SEC + 20.0;
        let timers = decode_neutral_timers_state(&runtime.neutral_timers)
            .unwrap_or_else(|| neutral_timers_default_runtime_state());

        let champions_snapshot = runtime.champions.clone();
        decide_champion_state(
            &mut runtime.champions[0],
            runtime.time_sec,
            &runtime.minions,
            &runtime.structures,
            &champions_snapshot,
            Some(&timers),
            &RuntimeTeamTactics::default(),
            &RuntimeTeamBuffState::default(),
        );

        assert_eq!(runtime.champions[0].state, "objective");
        assert_ne!(runtime.champions[0].support_last_roam_role, "MID");
    }

    #[test]
    fn teleport_uses_allied_lane_tower_from_base() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut top = test_champion("top-blue", "blue", "TOP", "top", base_position_for("blue"));
        top.summoner_spells = vec![RuntimeSummonerSpellSlot {
            key: "Teleport".to_string(),
            cd_until: 0.0,
        }];

        let target_tower =
            test_structure("blue-top-outer", "blue", "top", Vec2 { x: 0.11, y: 0.56 });
        let mut runtime = test_runtime(vec![top], vec![], vec![target_tower.clone()], neutral);
        runtime.time_sec = SUMMONER_TP_UNLOCK_AT_SEC + 10.0;

        resolve_champion_combat(&mut runtime);

        assert!(dist(runtime.champions[0].pos, target_tower.pos) < 0.0001);
        let tp_cd = runtime.champions[0]
            .summoner_spells
            .iter()
            .find(|spell| spell.key == "Teleport")
            .map(|spell| spell.cd_until)
            .unwrap_or(0.0);
        assert!(tp_cd > runtime.time_sec);
    }

    #[test]
    fn teleport_uses_allied_lane_minion_when_no_tower_available() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let mut top = test_champion("top-blue", "blue", "TOP", "top", base_position_for("blue"));
        top.summoner_spells = vec![RuntimeSummonerSpellSlot {
            key: "Teleport".to_string(),
            cd_until: 0.0,
        }];

        let lane_minion = test_minion("blue-top-m1", "blue", "top", Vec2 { x: 0.19, y: 0.35 });
        let mut runtime = test_runtime(vec![top], vec![lane_minion.clone()], vec![], neutral);
        runtime.time_sec = SUMMONER_TP_UNLOCK_AT_SEC + 10.0;

        resolve_champion_combat(&mut runtime);

        assert!(dist(runtime.champions[0].pos, lane_minion.pos) < 0.0001);
    }

    #[test]
    fn dragon_kind_is_mirrored_into_timer_entity_on_tick() {
        let mut entities = HashMap::new();
        entities.insert(
            "dragon".to_string(),
            test_neutral_timer(
                "dragon",
                Vec2 {
                    x: 0.6738,
                    y: 0.7031,
                },
                true,
            ),
        );

        let mut neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities,
            extra: HashMap::new(),
        };
        neutral
            .extra
            .insert("dragonCurrentKind".to_string(), Value::from("ocean"));

        let mut runtime = test_runtime(vec![], vec![], vec![], neutral);
        tick_neutral_timers(&mut runtime);

        let decoded = decode_neutral_timers_state(&runtime.neutral_timers)
            .unwrap_or_else(|| panic!("failed to decode neutral timers"));
        let dragon_timer = decoded
            .entities
            .get("dragon")
            .unwrap_or_else(|| panic!("dragon timer missing"));

        assert_eq!(
            dragon_timer
                .extra
                .get("dragonCurrentKind")
                .and_then(Value::as_str),
            Some("ocean")
        );
    }

    #[test]
    fn dragon_soul_unlocks_elder_after_fourth_stack() {
        let mut entities = HashMap::new();
        let mut dragon = test_neutral_timer(
            "dragon",
            Vec2 {
                x: 0.6738,
                y: 0.7031,
            },
            true,
        );
        dragon.next_spawn_at = Some(0.0);
        entities.insert("dragon".to_string(), dragon);

        let mut elder = test_neutral_timer(
            "elder",
            Vec2 {
                x: 0.6738,
                y: 0.7031,
            },
            false,
        );
        elder.unlocked = false;
        elder.next_spawn_at = None;
        entities.insert("elder".to_string(), elder);

        let mut neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities,
            extra: HashMap::new(),
        };
        neutral
            .extra
            .insert("dragonCurrentKind".to_string(), Value::from("infernal"));
        neutral
            .extra
            .insert("dragonSoulRiftKind".to_string(), Value::from("infernal"));

        let killer = test_champion("jgl-blue", "blue", "JGL", "bot", Vec2 { x: 0.67, y: 0.70 });
        let mut runtime = test_runtime(vec![killer], vec![], vec![], neutral);

        let buffs = RuntimeBuffState {
            blue: RuntimeTeamBuffState {
                dragon_stacks: 3,
                ..RuntimeTeamBuffState::default()
            },
            red: RuntimeTeamBuffState::default(),
        };
        set_runtime_buffs(&mut runtime, &buffs);

        let mut timers = decode_neutral_timers_state(&runtime.neutral_timers)
            .unwrap_or_else(|| panic!("failed to decode neutral timers"));
        let dragon_kind = process_dragon_capture(&mut runtime, &mut timers, "blue");

        assert_eq!(dragon_kind, "infernal");
        assert!(timers.dragon_soul_unlocked);
        assert!(timers.elder_unlocked);

        let elder_timer = timers
            .entities
            .get("elder")
            .unwrap_or_else(|| panic!("elder timer missing"));
        assert!(elder_timer.unlocked);
        assert!(elder_timer.next_spawn_at.is_some());

        let blue_buffs = team_buffs_for_runtime(runtime.extra.get("teamBuffs"), "blue");
        assert_eq!(blue_buffs.dragon_stacks, 4);
        assert_eq!(blue_buffs.soul_kind.as_deref(), Some("infernal"));
    }

    #[test]
    fn dragon_cycle_progresses_a_b_then_soul_rift_c_repeats() {
        let mut entities = HashMap::new();
        let mut dragon = test_neutral_timer(
            "dragon",
            Vec2 {
                x: 0.6738,
                y: 0.7031,
            },
            true,
        );
        dragon.next_spawn_at = Some(0.0);
        entities.insert("dragon".to_string(), dragon);

        let mut elder = test_neutral_timer(
            "elder",
            Vec2 {
                x: 0.6738,
                y: 0.7031,
            },
            false,
        );
        elder.unlocked = false;
        elder.next_spawn_at = None;
        entities.insert("elder".to_string(), elder);

        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities,
            extra: HashMap::new(),
        };

        let killer_blue =
            test_champion("jgl-blue", "blue", "JGL", "bot", Vec2 { x: 0.67, y: 0.70 });
        let killer_red = test_champion("jgl-red", "red", "JGL", "bot", Vec2 { x: 0.67, y: 0.70 });
        let mut runtime = test_runtime(vec![killer_blue, killer_red], vec![], vec![], neutral);

        let mut timers = decode_neutral_timers_state(&runtime.neutral_timers)
            .unwrap_or_else(|| panic!("failed to decode neutral timers"));

        runtime.time_sec = 600.0;
        let first_kind = process_dragon_capture(&mut runtime, &mut timers, "blue");
        runtime.time_sec += 5.0;
        let second_kind = process_dragon_capture(&mut runtime, &mut timers, "red");
        runtime.time_sec += 5.0;
        let third_kind = process_dragon_capture(&mut runtime, &mut timers, "blue");
        runtime.time_sec += 5.0;
        let fourth_kind = process_dragon_capture(&mut runtime, &mut timers, "red");

        assert_ne!(first_kind, second_kind);
        assert_ne!(third_kind, first_kind);
        assert_ne!(third_kind, second_kind);
        assert_eq!(fourth_kind, third_kind);

        assert_eq!(
            timers.extra.get("dragonFirstKind").and_then(Value::as_str),
            Some(first_kind.as_str())
        );
        assert_eq!(
            timers.extra.get("dragonSecondKind").and_then(Value::as_str),
            Some(second_kind.as_str())
        );
        assert_eq!(
            timers
                .extra
                .get("dragonSoulRiftKind")
                .and_then(Value::as_str),
            Some(third_kind.as_str())
        );
        assert_eq!(
            timers
                .extra
                .get("dragonCurrentKind")
                .and_then(Value::as_str),
            Some(third_kind.as_str())
        );
    }

    #[test]
    fn champion_levels_up_when_xp_threshold_reached() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let champion = test_champion("mid-blue", "blue", "MID", "mid", Vec2 { x: 0.5, y: 0.5 });
        let mut runtime = test_runtime(vec![champion], vec![], vec![], neutral);
        let champion_id = runtime.champions[0].id.clone();

        add_gold_xp_to_champion(&mut runtime, &champion_id, 0, 700);

        assert!(runtime.champions[0].level >= 3);
        assert!(runtime.champions[0].max_hp > 100.0);
    }

    #[test]
    fn nexus_is_not_targetable_while_nexus_towers_alive() {
        let neutral = NeutralTimersRuntime {
            dragon_soul_unlocked: false,
            elder_unlocked: false,
            entities: HashMap::new(),
            extra: HashMap::new(),
        };

        let laner = test_champion("mid-blue", "blue", "MID", "mid", Vec2 { x: 0.885, y: 0.12 });
        let mut nexus = test_structure(
            "red-nexus",
            "red",
            "base",
            Vec2 {
                x: 0.8912760416666666,
                y: 0.1171875,
            },
        );
        nexus.kind = "nexus".to_string();
        let nexus_tower = test_structure(
            "red-nexus-top-tower",
            "red",
            "base",
            Vec2 {
                x: 0.845703125,
                y: 0.1328125,
            },
        );

        let runtime = test_runtime(
            vec![laner],
            vec![],
            vec![nexus, nexus_tower],
            neutral.clone(),
        );
        let target = pick_combat_target(&runtime, 0, runtime.time_sec, &neutral);

        assert!(
            !matches!(target, Some(CombatTarget::Structure(idx)) if runtime.structures[idx].kind == "nexus")
        );
    }
}
