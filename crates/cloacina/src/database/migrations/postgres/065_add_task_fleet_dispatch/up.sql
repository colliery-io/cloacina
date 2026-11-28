-- CLOACINA-T-1084: the fleet dispatch a task is in, for as long as the
-- dispatch holds its claim.
--
-- The fleet executor writes both under its claim when it hands the packet to
-- an agent, and they are cleared with the claim. A row with a claim whose
-- heartbeat is stale and a dispatch id here is a dispatch whose server
-- replica stopped: another replica takes over its wait, so the task is not
-- sent twice.

-- The `dispatch_id` of the work packet. Only a report with this id resolves
-- the dispatch (CLOACINA-A-0016).
ALTER TABLE task_executions ADD COLUMN fleet_dispatch_id TEXT;
-- The id of the packet's `delivery_outbox` row, in the admin schema.
ALTER TABLE task_executions ADD COLUMN fleet_packet_row_id BIGINT;
