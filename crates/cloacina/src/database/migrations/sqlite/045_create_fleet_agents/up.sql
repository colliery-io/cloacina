-- CLOACI-T-0916: DB-backed execution-agent fleet roster.
-- See sibling postgres migration 045 for the full rationale.
--
-- The fleet is Postgres-only at runtime; this table exists on SQLite to keep
-- the unified diesel schema consistent across backends and for DAL tests.
CREATE TABLE fleet_agents (
    agent_id TEXT PRIMARY KEY,
    tenant_id TEXT NULL,
    target_triple TEXT NOT NULL,
    capabilities TEXT NOT NULL DEFAULT '[]',
    max_concurrency INTEGER NOT NULL,
    in_flight INTEGER NOT NULL DEFAULT 0,
    available_capacity INTEGER NOT NULL DEFAULT 0,
    registered_at TEXT NOT NULL,
    last_heartbeat_at TEXT NOT NULL
);

CREATE INDEX idx_fleet_agents_tenant_heartbeat
    ON fleet_agents (tenant_id, last_heartbeat_at);

CREATE INDEX idx_fleet_agents_last_heartbeat
    ON fleet_agents (last_heartbeat_at);
