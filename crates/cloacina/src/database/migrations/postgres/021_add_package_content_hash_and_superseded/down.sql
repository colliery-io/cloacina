DROP INDEX IF EXISTS idx_active_package_per_name;
ALTER TABLE workflow_packages DROP COLUMN superseded;
ALTER TABLE workflow_packages DROP COLUMN content_hash;
