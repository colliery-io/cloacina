CREATE TABLE detector_state (
    source_name VARCHAR(255) PRIMARY KEY NOT NULL,
    committed_state TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE pending_boundaries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_name VARCHAR(255) NOT NULL,
    boundary_json TEXT NOT NULL,
    received_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_pending_source ON pending_boundaries(source_name, id);

CREATE TABLE edge_drain_cursors (
    edge_id VARCHAR(255) PRIMARY KEY NOT NULL,
    source_name VARCHAR(255) NOT NULL,
    last_drain_id INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_cursor_source ON edge_drain_cursors(source_name);
