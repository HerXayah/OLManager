use db::game_database::GameDatabase;
use db::migrations::MIGRATION_COUNT;
use db::repositories::team_repo::{load_team, upsert_team};
use domain::team::{
    AcademyLifecycle, AcademyMetadata, ErlAssignment, ErlAssignmentRule, PlayStyle, Team, TeamKind,
};

fn test_db() -> GameDatabase {
    GameDatabase::open_in_memory().expect("in-memory database should open")
}

fn sample_team(id: &str, name: &str) -> Team {
    let mut team = Team::new(
        id.to_string(),
        name.to_string(),
        id.chars().take(3).collect::<String>().to_uppercase(),
        "DE".to_string(),
        "Berlin".to_string(),
        "Academy Lab".to_string(),
        12_000,
    );
    team.play_style = PlayStyle::Possession;
    team.finance = 3_000_000;
    team
}

fn sample_academy_metadata() -> AcademyMetadata {
    AcademyMetadata {
        lifecycle: AcademyLifecycle::Active,
        erl_assignment: ErlAssignment {
            erl_league_id: "prime-league".to_string(),
            country_rule: ErlAssignmentRule::Domestic,
            fallback_reason: None,
            reputation: 8,
            acquisition_cost: 750_000,
            acquired_at: "2026-04-26T18:45:00Z".to_string(),
            creation_cost: 750_000,
            created_at: "2026-04-26T18:45:00Z".to_string(),
        },
        source_team_id: "eintracht-spandau".to_string(),
        original_name: "Eintracht Spandau".to_string(),
        original_short_name: "EINS".to_string(),
        original_logo_url: Some("logos/eintracht-spandau.svg".to_string()),
        current_logo_url: Some("logos/berlin-bees-academy.svg".to_string()),
        acquisition_cost: 750_000,
        acquired_at: "2026-04-26T18:45:00Z".to_string(),
    }
}

#[test]
fn legacy_team_rows_load_as_main_without_academy_metadata() {
    let db = test_db();

    db.conn()
        .execute(
            r#"INSERT INTO teams
             (id, name, short_name, country, football_nation, city, stadium_name, stadium_capacity,
              finance, manager_id, reputation, wage_budget, transfer_budget,
              season_income, season_expenses, formation, play_style,
              training_focus, training_intensity, training_schedule,
              founded_year, colors_primary, colors_secondary,
              starting_xi_ids, match_roles, form, history, training_groups,
              weekly_scrim_opponent_ids, scrim_loss_streak, scrim_weekly_played,
              scrim_weekly_wins, scrim_weekly_losses, scrim_slot_results,
              financial_ledger, sponsorship, facilities)
             VALUES
             ('legacy-main', 'Legacy Main', 'LEG', 'DE', 'DE', 'Berlin', 'Legacy Arena', 18000,
              2500000, NULL, 600, 200000, 500000,
              0, 0, '5v5', 'Balanced',
              'Scrims', 'Medium', 'Balanced',
              2012, '#111111', '#eeeeee',
              '[]', '{"captain":null,"vice_captain":null,"penalty_taker":null,"free_kick_taker":null,"corner_taker":null}', '[]', '[]', '[]',
              '[]', 0, 0, 0, 0, '[]',
              '[]', 'null', '{"training":1,"medical":1,"scouting":1}')"#,
            [],
        )
        .expect("legacy-style team row should insert using academy defaults");

    let loaded = load_team(db.conn(), "legacy-main")
        .expect("team load should not fail")
        .expect("legacy team should exist");

    assert_eq!(loaded.team_kind, TeamKind::Main);
    assert_eq!(loaded.parent_team_id, None);
    assert_eq!(loaded.academy_team_id, None);
    assert_eq!(loaded.academy, None);
}

