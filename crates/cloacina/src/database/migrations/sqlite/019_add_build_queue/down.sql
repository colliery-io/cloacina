DROP INDEX IF EXISTS idx_wfp_content_hash_success;
DROP INDEX IF EXISTS idx_pending_builds;
ALTER TABLE workflow_packages DROP COLUMN compiled_at;
ALTER TABLE workflow_packages DROP COLUMN build_claimed_at;
ALTER TABLE workflow_packages DROP COLUMN build_error;
ALTER TABLE workflow_packages DROP COLUMN build_status;
ALTER TABLE workflow_packages DROP COLUMN compiled_data;
