DROP INDEX IF EXISTS idx_signatures_org;
-- SQLite 3.35+ supports DROP COLUMN.
ALTER TABLE package_signatures DROP COLUMN org_id;
