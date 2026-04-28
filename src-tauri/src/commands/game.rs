use chrono::{Datelike, TimeZone};
use domain::message::{InboxMessage, MessageCategory, MessageContext, MessagePriority};
use domain::player::{Player, PlayerAttributes, Position};
use domain::team::{
    AcademyLifecycle, AcademyMetadata, ErlAssignment, ErlAssignmentRule, Team, TeamKind,
};
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use tauri::Manager as TauriManager;
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

const ACADEMY_FALLBACK_PHOTO: &str = "/player-photos/107455908655055017.png";

#[derive(Debug, Clone)]
pub(crate) struct ExampleAcademyPlayerSeed {
    pub(crate) role: String,
    pub(crate) nickname: String,
    pub(crate) full_name: String,
    pub(crate) nationality: String,
    pub(crate) dob: Option<String>,
    pub(crate) image_url: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ExampleAcademyTeamSeed {
    pub(crate) league_id: String,
    pub(crate) league_name: String,
    pub(crate) country_code: String,
    pub(crate) team_name: String,
    pub(crate) short_name: String,
    pub(crate) logo_url: Option<String>,
    pub(crate) players: Vec<ExampleAcademyPlayerSeed>,
}

fn normalize_academy_key(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

pub(crate) fn slugify_academy_key(value: &str) -> String {
    let slug: String = value
        .to_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch
            } else {
                '-'
            }
        })
        .collect();
    slug.trim_matches('-')
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub(crate) fn academy_seed_team_id(league_id: &str, team_name: &str) -> String {
    let mut academy_id = format!("academy-{}-{}", league_id, slugify_academy_key(team_name));
    if academy_id == format!("academy-{}-", league_id) {
        academy_id = format!("academy-{}", league_id);
    }
    academy_id
}

fn short_name_from_team_name(team_name: &str) -> String {
    let words: Vec<&str> = team_name
        .split_whitespace()
        .filter(|part| !part.trim().is_empty())
        .collect();
    if words.is_empty() {
        return "ACD".to_string();
    }

    let mut short = String::new();
    for part in words.iter().take(4) {
        if let Some(ch) = part.chars().find(|ch| ch.is_ascii_alphanumeric()) {
            short.push(ch.to_ascii_uppercase());
        }
    }

    if short.is_empty() {
        "ACD".to_string()
    } else {
        short
    }
}

fn sanitize_image_url(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() || !trimmed.starts_with("http") {
        return ACADEMY_FALLBACK_PHOTO.to_string();
    }
    trimmed.to_string()
}

fn looks_like_url(value: &str) -> bool {
    let trimmed = value.trim().to_lowercase();
    trimmed.starts_with("http://") || trimmed.starts_with("https://")
}

fn infer_team_name_from_url(url: &str) -> Option<String> {
    let lower = url.trim().to_lowercase();
    if lower.contains("team_vitality") {
        return Some("Team Vitality.Bee".to_string());
    }
    None
}

fn parse_example_date(raw: &str) -> Option<String> {
    let cleaned = raw.replace('\t', " ").trim().to_string();
    if cleaned.is_empty() || cleaned.contains('?') {
        return None;
    }

    let formats = ["%B %d, %Y", "%b %d, %Y", "%Y-%m-%d"];
    for format in formats {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(&cleaned, format) {
            return Some(date.format("%Y-%m-%d").to_string());
        }
    }

    None
}

fn generate_fallback_dob(seed: &str) -> String {
    let hash = seed
        .bytes()
        .fold(0_u32, |acc, byte| acc.wrapping_mul(31).wrapping_add(u32::from(byte)));
    let year = 2004 + (hash % 4);
    let month = 1 + ((hash / 7) % 12);
    let day = 1 + ((hash / 13) % 28);
    format!("{year:04}-{month:02}-{day:02}")
}

