ALTER TABLE player_match_stats
    ADD COLUMN side TEXT NOT NULL DEFAULT 'Blue';

ALTER TABLE player_match_stats
    ADD COLUMN result TEXT NOT NULL DEFAULT 'Loss';

ALTER TABLE player_match_stats
    ADD COLUMN role TEXT NOT NULL DEFAULT 'Unknown';

ALTER TABLE player_match_stats
    ADD COLUMN duration_seconds INTEGER NOT NULL DEFAULT 0;

ALTER TABLE player_match_stats
    ADD COLUMN kills INTEGER NOT NULL DEFAULT 0;

ALTER TABLE player_match_stats
    ADD COLUMN deaths INTEGER NOT NULL DEFAULT 0;

ALTER TABLE player_match_stats
    ADD COLUMN creep_score INTEGER NOT NULL DEFAULT 0;

ALTER TABLE player_match_stats
    ADD COLUMN gold_earned INTEGER NOT NULL DEFAULT 0;

ALTER TABLE player_match_stats
    ADD COLUMN damage_dealt INTEGER NOT NULL DEFAULT 0;

ALTER TABLE player_match_stats
    ADD COLUMN vision_score INTEGER NOT NULL DEFAULT 0;

ALTER TABLE player_match_stats
    ADD COLUMN wards_placed INTEGER NOT NULL DEFAULT 0;

UPDATE player_match_stats
SET
    side = CASE
        WHEN team_id = home_team_id THEN 'Blue'
        ELSE 'Red'
    END,
    result = CASE
        WHEN team_id = home_team_id AND home_goals > away_goals THEN 'Win'
        WHEN team_id = home_team_id AND home_goals < away_goals THEN 'Loss'
        WHEN team_id = away_team_id AND away_goals > home_goals THEN 'Win'
        WHEN team_id = away_team_id AND away_goals < home_goals THEN 'Loss'
        ELSE 'Loss'
    END,
    duration_seconds = minutes_played * 60,
    kills = goals,
    deaths = shots_on_target,
    creep_score = shots,
    gold_earned = 0,
    damage_dealt = 0,
    vision_score = passes_completed,
    wards_placed = passes_attempted,
    role = 'Unknown'
WHERE duration_seconds = 0
  AND kills = 0
  AND deaths = 0
  AND creep_score = 0
  AND gold_earned = 0
  AND damage_dealt = 0
  AND vision_score = 0
  AND wards_placed = 0;

CREATE INDEX IF NOT EXISTS idx_player_match_stats_player_side_date
    ON player_match_stats(player_id, side, date DESC, matchday DESC);

ALTER TABLE team_match_stats
    ADD COLUMN side TEXT NOT NULL DEFAULT 'Blue';

ALTER TABLE team_match_stats
    ADD COLUMN result TEXT NOT NULL DEFAULT 'Loss';

ALTER TABLE team_match_stats
    ADD COLUMN duration_seconds INTEGER NOT NULL DEFAULT 0;

ALTER TABLE team_match_stats
    ADD COLUMN kills INTEGER NOT NULL DEFAULT 0;

ALTER TABLE team_match_stats
    ADD COLUMN deaths INTEGER NOT NULL DEFAULT 0;

ALTER TABLE team_match_stats
    ADD COLUMN gold_earned INTEGER NOT NULL DEFAULT 0;

ALTER TABLE team_match_stats
    ADD COLUMN damage_dealt INTEGER NOT NULL DEFAULT 0;

ALTER TABLE team_match_stats
    ADD COLUMN objectives INTEGER NOT NULL DEFAULT 0;

UPDATE team_match_stats
SET
    side = CASE
        WHEN team_id = home_team_id THEN 'Blue'
        ELSE 'Red'
    END,
    result = CASE
        WHEN goals_for > goals_against THEN 'Win'
        WHEN goals_for < goals_against THEN 'Loss'
        ELSE 'Loss'
    END,
    duration_seconds = 0,
    kills = shots,
    deaths = shots_on_target,
    gold_earned = passes_attempted,
    damage_dealt = passes_completed,
    objectives = tackles_won
WHERE duration_seconds = 0
  AND kills = 0
  AND deaths = 0
  AND gold_earned = 0
  AND damage_dealt = 0
  AND objectives = 0;

CREATE INDEX IF NOT EXISTS idx_team_match_stats_team_side_date
    ON team_match_stats(team_id, side, date DESC, matchday DESC);
