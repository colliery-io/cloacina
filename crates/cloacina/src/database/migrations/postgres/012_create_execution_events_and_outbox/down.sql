-- Drop trigger and function first
DROP TRIGGER IF EXISTS task_outbox_notify ON task_outbox;
DROP FUNCTION IF EXISTS notify_task_ready();

-- Drop tables
DROP TABLE IF EXISTS task_outbox;
DROP TABLE IF EXISTS execution_events;
