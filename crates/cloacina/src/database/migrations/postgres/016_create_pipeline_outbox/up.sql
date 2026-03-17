-- Pipeline outbox table for work distribution.
-- Transient: rows are deleted immediately upon claiming.
-- Replaces polling on pipeline_executions.status IN ('Pending', 'Running').

CREATE TABLE pipeline_outbox (
    id BIGSERIAL PRIMARY KEY,
    pipeline_execution_id UUID NOT NULL REFERENCES pipeline_executions(id) ON DELETE CASCADE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- For FIFO claiming with FOR UPDATE SKIP LOCKED
CREATE INDEX idx_pipeline_outbox_created ON pipeline_outbox(created_at);


-- Trigger to notify schedulers when new pipeline work is available (Postgres-specific optimization)
CREATE OR REPLACE FUNCTION notify_pipeline_ready() RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('pipeline_ready', NEW.pipeline_execution_id::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER pipeline_outbox_notify
    AFTER INSERT ON pipeline_outbox
    FOR EACH ROW EXECUTE FUNCTION notify_pipeline_ready();
