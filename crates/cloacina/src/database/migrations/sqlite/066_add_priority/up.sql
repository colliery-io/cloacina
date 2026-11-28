-- CLOACINA-T-1045: a workflow and a task each carry a priority, and waiting
-- work is dispatched in that order. A priority is an integer from 0 (the
-- highest) to 10 (the lowest). The default is 5.

-- The workflow priority that was asked for (by the run request, the schedule
-- or the workflow's own declaration) and the one the execution runs at (the
-- asked one, held to the limit of who asked). priority_asked is NULL on a row
-- from before this migration.
ALTER TABLE workflow_executions ADD COLUMN priority_asked INTEGER;
ALTER TABLE workflow_executions ADD COLUMN priority INTEGER NOT NULL DEFAULT 5;

-- The two numbers of the order of a waiting task, as given: the priority of
-- its workflow, then its own. The third is waiting_since. Ageing is worked
-- out when the order is read; these columns do not change while a task waits.
ALTER TABLE task_executions ADD COLUMN workflow_priority INTEGER NOT NULL DEFAULT 5;
ALTER TABLE task_executions ADD COLUMN task_priority INTEGER NOT NULL DEFAULT 5;

-- The waiting set in its order.
CREATE INDEX idx_task_executions_waiting_order
    ON task_executions (workflow_priority, task_priority, waiting_since)
    WHERE status = 'Ready';
