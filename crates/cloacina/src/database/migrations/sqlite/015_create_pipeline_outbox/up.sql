-- SQLite version: Pipeline outbox table for work distribution.
-- Transient: rows are deleted immediately upon claiming.
-- Replaces polling on pipeline_executions.status IN ('Pending', 'Running').
-- UUID stored as BLOB (16 bytes), TIMESTAMP stored as TEXT (RFC3339 format)

CREATE TABLE pipeline_outbox (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pipeline_execution_id BLOB NOT NULL REFERENCES pipeline_executions(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- For FIFO claiming
CREATE INDEX idx_pipeline_outbox_created ON pipeline_outbox(created_at);
