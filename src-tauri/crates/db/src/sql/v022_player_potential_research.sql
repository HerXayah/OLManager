ALTER TABLE players
ADD COLUMN potential_base INTEGER NOT NULL DEFAULT 99;

ALTER TABLE players
ADD COLUMN potential_revealed INTEGER;

ALTER TABLE players
ADD COLUMN potential_research_started_on TEXT;

ALTER TABLE players
ADD COLUMN potential_research_eta_days INTEGER;
