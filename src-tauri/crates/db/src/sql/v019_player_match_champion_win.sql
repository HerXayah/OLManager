ALTER TABLE player_match_stats
    ADD COLUMN champion_win INTEGER;

CREATE INDEX IF NOT EXISTS idx_player_match_stats_player_champion_win
    ON player_match_stats(player_id, champion_id, champion_win, date DESC, matchday DESC);