fn nationality_name_to_code(raw: &str) -> String {
    let key = normalize_academy_key(raw);
    match key.as_str() {
        "spain" => "ES",
        "france" => "FR",
        "germany" => "DE",
        "portugal" => "PT",
        "poland" => "PL",
        "turkey" => "TR",
        "korea" | "southkorea" => "KR",
        "ukraine" => "UA",
        "belgium" => "BE",
        "denmark" => "DK",
        "sweden" => "SE",
        "netherlands" => "NL",
        "algeria" => "DZ",
        "austria" => "AT",
        "romania" => "RO",
        "norway" => "NO",
        "greece" => "GR",
        "czechrepublic" => "CZ",
        "unitedkingdom" => "GB",
        "hungary" => "HU",
        "serbia" => "RS",
        "lithuania" => "LT",
        "northmacedonia" => "MK",
        "croatia" => "HR",
        "montenegro" => "ME",
        "andorra" => "AD",
        "albania" => "AL",
        "unitedstates" => "US",
        "jordan" => "JO",
        _ => "EU",
    }
    .to_string()
}

fn seed_role_to_canonical(role: &str) -> String {
    match normalize_academy_key(role).as_str() {
        "top" | "toplaner" => "top",
        "jungle" | "jungler" => "jungle",
        "mid" | "midlaner" | "middle" => "mid",
        "adc" | "bot" | "bottom" => "adc",
        "support" | "sup" => "support",
        _ => "mid",
    }
    .to_string()
}

fn parse_example_academy_file(
    league_id: &str,
    league_name: &str,
    country_code: &str,
    content: &str,
) -> Vec<ExampleAcademyTeamSeed> {
    let mut teams: Vec<ExampleAcademyTeamSeed> = Vec::new();
    let mut current_team: Option<ExampleAcademyTeamSeed> = None;
    let mut current_player: Option<ExampleAcademyPlayerSeed> = None;

    let push_player = |team: &mut Option<ExampleAcademyTeamSeed>,
                       player: &mut Option<ExampleAcademyPlayerSeed>| {
        if let (Some(team_ref), Some(player_ref)) = (team.as_mut(), player.take()) {
            if !player_ref.nickname.trim().is_empty() {
                team_ref.players.push(player_ref);
            }
        }
    };

    let push_team = |teams_vec: &mut Vec<ExampleAcademyTeamSeed>,
                     team: &mut Option<ExampleAcademyTeamSeed>,
                     player: &mut Option<ExampleAcademyPlayerSeed>| {
        push_player(team, player);
        if let Some(team_ref) = team.take() {
            if !team_ref.players.is_empty() {
                teams_vec.push(team_ref);
            }
        }
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(name) = trimmed.strip_prefix("Team:") {
            push_team(&mut teams, &mut current_team, &mut current_player);
            let raw_team_name = name.trim();
            let team_name = if looks_like_url(raw_team_name) {
                infer_team_name_from_url(raw_team_name)
                    .unwrap_or_else(|| "Unknown Academy Team".to_string())
            } else {
                raw_team_name.to_string()
            };
            current_team = Some(ExampleAcademyTeamSeed {
                league_id: league_id.to_string(),
                league_name: league_name.to_string(),
                country_code: country_code.to_string(),
                short_name: short_name_from_team_name(&team_name),
                team_name,
                logo_url: None,
                players: Vec::new(),
            });
            continue;
        }

        if let Some(logo) = trimmed.strip_prefix("Team Logo:") {
            if let Some(team) = current_team.as_mut() {
                let url = logo.trim();
                if !url.is_empty() && looks_like_url(url) {
                    team.logo_url = Some(url.to_string());
                }
                if team.team_name == "Unknown Academy Team" {
                    if let Some(inferred_name) = infer_team_name_from_url(url) {
                        team.team_name = inferred_name;
                        team.short_name = short_name_from_team_name(&team.team_name);
                    }
                }
            }
            continue;
        }

        let role_prefixes = [
            ("Toplaner:", "top"),
            ("Jungle:", "jungle"),
            ("Midlaner:", "mid"),
            ("ADC:", "adc"),
            ("Support:", "support"),
        ];
        let mut created_player = false;
        for (prefix, role) in role_prefixes {
            if let Some(nickname) = trimmed.strip_prefix(prefix) {
                push_player(&mut current_team, &mut current_player);
                current_player = Some(ExampleAcademyPlayerSeed {
                    role: role.to_string(),
                    nickname: nickname.trim().to_string(),
                    full_name: nickname.trim().to_string(),
                    nationality: "EU".to_string(),
                    dob: None,
                    image_url: ACADEMY_FALLBACK_PHOTO.to_string(),
                });
                created_player = true;
                break;
            }
        }
        if created_player {
            continue;
        }

        if let Some(raw) = trimmed.strip_prefix("Born:") {
            if let Some(player) = current_player.as_mut() {
                let born_raw = raw.trim();
                if let Some(parsed_dob) = parse_example_date(born_raw) {
                    player.dob = Some(parsed_dob);
                } else if !born_raw.chars().any(|ch| ch.is_ascii_digit()) {
                    player.nationality = nationality_name_to_code(born_raw);
                }
            }
            continue;
        }

        if let Some(raw) = trimmed.strip_prefix("Nationality:") {
            if let Some(player) = current_player.as_mut() {
                let nationality_raw = raw.trim();
                if let Some(parsed_dob) = parse_example_date(nationality_raw) {
                    if player.dob.is_none() {
                        player.dob = Some(parsed_dob);
                    }
                } else {
                    player.nationality = nationality_name_to_code(nationality_raw);
                }
            }
            continue;
        }

        if let Some(raw) = trimmed.strip_prefix("Full name:") {
            if let Some(player) = current_player.as_mut() {
                let full_name = raw.trim();
                if !full_name.is_empty() {
                    player.full_name = full_name.to_string();
                }
            }
            continue;
        }

        if let Some(raw) = trimmed.strip_prefix("Image:") {
            if let Some(player) = current_player.as_mut() {
                player.image_url = sanitize_image_url(raw);
            }
            continue;
        }
    }

    push_team(&mut teams, &mut current_team, &mut current_player);
    teams
}

