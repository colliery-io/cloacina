-- CLOACINA-T-1020: a limit on the active runs of a cron schedule.
-- NULL = not declared: a `run_all` schedule runs one fire at a time and a
-- `skip` schedule has no limit, as before this column. A value is the most
-- runs of the schedule that may be active at once; at that limit the
-- scheduler takes no fire and leaves the schedule due. ADD COLUMN only.
ALTER TABLE schedules ADD COLUMN max_active_runs INTEGER;
