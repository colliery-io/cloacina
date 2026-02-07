-- SQLite version: Execution events table for complete audit trail of task/pipeline state transitions.
-- Append-only: events are never updated after creation.
-- Used for debugging, compliance, and replay capability.
-- UUID stored as BLOB (16 bytes), TIMESTAMP stored as TEXT (RFC3339 format)

CREATE TABLE execution_events (
    id BLOB NOT NULL,
    pipeline_execution_id BLOB NOT NULL REFERENCES pipeline_executions(id) ON DELETE CASCADE,
    task_execution_id BLOB REFERENCES task_executions(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    event_data TEXT DEFAULT '{}' CHECK (event_data IS NULL OR json_valid(event_data)),
    worker_id TEXT,
    created_at TEXT NOT NULL,
    sequence_num INTEGER PRIMARY KEY AUTOINCREMENT
);

-- Unique constraint on id for lookups
CREATE UNIQUE INDEX idx_execution_events_id ON execution_events(id);

-- Primary access patterns: query by pipeline or task
CREATE INDEX idx_execution_events_pipeline ON execution_events(pipeline_execution_id, sequence_num);
CREATE INDEX idx_execution_events_task ON execution_events(task_execution_id, sequence_num) WHERE task_execution_id IS NOT NULL;

-- For dashboard queries and retention cleanup
CREATE INDEX idx_execution_events_created ON execution_events(created_at DESC);

-- For filtering by event type
CREATE INDEX idx_execution_events_type ON execution_events(event_type);


-- Task outbox table for work distribution.
-- Transient: rows are deleted immediately upon claiming.
-- Replaces polling on task_executions.status = 'Ready'.

CREATE TABLE task_outbox (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_execution_id BLOB NOT NULL,
    created_at TEXT NOT NULL
);

-- For FIFO claiming
CREATE INDEX idx_task_outbox_created ON task_outbox(created_at);