pub(crate) fn example_academy_seed_catalog() -> &'static Vec<ExampleAcademyTeamSeed> {
    static CATALOG: OnceLock<Vec<ExampleAcademyTeamSeed>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut teams = Vec::new();
        teams.extend(parse_example_academy_file(
            "liga-espanola",
            "Liga Espanola",
            "ES",
            include_str!("../../../EXAMPLE/les.txt"),
        ));
        teams.extend(parse_example_academy_file(
            "lfl",
            "LFL",
            "FR",
            include_str!("../../../EXAMPLE/lfl.txt"),
        ));
        teams.extend(parse_example_academy_file(
            "prime-league",
            "Prime League",
            "DE",
            include_str!("../../../EXAMPLE/Prime League.txt"),
        ));
        teams
    })
}

fn academy_team_alias_for_parent(parent_name: &str) -> Option<&'static str> {
    match normalize_academy_key(parent_name).as_str() {
        "movistarkoi" => Some("movistarkoifnix"),
        "g2esports" => Some("g2nord"),
        "giantx" => Some("giantxitero"),
        "karminecorp" => Some("karminecorpblue"),
        "teamvitality" => Some("teamvitalitybee"),
        "teamheretics" | "heretics" => Some("teamheretics"),
        _ => None,
    }
}

fn generated_academy_ovr(player_name: &str) -> u8 {
    let hash = player_name.bytes().fold(0_u32, |acc, byte| {
        acc.wrapping_mul(37).wrapping_add(u32::from(byte))
    });
    (60 + (hash % 11)) as u8
}

fn generated_academy_potential(player_name: &str, ovr: u8) -> u8 {
    let hash = player_name.bytes().fold(0_u32, |acc, byte| {
        acc.wrapping_mul(41).wrapping_add(u32::from(byte))
    });
    let elite = (hash % 100) < 15;
    let mut potential = if elite {
        84 + ((hash / 7) % 7) as u8
    } else {
        75 + ((hash / 5) % 9) as u8
    };
    let minimum = ovr.saturating_add(4).min(90);
    if potential < minimum {
        potential = minimum;
    }
    potential.min(90)
}

fn academy_overview_message(
    parent_team: &Team,
    academy_team: &Team,
    academy_players: usize,
    date: &str,
) -> InboxMessage {
    InboxMessage::new(
        format!("academy-overview-{}", parent_team.id),
        format!("Academia activa: {}", academy_team.name),
        format!(
            "Tu club ya cuenta con academia vinculada. {} tiene {} prospectos cargados y listos para seguimiento/promocion.",
            academy_team.name, academy_players
        ),
        "Director Deportivo".to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::Welcome)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("Director Deportivo")
    .with_context(MessageContext {
        team_id: Some(parent_team.id.clone()),
        ..Default::default()
    })
}

