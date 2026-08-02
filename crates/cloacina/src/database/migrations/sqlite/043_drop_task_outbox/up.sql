-- CLOACI-T-0914: remove the vestigial pull-dispatch task_outbox subsystem.
-- The table was write-only in production (every mark_ready/schedule_retry
-- inserted a row, but the pull consumer had zero production callers) and grew
-- unboundedly. The push dispatcher selects Ready tasks directly from
-- task_executions.

DROP TABLE IF EXISTS task_outbox;
