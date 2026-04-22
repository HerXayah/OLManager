CREATE TABLE IF NOT EXISTS lol_player_match_stats (
    fixture_id TEXT NOT NULL,
    season INTEGER NOT NULL,
    matchday INTEGER NOT NULL,
    date TEXT NOT NULL,
    competition TEXT NOT NULL DEFAULT 'League',
    player_id TEXT NOT NULL,
    team_id TEXT NOT NULL,
    opponent_team_id TEXT NOT NULL,
    side TEXT NOT NULL DEFAULT 'Blue',
    result TEXT NOT NULL DEFAULT 'Loss',
    role TEXT NOT NULL DEFAULT 'Unknown',
    champion_id TEXT,
    duration_seconds INTEGER NOT NULL DEFAULT 0,
    kills INTEGER NOT NULL DEFAULT 0,
    deaths INTEGER NOT NULL DEFAULT 0,
    assists INTEGER NOT NULL DEFAULT 0,
    creep_score INTEGER NOT NULL DEFAULT 0,
    gold_earned INTEGER NOT NULL DEFAULT 0,
    damage_dealt INTEGER NOT NULL DEFAULT 0,
    vision_score INTEGER NOT NULL DEFAULT 0,
    wards_placed INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (fixture_id, player_id)
);

CREATE INDEX IF NOT EXISTS idx_lol_player_match_stats_player_date
    ON lol_player_match_stats(player_id, date DESC, matchday DESC);

CREATE INDEX IF NOT EXISTS idx_lol_player_match_stats_player_side_date
    ON lol_player_match_stats(player_id, side, date DESC, matchday DESC);

CREATE INDEX IF NOT EXISTS idx_lol_player_match_stats_player_champion
    ON lol_player_match_stats(player_id, champion_id, date DESC, matchday DESC);

CREATE TABLE IF NOT EXISTS lol_team_match_stats (
    fixture_id TEXT NOT NULL,
    season INTEGER NOT NULL,
    matchday INTEGER NOT NULL,
    date TEXT NOT NULL,
    competition TEXT NOT NULL DEFAULT 'League',
    team_id TEXT NOT NULL,
    opponent_team_id TEXT NOT NULL,
    side TEXT NOT NULL DEFAULT 'Blue',
    result TEXT NOT NULL DEFAULT 'Loss',
    duration_seconds INTEGER NOT NULL DEFAULT 0,
    kills INTEGER NOT NULL DEFAULT 0,
    deaths INTEGER NOT NULL DEFAULT 0,
    gold_earned INTEGER NOT NULL DEFAULT 0,
    damage_dealt INTEGER NOT NULL DEFAULT 0,
    objectives INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (fixture_id, team_id)
);

CREATE INDEX IF NOT EXISTS idx_lol_team_match_stats_team_date
    ON lol_team_match_stats(team_id, date DESC, matchday DESC);

CREATE INDEX IF NOT EXISTS idx_lol_team_match_stats_team_side_date
    ON lol_team_match_stats(team_id, side, date DESC, matchday DESC);

INSERT OR IGNORE INTO lol_player_match_stats (
    fixture_id,
    season,
    matchday,
    date,
    competition,
    player_id,
    team_id,
    opponent_team_id,
    side,
    result,
    role,
    champion_id,
    duration_seconds,
    kills,
    deaths,
    assists,
    creep_score,
    gold_earned,
    damage_dealt,
    vision_score,
    wards_placed
)
SELECT
    fixture_id,
    season,
    matchday,
    date,
    competition,
    player_id,
    team_id,
    opponent_team_id,
    COALESCE(side, CASE WHEN team_id = home_team_id THEN 'Blue' ELSE 'Red' END) AS side,
    COALESCE(
        result,
        CASE
            WHEN team_id = home_team_id AND home_goals > away_goals THEN 'Win'
            WHEN team_id = away_team_id AND away_goals > home_goals THEN 'Win'
            ELSE 'Loss'
        END
    ) AS result,
    COALESCE(role, 'Unknown') AS role,
    champion_id,
    COALESCE(duration_seconds, minutes_played * 60, 0) AS duration_seconds,
    COALESCE(kills, goals, 0) AS kills,
    COALESCE(deaths, shots_on_target, 0) AS deaths,
    COALESCE(assists, 0) AS assists,
    COALESCE(creep_score, shots, 0) AS creep_score,
    COALESCE(gold_earned, 0) AS gold_earned,
    COALESCE(damage_dealt, 0) AS damage_dealt,
    COALESCE(vision_score, passes_completed, 0) AS vision_score,
    COALESCE(wards_placed, passes_attempted, 0) AS wards_placed
FROM player_match_stats;

INSERT OR IGNORE INTO lol_team_match_stats (
    fixture_id,
    season,
    matchday,
    date,
    competition,
    team_id,
    opponent_team_id,
    side,
    result,
    duration_seconds,
    kills,
    deaths,
    gold_earned,
    damage_dealt,
    objectives
)
SELECT
    fixture_id,
    season,
    matchday,
    date,
    competition,
    team_id,
    opponent_team_id,
    COALESCE(side, CASE WHEN team_id = home_team_id THEN 'Blue' ELSE 'Red' END) AS side,
    COALESCE(
        result,
        CASE
            WHEN goals_for > goals_against THEN 'Win'
            ELSE 'Loss'
        END
    ) AS result,
    COALESCE(duration_seconds, 0) AS duration_seconds,
    COALESCE(kills, shots, 0) AS kills,
    COALESCE(deaths, shots_on_target, 0) AS deaths,
    COALESCE(gold_earned, passes_attempted, 0) AS gold_earned,
    COALESCE(damage_dealt, passes_completed, 0) AS damage_dealt,
    COALESCE(objectives, tackles_won, 0) AS objectives
FROM team_match_stats;