pub(crate) fn bootstrap_example_academy_pool_from_example(
    teams: &mut Vec<Team>,
    players: &mut Vec<Player>,
    current_date_iso: &str,
) {
    let seed_catalog = example_academy_seed_catalog();
    if seed_catalog.is_empty() {
        return;
    }

    let mut existing_team_ids: HashSet<String> = teams.iter().map(|team| team.id.clone()).collect();

    for seed_team in seed_catalog.iter() {
        let academy_id = academy_seed_team_id(&seed_team.league_id, &seed_team.team_name);
        if existing_team_ids.contains(&academy_id) {
            continue;
        }

        let mut academy_team = Team::new(
            academy_id.clone(),
            seed_team.team_name.clone(),
            seed_team.short_name.clone(),
            seed_team.country_code.clone(),
            seed_team.league_name.clone(),
            format!("{} Academy Arena", seed_team.short_name),
            2_500,
        );
        academy_team.team_kind = TeamKind::Academy;
        academy_team.parent_team_id = None;
        academy_team.manager_id = None;
        academy_team.reputation = 6_000;
        academy_team.finance = 0;
        academy_team.wage_budget = 0;
        academy_team.transfer_budget = 0;
        academy_team.academy = Some(AcademyMetadata {
            lifecycle: AcademyLifecycle::Planned,
            erl_assignment: ErlAssignment {
                erl_league_id: seed_team.league_id.clone(),
                country_rule: ErlAssignmentRule::Domestic,
                fallback_reason: Some(format!("Seeded from {} academy roster", seed_team.league_name)),
                reputation: 60,
                acquisition_cost: 0,
                acquired_at: String::new(),
                creation_cost: 0,
                created_at: current_date_iso.to_string(),
            },
            source_team_id: academy_id.clone(),
            original_name: seed_team.team_name.clone(),
            original_short_name: seed_team.short_name.clone(),
            original_logo_url: seed_team.logo_url.clone(),
            current_logo_url: seed_team.logo_url.clone(),
            acquisition_cost: 0,
            acquired_at: String::new(),
        });

        for (player_index, seed_player) in seed_team.players.iter().enumerate() {
            let ovr = generated_academy_ovr(&seed_player.nickname);
            let potential = generated_academy_potential(&seed_player.nickname, ovr);
            let seed = DraftPlayerSeed {
                ign: seed_player.nickname.clone(),
                first_name: None,
                last_name: None,
                dob: Some(
                    seed_player
                        .dob
                        .clone()
                        .unwrap_or_else(|| generate_fallback_dob(&seed_player.nickname)),
                ),
                nationality: Some(seed_player.nationality.clone()),
                role: Some(seed_role_to_canonical(&seed_player.role)),
                team_id: Some(academy_id.clone()),
                rating: Some(ovr),
                potential: Some(potential),
                salary: Some(8_000),
                contract_end: Some("2028-11-30".to_string()),
                market_value: Some(220_000 + u64::from(ovr.saturating_sub(60)) * 20_000),
                reputation: Some(62),
            };

            let attributes = build_attributes_from_seed(&seed);
            let position = role_to_position(seed.role.as_deref());
            let player_id = format!("{}-player-{}", academy_id, player_index + 1);

            let mut player = Player::new(
                player_id,
                seed_player.nickname.clone(),
                seed_player.full_name.clone(),
                seed
                    .dob
                    .clone()
                    .unwrap_or_else(|| generate_fallback_dob(&seed_player.nickname)),
                seed_player.nationality.clone(),
                position,
                attributes,
            );
            player.team_id = Some(academy_id.clone());
            player.contract_end = seed.contract_end.clone();
            player.wage = seed.salary.unwrap_or(8_000);
            player.market_value = seed.market_value.unwrap_or(240_000);
            player.potential_base = potential;
            player.morale = 68;
            player.condition = 100;

            if seed_player.image_url.is_empty() {
                let _ = ACADEMY_FALLBACK_PHOTO;
            }

            players.push(player);
        }

        existing_team_ids.insert(academy_id.clone());
        teams.push(academy_team);
    }

    let parent_snapshots: Vec<(usize, String, String)> = teams
        .iter()
        .enumerate()
        .filter(|(_, team)| team.team_kind == TeamKind::Main)
        .map(|(index, team)| (index, team.id.clone(), team.name.clone()))
        .collect();

    for (parent_index, parent_id, parent_name) in parent_snapshots {
        let Some(academy_alias) = academy_team_alias_for_parent(&parent_name) else {
            continue;
        };

        let Some(parent) = teams.get(parent_index) else {
            continue;
        };
        if parent.academy_team_id.is_some() {
            continue;
        }

        let Some(seed_team) = seed_catalog.iter().find(|candidate| {
            normalize_academy_key(&candidate.team_name) == academy_alias
        }) else {
            continue;
        };

        let academy_id = academy_seed_team_id(&seed_team.league_id, &seed_team.team_name);
        let Some(academy_index) = teams
            .iter()
            .position(|team| team.id == academy_id && team.team_kind == TeamKind::Academy)
        else {
            continue;
        };

        if let Some(parent_mut) = teams.iter_mut().find(|team| team.id == parent_id) {
            parent_mut.academy_team_id = Some(academy_id.clone());
        }

        if let Some(academy_team) = teams.get_mut(academy_index) {
            academy_team.parent_team_id = Some(parent_id.clone());
            academy_team.manager_id = None;
            academy_team.reputation = 6_200;
            academy_team.academy = Some(AcademyMetadata {
                lifecycle: AcademyLifecycle::Active,
                erl_assignment: ErlAssignment {
                    erl_league_id: seed_team.league_id.clone(),
                    country_rule: ErlAssignmentRule::Domestic,
                    fallback_reason: Some(format!(
                        "Seeded from {} academy roster",
                        seed_team.league_name
                    )),
                    reputation: 62,
                    acquisition_cost: 0,
                    acquired_at: current_date_iso.to_string(),
                    creation_cost: 0,
                    created_at: current_date_iso.to_string(),
                },
                source_team_id: academy_id,
                original_name: seed_team.team_name.clone(),
                original_short_name: seed_team.short_name.clone(),
                original_logo_url: seed_team.logo_url.clone(),
                current_logo_url: seed_team.logo_url.clone(),
                acquisition_cost: 0,
                acquired_at: current_date_iso.to_string(),
            });
        }
    }
}