#[test]
fn academy_team_roundtrips_parent_link_and_erl_metadata() {
    let db = test_db();
    let mut academy = sample_team("academy-001", "Berlin Bees Academy");
    academy.team_kind = TeamKind::Academy;
    academy.parent_team_id = Some("main-001".to_string());
    academy.academy = Some(sample_academy_metadata());

    upsert_team(db.conn(), &academy).expect("academy team should persist");
    let loaded = load_team(db.conn(), "academy-001")
        .expect("academy load should not fail")
        .expect("academy should exist");

    assert_eq!(loaded.team_kind, TeamKind::Academy);
    assert_eq!(loaded.parent_team_id.as_deref(), Some("main-001"));
    assert_eq!(loaded.academy_team_id, None);

    let metadata = loaded.academy.expect("academy metadata should roundtrip");
    assert_eq!(metadata.lifecycle, AcademyLifecycle::Active);
    assert_eq!(metadata.erl_assignment.erl_league_id, "prime-league");
    assert_eq!(
        metadata.erl_assignment.country_rule,
        ErlAssignmentRule::Domestic
    );
    assert_eq!(metadata.erl_assignment.reputation, 8);
    assert_eq!(metadata.erl_assignment.acquisition_cost, 750_000);
    assert_eq!(metadata.erl_assignment.acquired_at, "2026-04-26T18:45:00Z");
    assert_eq!(metadata.source_team_id, "eintracht-spandau");
    assert_eq!(metadata.original_name, "Eintracht Spandau");
    assert_eq!(metadata.original_short_name, "EINS");
    assert_eq!(
        metadata.original_logo_url.as_deref(),
        Some("logos/eintracht-spandau.svg")
    );
    assert_eq!(
        metadata.current_logo_url.as_deref(),
        Some("logos/berlin-bees-academy.svg")
    );
    assert_eq!(metadata.acquisition_cost, 750_000);
    assert_eq!(metadata.acquired_at, "2026-04-26T18:45:00Z");
}

#[test]
fn main_team_roundtrips_academy_link() {
    let db = test_db();
    let mut main = sample_team("main-001", "Berlin Bees");
    main.team_kind = TeamKind::Main;
    main.academy_team_id = Some("academy-001".to_string());

    upsert_team(db.conn(), &main).expect("main team should persist");
    let loaded = load_team(db.conn(), "main-001")
        .expect("main load should not fail")
        .expect("main team should exist");

    assert_eq!(loaded.team_kind, TeamKind::Main);
    assert_eq!(loaded.parent_team_id, None);
    assert_eq!(loaded.academy_team_id.as_deref(), Some("academy-001"));
    assert_eq!(loaded.academy, None);
}

#[test]
fn nullable_academy_fields_do_not_break_existing_team_loads() {
    let db = test_db();
    let team = sample_team("nullable-001", "Nullable FC");
    upsert_team(db.conn(), &team).expect("team should persist");

    db.conn()
        .execute(
            "UPDATE teams
             SET parent_team_id = NULL,
                 academy_team_id = NULL,
                 academy_metadata = NULL
             WHERE id = 'nullable-001'",
            [],
        )
        .expect("academy fields should allow null legacy values");

    let loaded = load_team(db.conn(), "nullable-001")
        .expect("nullable academy fields should load safely")
        .expect("team should exist");

    assert_eq!(loaded.team_kind, TeamKind::Main);
    assert_eq!(loaded.parent_team_id, None);
    assert_eq!(loaded.academy_team_id, None);
    assert_eq!(loaded.academy, None);
}

#[test]
fn migration_adds_extensible_academy_columns_with_safe_defaults() {
    let db = test_db();

    let version: i64 = db
        .conn()
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("schema version should be readable");
    assert_eq!(version, MIGRATION_COUNT as i64);

    let columns: Vec<(String, Option<String>)> = db
        .conn()
        .prepare("PRAGMA table_info(teams)")
        .expect("teams schema should be inspectable")
        .query_map([], |row| Ok((row.get(1)?, row.get(4)?)))
        .expect("team columns should be queryable")
        .collect::<Result<Vec<_>, _>>()
        .expect("team columns should map cleanly");

    assert!(columns.contains(&("team_kind".to_string(), Some("'Main'".to_string()))));
    assert!(columns.iter().any(|(name, _)| name == "parent_team_id"));
    assert!(columns.iter().any(|(name, _)| name == "academy_team_id"));
    assert!(columns.iter().any(|(name, _)| name == "academy_metadata"));
}
