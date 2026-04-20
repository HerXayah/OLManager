-- V17: Add optional manager nickname/handle.

ALTER TABLE managers
ADD COLUMN nickname TEXT NOT NULL DEFAULT '';
