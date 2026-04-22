use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};

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
    pub wave_spawn_at: f64,
    pub next_minion_id: u64,
    pub snapshot: Value,
    pub champion_by_player_id: HashMap<String, String>,
    pub champion_profiles_by_id: HashMap<String, LolChampionCombatProfileInput>,
    pub lane_combat_state_by_champion: HashMap<String, LanerCombatStateRuntime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanerCombatStateRuntime {
    pub last_disengage_at: f64,
    pub reengage_at: f64,
    pub recent_trade_until: f64,
}

impl Default for LanerCombatStateRuntime {
    fn default() -> Self {
        Self {
            last_disengage_at: -999.0,
            reengage_at: -999.0,
            recent_trade_until: -999.0,
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
pub struct LolSimV2InitRequest {
    pub session_id: String,
    pub seed: String,
    pub snapshot: Value,
    #[serde(default)]
    pub champion_by_player_id: HashMap<String, String>,
    #[serde(default)]
    pub champion_profiles_by_id: HashMap<String, LolChampionCombatProfileInput>,
    pub initial_state: Option<Value>,
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

pub fn init(store: &LolSimV2StoreState, request: LolSimV2InitRequest) -> Result<LolSimV2StateResponse, String> {
    if request.session_id.trim().is_empty() {
        return Err("sessionId is required".to_string());
    }

    let mut state = create_initial_state(
        &request.seed,
        &request.snapshot,
        &request.champion_by_player_id,
        &request.champion_profiles_by_id,
    );
    ensure_runtime_state_defaults(&mut state);

    let session = LolSimV2Session {
        id: request.session_id.clone(),
        seed: request.seed,
        state: state.clone(),
        wave_spawn_at: MINION_FIRST_WAVE_AT,
        next_minion_id: 1,
        snapshot: request.snapshot,
        champion_by_player_id: request.champion_by_player_id,
        champion_profiles_by_id: request.champion_profiles_by_id,
        lane_combat_state_by_champion: HashMap::new(),
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

pub fn tick(store: &LolSimV2StoreState, request: LolSimV2TickRequest) -> Result<LolSimV2StateResponse, String> {
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

    spawn_waves_if_due(&mut runtime, session);
    move_champions(&mut runtime, dt);
    move_minions(&mut runtime, dt);
    resolve_minion_combat(&mut runtime);
    resolve_champion_combat(&mut runtime);
    resolve_structure_combat(&mut runtime);
    tick_neutral_timers(&mut runtime);
    cleanup_tick(&mut runtime);

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

pub fn reset(store: &LolSimV2StoreState, request: LolSimV2ResetRequest) -> Result<LolSimV2StateResponse, String> {
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
    );
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

fn default_runtime_state() -> Value {
    json!({
        "timeSec": 0.0,
        "running": true,
        "speed": 1.0,
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
        "events": [{ "t": 0.0, "text": "Match started", "type": "info" }]
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
            Value::Array(vec![json!({ "t": 0.0, "text": "Match started", "type": "info" })]),
        );
    }
}

#[derive(Clone)]
struct SnapshotPlayer {
    id: String,
    name: String,
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
    winner: Option<String>,
    show_walls: bool,
    champions: Vec<ChampionRuntime>,
    minions: Vec<MinionRuntime>,
    structures: Vec<StructureRuntime>,
    objectives: Value,
    neutral_timers: Value,
    stats: RuntimeStats,
    events: Vec<RuntimeEvent>,
    #[serde(default, skip)]
    lane_combat_state_by_champion: HashMap<String, LanerCombatStateRuntime>,
    #[serde(default)]
    #[serde(flatten)]
    extra: HashMap<String, Value>,
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
struct ChampionRuntime {
    id: String,
    name: String,
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
    xp: i64,
    level: i64,
    last_damaged_by_champion_id: Option<String>,
    last_damaged_at: f64,
    state: String,
    recall_anchor: Option<Vec2>,
    recall_channel_until: f64,
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

const BASE_POSITION_BLUE: Vec2 = Vec2 {
    x: 0.115,
    y: 0.882,
};
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
    StructureSeed { id: "blue-top-outer", team: "blue", lane: "top", kind: "tower", pos: Vec2 { x: 0.072265625, y: 0.2838541666666667 } },
    StructureSeed { id: "blue-top-inner", team: "blue", lane: "top", kind: "tower", pos: Vec2 { x: 0.099609375, y: 0.5533854166666666 } },
    StructureSeed { id: "blue-top-inhib-tower", team: "blue", lane: "top", kind: "tower", pos: Vec2 { x: 0.09049479166666667, y: 0.69921875 } },
    StructureSeed { id: "blue-mid-outer", team: "blue", lane: "mid", kind: "tower", pos: Vec2 { x: 0.4016927083333333, y: 0.5755208333333334 } },
    StructureSeed { id: "blue-mid-inner", team: "blue", lane: "mid", kind: "tower", pos: Vec2 { x: 0.3470052083333333, y: 0.6705729166666666 } },
    StructureSeed { id: "blue-mid-inhib-tower", team: "blue", lane: "mid", kind: "tower", pos: Vec2 { x: 0.2623697916666667, y: 0.7408854166666666 } },
    StructureSeed { id: "blue-bot-inner", team: "blue", lane: "bot", kind: "tower", pos: Vec2 { x: 0.4720052083333333, y: 0.8958333333333334 } },
    StructureSeed { id: "blue-bot-outer", team: "blue", lane: "bot", kind: "tower", pos: Vec2 { x: 0.720703125, y: 0.9231770833333334 } },
    StructureSeed { id: "blue-bot-inhib-tower", team: "blue", lane: "bot", kind: "tower", pos: Vec2 { x: 0.298828125, y: 0.9127604166666666 } },
    StructureSeed { id: "blue-inhib-top", team: "blue", lane: "base", kind: "inhib", pos: Vec2 { x: 0.08658854166666667, y: 0.7591145833333334 } },
    StructureSeed { id: "blue-inhib-mid", team: "blue", lane: "base", kind: "inhib", pos: Vec2 { x: 0.224609375, y: 0.7864583333333334 } },
    StructureSeed { id: "blue-inhib-bot", team: "blue", lane: "base", kind: "inhib", pos: Vec2 { x: 0.24544270833333334, y: 0.9114583333333334 } },
    StructureSeed { id: "blue-nexus-top-tower", team: "blue", lane: "base", kind: "tower", pos: Vec2 { x: 0.126953125, y: 0.8372395833333334 } },
    StructureSeed { id: "blue-nexus-bot-tower", team: "blue", lane: "base", kind: "tower", pos: Vec2 { x: 0.15950520833333334, y: 0.875 } },
    StructureSeed { id: "blue-nexus", team: "blue", lane: "base", kind: "nexus", pos: Vec2 { x: 0.115234375, y: 0.8815104166666666 } },
    StructureSeed { id: "red-top-outer", team: "red", lane: "top", kind: "tower", pos: Vec2 { x: 0.275390625, y: 0.07161458333333333 } },
    StructureSeed { id: "red-top-inner", team: "red", lane: "top", kind: "tower", pos: Vec2 { x: 0.533203125, y: 0.08203125 } },
    StructureSeed { id: "red-top-inhib-tower", team: "red", lane: "top", kind: "tower", pos: Vec2 { x: 0.912109375, y: 0.3125 } },
    StructureSeed { id: "red-mid-outer", team: "red", lane: "mid", kind: "tower", pos: Vec2 { x: 0.6569010416666666, y: 0.33203125 } },
    StructureSeed { id: "red-mid-inner", team: "red", lane: "mid", kind: "tower", pos: Vec2 { x: 0.595703125, y: 0.44140625 } },
    StructureSeed { id: "red-mid-inhib-tower", team: "red", lane: "mid", kind: "tower", pos: Vec2 { x: 0.740234375, y: 0.26171875 } },
    StructureSeed { id: "red-bot-inner", team: "red", lane: "bot", kind: "tower", pos: Vec2 { x: 0.9016927083333334, y: 0.44921875 } },
    StructureSeed { id: "red-bot-outer", team: "red", lane: "bot", kind: "tower", pos: Vec2 { x: 0.9303385416666666, y: 0.7057291666666666 } },
    StructureSeed { id: "red-bot-inhib-tower", team: "red", lane: "bot", kind: "tower", pos: Vec2 { x: 0.7024739583333334, y: 0.09375 } },
    StructureSeed { id: "red-inhib-top", team: "red", lane: "base", kind: "inhib", pos: Vec2 { x: 0.7545572916666666, y: 0.09114583333333333 } },
    StructureSeed { id: "red-inhib-mid", team: "red", lane: "base", kind: "inhib", pos: Vec2 { x: 0.783203125, y: 0.22395833333333334 } },
    StructureSeed { id: "red-inhib-bot", team: "red", lane: "base", kind: "inhib", pos: Vec2 { x: 0.9108072916666666, y: 0.24869791666666666 } },
    StructureSeed { id: "red-nexus-top-tower", team: "red", lane: "base", kind: "tower", pos: Vec2 { x: 0.845703125, y: 0.1328125 } },
    StructureSeed { id: "red-nexus-bot-tower", team: "red", lane: "base", kind: "tower", pos: Vec2 { x: 0.8717447916666666, y: 0.1640625 } },
    StructureSeed { id: "red-nexus", team: "red", lane: "base", kind: "nexus", pos: Vec2 { x: 0.8912760416666666, y: 0.1171875 } },
];

const MINION_FIRST_WAVE_AT: f64 = 30.0;
const LANE_COMBAT_UNLOCK_AT: f64 = MINION_FIRST_WAVE_AT + 8.0;
const FIRST_WAVE_CONTEST_UNTIL: f64 = MINION_FIRST_WAVE_AT + 45.0;
const CHAMPION_DECISION_CADENCE_SEC: f64 = 0.8;
const MINION_DAMAGE_TO_MINION_MULTIPLIER: f64 = 0.52;
const MINION_DAMAGE_TO_CHAMPION_MULTIPLIER: f64 = 0.44;
const CHAMPION_DAMAGE_TO_MINION_MULTIPLIER: f64 = 0.36;
const RECALL_TRIGGER_HP_RATIO: f64 = 0.34;
const RECALL_CHANNEL_SEC: f64 = 6.5;
const RECALL_REACH_BUFFER_SEC: f64 = 0.8;
const RECALL_SAFE_ENEMY_RADIUS: f64 = 0.2;
const LANE_CHAMPION_TRADE_RADIUS: f64 = 0.19;
const LANE_CHASE_LEASH_RADIUS: f64 = 0.11;
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
const TRADE_RETREAT_HP_RATIO: f64 = 0.36;
const ASSIST_RADIUS: f64 = 0.11;
const CHAMPION_KILL_GOLD: i64 = 300;
const CHAMPION_ASSIST_GOLD_TOTAL: i64 = 150;
const CHAMPION_KILL_XP: i64 = 220;
const CHAMPION_ATTACK_CADENCE_SEC: f64 = 0.85;
const TOWER_SHOT_DAMAGE: f64 = 32.0;
const TOWER_SHOT_DAMAGE_TO_MINION: f64 = 24.0;
const TOWER_ATTACK_RANGE: f64 = 0.08;
const TOWER_ATTACK_CADENCE_SEC: f64 = 1.0;
const TOWER_AGGRO_LOCK_SEC: f64 = 2.6;
const TOWER_AGGRO_VICTIM_RADIUS: f64 = 0.09;
const TOWER_AGGRO_ATTACKER_RADIUS: f64 = 0.10;
const EVENT_CAP: usize = 80;
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
const OBJECTIVE_NEXT_SPAWN_FALLBACK: f64 = 9_999_999.0;
const NAV_GRID_SIZE: usize = 120;
const NAV_PATH_MIN_DIRECT_DIST: f64 = 0.012;
const NAV_PATH_TRIVIAL_NODE_EPSILON: f64 = 0.0095;

const JUNGLE_DISENGAGE_FALLBACK_ORDER_BLUE: [&str; 8] = [
    "gromp-blue",
    "blue-buff-blue",
    "wolves-blue",
    "raptors-blue",
    "red-buff-blue",
    "krugs-blue",
    "scuttle-top",
    "scuttle-bot",
];

const JUNGLE_DISENGAGE_FALLBACK_ORDER_RED: [&str; 8] = [
    "gromp-red",
    "blue-buff-red",
    "wolves-red",
    "raptors-red",
    "red-buff-red",
    "krugs-red",
    "scuttle-bot",
    "scuttle-top",
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
) -> Value {
    // TODO(parity-chunk-b): port movement, wave spawn/advance, and combat systems from TS simulation.ts.
    let champions = create_champions(seed, snapshot, champion_by_player_id, champion_profiles_by_id);
    let structures = create_structures();
    let neutral_timers = build_neutral_timers_state();

    json!({
        "timeSec": 0.0,
        "running": true,
        "speed": 1.0,
        "winner": Value::Null,
        "champions": champions,
        "minions": [],
        "structures": structures,
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
        "showWalls": false,
    })
}

fn create_champions(
    seed: &str,
    snapshot: &Value,
    champion_by_player_id: &HashMap<String, String>,
    champion_profiles_by_id: &HashMap<String, LolChampionCombatProfileInput>,
) -> Vec<Value> {
    let mut rng = Mulberry32::new(hash_seed(seed));
    let mut champions = Vec::new();

    let home_players = snapshot_team_players(snapshot, "home_team");
    let away_players = snapshot_team_players(snapshot, "away_team");

    seed_team(
        &mut champions,
        &home_players,
        "blue",
        BASE_POSITION_BLUE,
        champion_by_player_id,
        champion_profiles_by_id,
        &mut rng,
    );

    seed_team(
        &mut champions,
        &away_players,
        "red",
        BASE_POSITION_RED,
        champion_by_player_id,
        champion_profiles_by_id,
        &mut rng,
    );

    champions
}

fn seed_team(
    champions: &mut Vec<Value>,
    players: &[SnapshotPlayer],
    team: &str,
    base_pos: Vec2,
    champion_by_player_id: &HashMap<String, String>,
    champion_profiles_by_id: &HashMap<String, LolChampionCombatProfileInput>,
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
            .unwrap_or(if attack_type == "ranged" { 0.056 } else { 0.049 });

        champions.push(json!({
            "id": player.id,
            "name": player.name,
            "team": team,
            "role": role_seed.role,
            "lane": role_seed.lane,
            "pos": {
                "x": base_pos.x + role_seed.offset.x,
                "y": base_pos.y + role_seed.offset.y,
            },
            "hp": max_hp,
            "maxHp": max_hp,
            "alive": true,
            "respawnAt": 0.0,
            "attackCdUntil": 0.0,
            "moveSpeed": 0.043 + rng.next_f64() * 0.008,
            "attackRange": attack_range,
            "attackType": attack_type,
            "attackDamage": 14.0 + rng.next_f64() * 5.0,
            "targetPath": [],
            "targetPathIndex": 0,
            "nextDecisionAt": 0.0,
            "kills": 0,
            "deaths": 0,
            "assists": 0,
            "gold": 500,
            "xp": 0,
            "level": 1,
            "lastDamagedByChampionId": Value::Null,
            "lastDamagedAt": -999.0,
            "state": "lane",
            "recallAnchor": Value::Null,
            "recallChannelUntil": 0.0,
        }));
    }
}

fn create_structures() -> Vec<Value> {
    STRUCTURE_LAYOUT
        .iter()
        .map(|s| {
            let hp = match s.kind {
                "nexus" => 2300.0,
                "inhib" => 1500.0,
                _ => 1400.0,
            };
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
        NeutralTimerTemplate { key: "blue-buff-blue", label: "Blue Blue Buff", first_spawn_at: JUNGLE_INITIAL_SPAWN_AT, max_hp: 470.0, respawn_delay_sec: Some(300.0), one_shot: false, window_close_at: None, combat_grace_until: None, unlocked: true, pos: Vec2 { x: 0.24934895833333334, y: 0.4622395833333333 } },
        NeutralTimerTemplate { key: "blue-buff-red", label: "Red Blue Buff", first_spawn_at: JUNGLE_INITIAL_SPAWN_AT, max_hp: 470.0, respawn_delay_sec: Some(300.0), one_shot: false, window_close_at: None, combat_grace_until: None, unlocked: true, pos: Vec2 { x: 0.478515625, y: 0.26171875 } },
        NeutralTimerTemplate { key: "red-buff-blue", label: "Blue Red Buff", first_spawn_at: JUNGLE_INITIAL_SPAWN_AT, max_hp: 500.0, respawn_delay_sec: Some(300.0), one_shot: false, window_close_at: None, combat_grace_until: None, unlocked: true, pos: Vec2 { x: 0.5266927083333334, y: 0.7421875 } },
        NeutralTimerTemplate { key: "red-buff-red", label: "Red Red Buff", first_spawn_at: JUNGLE_INITIAL_SPAWN_AT, max_hp: 500.0, respawn_delay_sec: Some(300.0), one_shot: false, window_close_at: None, combat_grace_until: None, unlocked: true, pos: Vec2 { x: 0.7545572916666666, y: 0.5403645833333334 } },
        NeutralTimerTemplate { key: "wolves-blue", label: "Blue Wolves", first_spawn_at: JUNGLE_INITIAL_SPAWN_AT, max_hp: 380.0, respawn_delay_sec: Some(135.0), one_shot: false, window_close_at: None, combat_grace_until: None, unlocked: true, pos: Vec2 { x: 0.2584635416666667, y: 0.56640625 } },
        NeutralTimerTemplate { key: "wolves-red", label: "Red Wolves", first_spawn_at: JUNGLE_INITIAL_SPAWN_AT, max_hp: 380.0, respawn_delay_sec: Some(135.0), one_shot: false, window_close_at: None, combat_grace_until: None, unlocked: true, pos: Vec2 { x: 0.525390625, y: 0.3528645833333333 } },
        NeutralTimerTemplate { key: "raptors-blue", label: "Blue Raptors", first_spawn_at: JUNGLE_INITIAL_SPAWN_AT, max_hp: 390.0, respawn_delay_sec: Some(135.0), one_shot: false, window_close_at: None, combat_grace_until: None, unlocked: true, pos: Vec2 { x: 0.4759114583333333, y: 0.6432291666666666 } },
        NeutralTimerTemplate { key: "raptors-red", label: "Red Raptors", first_spawn_at: JUNGLE_INITIAL_SPAWN_AT, max_hp: 390.0, respawn_delay_sec: Some(135.0), one_shot: false, window_close_at: None, combat_grace_until: None, unlocked: true, pos: Vec2 { x: 0.748046875, y: 0.4361979166666667 } },
        NeutralTimerTemplate { key: "gromp-blue", label: "Blue Gromp", first_spawn_at: JUNGLE_INITIAL_SPAWN_AT, max_hp: 520.0, respawn_delay_sec: Some(135.0), one_shot: false, window_close_at: None, combat_grace_until: None, unlocked: true, pos: Vec2 { x: 0.14908854166666666, y: 0.43359375 } },
        NeutralTimerTemplate { key: "gromp-red", label: "Red Gromp", first_spawn_at: JUNGLE_INITIAL_SPAWN_AT, max_hp: 520.0, respawn_delay_sec: Some(135.0), one_shot: false, window_close_at: None, combat_grace_until: None, unlocked: true, pos: Vec2 { x: 0.4381510416666667, y: 0.16536458333333334 } },
        NeutralTimerTemplate { key: "krugs-blue", label: "Blue Krugs", first_spawn_at: JUNGLE_INITIAL_SPAWN_AT, max_hp: 560.0, respawn_delay_sec: Some(135.0), one_shot: false, window_close_at: None, combat_grace_until: None, unlocked: true, pos: Vec2 { x: 0.568359375, y: 0.828125 } },
        NeutralTimerTemplate { key: "krugs-red", label: "Red Krugs", first_spawn_at: JUNGLE_INITIAL_SPAWN_AT, max_hp: 560.0, respawn_delay_sec: Some(135.0), one_shot: false, window_close_at: None, combat_grace_until: None, unlocked: true, pos: Vec2 { x: 0.8483072916666666, y: 0.56640625 } },
        NeutralTimerTemplate { key: "scuttle-top", label: "Scuttle Top", first_spawn_at: SCUTTLE_INITIAL_SPAWN_AT, max_hp: 560.0, respawn_delay_sec: Some(150.0), one_shot: false, window_close_at: None, combat_grace_until: None, unlocked: true, pos: Vec2 { x: 0.2845052083333333, y: 0.34765625 } },
        NeutralTimerTemplate { key: "scuttle-bot", label: "Scuttle Bot", first_spawn_at: SCUTTLE_INITIAL_SPAWN_AT, max_hp: 560.0, respawn_delay_sec: Some(150.0), one_shot: false, window_close_at: None, combat_grace_until: None, unlocked: true, pos: Vec2 { x: 0.6998697916666666, y: 0.6419270833333334 } },
        NeutralTimerTemplate { key: "dragon", label: "Dragon", first_spawn_at: 5.0 * 60.0, max_hp: 3600.0, respawn_delay_sec: Some(5.0 * 60.0), one_shot: false, window_close_at: None, combat_grace_until: None, unlocked: true, pos: Vec2 { x: 0.673828125, y: 0.703125 } },
        NeutralTimerTemplate { key: "voidgrubs", label: "Voidgrubs", first_spawn_at: 8.0 * 60.0, max_hp: 2800.0, respawn_delay_sec: None, one_shot: true, window_close_at: Some(VOIDGRUBS_SOFT_CLOSE_AT), combat_grace_until: Some(VOIDGRUBS_HARD_CLOSE_AT), unlocked: true, pos: Vec2 { x: 0.3274739583333333, y: 0.2981770833333333 } },
        NeutralTimerTemplate { key: "herald", label: "Rift Herald", first_spawn_at: 15.0 * 60.0, max_hp: 5500.0, respawn_delay_sec: None, one_shot: true, window_close_at: Some(HERALD_SOFT_CLOSE_AT), combat_grace_until: Some(HERALD_HARD_CLOSE_AT), unlocked: true, pos: Vec2 { x: 0.3274739583333333, y: 0.2981770833333333 } },
        NeutralTimerTemplate { key: "baron", label: "Baron", first_spawn_at: 20.0 * 60.0, max_hp: 9000.0, respawn_delay_sec: Some(6.0 * 60.0), one_shot: false, window_close_at: None, combat_grace_until: None, unlocked: true, pos: Vec2 { x: 0.3274739583333333, y: 0.2981770833333333 } },
        NeutralTimerTemplate { key: "elder", label: "Elder Dragon", first_spawn_at: 0.0, max_hp: 7200.0, respawn_delay_sec: Some(6.0 * 60.0), one_shot: false, window_close_at: None, combat_grace_until: None, unlocked: false, pos: Vec2 { x: 0.673828125, y: 0.703125 } },
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
                    Some(SnapshotPlayer { id, name })
                })
                .collect()
        })
        .unwrap_or_default()
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
    if team == "red" { "red" } else { "blue" }
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
        let cell_distance = ((bx as f64 - ax as f64).powi(2) + (by as f64 - ay as f64).powi(2)).sqrt();
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
                    .then_with(|| heuristic(*a).partial_cmp(&heuristic(*b)).unwrap_or(Ordering::Equal))
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

fn set_champion_direct_path_hysteresis(champion: &mut ChampionRuntime, target: Vec2, min_target_delta: f64) {
    if let Some(current_target) = current_champion_path_target(champion) {
        if dist(current_target, target) <= min_target_delta {
            return;
        }
    }
    set_champion_direct_path(champion, target);
}

fn jungle_disengage_fallback_order_for_team(team: &str) -> &'static [&'static str] {
    if normalized_team(team) == "red" {
        &JUNGLE_DISENGAGE_FALLBACK_ORDER_RED
    } else {
        &JUNGLE_DISENGAGE_FALLBACK_ORDER_BLUE
    }
}

fn pick_jungle_farm_fallback_pos(
    champion: &ChampionRuntime,
    neutral_timers: &NeutralTimersRuntime,
    threat_pos: Option<Vec2>,
) -> Option<Vec2> {
    let mut first_alive_fallback: Option<Vec2> = None;

    for key in jungle_disengage_fallback_order_for_team(&champion.team) {
        let Some(timer) = neutral_timers.entities.get(*key) else {
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

fn jgl_disengage_fallback_pos(runtime: &RuntimeState, champion: &ChampionRuntime, threat_pos: Vec2) -> Vec2 {
    let neutral_timers = decode_neutral_timers_state(&runtime.neutral_timers)
        .unwrap_or_else(|| neutral_timers_default_runtime_state());
    if let Some(camp_pos) = pick_jungle_farm_fallback_pos(champion, &neutral_timers, Some(threat_pos)) {
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

fn lane_fallback_pos_from_tower(champion: &ChampionRuntime, tower_pos: Vec2, toward_base: bool) -> Vec2 {
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

fn lane_minion_context_distance(champion: &ChampionRuntime, pos: Vec2, minions: &[MinionRuntime]) -> f64 {
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
    let _local_pressure = lane_pressure_at(champion, pos, champions, minions, LANE_LOCAL_PRESSURE_RADIUS);

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

    let pressure = lane_pressure_at(champion, target_pos, champions, minions, LANE_LOCAL_PRESSURE_RADIUS);
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

    let pressure = lane_pressure_at(champion, enemy.pos, champions, minions, LANE_LOCAL_PRESSURE_RADIUS);
    if pressure.ally_champions > pressure.enemy_champions && self_hp >= 0.32 {
        return true;
    }

    pressure.ally_score >= pressure.enemy_score + 0.9 && self_hp >= enemy_hp
}

fn can_open_trade_window(
    champion: &ChampionRuntime,
    enemy: &ChampionRuntime,
    now: f64,
    champions: &[ChampionRuntime],
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
    lane_combat_state_by_champion: &HashMap<String, LanerCombatStateRuntime>,
) -> bool {
    if champion.role == "JGL" {
        return true;
    }
    if dist(champion.pos, enemy.pos) > LANE_CHAMPION_TRADE_RADIUS {
        return false;
    }
    if !in_lane_trade_context(champion, champion.pos, false, champions, minions, structures) {
        return false;
    }
    if !in_lane_trade_context(champion, enemy.pos, true, champions, minions, structures) {
        return false;
    }
    if should_force_laner_disengage(champion, enemy.pos, Some(enemy), champions, minions, structures) {
        return false;
    }
    let clear_win_condition = should_commit_all_in_trade(champion, enemy, champions, minions);
    if (lane_trade_cooldown_active(champion, now, lane_combat_state_by_champion)
        || lane_recent_trade_lock_active(champion, now, lane_combat_state_by_champion))
        && !clear_win_condition
    {
        return false;
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

    let pressure = lane_pressure_at(champion, enemy.pos, champions, minions, LANE_LOCAL_PRESSURE_RADIUS);
    let numbers_advantage = pressure.ally_champions > pressure.enemy_champions;
    if numbers_advantage && hp_ratio + 0.02 >= enemy_hp_ratio && hp_ratio >= 0.32 {
        return true;
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
        return false;
    }
    if is_first_wave_contest_active(champion, now)
        && (ally_minions_near_fight < 2 || enemy_minions_near_fight < 2)
    {
        return false;
    }
    if ally_minions_near_fight == 0 {
        let low_enemy_window = enemy_hp_ratio <= 0.34;
        let hp_safe_to_trade = hp_ratio >= 0.5;
        if !(low_enemy_window && hp_safe_to_trade) {
            return false;
        }
    }

    let hp_advantage = hp_ratio + 0.08 >= enemy_hp_ratio;
    let wave_pressure = pressure.ally_lane_minions >= pressure.enemy_lane_minions;
    let score_pressure = pressure.ally_score >= pressure.enemy_score - 0.05;
    hp_advantage && wave_pressure && score_pressure
}

fn should_disengage_champion_trade(
    champion: &ChampionRuntime,
    enemy: &ChampionRuntime,
    champions: &[ChampionRuntime],
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
) -> bool {
    if champion.role == "JGL" {
        return false;
    }

    if should_force_laner_disengage(champion, enemy.pos, Some(enemy), champions, minions, structures) {
        return true;
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
    if self_hp_ratio < TRADE_RETREAT_HP_RATIO {
        return true;
    }
    if self_hp_ratio + TRADE_HP_DISADVANTAGE_ALLOWANCE < enemy_hp_ratio {
        return true;
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
        return true;
    }

    let lane_anchor = lane_anchor_pos(champion, minions, structures);
    dist(enemy.pos, lane_anchor) > LANE_CHASE_LEASH_RADIUS && enemy_pressure >= allied_pressure
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
        let contest_advance = f64::max(0.014, f64::min(approach * 0.95, dist(lane_anchor, wave_front) * 0.6));
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
                x: clamp(tower.pos.x + to_wave_from_tower.x * front_offset, 0.01, 0.99),
                y: clamp(tower.pos.y + to_wave_from_tower.y * front_offset, 0.01, 0.99),
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
    let Some(tower_idx) = pick_allied_lane_fallback_tower(champion, threat_pos, emergency, structures) else {
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

    let pressure = lane_pressure_at(champion, threat_pos, champions, minions, LANE_LOCAL_PRESSURE_RADIUS);
    let strongly_unfavorable = pressure.enemy_score
        >= pressure.ally_score + profile.outnumber_tolerance + LANE_STRONG_UNFAVORABLE_PRESSURE_DELTA
        || pressure.enemy_champions >= pressure.ally_champions + 1;
    if !strongly_unfavorable {
        return false;
    }

    hp_ratio < LANE_HEALTHY_RETREAT_HP_RATIO || pressure.enemy_champions >= pressure.ally_champions + 2
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

    towers.sort_by(|(idx_a, path_a), (idx_b, path_b)| path_a.cmp(path_b).then_with(|| idx_a.cmp(idx_b)));
    if towers.is_empty() {
        return None;
    }

    let threat_index = closest_lane_path_index(threat_pos, &lane_path);
    let mut selected = towers
        .iter()
        .filter(|(_, path_index)| *path_index <= threat_index + 1)
        .max_by(|(idx_a, path_a), (idx_b, path_b)| path_a.cmp(path_b).then_with(|| idx_a.cmp(idx_b)))
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
    lane_defense_band.sort_by(|(idx_a, path_a), (idx_b, path_b)| path_b.cmp(path_a).then_with(|| idx_a.cmp(idx_b)));
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
        let nearest = nearest_enemy_champion_snapshot(champion, champions, RECALL_SAFE_ENEMY_RADIUS)
            .or_else(|| nearest_enemy_champion_snapshot(champion, champions, f64::INFINITY));
        if let Some(threat) = nearest {
            if champion.role == "JGL" {
                Some(recall_fallback_toward_base(champion, Some(threat)))
            } else {
                Some(lane_retreat_anchor_pos(
                    champion,
                    threat.pos,
                    now,
                    champions,
                    minions,
                    structures,
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
        push_event(events, now, &format!("{} recall interrupted", champion.name), "recall");
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
        champion.state = "lane".to_string();
        champion.recall_anchor = None;
        champion.recall_channel_until = 0.0;
        champion.target_path.clear();
        champion.target_path_index = 0;
        champion.next_decision_at = now;
        push_event(events, now, &format!("{} recalled", champion.name), "recall");
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
    push_event(events, now, &format!("{} started recall", champion.name), "recall");
    true
}

fn decide_champion_state(
    champion: &mut ChampionRuntime,
    now: f64,
    minions: &[MinionRuntime],
    structures: &[StructureRuntime],
    champions: &[ChampionRuntime],
    neutral_timers: Option<&NeutralTimersRuntime>,
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
            if let Some(attempt) = active_objective_attempt_for_team(&champion.team, champions, timers) {
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
            if let Some(objective_pos) = pick_macro_objective_pos(champion, timers, now) {
                champion.state = "objective".to_string();
                set_champion_direct_path_hysteresis(
                    champion,
                    objective_pos,
                    OBJECTIVE_PATH_MIN_TARGET_DELTA,
                );
                return;
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

    let Some(attempt) = active_objective_attempt_for_team(&champion.team, champions, neutral_timers) else {
        return false;
    };

    let lane = normalized_lane(&champion.lane);
    if !objective_adjacent_lanes(&attempt.key).iter().any(|adj| *adj == lane) {
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

    if nearby_contestants == 0 && attempt.hp > attempt.max_hp * 0.82 {
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

fn pick_macro_objective_pos(
    champion: &ChampionRuntime,
    neutral_timers: &NeutralTimersRuntime,
    now: f64,
) -> Option<Vec2> {
    if champion.role != "JGL" {
        return None;
    }

    let objective_lead_time = 35.0;
    for key in [
        "elder",
        "baron",
        "herald",
        "voidgrubs",
        "dragon",
        "scuttle-top",
        "scuttle-bot",
        "blue-buff-blue",
        "blue-buff-red",
        "red-buff-blue",
        "red-buff-red",
        "wolves-blue",
        "wolves-red",
        "raptors-blue",
        "raptors-red",
        "gromp-blue",
        "gromp-red",
        "krugs-blue",
        "krugs-red",
    ] {
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
        x: clamp(origin.x - direction.x * depth + perpendicular.x * lateral, 0.01, 0.99),
        y: clamp(origin.y - direction.y * depth + perpendicular.y * lateral, 0.01, 0.99),
    }
}

fn move_champions(runtime: &mut RuntimeState, dt: f64) {
    let now = runtime.time_sec;
    let champion_snapshot = runtime.champions.clone();
    let neutral_timers_snapshot = decode_neutral_timers_state(&runtime.neutral_timers);

    for champion in &mut runtime.champions {
        if !champion.alive {
            if now >= champion.respawn_at {
                champion.alive = true;
                champion.hp = champion.max_hp;
                champion.pos = base_position_for(&champion.team);
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

        if let Some(target) = champion.target_path.get(champion.target_path_index).copied() {
            move_entity(&mut champion.pos, target, champion.move_speed, dt);
            if dist(champion.pos, target) < 0.01
                && champion.target_path_index < champion.target_path.len().saturating_sub(1)
            {
                champion.target_path_index += 1;
            }
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
            if dist(minion.pos, target) < 0.01 && minion.path_index < minion.path.len().saturating_sub(1) {
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

        let cadence = minion_stats(&runtime.minions[i].kind).3;
        let structure_range = runtime.minions[i].attack_range.max(MINION_STRUCTURE_AGGRO_RANGE);
        let enemy_structure = nearest_enemy_structure_index(
            &runtime.structures,
            &runtime.minions[i].team,
            &runtime.minions[i].lane,
            runtime.minions[i].pos,
            structure_range,
        );

        if let Some(structure_idx) = enemy_structure {
            if !runtime.structures[structure_idx].alive {
                continue;
            }

            let attacker_team = runtime.minions[i].team.clone();
            let damage = runtime.minions[i].attack_damage;
            runtime.structures[structure_idx].hp -= damage;
            runtime.minions[i].attack_cd_until = now + cadence;

            if runtime.structures[structure_idx].hp <= 0.0 {
                destroy_structure(runtime, structure_idx, &attacker_team);
            }
            continue;
        }

        let enemy_minion = nearest_enemy_minion_index(&runtime.minions, i, runtime.minions[i].attack_range.max(0.05));

        if let Some(enemy_idx) = enemy_minion {
            let damage = runtime.minions[i].attack_damage * MINION_DAMAGE_TO_MINION_MULTIPLIER;
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

        let attacker_team = runtime.minions[i].team.clone();
        let attacker_lane = runtime.minions[i].lane.clone();
        let attacker_pos = runtime.minions[i].pos;
        let attacker_damage = runtime.minions[i].attack_damage;
        let attacker_range = runtime.minions[i].attack_range.max(MINION_CHAMPION_AGGRO_MIN_RANGE);

        let enemy_champion = nearest_enemy_champion_for_minion(
            &runtime.champions,
            &attacker_team,
            &attacker_lane,
            attacker_pos,
            attacker_range,
        );

        if let Some(champion_idx) = enemy_champion {
            runtime.champions[champion_idx].hp -= attacker_damage * MINION_DAMAGE_TO_CHAMPION_MULTIPLIER;
            runtime.champions[champion_idx].last_damaged_by_champion_id = None;
            runtime.champions[champion_idx].last_damaged_at = now;
            cancel_recall(&mut runtime.champions[champion_idx], now, &mut runtime.events);
            runtime.minions[i].attack_cd_until = now + cadence;

            if runtime.champions[champion_idx].hp <= 0.0 && runtime.champions[champion_idx].alive {
                runtime.champions[champion_idx].alive = false;
                runtime.champions[champion_idx].deaths += 1;
                runtime.champions[champion_idx].respawn_at = now + 12.0;
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
        .filter(|s| s.alive && s.kind == "tower" && normalized_team(&s.team) == normalized_team(&champion.team))
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
        0.14
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
    let enemy_hp_ratio = if enemy.max_hp <= 0.0 { 1.0 } else { enemy.hp / enemy.max_hp };
    let low_enemy = enemy_hp_ratio <= 0.48;

    (ttk_enemy <= ttk_self * 0.95 || low_enemy) && ally_pressure + 0.5 >= enemy_pressure
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
                && has_credible_kill_chance(runtime, champion_idx, *idx, now)
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
    if let Some(enemy_idx) = kill_window_enemy {
        return Some(CombatTarget::Champion(enemy_idx));
    }

    if champion.role == "JGL" {
        if let Some(neutral_key) = nearest_attackable_neutral_key(
            champion,
            neutral_timers,
            JUNGLE_CAMP_ENGAGE_RADIUS,
            OBJECTIVE_ATTEMPT_RADIUS,
        ) {
            return Some(CombatTarget::Neutral(neutral_key));
        }

        let nearby_enemy = runtime
            .champions
            .iter()
            .enumerate()
            .filter(|(_, enemy)| {
                enemy.alive
                    && normalized_team(&enemy.team) == enemy_team
                    && dist(champion.pos, enemy.pos) <= 0.13
            })
            .min_by(|(idx_a, a), (idx_b, b)| {
                dist(champion.pos, a.pos)
                    .partial_cmp(&dist(champion.pos, b.pos))
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| idx_a.cmp(idx_b))
            })
            .map(|(idx, _)| idx);
        return nearby_enemy.map(CombatTarget::Champion);
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
            dist(champion.pos, a.pos)
                .partial_cmp(&dist(champion.pos, b.pos))
                .unwrap_or(Ordering::Equal)
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
                && dist(champion.pos, enemy.pos) <= 0.12
                && runtime.champions.iter().any(|ally| {
                    ally.alive
                        && normalized_team(&ally.team) == normalized_team(&champion.team)
                        && dist(ally.pos, champion.pos) <= 0.12
                        && ally
                            .last_damaged_by_champion_id
                            .as_ref()
                            .map(|id| id == &enemy.id)
                            .unwrap_or(false)
                        && now - ally.last_damaged_at <= 2.4
                })
                && can_open_trade_window(
                    champion,
                    enemy,
                    now,
                    &runtime.champions,
                    &runtime.minions,
                    &runtime.structures,
                    &runtime.lane_combat_state_by_champion,
                )
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            dist(champion.pos, a.pos)
                .partial_cmp(&dist(champion.pos, b.pos))
                .unwrap_or(Ordering::Equal)
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx);
    if let Some(enemy_idx) = threatening_enemy {
        return Some(CombatTarget::Champion(enemy_idx));
    }

    if let Some(enemy_idx) = enemy_pressuring_allied_tower_idx(champion, &runtime.champions, &runtime.structures) {
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
                )
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
    if let Some(enemy_idx) = numbers_advantage_enemy {
        return Some(CombatTarget::Champion(enemy_idx));
    }

    let objective_assist_active = should_assist_objective_attempt(champion, &runtime.champions, neutral_timers);
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
                && dist(champion.pos, m.pos) <= laner_farm_search_radius(champion)
                && m.hp
                    <= champion.attack_damage * CHAMPION_DAMAGE_TO_MINION_MULTIPLIER * 1.08
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
                && dist(champion.pos, enemy.pos) <= LANE_CHAMPION_TRADE_RADIUS
                && can_open_trade_window(
                    champion,
                    enemy,
                    now,
                    &runtime.champions,
                    &runtime.minions,
                    &runtime.structures,
                    &runtime.lane_combat_state_by_champion,
                )
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            dist(champion.pos, a.pos)
                .partial_cmp(&dist(champion.pos, b.pos))
                .unwrap_or(Ordering::Equal)
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
                && (normalized_lane(&s.lane) == normalized_lane(&champion.lane) || s.kind == "nexus")
                && dist(champion.pos, s.pos) <= LANE_STRUCTURE_PRESSURE_RADIUS)
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
                || !(normalized_lane(&s.lane) == normalized_lane(&champion.lane) || s.kind == "nexus")
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
                )
        })
        .min_by(|(idx_a, a), (idx_b, b)| {
            dist(champion.pos, a.pos)
                .partial_cmp(&dist(champion.pos, b.pos))
                .unwrap_or(Ordering::Equal)
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx);

    let nearby_neutral = nearest_attackable_neutral_key(champion, neutral_timers, JUNGLE_CAMP_ENGAGE_RADIUS, 0.0)
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
                score: d - 0.015,
                distance: d,
                kind_rank: 0,
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

    fallback_candidates.first().map(|candidate| candidate.target.clone())
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

fn is_local_combat_target(runtime: &RuntimeState, champion_idx: usize, target: &CombatTarget) -> bool {
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
    if matches!(target, CombatTarget::Structure(_)) && target_distance > LOCAL_STRUCTURE_ENGAGE_RADIUS {
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

    for idx in 0..runtime.champions.len() {
        if !runtime.champions[idx].alive
            || runtime.champions[idx].state == "recall"
            || now < runtime.champions[idx].attack_cd_until
        {
            continue;
        }

        let team = normalized_team(&runtime.champions[idx].team).to_string();
        let attack_range = runtime.champions[idx].attack_range.max(0.04);
        let is_hard_assist = {
            let contested = contested_dragon_attempt_for_team(&team, &runtime.champions, &neutral_timers);
            should_hard_assist_contested_dragon(&runtime.champions[idx], contested)
        };

        if is_hard_assist {
            if let Some(dragon) = contested_dragon_attempt_for_team(&team, &runtime.champions, &neutral_timers) {
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

        let is_objective_assist =
            should_assist_objective_attempt(&runtime.champions[idx], &runtime.champions, &neutral_timers);
        if is_objective_assist && runtime.champions[idx].state == "objective" {
            if let Some(attempt) = active_objective_attempt_for_team(&team, &runtime.champions, &neutral_timers) {
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

                if attacker_snapshot.role != "JGL"
                    && !can_open_trade_window(
                        &attacker_snapshot,
                        &target_snapshot,
                        now,
                        &runtime.champions,
                        &runtime.minions,
                        &runtime.structures,
                        &runtime.lane_combat_state_by_champion,
                    )
                {
                    issue_lane_disengage(runtime, idx, target_snapshot.pos);
                    continue;
                }

                if should_disengage_champion_trade(
                    &attacker_snapshot,
                    &target_snapshot,
                    &runtime.champions,
                    &runtime.minions,
                    &runtime.structures,
                ) {
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
                let damage = runtime.champions[idx].attack_damage * CHAMPION_DAMAGE_TO_MINION_MULTIPLIER;
                runtime.minions[minion_idx].hp -= damage;
                runtime.minions[minion_idx].last_hit_by_champion_id = Some(runtime.champions[idx].id.clone());
                runtime.champions[idx].attack_cd_until = now + 0.75;
                if runtime.minions[minion_idx].hp <= 0.0 {
                    register_minion_death(runtime, minion_idx);
                }
                continue;
            }
            CombatTarget::Structure(structure_idx) => {
                if structure_idx >= runtime.structures.len() || !runtime.structures[structure_idx].alive {
                    continue;
                }
                runtime.structures[structure_idx].hp -= runtime.champions[idx].attack_damage;
                runtime.champions[idx].attack_cd_until = now + 0.9;
                if runtime.structures[structure_idx].hp <= 0.0 {
                    destroy_structure(runtime, structure_idx, &team);
                }
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
            runtime.minions[minion_idx].hp -= TOWER_SHOT_DAMAGE_TO_MINION;
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

    if is_jungle_camp_key(key) {
        if let Some((gold, xp)) = jungle_camp_reward(key) {
            add_gold_xp_to_champion(runtime, &killer_id, gold, xp);
        }
        log_event(runtime, &format!("{} cleared {}", killer_name, timer_label), "info");
        return;
    }

    if key == "dragon" {
        team_stats_mut(&mut runtime.stats, &killer_team).dragons += 1;
        add_gold_xp_to_champion(runtime, &killer_id, DRAGON_SECURE_GOLD, DRAGON_SECURE_XP);
        log_event(
            runtime,
            &format!("{} secured dragon", normalized_team(&killer_team).to_uppercase()),
            "dragon",
        );
        return;
    }

    if key == "baron" {
        team_stats_mut(&mut runtime.stats, &killer_team).barons += 1;
        add_gold_xp_to_champion(runtime, &killer_id, BARON_SECURE_GOLD, BARON_SECURE_XP);
        log_event(
            runtime,
            &format!("{} secured baron", normalized_team(&killer_team).to_uppercase()),
            "baron",
        );
        return;
    }

    if is_objective_neutral_key(key) {
        add_gold_xp_to_champion(runtime, &killer_id, OBJECTIVE_SECURE_GOLD, OBJECTIVE_SECURE_XP);
        log_event(
            runtime,
            &format!("{} secured {}", normalized_team(&killer_team).to_uppercase(), timer_label),
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
    if let Some(timer_mut) = neutral_timers.entities.get_mut(key) {
        timer_mut.hp -= damage;
        killed = timer_mut.hp <= 0.0;
    }
    if killed {
        mark_neutral_taken(runtime, neutral_timers, key, Some(champion_idx));
    }

    true
}

fn sync_objectives_from_neutral_timers(runtime: &mut RuntimeState, neutral_timers: &NeutralTimersRuntime) {
    let Some(objectives) = runtime.objectives.as_object_mut() else {
        return;
    };

    if let Some(dragon_timer) = neutral_timers.entities.get("dragon") {
        if let Some(dragon_obj) = objectives.get_mut("dragon").and_then(Value::as_object_mut) {
            dragon_obj.insert("alive".to_string(), Value::from(dragon_timer.alive));
            dragon_obj.insert(
                "nextSpawnAt".to_string(),
                Value::from(dragon_timer.next_spawn_at.unwrap_or(OBJECTIVE_NEXT_SPAWN_FALLBACK)),
            );
        }
    }

    if let Some(baron_timer) = neutral_timers.entities.get("baron") {
        if let Some(baron_obj) = objectives.get_mut("baron").and_then(Value::as_object_mut) {
            baron_obj.insert("alive".to_string(), Value::from(baron_timer.alive));
            baron_obj.insert(
                "nextSpawnAt".to_string(),
                Value::from(baron_timer.next_spawn_at.unwrap_or(OBJECTIVE_NEXT_SPAWN_FALLBACK)),
            );
        }
    }
}

fn tick_neutral_timers(runtime: &mut RuntimeState) {
    let mut neutral_timers = decode_neutral_timers_state(&runtime.neutral_timers)
        .unwrap_or_else(|| neutral_timers_default_runtime_state());
    let now = runtime.time_sec;

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
                        timer.alive = false;
                        timer.hp = 0.0;
                        timer.next_spawn_at = None;
                        despawn_text = Some(format!("{} despawned", timer.label));
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

fn should_engage_enemy_champion(runtime: &RuntimeState, attacker_idx: usize, target_idx: usize) -> bool {
    if attacker_idx >= runtime.champions.len() || target_idx >= runtime.champions.len() {
        return false;
    }

    let attacker = &runtime.champions[attacker_idx];
    let target = &runtime.champions[target_idx];
    if !attacker.alive || !target.alive || normalized_team(&attacker.team) == normalized_team(&target.team) {
        return false;
    }

    let hp_ratio = if attacker.max_hp <= 0.0 {
        1.0
    } else {
        attacker.hp / attacker.max_hp
    };

    if attacker.role != "JGL" && hp_ratio <= TRADE_RETREAT_HP_RATIO {
        return false;
    }

    if attacker.role != "JGL"
        && !can_open_trade_window(
            attacker,
            target,
            runtime.time_sec,
            &runtime.champions,
            &runtime.minions,
            &runtime.structures,
            &runtime.lane_combat_state_by_champion,
        )
    {
        return false;
    }

    if should_disengage_champion_trade(
        attacker,
        target,
        &runtime.champions,
        &runtime.minions,
        &runtime.structures,
    ) {
        return false;
    }

    can_champion_tower_dive(runtime, attacker, target)
}

fn can_champion_tower_dive(runtime: &RuntimeState, attacker: &ChampionRuntime, target: &ChampionRuntime) -> bool {
    let defending_tower = runtime
        .structures
        .iter()
        .find(|structure| {
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
    if attacker_hp_ratio < 0.48 {
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
    let enemy_nearby = runtime
        .champions
        .iter()
        .filter(|champion| {
            champion.alive
                && normalized_team(&champion.team) == normalized_team(&target.team)
                && dist(champion.pos, target.pos) <= 0.12
        })
        .count();

    if allied_minions_near_tower == 0 && attacker_hp_ratio < 0.65 {
        return false;
    }

    ally_nearby >= enemy_nearby
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

        defender.hp -= attacker.attack_damage;
        defender.last_damaged_by_champion_id = Some(attacker.id.clone());
        defender.last_damaged_at = now;
        cancel_recall(defender, now, &mut runtime.events);
        attacker.attack_cd_until = now + CHAMPION_ATTACK_CADENCE_SEC;

        if defender.hp <= 0.0 && defender.alive {
            defender.alive = false;
            defender.hp = 0.0;
            defender.deaths += 1;
            defender.respawn_at = now + 12.0;
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

        defender.hp -= attacker.attack_damage;
        defender.last_damaged_by_champion_id = Some(attacker.id.clone());
        defender.last_damaged_at = now;
        cancel_recall(defender, now, &mut runtime.events);
        attacker.attack_cd_until = now + CHAMPION_ATTACK_CADENCE_SEC;

        if defender.hp <= 0.0 && defender.alive {
            defender.alive = false;
            defender.hp = 0.0;
            defender.deaths += 1;
            defender.respawn_at = now + 12.0;
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

    let killer_team_stats = team_stats_mut(&mut runtime.stats, &killer_team);
    killer_team_stats.kills += 1;
    add_gold_xp_to_champion(runtime, &killer_id, CHAMPION_KILL_GOLD, CHAMPION_KILL_XP);

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
        let shared_xp = (CHAMPION_KILL_XP / 2) / assisters.len() as i64;
        for assist_id in assisters {
            if let Some(champion) = runtime.champions.iter_mut().find(|champion| champion.id == assist_id) {
                champion.assists += 1;
            }
            add_gold_xp_to_champion(runtime, &assist_id, shared_gold, shared_xp);
        }
    }

    log_event(runtime, &format!("{} killed {}", killer_name, victim_name), "kill");
}

fn mark_tower_aggro_on_champion_attack(
    runtime: &mut RuntimeState,
    attacker: &ChampionRuntime,
    victim: &ChampionRuntime,
    now: f64,
) {
    for tower in &mut runtime.structures {
        if !tower.alive || tower.kind != "tower" || normalized_team(&tower.team) != normalized_team(&victim.team) {
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

fn apply_tower_shot_to_champion(runtime: &mut RuntimeState, structure_idx: usize, champion_idx: usize) {
    let now = runtime.time_sec;
    runtime.champions[champion_idx].hp -= TOWER_SHOT_DAMAGE;
    runtime.champions[champion_idx].last_damaged_by_champion_id = None;
    runtime.champions[champion_idx].last_damaged_at = now;
    cancel_recall(&mut runtime.champions[champion_idx], now, &mut runtime.events);
    runtime.structures[structure_idx].attack_cd_until = now + TOWER_ATTACK_CADENCE_SEC;
    if runtime.champions[champion_idx].hp <= 0.0 && runtime.champions[champion_idx].alive {
        runtime.champions[champion_idx].alive = false;
        runtime.champions[champion_idx].hp = 0.0;
        runtime.champions[champion_idx].deaths += 1;
        runtime.champions[champion_idx].respawn_at = now + 12.0;
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
        let team_stats = team_stats_mut(&mut runtime.stats, &champion.team);
        team_stats.gold += gold;
    }
}

fn register_minion_death(runtime: &mut RuntimeState, minion_idx: usize) {
    if !runtime.minions[minion_idx].alive {
        return;
    }

    runtime.minions[minion_idx].alive = false;
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
        if let Some(champion) = runtime
            .champions
            .iter_mut()
            .find(|champion| champion.id == champion_id)
        {
            champion.gold += gold;
            champion.xp += xp;
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
    from: Vec2,
    range: f64,
) -> Option<usize> {
    champions
        .iter()
        .enumerate()
        .filter(|(_, enemy)| {
            enemy.alive
                && normalized_team(&enemy.team) != normalized_team(attacker_team)
                && normalized_lane(&enemy.lane) == normalized_lane(attacker_lane)
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

fn nearest_enemy_minion_index(minions: &[MinionRuntime], source_idx: usize, range: f64) -> Option<usize> {
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
                && (normalized_lane(&structure.lane) == normalized_lane(lane) || structure.kind == "nexus")
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
    push_event(&mut runtime.events, runtime.time_sec, text, kind);
}

fn team_stats_mut<'a>(stats: &'a mut RuntimeStats, team: &str) -> &'a mut RuntimeTeamStats {
    if normalized_team(team) == "red" {
        &mut stats.red
    } else {
        &mut stats.blue
    }
}

fn cleanup_tick(runtime: &mut RuntimeState) {
    runtime
        .minions
        .retain(|minion| minion.alive && minion.path_index < minion.path.len());

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
            xp: 0,
            level: 1,
            last_damaged_by_champion_id: None,
            last_damaged_at: -999.0,
            state: "lane".to_string(),
            recall_anchor: None,
            recall_channel_until: 0.0,
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
            winner: None,
            show_walls: false,
            champions,
            minions,
            structures,
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
            path.iter()
                .all(|p| !active_nav_walls().iter().any(|w| point_in_polygon(*p, &w.points))),
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
        blue.path = vec![
            Vec2 { x: 0.5, y: 0.5 },
            Vec2 { x: 0.7, y: 0.5 },
        ];
        blue.path_index = 1;

        let mut red = test_minion("m-red-1", "red", "mid", Vec2 { x: 0.54, y: 0.5 });
        red.path = vec![
            Vec2 { x: 0.54, y: 0.5 },
            Vec2 { x: 0.3, y: 0.5 },
        ];
        red.path_index = 1;

        let start_pos = blue.pos;
        let mut runtime = test_runtime(vec![], vec![blue, red], vec![], neutral);

        move_minions(&mut runtime, 0.05);

        assert!(dist(runtime.minions[0].pos, start_pos) < 1e-6);
    }

    #[test]
    fn minion_prioritizes_structure_over_minion_when_both_in_range() {
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
        let mut red_tower = test_structure("red-mid-outer", "red", "mid", Vec2 { x: 0.535, y: 0.5 });
        red_tower.hp = 100.0;

        let mut runtime = test_runtime(vec![], vec![blue, red_minion], vec![red_tower], neutral);

        let tower_hp_before = runtime.structures[0].hp;
        let minion_hp_before = runtime.minions[1].hp;
        resolve_minion_combat(&mut runtime);

        assert!(runtime.structures[0].hp < tower_hp_before);
        assert_eq!(runtime.minions[1].hp, minion_hp_before);
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

        let runtime = test_runtime(vec![adc, jungler, enemy], vec![minion], vec![], neutral.clone());

        let target = pick_combat_target(&runtime, 0, runtime.time_sec, &neutral);
        assert!(matches!(target, Some(CombatTarget::Neutral(ref key)) if key == "dragon"));
    }

    #[test]
    fn structure_pressure_blocked_with_two_enemy_minions_near_tower() {
        let laner = test_champion("top-blue", "blue", "TOP", "top", Vec2 { x: 0.28, y: 0.09 });
        let tower = test_structure("red-top-outer", "red", "top", Vec2 { x: 0.275390625, y: 0.07161458333333333 });

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
}
