-- Reverse CLOACI-T-0749 pause columns.
DROP INDEX IF EXISTS idx_workflow_packages_paused;
ALTER TABLE workflow_packages DROP COLUMN IF EXISTS paused_at;
ALTER TABLE workflow_packages DROP COLUMN IF EXISTS paused;

DROP INDEX IF EXISTS idx_schedules_paused;
ALTER TABLE schedules DROP COLUMN IF EXISTS paused_at;
ALTER TABLE schedules DROP COLUMN IF EXISTS paused;
