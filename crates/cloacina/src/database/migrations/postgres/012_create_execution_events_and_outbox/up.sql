-- Execution events table for complete audit trail of task/pipeline state transitions.
-- Append-only: events are never updated after creation.
-- Used for debugging, compliance, and replay capability.

CREATE TABLE execution_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline_execution_id UUID NOT NULL REFERENCES pipeline_executions(id) ON DELETE CASCADE,
    task_execution_id UUID REFERENCES task_executions(id) ON DELETE CASCADE,
    event_type VARCHAR(50) NOT NULL,
    event_data TEXT DEFAULT '{}' CHECK (event_data IS NULL OR event_data::json IS NOT NULL),
    worker_id VARCHAR(100),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    sequence_num BIGINT GENERATED ALWAYS AS IDENTITY
);

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
