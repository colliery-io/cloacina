-- CLOACINA-T-1077: why the last poll of a trigger gave no answer.
-- A poll trigger of a server's package runs on an agent of the tenant. When
-- no agent can take the poll, no answer comes in time, the agent refuses it,
-- or the trigger returns an error, the scheduler writes the reason here and
-- fires nothing. A poll that answers fire or skip sets it back to NULL.
-- NULL on cron rows. ADD COLUMN only. Migration 068: the next free number
-- after 067.
ALTER TABLE schedules ADD COLUMN last_poll_error TEXT;