pub(crate) fn ensure_example_academy_pool(game: &mut Game) {
    let bootstrap_date = game.clock.current_date.format("%Y-%m-%d").to_string();
    bootstrap_example_academy_pool_from_example(&mut game.teams, &mut game.players, &bootstrap_date);
}

fn resolve_default_world_path(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("Failed to read current dir: {}", e))?;
    let mut candidates = vec![
        app_handle
            .path()
            .resource_dir()
            .ok()
            .map(|dir| dir.join("databases").join("lec_world.json")),
        cwd.join("src-tauri")
            .join("databases")
            .join("lec_world.json")
            .into(),
        cwd.join("databases").join("lec_world.json").into(),
    ];

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.push(Some(exe_dir.join("databases").join("lec_world.json")));
            candidates.push(Some(
                exe_dir
                    .join("resources")
                    .join("databases")
                    .join("lec_world.json"),
            ));
        }
    }

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    let embedded_world_json = include_str!("../../databases/lec_world.json");
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Default LEC world database not found and app data dir is unavailable: {}", e))?;
    let db_dir = app_data_dir.join("databases");
    std::fs::create_dir_all(&db_dir)
        .map_err(|e| format!("Failed to create fallback databases directory: {}", e))?;

    let fallback_path = db_dir.join("lec_world.json");
    if !fallback_path.exists() {
        std::fs::write(&fallback_path, embedded_world_json)
            .map_err(|e| format!("Failed to write fallback LEC world database: {}", e))?;
    }

    if fallback_path.exists() {
        return Ok(fallback_path);
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
    #[serde(default)]
    free_agent_seeds: Vec<DraftPlayerSeed>,
}

