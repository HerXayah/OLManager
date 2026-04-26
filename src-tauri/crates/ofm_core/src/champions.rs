use crate::game::Game;
use crate::staff_effects::LolStaffEffects;
use chrono::{Datelike, NaiveDate};
use domain::message::{InboxMessage, MessageCategory, MessagePriority};
use domain::staff::StaffRole;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

const MIN_MASTERY: u8 = 25;
const MASTERY_CAP: u8 = 100;
const PATCH_INTERVAL_DAYS: i64 = 14;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoloQTier {
    Challenger,
    Grandmaster,
    Master,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChampionPatchChange {
    Buff,
    Nerf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChampionMasteryEntry {
    pub player_id: String,
    pub champion_id: String,
    pub mastery: u8,
    pub last_active_on: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChampionMetaEntry {
    pub champion_id: String,
    pub role: String,
    #[serde(default = "default_meta_tier")]
    pub tier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChampionPatchNote {
    pub champion_id: String,
    pub role: String,
    pub change: ChampionPatchChange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChampionPatchState {
    pub current_patch: u32,
    #[serde(default)]
    pub current_patch_label: String,
    #[serde(default)]
    pub patch_year: u16,
    #[serde(default)]
    pub patch_index_in_year: u16,
    pub last_patch_date: Option<String>,
    #[serde(default)]
    pub hidden_meta: Vec<ChampionMetaEntry>,
    #[serde(default)]
    pub patch_notes: Vec<ChampionPatchNote>,
    #[serde(default)]
    pub discovered_champion_ids: Vec<String>,
    #[serde(default)]
    pub rng_seed: u64,
}

impl Default for ChampionPatchState {
    fn default() -> Self {
        Self {
            current_patch: 0,
            current_patch_label: String::new(),
            patch_year: 0,
            patch_index_in_year: 0,
            last_patch_date: None,
            hidden_meta: Vec::new(),
            patch_notes: Vec::new(),
            discovered_champion_ids: Vec::new(),
            rng_seed: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SeedPlayer {
    ign: String,
    champions: Vec<(String, u8)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SeedRoot {
    data: SeedData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SeedData {
    rostered_seeds: Vec<SeedPlayer>,
    #[serde(default)]
    free_agent_seeds: Vec<SeedPlayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChampionCatalogRoot {
    data: ChampionCatalogData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChampionCatalogData {
    roles: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
struct WorkingMeta {
    champion_id: String,
    role: String,
    score: i16,
}

static CHAMPION_CATALOG: OnceLock<Vec<(String, String)>> = OnceLock::new();
static PLAYER_MASTERY_SEED: OnceLock<Vec<SeedPlayer>> = OnceLock::new();

fn default_meta_tier() -> String {
    "C".to_string()
}

fn today_str(game: &Game) -> String {
    game.clock.current_date.format("%Y-%m-%d").to_string()
}

fn parse_day(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

fn normalize_key(value: &str) -> String {
    value
        .to_lowercase()
        .replace(|ch: char| !ch.is_ascii_alphanumeric(), "")
}

fn normalize_role(value: &str) -> Option<String> {
    match normalize_key(value).as_str() {
        "top" => Some("Top".to_string()),
        "jungle" => Some("Jungle".to_string()),
        "mid" | "middle" => Some("Mid".to_string()),
        "bot" | "adc" | "bottom" => Some("ADC".to_string()),
        "support" | "utility" | "sup" => Some("Support".to_string()),
        _ => None,
    }
}

fn champion_role_key(champion_id: &str, role: &str) -> String {
    format!("{}:{}", normalize_key(champion_id), normalize_key(role))
}

fn hash_text(value: &str) -> u32 {
    let mut hash = 0u32;
    for ch in value.chars() {
        hash = hash.wrapping_mul(31).wrapping_add(ch as u32);
    }
    hash
}

fn pseudo_random(seed: &str) -> f64 {
    f64::from(hash_text(seed) % 10_000) / 10_000.0
}

fn days_between(start: chrono::DateTime<chrono::Utc>, end: chrono::DateTime<chrono::Utc>) -> i64 {
    ((end - start).num_days()).max(0)
}

fn mastery_signal_for_player(game: &Game, player_id: &str) -> f64 {
    let mut values: Vec<u8> = game
        .champion_masteries
        .iter()
        .filter(|entry| entry.player_id == player_id)
        .map(|entry| entry.mastery)
        .collect();
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by_key(|value| std::cmp::Reverse(*value));
    let top = values.into_iter().take(3).collect::<Vec<_>>();
    let avg = top.iter().map(|value| f64::from(*value)).sum::<f64>() / top.len() as f64;
    (avg - 60.0).max(0.0)
}

const SOLOQ_POINTS_BASELINE: f64 = 3000.0;
const SOLOQ_POINTS_MIN: f64 = 3000.0;
const SOLOQ_POINTS_MAX: f64 = 7000.0;
const SOLOQ_GRANDMASTER_LP_CUTOFF: f64 = 800.0;
const SOLOQ_CHALLENGER_LP_CUTOFF: f64 = 1300.0;

fn soloq_points_for_player(game: &Game, player: &domain::player::Player) -> f64 {
    let ovr = f64::from(crate::potential::calculate_lol_ovr(player));
    let day_index = days_between(game.clock.start_date, game.clock.current_date);
    let mastery_signal = mastery_signal_for_player(game, &player.id);
    let baseline = 3520.0 + (ovr - 76.0) * 52.0 + (f64::from(hash_text(&player.id) % 121) - 60.0);

    let mut points = baseline;
    for day in 1..=day_index {
        let rand = pseudo_random(&format!("{}:{}", player.id, day));
        let rand_delta = (rand * 48.0 - 24.0).round();
        let skill_drift = ((ovr - 78.0) * 0.35).round();
        let mastery_drift = (mastery_signal * 0.20).round();
        points += rand_delta + skill_drift + mastery_drift;
        points = points.clamp(SOLOQ_POINTS_MIN, SOLOQ_POINTS_MAX);
    }

    points
}

fn soloq_lp_for_player(game: &Game, player: &domain::player::Player) -> f64 {
    (soloq_points_for_player(game, player) - SOLOQ_POINTS_BASELINE).max(0.0)
}

pub fn soloq_tier_for_player(game: &Game, player: &domain::player::Player) -> SoloQTier {
    let lp = soloq_lp_for_player(game, player);

    if lp >= SOLOQ_CHALLENGER_LP_CUTOFF {
        SoloQTier::Challenger
    } else if lp >= SOLOQ_GRANDMASTER_LP_CUTOFF {
        SoloQTier::Grandmaster
    } else {
        SoloQTier::Master
    }
}

pub fn mastery_gain_multiplier_for_player(game: &Game, player_id: &str) -> f64 {
    let Some(player) = game
        .players
        .iter()
        .find(|candidate| candidate.id == player_id)
    else {
        return 1.0;
    };
    match soloq_tier_for_player(game, player) {
        SoloQTier::Challenger => 1.2,
        SoloQTier::Grandmaster => 1.0,
        SoloQTier::Master => 0.8,
    }
}

fn two_digit_year(game: &Game) -> u16 {
    (game.clock.current_date.year().rem_euclid(100)) as u16
}

fn format_patch_label(year: u16, patch_index: u16) -> String {
    format!("{}.{}", year, patch_index)
}

fn tier_to_score(tier: &str) -> i16 {
    match tier {
        "S" => 82,
        "A" => 70,
        "B" => 58,
        "C" => 46,
        "D" => 34,
        _ => 46,
    }
}

fn tier_from_rank(rank_index: usize, total: usize) -> &'static str {
    if total == 0 {
        return "C";
    }
    let pct = (rank_index + 1) as f64 / total as f64;
    if pct <= 0.12 {
        "S"
    } else if pct <= 0.34 {
        "A"
    } else if pct <= 0.64 {
        "B"
    } else if pct <= 0.86 {
        "C"
    } else {
        "D"
    }
}

fn tier_map_from_working(working: &[WorkingMeta]) -> HashMap<String, String> {
    let mut sorted: Vec<usize> = (0..working.len()).collect();
    sorted.sort_by_key(|index| std::cmp::Reverse(working[*index].score));

    let mut tier_map = HashMap::new();
    for (rank, index) in sorted.iter().enumerate() {
        tier_map.insert(
            champion_role_key(&working[*index].champion_id, &working[*index].role),
            tier_from_rank(rank, sorted.len()).to_string(),
        );
    }
    tier_map
}

fn champion_catalog() -> &'static Vec<(String, String)> {
    CHAMPION_CATALOG.get_or_init(|| {
        let raw = include_str!("../../../../data/lec/draft/champions.json");
        let parsed: ChampionCatalogRoot =
            serde_json::from_str(raw).unwrap_or(ChampionCatalogRoot {
                data: ChampionCatalogData {
                    roles: HashMap::new(),
                },
            });
        let mut entries: Vec<(String, String)> = Vec::new();
        for (champion_id, roles) in parsed.data.roles {
            let mut normalized_roles: Vec<String> = roles
                .iter()
                .filter_map(|role| normalize_role(role))
                .collect();

            normalized_roles.sort();
            normalized_roles.dedup();

            if normalized_roles.is_empty() {
                entries.push((champion_id.clone(), "Mid".to_string()));
            } else {
                for role in normalized_roles {
                    entries.push((champion_id.clone(), role));
                }
            }
        }

        entries
    })
}

fn seed_players() -> &'static Vec<SeedPlayer> {
    PLAYER_MASTERY_SEED.get_or_init(|| {
        let raw = include_str!("../../../../data/lec/draft/players.json");
        serde_json::from_str::<SeedRoot>(raw)
            .map(|root| {
                let mut all = root.data.rostered_seeds;
                all.extend(root.data.free_agent_seeds);
                all
            })
            .unwrap_or_default()
    })
}

fn base_role_score(role: &str) -> i16 {
    match role {
        "Top" => 56,
        "Jungle" => 57,
        "Mid" => 58,
        "ADC" => 56,
        "Support" => 55,
        _ => 54,
    }
}

fn upsert_mastery(game: &mut Game, player_id: &str, champion_id: &str, value: u8) {
    let today = today_str(game);
    if let Some(existing) = game.champion_masteries.iter_mut().find(|entry| {
        entry.player_id == player_id
            && normalize_key(&entry.champion_id) == normalize_key(champion_id)
    }) {
        existing.mastery = value.clamp(MIN_MASTERY, MASTERY_CAP);
        existing.last_active_on = today;
        return;
    }

    game.champion_masteries.push(ChampionMasteryEntry {
        player_id: player_id.to_string(),
        champion_id: champion_id.to_string(),
        mastery: value.clamp(MIN_MASTERY, MASTERY_CAP),
        last_active_on: today,
    });
}

pub fn bootstrap_seed_masteries(game: &mut Game) {
    if !game.champion_masteries.is_empty() {
        return;
    }

    let players = seed_players();
    let game_players: Vec<(String, String)> = game
        .players
        .iter()
        .map(|player| (player.id.clone(), player.match_name.clone()))
        .collect();

    for (player_id, match_name) in game_players {
        let Some(seed) = players
            .iter()
            .find(|candidate| normalize_key(&candidate.ign) == normalize_key(&match_name))
        else {
            continue;
        };

        for (champion, mastery) in &seed.champions {
            upsert_mastery(game, &player_id, champion, (*mastery).max(MIN_MASTERY));
        }
    }
}

fn ensure_patch_seed(state: &mut ChampionPatchState) {
    if state.rng_seed == 0 {
        state.rng_seed = rand::rng().random();
    }
}

fn ensure_multirole_meta_shape(game: &mut Game) {
    if game.champion_patch.hidden_meta.is_empty() {
        return;
    }

    let mut existing_role_keys: HashSet<String> = game
        .champion_patch
        .hidden_meta
        .iter()
        .map(|entry| champion_role_key(&entry.champion_id, &entry.role))
        .collect();

    let mut tier_by_champion: HashMap<String, String> = HashMap::new();
    game.champion_patch.hidden_meta.iter().for_each(|entry| {
        let champion_key = normalize_key(&entry.champion_id);
        let tier = (entry.tier.clone()).to_uppercase();
        let previous = tier_by_champion.get(&champion_key).cloned();
        if let Some(prev) = previous {
            let prev_weight = tier_to_score(&prev);
            let current_weight = tier_to_score(&tier);
            if current_weight > prev_weight {
                tier_by_champion.insert(champion_key, tier);
            }
        } else {
            tier_by_champion.insert(champion_key, tier);
        }
    });

    let mut migrated_entries: Vec<ChampionMetaEntry> = Vec::new();
    for (champion_id, role) in champion_catalog().iter() {
        let role_key = champion_role_key(champion_id, role);
        if existing_role_keys.contains(&role_key) {
            continue;
        }

        let champion_key = normalize_key(champion_id);
        let Some(tier) = tier_by_champion.get(&champion_key) else {
            continue;
        };

        migrated_entries.push(ChampionMetaEntry {
            champion_id: champion_id.clone(),
            role: role.clone(),
            tier: tier.clone(),
        });
        existing_role_keys.insert(role_key);
    }

    if !migrated_entries.is_empty() {
        game.champion_patch.hidden_meta.extend(migrated_entries);
    }
}

fn ensure_initial_patch_state(game: &mut Game) {
    if !game.champion_patch.hidden_meta.is_empty() {
        ensure_multirole_meta_shape(game);
        if game.champion_patch.current_patch_label.is_empty() {
            let year = if game.champion_patch.patch_year == 0 {
                two_digit_year(game)
            } else {
                game.champion_patch.patch_year
            };
            let idx = if game.champion_patch.patch_index_in_year == 0 {
                1
            } else {
                game.champion_patch.patch_index_in_year
            };
            game.champion_patch.current_patch_label = format_patch_label(year, idx);
        }
        return;
    }

    ensure_patch_seed(&mut game.champion_patch);
    let mut rng = rand::rng();

    let initial_working: Vec<WorkingMeta> = champion_catalog()
        .iter()
        .map(|(champion_id, role)| {
            let score = (base_role_score(role) + rng.random_range(-4..=4)).clamp(32, 82);
            WorkingMeta {
                champion_id: champion_id.clone(),
                role: role.clone(),
                score,
            }
        })
        .collect();

    let initial_tier_map = tier_map_from_working(&initial_working);
    let initial_meta: Vec<ChampionMetaEntry> = initial_working
        .iter()
        .map(|entry| ChampionMetaEntry {
            champion_id: entry.champion_id.clone(),
            role: entry.role.clone(),
            tier: initial_tier_map
                .get(&champion_role_key(&entry.champion_id, &entry.role))
                .cloned()
                .unwrap_or_else(|| "C".to_string()),
        })
        .collect();

    let year = two_digit_year(game);
    game.champion_patch.current_patch = game.champion_patch.current_patch.max(1);
    game.champion_patch.patch_year = year;
    game.champion_patch.patch_index_in_year = 1;
    game.champion_patch.current_patch_label = format_patch_label(year, 1);
    game.champion_patch.last_patch_date = Some(today_str(game));
    game.champion_patch.hidden_meta = initial_meta;
    game.champion_patch.patch_notes.clear();
}

pub fn bootstrap_champion_state(game: &mut Game) {
    bootstrap_seed_masteries(game);
    ensure_initial_patch_state(game);
}

pub fn set_player_training_target(
    game: &mut Game,
    player_id: &str,
    priority_index: usize,
    champion_id: Option<String>,
) -> Result<(), String> {
    let player = game
        .players
        .iter_mut()
        .find(|candidate| candidate.id == player_id)
        .ok_or_else(|| format!("Player not found: {}", player_id))?;

    if priority_index >= 3 {
        return Err("priority_index must be between 0 and 2".to_string());
    }

    if player.champion_training_targets.len() < 3 {
        player.champion_training_targets.resize(3, String::new());
    }

    // Backward compatibility: if legacy single-target exists and new slots are empty,
    // migrate it to priority slot 1.
    if player
        .champion_training_targets
        .iter()
        .all(|slot| slot.is_empty())
        && let Some(legacy) = player.champion_training_target.clone()
        && !legacy.trim().is_empty()
    {
        player.champion_training_targets[0] = legacy;
    }

    if let Some(champion) = champion_id.clone() {
        let normalized = normalize_key(&champion);
        // Remove duplicates from other slots before setting the requested priority.
        for (index, slot) in player.champion_training_targets.iter_mut().enumerate() {
            if index != priority_index && normalize_key(slot) == normalized {
                *slot = String::new();
            }
        }
        player.champion_training_targets[priority_index] = champion;
    } else {
        player.champion_training_targets[priority_index] = String::new();
    }

    // Compact empty slots while preserving priority order among non-empty slots.
    let mut compacted: Vec<String> = player
        .champion_training_targets
        .iter()
        .filter(|slot| !slot.trim().is_empty())
        .cloned()
        .collect();
    compacted.truncate(3);
    player.champion_training_targets = compacted;
    player.champion_training_targets.resize(3, String::new());

    player.champion_training_target = player
        .champion_training_targets
        .iter()
        .find(|slot| !slot.trim().is_empty())
        .cloned();

    if let Some(champion) = champion_id {
        let current = mastery_for_player_champion(game, player_id, &champion);
        upsert_mastery(game, player_id, &champion, current.max(MIN_MASTERY));
    }
    Ok(())
}

pub fn training_targets_for_player(player: &domain::player::Player) -> Vec<String> {
    let mut targets: Vec<String> = player
        .champion_training_targets
        .iter()
        .filter(|slot| !slot.trim().is_empty())
        .cloned()
        .collect();

    if targets.is_empty()
        && let Some(legacy) = player.champion_training_target.clone()
        && !legacy.trim().is_empty()
    {
        targets.push(legacy);
    }

    targets.truncate(3);
    targets
}

pub fn mastery_for_player_champion(game: &Game, player_id: &str, champion_id: &str) -> u8 {
    game.champion_masteries
        .iter()
        .find(|entry| {
            entry.player_id == player_id
                && normalize_key(&entry.champion_id) == normalize_key(champion_id)
        })
        .map(|entry| entry.mastery)
        .unwrap_or(MIN_MASTERY)
}

pub fn apply_training_mastery_progress(
    game: &mut Game,
    player_id: &str,
    champion_id: &str,
    gain_factor: f64,
) {
    let current = mastery_for_player_champion(game, player_id, champion_id);
    let Some(player) = game
        .players
        .iter()
        .find(|candidate| candidate.id == player_id)
    else {
        return;
    };

    let mechanics = f64::from(player.attributes.dribbling.min(100)) / 100.0;
    let champion_pool = f64::from(player.attributes.agility.min(100)) / 100.0;
    let stat_push = (mechanics * 0.6) + (champion_pool * 0.6);

    let headroom = f64::from(MASTERY_CAP.saturating_sub(current)) / 75.0;
    let chance = (0.16 + gain_factor * 0.26 + headroom * 0.2 + stat_push * 0.18).clamp(0.14, 0.88);
    let mut rng = rand::rng();
    let roll: f64 = rng.random_range(0.0..1.0);
    if roll > chance {
        return;
    }

    let stat_bonus = if mechanics >= 0.85 && champion_pool >= 0.85 {
        2
    } else if mechanics >= 0.72 || champion_pool >= 0.72 {
        1
    } else {
        0
    };

    let gain = if current >= 90 {
        2 + stat_bonus
    } else if current >= 75 {
        3 + stat_bonus
    } else {
        4 + stat_bonus
    };
    let next = current.saturating_add(gain).min(MASTERY_CAP);
    upsert_mastery(game, player_id, champion_id, next);
}

pub fn apply_match_mastery_progress(
    game: &mut Game,
    winner_team_id: &str,
    picks: &[(String, String)],
) {
    for (player_id, champion_id) in picks {
        let current = mastery_for_player_champion(game, player_id, champion_id);
        let mut raw_gain = if current >= 90 {
            1.0
        } else if current >= 70 {
            2.0
        } else {
            3.0
        };

        let player_team_id = game
            .players
            .iter()
            .find(|player| player.id == *player_id)
            .and_then(|player| player.team_id.as_deref())
            .map(str::to_string);

        if player_team_id.as_deref() == Some(winner_team_id) {
            raw_gain += 1.0;
        }

        let staff_mult = player_team_id
            .as_deref()
            .map(|team_id| {
                LolStaffEffects::for_team(&game.staff, team_id).match_mastery_multiplier()
            })
            .unwrap_or(1.0);
        let gain = (raw_gain * staff_mult).round().clamp(1.0, 5.0) as u8;

        upsert_mastery(
            game,
            player_id,
            champion_id,
            current.saturating_add(gain).min(MASTERY_CAP),
        );
    }
}

fn apply_mastery_decay(game: &mut Game) {
    let today = game.clock.current_date.date_naive();
    for entry in &mut game.champion_masteries {
        if entry.mastery <= MIN_MASTERY {
            continue;
        }

        let Some(last_active) = parse_day(&entry.last_active_on) else {
            continue;
        };

        let inactive_days = (today - last_active).num_days();
        if inactive_days >= 56 && inactive_days % 28 == 0 {
            entry.mastery = entry.mastery.saturating_sub(1).max(MIN_MASTERY);
        }
    }
}

fn should_roll_patch(game: &Game, state: &ChampionPatchState) -> bool {
    if game.clock.current_date.weekday().num_days_from_monday() != 2 {
        return false;
    }

    let today = game.clock.current_date.date_naive();
    let Some(last_patch) = state.last_patch_date.as_deref().and_then(parse_day) else {
        return true;
    };

    (today - last_patch).num_days() >= PATCH_INTERVAL_DAYS
}

fn pick_unique_indices(candidates: &[usize], count: usize) -> Vec<usize> {
    let mut rng = rand::rng();
    let mut pool = candidates.to_vec();
    let mut picked = Vec::new();
    let target = count.min(pool.len());

    while picked.len() < target {
        let idx = rng.random_range(0..pool.len());
        picked.push(pool.swap_remove(idx));
    }

    picked
}

fn apply_patch(game: &mut Game) {
    ensure_patch_seed(&mut game.champion_patch);
    let mut rng = rand::rng();
    let catalog = champion_catalog();
    if catalog.is_empty() {
        return;
    }

    let previous_tier_by_role: HashMap<String, String> = game
        .champion_patch
        .hidden_meta
        .iter()
        .map(|entry| {
            (
                champion_role_key(&entry.champion_id, &entry.role),
                entry.tier.clone(),
            )
        })
        .collect();

    let mut working: Vec<WorkingMeta> = catalog
        .iter()
        .map(|(champion_id, role)| {
            let previous_score = previous_tier_by_role
                .get(&champion_role_key(champion_id, role))
                .map(|tier| tier_to_score(tier))
                .unwrap_or(base_role_score(role));
            let mean = base_role_score(role);
            let drift: i16 = rng.random_range(-5..=5);
            let reversion = (mean - previous_score) / 4;
            let score = (previous_score + drift + reversion).clamp(30, 84);
            WorkingMeta {
                champion_id: champion_id.clone(),
                role: role.clone(),
                score,
            }
        })
        .collect();

    let mut sorted: Vec<usize> = (0..working.len()).collect();
    sorted.sort_by_key(|index| std::cmp::Reverse(working[*index].score));

    let top_window = sorted.len().max(8) / 4;
    let low_window = sorted.len().max(10) / 3;

    let nerf_pool: Vec<usize> = sorted.iter().take(top_window.max(1)).copied().collect();
    let buff_pool: Vec<usize> = sorted
        .iter()
        .rev()
        .take(low_window.max(1))
        .copied()
        .collect();

    let nerf_indices = pick_unique_indices(&nerf_pool, 4);
    let buff_indices = pick_unique_indices(&buff_pool, 4);

    let mut notes: Vec<ChampionPatchNote> = Vec::new();

    for index in &buff_indices {
        working[*index].score = (working[*index].score + 9).clamp(30, 88);
        notes.push(ChampionPatchNote {
            champion_id: working[*index].champion_id.clone(),
            role: working[*index].role.clone(),
            change: ChampionPatchChange::Buff,
        });
    }

    for index in &nerf_indices {
        working[*index].score = (working[*index].score - 9).clamp(24, 84);
        notes.push(ChampionPatchNote {
            champion_id: working[*index].champion_id.clone(),
            role: working[*index].role.clone(),
            change: ChampionPatchChange::Nerf,
        });
    }

    let next_tier_map = tier_map_from_working(&working);
    let next_meta: Vec<ChampionMetaEntry> = working
        .iter()
        .map(|entry| ChampionMetaEntry {
            champion_id: entry.champion_id.clone(),
            role: entry.role.clone(),
            tier: next_tier_map
                .get(&champion_role_key(&entry.champion_id, &entry.role))
                .cloned()
                .unwrap_or_else(|| "C".to_string()),
        })
        .collect();

    let changed_set: HashSet<String> = notes
        .iter()
        .map(|note| normalize_key(&note.champion_id))
        .collect();
    let next_meta_set: HashSet<String> = next_meta
        .iter()
        .map(|entry| normalize_key(&entry.champion_id))
        .collect();
    let retained_discovery: Vec<String> = game
        .champion_patch
        .discovered_champion_ids
        .iter()
        .filter(|champion| {
            let key = normalize_key(champion);
            !changed_set.contains(&key) && next_meta_set.contains(&key)
        })
        .cloned()
        .collect();

    let year = two_digit_year(game);
    let next_index =
        if game.champion_patch.patch_year == year && game.champion_patch.patch_index_in_year > 0 {
            game.champion_patch.patch_index_in_year.saturating_add(1)
        } else {
            1
        };

    game.champion_patch.current_patch = game.champion_patch.current_patch.saturating_add(1);
    game.champion_patch.patch_year = year;
    game.champion_patch.patch_index_in_year = next_index;
    game.champion_patch.current_patch_label = format_patch_label(year, next_index);
    game.champion_patch.last_patch_date = Some(today_str(game));
    game.champion_patch.hidden_meta = next_meta;
    game.champion_patch.patch_notes = notes.clone();
    game.champion_patch.discovered_champion_ids = retained_discovery;

    let buffs: Vec<String> = notes
        .iter()
        .filter(|note| note.change == ChampionPatchChange::Buff)
        .map(|note| note.champion_id.clone())
        .collect();
    let nerfs: Vec<String> = notes
        .iter()
        .filter(|note| note.change == ChampionPatchChange::Nerf)
        .map(|note| note.champion_id.clone())
        .collect();

    let label = game.champion_patch.current_patch_label.clone();
    let body = format!(
        "Patch {} deployed.\n\nBuffed: {}\nNerfed: {}\n\nYour staff is already scouting the new tier shifts.",
        label,
        if buffs.is_empty() {
            "-".to_string()
        } else {
            buffs.join(", ")
        },
        if nerfs.is_empty() {
            "-".to_string()
        } else {
            nerfs.join(", ")
        },
    );

    let msg = InboxMessage::new(
        format!("msg_patch_{}", uuid::Uuid::new_v4()),
        format!("Patch {} Notes", label),
        body,
        "League Operations".to_string(),
        game.clock.current_date.to_rfc3339(),
    )
    .with_category(MessageCategory::System)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("Competition Team");

    game.messages.push(msg);
}

fn process_meta_discovery(game: &mut Game) {
    if game.champion_patch.hidden_meta.is_empty() {
        return;
    }

    let Some(manager_team_id) = game.manager.team_id.as_deref() else {
        return;
    };

    let scouts: Vec<_> = game
        .staff
        .iter()
        .filter(|staff| {
            staff.team_id.as_deref() == Some(manager_team_id) && staff.role == StaffRole::Scout
        })
        .collect();

    if scouts.is_empty() {
        return;
    }

    let avg_scouting = scouts
        .iter()
        .map(|scout| scout.attributes.judging_ability as f64)
        .sum::<f64>()
        / scouts.len() as f64;
    let avg_potential = scouts
        .iter()
        .map(|scout| scout.attributes.judging_potential as f64)
        .sum::<f64>()
        / scouts.len() as f64;
    let staff_effects = LolStaffEffects::for_team(&game.staff, manager_team_id);

    let mut reveals = 6usize;
    reveals += scouts.len() * 2;
    reveals += (avg_scouting / 25.0).floor() as usize;
    // Ability discovers the current obvious meta; potential adds a small read on
    // sleeper/future-ish picks. The data model only stores discovered champions,
    // so we express that as a conservative extra reveal count instead of inventing
    // speculative hints.
    reveals += (avg_potential / 50.0).floor() as usize;
    reveals = ((reveals as f64) * staff_effects.meta_discovery).round() as usize;

    let mut rng = rand::rng();
    reveals += rng.random_range(0..=4);

    let discovered_set: HashSet<String> = game
        .champion_patch
        .discovered_champion_ids
        .iter()
        .map(|value| normalize_key(value))
        .collect();

    let mut candidate_keys: Vec<String> = game
        .champion_patch
        .hidden_meta
        .iter()
        .map(|entry| normalize_key(&entry.champion_id))
        .filter(|key| !discovered_set.contains(key))
        .collect();
    candidate_keys.sort();
    candidate_keys.dedup();

    if candidate_keys.is_empty() {
        return;
    }

    let reveal_count = reveals.min(candidate_keys.len()).min(20);
    let mut candidate_indices: Vec<usize> = (0..candidate_keys.len()).collect();
    for _ in 0..reveal_count {
        if candidate_indices.is_empty() {
            break;
        }
        let pick = rng.random_range(0..candidate_indices.len());
        let chosen_idx = candidate_indices.swap_remove(pick);
        game.champion_patch
            .discovered_champion_ids
            .push(candidate_keys[chosen_idx].clone());
    }
}

pub fn process_daily_champion_system(game: &mut Game) {
    bootstrap_champion_state(game);
    apply_mastery_decay(game);

    if should_roll_patch(game, &game.champion_patch) {
        apply_patch(game);
    }

    process_meta_discovery(game);
}
