-- CLOACINA-T-1021: agent pools.
-- See sibling postgres migration 055 for the full rationale.
--
-- The fleet is Postgres-only at runtime; this table exists on SQLite to keep
-- the unified diesel schema consistent across backends and for DAL tests.
CREATE TABLE agent_pools (
    tenant_id TEXT NOT NULL,
    pool TEXT NOT NULL,
    settings TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (tenant_id, pool)
);
