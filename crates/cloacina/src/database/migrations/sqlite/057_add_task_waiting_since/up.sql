-- CLOACINA-T-1036: the time from which a task waits for capacity.
--
-- Set when the task becomes Ready (now), or to the due time of a retry (the
-- backoff is not a wait for capacity). A dispatch that is put off for
-- capacity does not move it. Cleared (NULL) when the task starts or ends.
-- Feeds cloacina_waiting_tasks and cloacina_oldest_waiting_task_age_seconds.

ALTER TABLE task_executions ADD COLUMN waiting_since TEXT;

-- Rows that are Ready now: a retry waits from its due time, any other task
-- from its last write. Rows in other states stay NULL.
UPDATE task_executions
SET waiting_since = COALESCE(retry_at, updated_at)
WHERE status = 'Ready';
