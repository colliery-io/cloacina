-- CLOACINA-T-1084: the fleet dispatch a task is in, for as long as the
-- dispatch holds its claim. See the Postgres migration of the same number.

ALTER TABLE task_executions ADD COLUMN fleet_dispatch_id TEXT;
ALTER TABLE task_executions ADD COLUMN fleet_packet_row_id BIGINT;