#[derive(Debug, Deserialize, Clone)]
struct DraftPlayerSeed {
    ign: String,
    #[serde(default)]
    #[serde(rename = "firstName")]
    first_name: Option<String>,
    #[serde(default)]
    #[serde(rename = "lastName")]
    last_name: Option<String>,
    #[serde(default)]
    dob: Option<String>,
    #[serde(default)]
    nationality: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    #[serde(rename = "teamId")]
    team_id: Option<String>,
    #[serde(default)]
    rating: Option<u8>,
    #[serde(default)]
    potential: Option<u8>,
    #[serde(default)]
    salary: Option<u32>,
    #[serde(default)]
    #[serde(rename = "contractEnd")]
    contract_end: Option<String>,
    #[serde(default)]
    #[serde(rename = "marketValue")]
    market_value: Option<u64>,
    #[serde(default)]
    reputation: Option<u8>,
}

fn load_draft_seed_root() -> DraftSeedRoot {
    let content = include_str!("../../../data/lec/draft/players.json");
    let mut merged = serde_json::from_str::<DraftSeedRoot>(content).unwrap_or(DraftSeedRoot {
        data: DraftSeedData {
            rostered_seeds: vec![],
            free_agent_seeds: vec![],
        },
    });

    if let Some(external) = load_external_more_fa_seed() {
        let mut seen = HashSet::new();
        let mut combined = merged.data.free_agent_seeds;
        combined.extend(external.data.free_agent_seeds);
        combined.retain(|seed| {
            let key = normalize_seed_name(&seed.ign);
            if key.is_empty() || seen.contains(&key) {
                return false;
            }
            seen.insert(key)
        });
        merged.data.free_agent_seeds = combined;
    }

    merged
}

fn load_external_more_fa_seed() -> Option<DraftSeedRoot> {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();

    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        candidates.push(
            std::path::PathBuf::from(user_profile)
                .join("Downloads")
                .join("MoreFA_Players.json"),
        );
    }

    if let Ok(home) = std::env::var("HOME") {
        candidates.push(
            std::path::PathBuf::from(home)
                .join("Downloads")
                .join("MoreFA_Players.json"),
        );
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("MoreFA_Players.json"));
        candidates.push(
            cwd.join("data")
                .join("lec")
                .join("draft")
                .join("MoreFA_Players.json"),
        );
    }

    for path in candidates {
        if !path.exists() {
            continue;
        }

        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };

        if let Ok(parsed) = serde_json::from_str::<DraftSeedRoot>(&raw) {
            return Some(parsed);
        }
    }

    None
}

fn draft_seed_root() -> &'static DraftSeedRoot {
    static ROOT: OnceLock<DraftSeedRoot> = OnceLock::new();
    ROOT.get_or_init(load_draft_seed_root)
}

