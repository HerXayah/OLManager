ALTER TABLE teams ADD COLUMN weekly_scrim_opponent_ids TEXT NOT NULL DEFAULT '[]';
ALTER TABLE teams ADD COLUMN scrim_loss_streak INTEGER NOT NULL DEFAULT 0;
