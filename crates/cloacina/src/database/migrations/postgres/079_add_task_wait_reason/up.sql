-- CLOACINA-T-1148: why a waiting task waits. wait_reason is a short code
-- (for example no_capacity), wait_message says it in words. Both NULL: the
-- task does not wait, or nothing put it off since it became Ready, or the
-- row is from before this migration.
ALTER TABLE task_executions ADD COLUMN wait_reason TEXT;
ALTER TABLE task_executions ADD COLUMN wait_message TEXT;
