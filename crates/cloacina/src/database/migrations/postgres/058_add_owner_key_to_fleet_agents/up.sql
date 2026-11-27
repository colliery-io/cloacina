-- CLOACINA-T-1051: an agent row has an owner.
--
-- The owner is the row's tenant and the API key that registered the agent
-- (`owner_key_id` is `api_keys.id`). Only that key may register the id again,
-- heartbeat, report a result, top up the one-time keys, or open the
-- `agent:<id>` delivery socket.
--
-- No foreign key: a revoked key's agent stops heartbeating (its calls get
-- 401) and the liveness sweep removes the row.
--
-- Rows from before this column have NULL. The next register for the id by a
-- key of the row's tenant claims the row.

ALTER TABLE fleet_agents ADD COLUMN owner_key_id UUID NULL;
