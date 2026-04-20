use domain::league::FixtureCompetition;
use domain::stats::{PlayerMatchStatsRecord, StatsState, TeamMatchStatsRecord};
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

pub fn replace_stats_state(conn: &Connection, stats: &StatsState) -> Result<(), String> {
    conn.execute("DELETE FROM player_match_stats", [])
        .map_err(|e| format!("Failed to clear player_match_stats: {}", e))?;
    conn.execute("DELETE FROM team_match_stats", [])
        .map_err(|e| format!("Failed to clear team_match_stats: {}", e))?;

    for record in &stats.player_matches {
        conn.execute(
            "INSERT INTO player_match_stats (
                fixture_id, season, matchday, date, competition, player_id, team_id,
                opponent_team_id, home_team_id, away_team_id, home_goals, away_goals,
                champion_id, champion_win,
                minutes_played, goals, assists, shots, shots_on_target, passes_completed,
                passes_attempted, tackles_won, interceptions, fouls_committed,
                yellow_cards, red_cards, rating
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27)",
            params![
                record.fixture_id,
                record.season,
                record.matchday,
                record.date,
                competition_to_string(&record.competition),
                record.player_id,
                record.team_id,
                record.opponent_team_id,
                record.home_team_id,
                record.away_team_id,
                record.home_goals,
                record.away_goals,
                record.champion_id,
                record.champion_win.map(|won| if won { 1_i64 } else { 0_i64 }),
                record.minutes_played,
                record.goals,
                record.assists,
                record.shots,
                record.shots_on_target,
                record.passes_completed,
                record.passes_attempted,
                record.tackles_won,
                record.interceptions,
                record.fouls_committed,
                record.yellow_cards,
                record.red_cards,
                record.rating,
            ],
        )
        .map_err(|e| format!("Failed to insert player_match_stats row: {}", e))?;
    }

    for record in &stats.team_matches {
        conn.execute(
            "INSERT INTO team_match_stats (
                fixture_id, season, matchday, date, competition, team_id, opponent_team_id,
                home_team_id, away_team_id, goals_for, goals_against, possession_pct,
                shots, shots_on_target, passes_completed, passes_attempted, tackles_won,
                interceptions, fouls_committed, yellow_cards, red_cards
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                record.fixture_id,
                record.season,
                record.matchday,
                record.date,
                competition_to_string(&record.competition),
                record.team_id,
                record.opponent_team_id,
                record.home_team_id,
                record.away_team_id,
                record.goals_for,
                record.goals_against,
                record.possession_pct,
                record.shots,
                record.shots_on_target,
                record.passes_completed,
                record.passes_attempted,
                record.tackles_won,
                record.interceptions,
                record.fouls_committed,
                record.yellow_cards,
                record.red_cards,
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
                    opponent_team_id, home_team_id, away_team_id, home_goals, away_goals,
                    champion_id, champion_win,
                    minutes_played, goals, assists, shots, shots_on_target, passes_completed,
                    passes_attempted, tackles_won, interceptions, fouls_committed,
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
                home_team_id: row.get(8)?,
                away_team_id: row.get(9)?,
                home_goals: row.get(10)?,
                away_goals: row.get(11)?,
                champion_id: row.get(12)?,
                champion_win: row
                    .get::<_, Option<i64>>(13)?
                    .map(|value| value > 0),
                minutes_played: row.get(14)?,
                goals: row.get(15)?,
                assists: row.get(16)?,
                shots: row.get(17)?,
                shots_on_target: row.get(18)?,
                passes_completed: row.get(19)?,
                passes_attempted: row.get(20)?,
                tackles_won: row.get(21)?,
                interceptions: row.get(22)?,
                fouls_committed: row.get(23)?,
                yellow_cards: row.get(24)?,
                red_cards: row.get(25)?,
                rating: row.get(26)?,
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
                home_team_id: row.get(7)?,
                away_team_id: row.get(8)?,
                goals_for: row.get(9)?,
                goals_against: row.get(10)?,
                possession_pct: row.get(11)?,
                shots: row.get(12)?,
                shots_on_target: row.get(13)?,
                passes_completed: row.get(14)?,
                passes_attempted: row.get(15)?,
                tackles_won: row.get(16)?,
                interceptions: row.get(17)?,
                fouls_committed: row.get(18)?,
                yellow_cards: row.get(19)?,
                red_cards: row.get(20)?,
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
