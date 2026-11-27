-- CLOACINA-T-1020: the backfill.
-- A backfill is a bounded second schedule for one workflow. It is a row of
-- `schedules` with `schedule_type = 'backfill'`, so the type check takes the
-- new value. `backfill` holds the range of the row as JSON: `start`, `end`
-- (the last fire time of the range), `total`, and for a retry the list
-- `fire_times` and `retry_of`. NULL on every other row.
ALTER TABLE schedules DROP CONSTRAINT IF EXISTS schedules_schedule_type_check;
ALTER TABLE schedules ADD CONSTRAINT schedules_schedule_type_check
    CHECK (schedule_type IN ('cron', 'trigger', 'backfill'));
ALTER TABLE schedules ADD COLUMN backfill TEXT;

-- The scheduler reads the due backfills on each cron pass.
CREATE INDEX idx_schedules_backfill_polling
ON schedules (enabled, next_run_at)
WHERE enabled = true AND schedule_type = 'backfill';
