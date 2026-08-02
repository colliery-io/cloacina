-- Recreate the task_outbox table exactly as 012_create_execution_events_and_outbox
-- defined it (table, index, notify function, and trigger).

-- Task outbox table for work distribution.
-- Transient: rows are deleted immediately upon claiming.
-- Replaces polling on task_executions.status = 'Ready'.

CREATE TABLE task_outbox (
    id BIGSERIAL PRIMARY KEY,
    task_execution_id UUID NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- For FIFO claiming with FOR UPDATE SKIP LOCKED
CREATE INDEX idx_task_outbox_created ON task_outbox(created_at);


-- Trigger to notify workers when new work is available (Postgres-specific optimization)
CREATE OR REPLACE FUNCTION notify_task_ready() RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('task_ready', NEW.task_execution_id::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER task_outbox_notify
    AFTER INSERT ON task_outbox
    FOR EACH ROW EXECUTE FUNCTION notify_task_ready();
