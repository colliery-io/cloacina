-- SQLite doesn't support DROP COLUMN before 3.35.0.
-- These columns are nullable so leaving them is safe.
DROP TABLE IF EXISTS runner_instances;
