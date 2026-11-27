-- CLOACINA-T-1042: one cron schedule per (workflow, cron expression, timezone).
--
-- A packaged cron schedule is identified by its workflow name, its cron
-- expression and its timezone. Two server replicas that loaded the same
-- package at the same moment could each insert the row, and the schedule
-- then fired twice at each fire time.
--
-- The index covers the anonymous cron schedules only. A named instance
-- (`instance_name` set) has its own unique index, and a row of another
-- schedule type (trigger, backfill) is not in it.
--
-- The statements are the same on PostgreSQL and SQLite.

-- 1. Existing doubles. Per identity one row is kept: the one with the latest
--    `last_run_at`, then the oldest, then the lowest id.
--
--    A schedule execution row of a removed schedule moves to the kept one.
--    Where the kept schedule would then have two rows for one fire time
--    (the double fire), one row stays: the row of the kept schedule, else
--    the oldest. The workflow executions are not touched.
DELETE FROM schedule_executions
WHERE id IN (
    SELECT ranked.id
    FROM (
        SELECT
            e.id AS id,
            row_number() OVER (
                PARTITION BY m.keep_id, e.scheduled_time
                ORDER BY (e.schedule_id <> m.keep_id) ASC, e.created_at ASC, e.id ASC
            ) AS rn
        FROM schedule_executions e
        JOIN (
            SELECT
                id,
                first_value(id) OVER (
                    PARTITION BY workflow_name, cron_expression, COALESCE(timezone, 'UTC')
                    ORDER BY (last_run_at IS NULL) ASC, last_run_at DESC, created_at ASC, id ASC
                ) AS keep_id
            FROM schedules
            WHERE schedule_type = 'cron'
              AND instance_name IS NULL
              AND cron_expression IS NOT NULL
        ) m ON m.id = e.schedule_id
        WHERE e.scheduled_time IS NOT NULL
    ) ranked
    WHERE ranked.rn > 1
);

UPDATE schedule_executions
SET schedule_id = (
    SELECT m.keep_id
    FROM (
        SELECT
            id,
            first_value(id) OVER (
                PARTITION BY workflow_name, cron_expression, COALESCE(timezone, 'UTC')
                ORDER BY (last_run_at IS NULL) ASC, last_run_at DESC, created_at ASC, id ASC
            ) AS keep_id
        FROM schedules
        WHERE schedule_type = 'cron'
          AND instance_name IS NULL
          AND cron_expression IS NOT NULL
    ) m
    WHERE m.id = schedule_executions.schedule_id
)
WHERE schedule_id IN (
    SELECT m.id
    FROM (
        SELECT
            id,
            first_value(id) OVER (
                PARTITION BY workflow_name, cron_expression, COALESCE(timezone, 'UTC')
                ORDER BY (last_run_at IS NULL) ASC, last_run_at DESC, created_at ASC, id ASC
            ) AS keep_id
        FROM schedules
        WHERE schedule_type = 'cron'
          AND instance_name IS NULL
          AND cron_expression IS NOT NULL
    ) m
    WHERE m.id <> m.keep_id
);

DELETE FROM schedules
WHERE id IN (
    SELECT m.id
    FROM (
        SELECT
            id,
            first_value(id) OVER (
                PARTITION BY workflow_name, cron_expression, COALESCE(timezone, 'UTC')
                ORDER BY (last_run_at IS NULL) ASC, last_run_at DESC, created_at ASC, id ASC
            ) AS keep_id
        FROM schedules
        WHERE schedule_type = 'cron'
          AND instance_name IS NULL
          AND cron_expression IS NOT NULL
    ) m
    WHERE m.id <> m.keep_id
);

-- 2. The index.
CREATE UNIQUE INDEX idx_schedules_cron_identity
    ON schedules (workflow_name, cron_expression, (COALESCE(timezone, 'UTC')))
    WHERE schedule_type = 'cron'
      AND instance_name IS NULL
      AND cron_expression IS NOT NULL;
