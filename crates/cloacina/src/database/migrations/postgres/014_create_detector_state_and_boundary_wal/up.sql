-- Detector state: committed checkpoint per data source.
CREATE TABLE detector_state (
    source_name VARCHAR(255) PRIMARY KEY,
    committed_state TEXT CHECK (committed_state IS NULL OR committed_state::json IS NOT NULL),
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Pending boundary log: per-source ordered log of boundaries not yet fully consumed.
CREATE TABLE pending_boundaries (
    id BIGSERIAL PRIMARY KEY,
    source_name VARCHAR(255) NOT NULL,
    boundary_json TEXT NOT NULL CHECK (boundary_json::json IS NOT NULL),
    received_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE INDEX idx_pending_source ON pending_boundaries(source_name, id);

-- Edge drain cursors: per-edge cursor into the pending_boundaries log.
CREATE TABLE edge_drain_cursors (
    edge_id VARCHAR(255) PRIMARY KEY,
    source_name VARCHAR(255) NOT NULL,
    last_drain_id BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX idx_cursor_source ON edge_drain_cursors(source_name);
