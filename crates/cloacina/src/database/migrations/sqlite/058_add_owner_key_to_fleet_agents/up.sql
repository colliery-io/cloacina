-- CLOACINA-T-1051: an agent row has an owner. See the Postgres migration of
-- the same number for the rule.
-- UUID stored as BLOB (16 bytes)

ALTER TABLE fleet_agents ADD COLUMN owner_key_id BLOB NULL;
