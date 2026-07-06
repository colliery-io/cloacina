-- CLOACI-I-0116 (T-0843): parameterized workflow instances.
-- A schedule row may now BE a named instance: `params` stores the instance's
-- FULLY-RESOLVED bound parameters (JSON object; defaults snapshotted at
-- instantiation), merged into the context at every cron/trigger fire.
-- `instance_name` is the human management identity, unique per workflow
-- (tenant isolation is the schema boundary). NULLs = today's anonymous
-- schedules, byte-for-byte unchanged behavior. ADD COLUMN only.
ALTER TABLE schedules ADD COLUMN params TEXT;
ALTER TABLE schedules ADD COLUMN instance_name TEXT;
CREATE UNIQUE INDEX idx_schedules_instance_name
    ON schedules (workflow_name, instance_name)
    WHERE instance_name IS NOT NULL;
