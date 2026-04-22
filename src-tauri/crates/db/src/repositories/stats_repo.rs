use domain::league::FixtureCompetition;
use domain::stats::{
    LolRole, MatchOutcome, PlayerMatchStatsRecord, StatsState, TeamMatchStatsRecord, TeamSide,
};
use rusqlite::{Connection, params};

fn competition_to_string(competition: &FixtureCompetition) -> String {
    match competition {
        FixtureCompetition::League => "League".to_string(),
        FixtureCompetition::Friendly => "Friendly".to_string(),
        FixtureCompetition::PreseasonTournament => "PreseasonTournament".to_string(),
        FixtureCompetition::Playoffs => "Playoffs".to_string(),
    }
}

fn parse_competition(value: &str) -> FixtureCompetition {
    match value {
        "Friendly" => FixtureCompetition::Friendly,
        "PreseasonTournament" => FixtureCompetition::PreseasonTournament,
        "Playoffs" => FixtureCompetition::Playoffs,
        _ => FixtureCompetition::League,
    }
}

fn team_side_to_string(side: TeamSide) -> &'static str {
    match side {
        TeamSide::Blue => "Blue",
        TeamSide::Red => "Red",
    }
}

fn parse_team_side(value: &str) -> TeamSide {
    match value {
        "Red" | "Away" => TeamSide::Red,
        _ => TeamSide::Blue,
    }
}

fn match_outcome_to_string(result: MatchOutcome) -> &'static str {
    match result {
        MatchOutcome::Win => "Win",
        MatchOutcome::Loss => "Loss",
    }
}

fn parse_match_outcome(value: &str) -> MatchOutcome {
    match value {
        "Win" => MatchOutcome::Win,
        // Compatibilidad legacy: Draw deja de ser válido y se degrada a Loss.
        "Loss" | "Draw" => MatchOutcome::Loss,
        _ => MatchOutcome::Loss,
    }
}

fn lol_role_to_string(role: LolRole) -> &'static str {
    match role {
        LolRole::Top => "Top",
        LolRole::Jungle => "Jungle",
        LolRole::Mid => "Mid",
        LolRole::Adc => "Adc",
        LolRole::Support => "Support",
        LolRole::Unknown => "Unknown",
    }
}

fn parse_lol_role(value: &str) -> LolRole {
    match value {
        "Top" => LolRole::Top,
        "Jungle" => LolRole::Jungle,
        "Mid" => LolRole::Mid,
        "Adc" | "ADC" => LolRole::Adc,
        "Support" => LolRole::Support,
        _ => LolRole::Unknown,
    }
}

fn legacy_score_pair(side: TeamSide, result: MatchOutcome) -> (u8, u8) {
    match (side, result) {
        (TeamSide::Blue, MatchOutcome::Win) => (1, 0),
        (TeamSide::Blue, MatchOutcome::Loss) => (0, 1),
        (TeamSide::Red, MatchOutcome::Win) => (0, 1),
        (TeamSide::Red, MatchOutcome::Loss) => (1, 0),
    }
}

fn legacy_side_ids(team_id: &str, opponent_team_id: &str, side: TeamSide) -> (String, String) {
    match side {
        TeamSide::Blue => (team_id.to_string(), opponent_team_id.to_string()),
        TeamSide::Red => (opponent_team_id.to_string(), team_id.to_string()),
    }
}

fn saturating_u8(value: u16) -> u8 {
    value.min(u16::from(u8::MAX)) as u8
}

pub fn replace_stats_state(conn: &Connection, stats: &StatsState) -> Result<(), String> {
    conn.execute("DELETE FROM player_match_stats", [])
        .map_err(|e| format!("Failed to clear player_match_stats: {}", e))?;
    conn.execute("DELETE FROM team_match_stats", [])
        .map_err(|e| format!("Failed to clear team_match_stats: {}", e))?;

    for record in &stats.player_matches {
        let (home_team_id, away_team_id) =
            legacy_side_ids(&record.team_id, &record.opponent_team_id, record.side);
        let (home_wins, away_wins) = legacy_score_pair(record.side, record.result);

        conn.execute(
            "INSERT INTO player_match_stats (
                fixture_id, season, matchday, date, competition, player_id, team_id,
                opponent_team_id, home_team_id, away_team_id, home_goals, away_goals,
                side, result, role, champion_id, champion_win,
                minutes_played, goals, assists, shots, shots_on_target, passes_completed,
                passes_attempted, tackles_won, interceptions, fouls_committed, duration_seconds,
                kills, deaths, creep_score, gold_earned, damage_dealt, vision_score,
                wards_placed,
                yellow_cards, red_cards, rating
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38)",
            params![
                record.fixture_id,
                record.season,
                record.matchday,
                record.date,
                competition_to_string(&record.competition),
                record.player_id,
                record.team_id,
                record.opponent_team_id,
                home_team_id,
                away_team_id,
                home_wins,
                away_wins,
                team_side_to_string(record.side),
                match_outcome_to_string(record.result),
                lol_role_to_string(record.role),
                record.champion,
                Some(matches!(record.result, MatchOutcome::Win) as i64),
                (record.duration_seconds / 60).min(u32::from(u8::MAX)) as u8,
                saturating_u8(record.kills),
                saturating_u8(record.assists),
                saturating_u8(record.creep_score),
                saturating_u8(record.deaths),
                saturating_u8(record.vision_score),
                saturating_u8(record.wards_placed),
                0_u8,
                0_u8,
                0_u8,
                record.duration_seconds,
                record.kills,
                record.deaths,
                record.creep_score,
                record.gold_earned,
                record.damage_dealt,
                record.vision_score,
                record.wards_placed,
                0_u8,
                0_u8,
                0.0_f32,
            ],
        )
        .map_err(|e| format!("Failed to insert player_match_stats row: {}", e))?;
    }

    for record in &stats.team_matches {
        let (home_team_id, away_team_id) =
            legacy_side_ids(&record.team_id, &record.opponent_team_id, record.side);
        let (home_wins, away_wins) = legacy_score_pair(record.side, record.result);

        conn.execute(
            "INSERT INTO team_match_stats (
                fixture_id, season, matchday, date, competition, team_id, opponent_team_id,
                home_team_id, away_team_id, goals_for, goals_against, side, result, possession_pct,
                shots, shots_on_target, passes_completed, passes_attempted, tackles_won,
                interceptions, fouls_committed, duration_seconds, kills, deaths, gold_earned,
                damage_dealt, objectives, yellow_cards, red_cards
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29)",
            params![
                record.fixture_id,
                record.season,
                record.matchday,
                record.date,
                competition_to_string(&record.competition),
                record.team_id,
                record.opponent_team_id,
                home_team_id,
                away_team_id,
                home_wins,
                away_wins,
                team_side_to_string(record.side),
                match_outcome_to_string(record.result),
                0_u8,
                record.kills,
                record.deaths,
                record.damage_dealt.min(u32::from(u16::MAX)) as u16,
                record.gold_earned.min(u32::from(u16::MAX)) as u16,
                record.objectives,
                0_u16,
                0_u16,
                record.duration_seconds,
                record.kills,
                record.deaths,
                record.gold_earned,
                record.damage_dealt,
                record.objectives,
                0_u8,
                0_u8,
            ],
        )
        .map_err(|e| format!("Failed to insert team_match_stats row: {}", e))?;
    }

    Ok(())
}

