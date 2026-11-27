-- Reverse CLOACINA-T-1020 backfill. The backfill rows go, with their
-- schedule execution rows (ON DELETE CASCADE): the older type check does not
-- take them.
DROP INDEX IF EXISTS idx_schedules_backfill_polling;
DELETE FROM schedules WHERE schedule_type = 'backfill';
ALTER TABLE schedules DROP COLUMN backfill;
ALTER TABLE schedules DROP CONSTRAINT IF EXISTS schedules_schedule_type_check;
ALTER TABLE schedules ADD CONSTRAINT schedules_schedule_type_check
    CHECK (schedule_type IN ('cron', 'trigger'));
