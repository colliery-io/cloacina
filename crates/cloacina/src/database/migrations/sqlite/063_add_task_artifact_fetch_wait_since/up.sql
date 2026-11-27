-- CLOACINA-T-1064: the time of the first dispatch of this attempt that an
-- agent refused because it could not fetch the package artifact.
--
-- Set at that refusal and kept by the later ones, so the fleet executor can
-- stop the wait after its limit (--fleet-artifact-fetch-wait-s). Cleared
-- (NULL) when a retry is scheduled: a new attempt gets the full wait.

ALTER TABLE task_executions ADD COLUMN artifact_fetch_wait_since TEXT;