pub fn load_stats_state(conn: &Connection) -> Result<StatsState, String> {
    let mut player_stmt = conn
        .prepare(
            "SELECT fixture_id, season, matchday, date, competition, player_id, team_id,
                    opponent_team_id, side, result, role, champion_id, champion_win,
                    minutes_played, goals, assists, shots, shots_on_target, passes_completed,
                    passes_attempted, tackles_won, interceptions, fouls_committed,
                    duration_seconds, kills, deaths, creep_score, gold_earned, damage_dealt,
                    vision_score, wards_placed,
                    yellow_cards, red_cards, rating
              FROM player_match_stats
              ORDER BY date, matchday, fixture_id, player_id",
        )
        .map_err(|e| format!("Failed to prepare player_match_stats query: {}", e))?;
    let player_rows = player_stmt
        .query_map([], |row| {
            Ok(PlayerMatchStatsRecord {
                fixture_id: row.get(0)?,
                season: row.get(1)?,
                matchday: row.get(2)?,
                date: row.get(3)?,
                competition: parse_competition(&row.get::<_, String>(4)?),
                player_id: row.get(5)?,
                team_id: row.get(6)?,
                opponent_team_id: row.get(7)?,
                side: parse_team_side(&row.get::<_, String>(8)?),
                result: parse_match_outcome(&row.get::<_, String>(9)?),
                role: parse_lol_role(&row.get::<_, String>(10)?),
                champion: row.get(11)?,
                duration_seconds: row.get(23)?,
                kills: row.get(24)?,
                deaths: row.get(25)?,
                assists: u16::from(row.get::<_, u8>(15)?),
                creep_score: row.get(26)?,
                gold_earned: row.get(27)?,
                damage_dealt: row.get(28)?,
                vision_score: row.get(29)?,
                wards_placed: row.get(30)?,
            })
        })
        .map_err(|e| format!("Failed to query player_match_stats: {}", e))?;

    let mut player_matches = Vec::new();
    for row in player_rows {
        player_matches
            .push(row.map_err(|e| format!("Failed to read player_match_stats row: {}", e))?);
    }

    let mut team_stmt = conn
        .prepare(
            "SELECT fixture_id, season, matchday, date, competition, team_id, opponent_team_id,
                    side, result, duration_seconds, kills, deaths, gold_earned,
                    damage_dealt, objectives,
                    home_team_id, away_team_id, goals_for, goals_against, possession_pct,
                    shots, shots_on_target, passes_completed, passes_attempted, tackles_won,
                    interceptions, fouls_committed, yellow_cards, red_cards
              FROM team_match_stats
             ORDER BY date, matchday, fixture_id, team_id",
        )
        .map_err(|e| format!("Failed to prepare team_match_stats query: {}", e))?;
    let team_rows = team_stmt
        .query_map([], |row| {
            Ok(TeamMatchStatsRecord {
                fixture_id: row.get(0)?,
                season: row.get(1)?,
                matchday: row.get(2)?,
                date: row.get(3)?,
                competition: parse_competition(&row.get::<_, String>(4)?),
                team_id: row.get(5)?,
                opponent_team_id: row.get(6)?,
                side: parse_team_side(&row.get::<_, String>(7)?),
                result: parse_match_outcome(&row.get::<_, String>(8)?),
                duration_seconds: row.get(9)?,
                kills: row.get(10)?,
                deaths: row.get(11)?,
                gold_earned: row.get(12)?,
                damage_dealt: row.get(13)?,
                objectives: row.get(14)?,
            })
        })
        .map_err(|e| format!("Failed to query team_match_stats: {}", e))?;

    let mut team_matches = Vec::new();
    for row in team_rows {
        team_matches.push(row.map_err(|e| format!("Failed to read team_match_stats row: {}", e))?);
    }

    Ok(StatsState {
        player_matches,
        team_matches,
    })
}
