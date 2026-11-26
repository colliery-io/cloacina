-- CLOACINA-T-1021: agent pools.
--
-- A tenant's managed agents are grouped in pools. A pool has its own size of
-- agent and its own Deployment. One row per (tenant, pool) that has settings:
-- `settings` is a JSON object with the resource requests and limits, the max
-- concurrency, the image, the service account and its annotations, the extra
-- egress rules, and the pool's min and max agent count. A field that is absent
-- means the server-wide default (CLOACINA_AGENT_IMAGE, CLOACINA_AGENT_*).
--
-- Every tenant has a pool named `default`. It needs no row: with none it
-- behaves as a tenant did before pools (server-wide defaults; its max is the
-- tenant's limit in agent_capacity_limits and its desired count is in
-- agent_desired_counts). The fleet actuator reads these rows on each reconcile.
CREATE TABLE agent_pools (
    tenant_id TEXT NOT NULL,
    pool TEXT NOT NULL,
    settings TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    PRIMARY KEY (tenant_id, pool)
);
