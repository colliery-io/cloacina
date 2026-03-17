-- Drop trigger and function first
DROP TRIGGER IF EXISTS pipeline_outbox_notify ON pipeline_outbox;
DROP FUNCTION IF EXISTS notify_pipeline_ready();

-- Drop table
DROP TABLE IF EXISTS pipeline_outbox;
