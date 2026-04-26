ALTER TABLE teams ADD COLUMN team_kind TEXT NOT NULL DEFAULT 'Main';
ALTER TABLE teams ADD COLUMN parent_team_id TEXT;
ALTER TABLE teams ADD COLUMN academy_team_id TEXT;
ALTER TABLE teams ADD COLUMN academy_metadata TEXT;
