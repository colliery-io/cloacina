-- Recreate the task_outbox table exactly as 011_create_execution_events_and_outbox
-- defined it (table and index).

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