fn draft_potential_map() -> &'static HashMap<String, u8> {
    static POTENTIALS: OnceLock<HashMap<String, u8>> = OnceLock::new();
    POTENTIALS.get_or_init(|| {
        let parsed = draft_seed_root();

        let mut all_seeds = parsed.data.rostered_seeds.clone();
        all_seeds.extend(parsed.data.free_agent_seeds.clone());

        all_seeds
            .into_iter()
            .filter_map(|seed| {
                let key = normalize_seed_name(&seed.ign);
                if key.is_empty() {
                    return None;
                }
                Some((key, seed.potential.unwrap_or(99)))
            })
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

fn seed_is_free_agent(seed: &DraftPlayerSeed) -> bool {
    seed.team_id
        .as_deref()
        .map(normalize_seed_name)
        .map(|team| team == "fa" || team == "freeagent")
        .unwrap_or(true)
}

fn role_to_position(role: Option<&str>) -> Position {
    let key = role.map(normalize_seed_name).unwrap_or_default();
    match key.as_str() {
        "top" => Position::Defender,
        "jungle" => Position::Midfielder,
        "mid" | "middle" => Position::AttackingMidfielder,
        "bot" | "adc" | "bottom" => Position::Forward,
        "support" | "sup" | "utility" => Position::DefensiveMidfielder,
        _ => Position::Midfielder,
    }
}

fn clamp_stat(value: i16) -> u8 {
    value.clamp(25, 99) as u8
}

fn build_lol_stats_from_seed(seed: &DraftPlayerSeed) -> [u8; 9] {
    let target = i16::from(seed.rating.unwrap_or(60).clamp(45, 95));
    let role_key = normalize_seed_name(seed.role.as_deref().unwrap_or(""));
    let role_bias: [i16; 9] = match role_key.as_str() {
        "top" => [1, 0, 1, 0, 1, 1, 0, 1, 2],
        "jungle" => [0, 0, 1, 2, 1, 2, 1, 1, 1],
        "mid" | "middle" => [2, 2, 0, 1, 0, 1, 1, 0, 0],
        "bot" | "adc" | "bottom" => [2, 2, 1, 0, 0, 0, 1, 0, 1],
        "support" | "sup" | "utility" => [0, 0, 1, 2, 1, 2, 0, 1, 1],
        _ => [0, 0, 0, 0, 0, 0, 0, 0, 0],
    };

    let base_hash = normalize_seed_name(&seed.ign)
        .chars()
        .fold(0_i16, |acc, ch| acc.wrapping_add(ch as i16));

    let mut values = [target; 9];
    for index in 0..9 {
        let jitter = ((base_hash + (index as i16 * 7)) % 5) - 2;
        values[index] = target + role_bias[index] + jitter;
    }

    let current_avg = values.iter().sum::<i16>() / 9;
    let mut delta = target - current_avg;
    let mut cursor = 0_usize;
    while delta != 0 {
        let direction = if delta > 0 { 1 } else { -1 };
        let candidate = values[cursor] + direction;
        if (25..=99).contains(&candidate) {
            values[cursor] = candidate;
            delta -= direction;
        }
        cursor = (cursor + 1) % 9;
    }

    [
        clamp_stat(values[0]),
        clamp_stat(values[1]),
        clamp_stat(values[2]),
        clamp_stat(values[3]),
        clamp_stat(values[4]),
        clamp_stat(values[5]),
        clamp_stat(values[6]),
        clamp_stat(values[7]),
        clamp_stat(values[8]),
    ]
}

fn build_attributes_from_seed(seed: &DraftPlayerSeed) -> PlayerAttributes {
    let [mechanics, laning, teamfighting, macro_play, consistency, shotcalling, champion_pool, discipline, mental_resilience] =
        build_lol_stats_from_seed(seed);

    let role_key = normalize_seed_name(seed.role.as_deref().unwrap_or(""));

    let defending = if role_key == "top" || role_key == "support" {
        clamp_stat(((i16::from(teamfighting) + i16::from(discipline)) / 2) + 4)
    } else {
        clamp_stat((i16::from(teamfighting) + i16::from(discipline)) / 2)
    };

    PlayerAttributes {
        pace: clamp_stat((i16::from(mechanics) + i16::from(laning)) / 2),
        stamina: mental_resilience,
        strength: clamp_stat((i16::from(teamfighting) + i16::from(discipline)) / 2),
        agility: champion_pool,
        passing: clamp_stat((i16::from(macro_play) + i16::from(shotcalling)) / 2),
        shooting: laning,
        tackling: clamp_stat((i16::from(discipline) + i16::from(teamfighting)) / 2),
        dribbling: mechanics,
        defending,
        positioning: clamp_stat((i16::from(macro_play) + i16::from(consistency)) / 2),
        vision: macro_play,
        decisions: consistency,
        composure: discipline,
        aggression: clamp_stat((i16::from(teamfighting) + i16::from(mental_resilience)) / 2 - 4),
        teamwork: teamfighting,
        leadership: shotcalling,
        handling: 20,
        reflexes: 22,
        aerial: if role_key == "top" {
            68
        } else if role_key == "support" {
            64
        } else {
            52
        },
    }
}

fn build_free_agent_player(seed: &DraftPlayerSeed, index: usize) -> Option<Player> {
    let ign = seed.ign.trim();
    if ign.is_empty() {
        return None;
    }

    let first_name = seed.first_name.as_deref().unwrap_or(ign).trim().to_string();
    let last_name = seed.last_name.as_deref().unwrap_or("").trim().to_string();
    let full_name = if last_name.is_empty() {
        first_name.clone()
    } else {
        format!("{} {}", first_name, last_name)
    };

    let dob = seed.dob.clone().unwrap_or_else(|| "2002-01-01".to_string());
    let nationality = seed.nationality.clone().unwrap_or_else(|| "KR".to_string());
    let position = role_to_position(seed.role.as_deref());
    let attributes = build_attributes_from_seed(seed);

    let seed_key = normalize_seed_name(ign);
    let id = format!("lec-fa-{}-{}", seed_key, index + 1);

    let mut player = Player::new(
        id,
        ign.to_string(),
        full_name,
        dob,
        nationality,
        position,
        attributes,
    );
    player.team_id = None;
    player.transfer_listed = true;
    player.loan_listed = false;
    player.market_value = seed.market_value.unwrap_or(300_000);
    player.wage = seed.salary.unwrap_or(40_000);
    player.contract_end = seed.contract_end.clone();
    player.condition = 100;
    player.morale = seed.reputation.unwrap_or(68).clamp(35, 95);
    player.potential_base = seed
        .potential
        .unwrap_or(seed.rating.unwrap_or(70))
        .clamp(45, 99);

    Some(player)
}

fn inject_seed_free_agents(players: &mut Vec<Player>) {
    let existing_ids: HashSet<String> = players
        .iter()
        .map(|player| normalize_seed_name(&player.match_name))
        .collect();
    let mut existing_ids = existing_ids;

    for (index, seed) in draft_seed_root().data.free_agent_seeds.iter().enumerate() {
        if !seed_is_free_agent(seed) {
            continue;
        }

        let seed_key = normalize_seed_name(&seed.ign);
        if seed_key.is_empty() || existing_ids.contains(&seed_key) {
            continue;
        }

        if let Some(player) = build_free_agent_player(seed, index) {
            players.push(player);
            existing_ids.insert(seed_key);
        }
    }
}

/// Step 1: Create manager + generate world. No team assigned yet.
/// Returns the Game object so the frontend can show team selection.
/// world_source: "random" (default) or a file path to a JSON world database.
#[tauri::command]
pub async fn start_new_game(
    app_handle: tauri::AppHandle,
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
        let path = resolve_default_world_path(&app_handle)?;
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

    inject_seed_free_agents(&mut players);
    apply_lol_seed_ratings(&mut players);
    let academy_bootstrap_date = clock.current_date.format("%Y-%m-%d").to_string();
    let mut teams = teams;
    bootstrap_example_academy_pool_from_example(
        &mut teams,
        &mut players,
        &academy_bootstrap_date,
    );

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
    if team.team_kind == TeamKind::Academy {
        return Err("Academy teams cannot be selected as manager team".to_string());
    }
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
    let team_ids: Vec<String> = game
        .teams
        .iter()
        .filter(|team| team.team_kind != TeamKind::Academy)
        .map(|team| team.id.clone())
        .collect();
    let mut league = ofm_core::schedule::generate_single_round_league_with_offsets_and_bo(
        "LEC Winter",
        season_year as u32,
        &team_ids,
        season_start,
        Some(&winter_round_offsets),
        ofm_core::schedule::regular_best_of(ofm_core::schedule::LecSplit::Winter),
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

    if let Some(parent_team) = game.teams.iter().find(|team| team.id == team_id) {
        if let Some(academy_team_id) = parent_team.academy_team_id.as_deref() {
            if let Some(academy_team) = game.teams.iter().find(|team| team.id == academy_team_id) {
                let academy_roster_count = game
                    .players
                    .iter()
                    .filter(|player| player.team_id.as_deref() == Some(academy_team_id))
                    .count();
                game.messages.push(academy_overview_message(
                    parent_team,
                    academy_team,
                    academy_roster_count,
                    &date_str,
                ));
            }
        }
    }

    let season_msg = ofm_core::messages::season_schedule_message(
        "LEC Winter",
        &season_start.format("%B %d, %Y").to_string(),
        &date_str,
    );
    game.messages.push(season_msg);

    let team_names: Vec<String> = game
        .teams
        .iter()
        .filter(|team| team.team_kind != TeamKind::Academy)
        .map(|team| team.name.clone())
        .collect();
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
    inject_seed_free_agents(&mut game.players);
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
            teams: game
                .teams
                .iter()
                .filter(|team| team.team_kind != TeamKind::Academy)
                .cloned()
                .collect(),
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
