-- Reverse CLOACI-I-0116 instance columns.
DROP INDEX IF EXISTS idx_schedules_instance_name;
ALTER TABLE schedules DROP COLUMN instance_name;
ALTER TABLE schedules DROP COLUMN params;
