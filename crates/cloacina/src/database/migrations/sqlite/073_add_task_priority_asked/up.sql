-- CLOACINA-T-1123: a task row keeps the priority its task asked for, beside
-- the one it was given (task_priority). NULL: the task declares no priority
-- of its own and takes its workflow's, or the row is from before this
-- migration.
ALTER TABLE task_executions ADD COLUMN task_priority_asked INTEGER;
