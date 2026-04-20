ALTER TABLE player_match_stats
    ADD COLUMN champion_id TEXT;

CREATE INDEX IF NOT EXISTS idx_player_match_stats_player_champion
    ON player_match_stats(player_id, champion_id, date DESC, matchday DESC);
