DROP INDEX IF EXISTS idx_signatures_org;
ALTER TABLE package_signatures DROP COLUMN org_id;
