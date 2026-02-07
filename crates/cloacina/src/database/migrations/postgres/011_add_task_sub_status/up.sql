-- Add sub_status column to task_executions for Running sub-state tracking.
-- When status = 'Running', sub_status distinguishes 'Active' (computing)
-- from 'Deferred' (slot released, polling external condition).
-- NULL for all other statuses.

ALTER TABLE task_executions ADD COLUMN sub_status VARCHAR;
