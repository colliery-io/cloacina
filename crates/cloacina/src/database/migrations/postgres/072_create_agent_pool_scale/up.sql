-- CLOACINA-T-1029: the autoscaler's state for each agent pool of a tenant.
-- `desired_count` is the count of a pool other than `default`; it is NULL
-- for `default`, whose count stays in `agent_desired_counts` (the provision
-- routes write it). `idle_since` is when the pool was last seen with no task
-- in flight and none waiting (NULL: it has work). `last_scaled_down_at` keeps
-- two scale-downs a cooldown apart. All times come from the database clock,
-- so the rules hold whichever server replica leads the fleet loop.
-- Server mode only: Postgres.
CREATE TABLE agent_pool_scale (
    tenant_id TEXT NOT NULL,
    pool TEXT NOT NULL,
    desired_count INTEGER,
    idle_since TIMESTAMPTZ,
    last_scaled_down_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, pool)
);
