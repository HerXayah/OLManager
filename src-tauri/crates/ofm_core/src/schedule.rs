use chrono::{DateTime, Duration, Utc};
use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League};
use uuid::Uuid;

fn build_fixture(
    matchday: u32,
    date: String,
    home_team_id: String,
    away_team_id: String,
    competition: FixtureCompetition,
    best_of: u8,
) -> Fixture {
    Fixture {
        id: Uuid::new_v4().to_string(),
        matchday,
        date,
        home_team_id,
        away_team_id,
        competition,
        best_of,
        status: FixtureStatus::Scheduled,
        result: None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LecSplit {
    Winter,
    Spring,
    Summer,
}

pub fn regular_best_of(split: LecSplit) -> u8 {
    match split {
        LecSplit::Winter => 1,
        LecSplit::Spring | LecSplit::Summer => 3,
    }
}

pub fn playoff_best_of(split: LecSplit, is_grand_final: bool) -> u8 {
    match split {
        LecSplit::Winter => {
            if is_grand_final {
                5
            } else {
                3
            }
        }
        LecSplit::Spring | LecSplit::Summer => 5,
    }
}

pub fn parse_lec_split(name: &str) -> Option<LecSplit> {
    let key = name.to_lowercase();
    if key.contains("winter") {
        Some(LecSplit::Winter)
    } else if key.contains("spring") {
        Some(LecSplit::Spring)
    } else if key.contains("summer") {
        Some(LecSplit::Summer)
    } else {
        None
    }
}

/// Generate a full double round-robin schedule (home & away) for the given teams.
/// Matchdays are spaced 7 days apart starting from `start_date`.
/// Uses the "circle method" for balanced scheduling.
pub fn generate_league(
    name: &str,
    season: u32,
    team_ids: &[String],
    start_date: DateTime<Utc>,
) -> League {
    let n = team_ids.len();
    assert!(n >= 2, "Need at least 2 teams for a league");

    let league_id = Uuid::new_v4().to_string();
    let mut league = League::new(league_id, name.to_string(), season, team_ids);

    // For round-robin with n teams (n must be even; if odd, add a "bye" — we assume even here)
    // Number of rounds in a single round-robin = n - 1
    // Each round has n / 2 matches
    let rounds = n - 1;
    let half = n / 2;

    // Build a mutable list of team indices (circle method: fix index 0, rotate the rest)
    let mut indices: Vec<usize> = (0..n).collect();

    let mut matchday: u32 = 1;

    // First leg (home)
    for _round in 0..rounds {
        let round_date = start_date + Duration::days((matchday as i64 - 1) * 7);
        let date_str = round_date.format("%Y-%m-%d").to_string();

        for i in 0..half {
            let home_idx = indices[i];
            let away_idx = indices[n - 1 - i];

            let fixture = Fixture {
                id: Uuid::new_v4().to_string(),
                matchday,
                date: date_str.clone(),
                home_team_id: team_ids[home_idx].clone(),
                away_team_id: team_ids[away_idx].clone(),
                competition: FixtureCompetition::League,
                best_of: 1,
                status: FixtureStatus::Scheduled,
                result: None,
            };
            league.fixtures.push(fixture);
        }

        matchday += 1;

        // Rotate: keep index 0 fixed, rotate the rest
        let last = indices.pop().unwrap();
        indices.insert(1, last);
    }

    // Second leg (reverse home/away)
    let mut indices2: Vec<usize> = (0..n).collect();

    for _round in 0..rounds {
        let round_date = start_date + Duration::days((matchday as i64 - 1) * 7);
        let date_str = round_date.format("%Y-%m-%d").to_string();

        for i in 0..half {
            let home_idx = indices2[n - 1 - i]; // Reversed
            let away_idx = indices2[i];

            let fixture = Fixture {
                id: Uuid::new_v4().to_string(),
                matchday,
                date: date_str.clone(),
                home_team_id: team_ids[home_idx].clone(),
                away_team_id: team_ids[away_idx].clone(),
                competition: FixtureCompetition::League,
                best_of: 1,
                status: FixtureStatus::Scheduled,
                result: None,
            };
            league.fixtures.push(fixture);
        }

        matchday += 1;

        let last = indices2.pop().unwrap();
        indices2.insert(1, last);
    }

    league
}

/// Generate a single round-robin league (each pair plays once).
/// Used for LEC Winter regular season (9 matchdays with 10 teams).
pub fn generate_single_round_league(
    name: &str,
    season: u32,
    team_ids: &[String],
    start_date: DateTime<Utc>,
) -> League {
    generate_single_round_league_with_offsets(name, season, team_ids, start_date, None)
}

/// Generate a single round-robin league (each pair plays once) with optional
/// per-round day offsets from `start_date`.
///
/// When `round_day_offsets` is provided, it must contain exactly n-1 values
/// (one date offset for each round). This is useful for tournament formats like
/// LEC Winter regular season superweeks (Sat/Sun/Mon).
pub fn generate_single_round_league_with_offsets(
    name: &str,
    season: u32,
    team_ids: &[String],
    start_date: DateTime<Utc>,
    round_day_offsets: Option<&[i64]>,
) -> League {
    generate_single_round_league_with_offsets_and_bo(
        name,
        season,
        team_ids,
        start_date,
        round_day_offsets,
        1,
    )
}

pub fn generate_single_round_league_with_offsets_and_bo(
    name: &str,
    season: u32,
    team_ids: &[String],
    start_date: DateTime<Utc>,
    round_day_offsets: Option<&[i64]>,
    best_of: u8,
) -> League {
    let n = team_ids.len();
    assert!(n >= 2, "Need at least 2 teams for a league");
    assert!(
        n % 2 == 0,
        "Single round-robin currently requires even team count"
    );

    let league_id = Uuid::new_v4().to_string();
    let mut league = League::new(league_id, name.to_string(), season, team_ids);

    let rounds = n - 1;
    let half = n / 2;

    if let Some(offsets) = round_day_offsets {
        assert!(
            offsets.len() == rounds,
            "round_day_offsets length must match round count"
        );
    }

    let mut indices: Vec<usize> = (0..n).collect();
    let mut matchday: u32 = 1;

    for round_idx in 0..rounds {
        let day_offset = round_day_offsets
            .and_then(|offsets| offsets.get(round_idx).copied())
            .unwrap_or((matchday as i64 - 1) * 7);
        let round_date = start_date + Duration::days(day_offset);
        let date_str = round_date.format("%Y-%m-%d").to_string();

        for i in 0..half {
            let home_idx = indices[i];
            let away_idx = indices[n - 1 - i];
            let (home_team_id, away_team_id) = if (matchday + i as u32) % 2 == 0 {
                (team_ids[home_idx].clone(), team_ids[away_idx].clone())
            } else {
                (team_ids[away_idx].clone(), team_ids[home_idx].clone())
            };

            league.fixtures.push(build_fixture(
                matchday,
                date_str.clone(),
                home_team_id,
                away_team_id,
                FixtureCompetition::League,
                best_of,
            ));
        }

        matchday += 1;
        let last = indices.pop().unwrap();
        indices.insert(1, last);
    }

    league
}

/// Generate a Winter playoffs bracket (Top 8, double elimination structure).
/// Seed order must be [1..8].
pub fn generate_winter_playoffs(
    seeded_team_ids: &[String],
    start_date: DateTime<Utc>,
    start_matchday: u32,
) -> Vec<Fixture> {
    assert!(seeded_team_ids.len() >= 8, "Need at least 8 seeded teams");

    let s = seeded_team_ids;
    let rounds: Vec<Vec<(String, String)>> = vec![
        // WB R1
        vec![
            (s[0].clone(), s[7].clone()),
            (s[3].clone(), s[4].clone()),
            (s[1].clone(), s[6].clone()),
            (s[2].clone(), s[5].clone()),
        ],
        // LB R1 (placeholder bracket using same initial seeds)
        vec![(s[4].clone(), s[7].clone()), (s[5].clone(), s[6].clone())],
        // WB R2
        vec![(s[0].clone(), s[3].clone()), (s[1].clone(), s[2].clone())],
        // LB R2
        vec![(s[4].clone(), s[2].clone()), (s[5].clone(), s[3].clone())],
        // WB Final
        vec![(s[0].clone(), s[1].clone())],
        // LB R3
        vec![(s[4].clone(), s[5].clone())],
        // LB Final
        vec![(s[2].clone(), s[4].clone())],
        // Grand Final
        vec![(s[0].clone(), s[2].clone())],
    ];

    let mut fixtures = Vec::new();

    for (round_index, pairings) in rounds.iter().enumerate() {
        let matchday = start_matchday + round_index as u32;
        let round_date = start_date + Duration::days(round_index as i64 * 7);
        let date_str = round_date.format("%Y-%m-%d").to_string();

        for (home_team_id, away_team_id) in pairings {
            fixtures.push(build_fixture(
                matchday,
                date_str.clone(),
                home_team_id.clone(),
                away_team_id.clone(),
                FixtureCompetition::Playoffs,
                playoff_best_of(LecSplit::Winter, round_index == rounds.len() - 1),
            ));
        }
    }

    fixtures
}

/// Spring/Summer playoffs bracket (Top 6, double elimination style).
/// Seed order must be [1..6].
pub fn generate_spring_summer_playoffs(
    seeded_team_ids: &[String],
    split: LecSplit,
    start_date: DateTime<Utc>,
    start_matchday: u32,
) -> Vec<Fixture> {
    assert!(
        matches!(split, LecSplit::Spring | LecSplit::Summer),
        "Spring/Summer playoff generator expects Spring or Summer split"
    );
    assert!(seeded_team_ids.len() >= 6, "Need at least 6 seeded teams");

    let s = seeded_team_ids;
    let rounds: Vec<Vec<(String, String)>> = vec![
        // UB R1
        vec![(s[0].clone(), s[3].clone()), (s[1].clone(), s[2].clone())],
        // LB R1 (losers of UB R1 face 6/5 seeds)
        vec![(s[3].clone(), s[5].clone()), (s[2].clone(), s[4].clone())],
        // UB Final
        vec![(s[0].clone(), s[1].clone())],
        // LB R2
        vec![(s[5].clone(), s[4].clone())],
        // LB Final
        vec![(s[3].clone(), s[5].clone())],
        // Grand Final
        vec![(s[0].clone(), s[3].clone())],
    ];

    let mut fixtures = Vec::new();
    for (round_index, pairings) in rounds.iter().enumerate() {
        let matchday = start_matchday + round_index as u32;
        let round_date = start_date + Duration::days(round_index as i64 * 7);
        let date_str = round_date.format("%Y-%m-%d").to_string();
        let is_grand_final = round_index == rounds.len() - 1;

        for (home_team_id, away_team_id) in pairings {
            fixtures.push(build_fixture(
                matchday,
                date_str.clone(),
                home_team_id.clone(),
                away_team_id.clone(),
                FixtureCompetition::Playoffs,
                playoff_best_of(split, is_grand_final),
            ));
        }
    }

    fixtures
}

pub fn generate_preseason_friendlies(
    user_team_id: &str,
    opponent_ids: &[String],
    season_start: DateTime<Utc>,
    max_friendlies: usize,
) -> Vec<Fixture> {
    opponent_ids
        .iter()
        .filter(|opponent_id| opponent_id.as_str() != user_team_id)
        .take(max_friendlies)
        .enumerate()
        .map(|(index, opponent_id)| {
            let weeks_before_start = (max_friendlies.saturating_sub(index)) as i64;
            let date = (season_start - Duration::days(weeks_before_start * 7))
                .format("%Y-%m-%d")
                .to_string();
            let (home_team_id, away_team_id) = if index % 2 == 0 {
                (user_team_id.to_string(), opponent_id.clone())
            } else {
                (opponent_id.clone(), user_team_id.to_string())
            };

            Fixture {
                id: Uuid::new_v4().to_string(),
                matchday: 0,
                date,
                home_team_id,
                away_team_id,
                competition: FixtureCompetition::Friendly,
                best_of: 1,
                status: FixtureStatus::Scheduled,
                result: None,
            }
        })
        .collect()
}

pub fn append_fixtures(league: &mut League, mut additional_fixtures: Vec<Fixture>) {
    league.fixtures.append(&mut additional_fixtures);
    league.fixtures.sort_by(|left, right| {
        left.date
            .cmp(&right.date)
            .then(left.matchday.cmp(&right.matchday))
            .then(left.id.cmp(&right.id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_generate_league_8_teams() {
        let teams: Vec<String> = (0..8).map(|i| format!("team_{}", i)).collect();
        let start = Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap();
        let league = generate_league("Test League", 2026, &teams, start);

        // 8 teams: 7 rounds * 4 matches * 2 legs = 56 fixtures
        assert_eq!(league.fixtures.len(), 56);

        // 14 matchdays (7 per leg)
        let max_md = league.fixtures.iter().map(|f| f.matchday).max().unwrap();
        assert_eq!(max_md, 14);

        // Each team plays 14 matches total
        for team in &teams {
            let count = league
                .fixtures
                .iter()
                .filter(|f| f.home_team_id == *team || f.away_team_id == *team)
                .count();
            assert_eq!(count, 14, "Team {} plays {} matches", team, count);
        }

        // 8 standings entries
        assert_eq!(league.standings.len(), 8);
    }

    #[test]
    fn test_generate_league_16_teams() {
        let teams: Vec<String> = (0..16).map(|i| format!("team_{}", i)).collect();
        let start = Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap();
        let league = generate_league("Premier Division", 2026, &teams, start);

        // 16 teams: 15 rounds * 8 matches * 2 legs = 240 fixtures
        assert_eq!(league.fixtures.len(), 240);

        // 30 matchdays (15 per leg)
        let max_md = league.fixtures.iter().map(|f| f.matchday).max().unwrap();
        assert_eq!(max_md, 30);

        // Each team plays 30 matches total (15 home + 15 away)
        for team in &teams {
            let count = league
                .fixtures
                .iter()
                .filter(|f| f.home_team_id == *team || f.away_team_id == *team)
                .count();
            assert_eq!(count, 30, "Team {} plays {} matches", team, count);
        }

        // 16 standings entries
        assert_eq!(league.standings.len(), 16);

        // No team plays itself
        for f in &league.fixtures {
            assert_ne!(f.home_team_id, f.away_team_id);
        }
    }

    #[test]
    fn generate_preseason_friendlies_marks_fixtures_as_friendlies() {
        let start = Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap();
        let friendlies = generate_preseason_friendlies(
            "team_1",
            &[
                "team_2".to_string(),
                "team_3".to_string(),
                "team_4".to_string(),
            ],
            start,
            3,
        );

        assert_eq!(friendlies.len(), 3);
        assert!(
            friendlies
                .iter()
                .all(|fixture| fixture.competition == FixtureCompetition::Friendly)
        );
        assert!(friendlies.iter().all(|fixture| fixture.matchday == 0));
        assert!(friendlies.iter().all(|fixture| fixture.best_of == 1));
        assert_eq!(friendlies[0].date, "2026-07-11");
        assert_eq!(friendlies[2].date, "2026-07-25");
    }

    #[test]
    fn winter_playoffs_use_bo3_except_final_bo5() {
        let seeds: Vec<String> = (1..=8).map(|i| format!("team_{}", i)).collect();
        let start = Utc.with_ymd_and_hms(2026, 3, 1, 0, 0, 0).unwrap();

        let fixtures = generate_winter_playoffs(&seeds, start, 10);
        assert!(!fixtures.is_empty());
        let max_matchday = fixtures
            .iter()
            .map(|fixture| fixture.matchday)
            .max()
            .unwrap();

        for fixture in fixtures {
            if fixture.matchday == max_matchday {
                assert_eq!(fixture.best_of, 5);
            } else {
                assert_eq!(fixture.best_of, 3);
            }
        }
    }

    #[test]
    fn spring_playoffs_use_bo5_and_expected_opening_matchups() {
        let seeds: Vec<String> = (1..=6).map(|i| format!("team_{}", i)).collect();
        let start = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();

        let fixtures = generate_spring_summer_playoffs(&seeds, LecSplit::Spring, start, 20);
        assert_eq!(fixtures.len(), 8);
        assert!(fixtures.iter().all(|fixture| fixture.best_of == 5));
        assert_eq!(fixtures[0].home_team_id, "team_1");
        assert_eq!(fixtures[0].away_team_id, "team_4");
        assert_eq!(fixtures[1].home_team_id, "team_2");
        assert_eq!(fixtures[1].away_team_id, "team_3");
    }
}
