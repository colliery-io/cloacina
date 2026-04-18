-- Build queue columns (CLOACI-I-0097). The compiler service claims pending
-- rows, persists compiled cdylib bytes on success, and records structured
-- errors on failure. Reconciler reads compiled_data directly and never shells
-- out to `cargo build` again.

ALTER TABLE workflow_packages ADD COLUMN compiled_data BYTEA NULL;
ALTER TABLE workflow_packages ADD COLUMN build_status TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE workflow_packages ADD COLUMN build_error TEXT NULL;
ALTER TABLE workflow_packages ADD COLUMN build_claimed_at TIMESTAMP NULL;
ALTER TABLE workflow_packages ADD COLUMN compiled_at TIMESTAMP NULL;

-- Partial index to keep the queue claim hot path fast. Only pending/building
-- rows that haven't been superseded can be claimed, so that's all we index.
CREATE INDEX idx_pending_builds
    ON workflow_packages (build_status, build_claimed_at)
    WHERE build_status IN ('pending', 'building') AND NOT superseded;

-- Index on content_hash for upload-time artifact reuse.
CREATE INDEX idx_wfp_content_hash_success
    ON workflow_packages (content_hash)
    WHERE build_status = 'success' AND compiled_data IS NOT NULL;
