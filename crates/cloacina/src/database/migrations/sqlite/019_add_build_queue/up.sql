-- Build queue columns (CLOACI-I-0097). See sibling postgres migration 022.
--
-- SQLite uses BLOB for bytes and TEXT for timestamps (ISO-8601). Partial
-- indexes work from SQLite 3.8+.

ALTER TABLE workflow_packages ADD COLUMN compiled_data BLOB NULL;
ALTER TABLE workflow_packages ADD COLUMN build_status TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE workflow_packages ADD COLUMN build_error TEXT NULL;
ALTER TABLE workflow_packages ADD COLUMN build_claimed_at TEXT NULL;
ALTER TABLE workflow_packages ADD COLUMN compiled_at TEXT NULL;

CREATE INDEX idx_pending_builds
    ON workflow_packages (build_status, build_claimed_at)
    WHERE build_status IN ('pending', 'building') AND superseded = 0;

CREATE INDEX idx_wfp_content_hash_success
    ON workflow_packages (content_hash)
    WHERE build_status = 'success' AND compiled_data IS NOT NULL;
