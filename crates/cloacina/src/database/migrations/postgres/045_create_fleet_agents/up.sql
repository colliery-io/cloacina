-- CLOACI-T-0916: DB-backed execution-agent fleet roster.
--
-- The roster (T-0631) was a per-replica in-memory map, so behind a non-affine
-- load balancer heartbeats landing on different replicas flapped ("agent not
-- registered" -> forced re-register), same-tenant selection saw a partial
-- fleet, and the dead-agent sweeper could "reclaim" work from agents alive on
-- another replica. Persisting register/heartbeat state makes every
-- cross-replica-relevant read (selection, capacity views, reclaim
-- eligibility) come from one shared table, heartbeat-recency-filtered.
--
-- Work-packet DISPATCH already needs no connection locality: packets ride the
-- delivery_outbox substrate and are picked up by whichever replica holds the
-- agent's delivery WebSocket (connection-ownership routing, A-0006). The only
-- state that stays replica-local is each agent's one-time ephemeral
-- secret-key pool (see agent_registry.rs) — a documented residual.
CREATE TABLE fleet_agents (
    agent_id TEXT PRIMARY KEY,
    tenant_id TEXT NULL,
    target_triple TEXT NOT NULL,
    -- JSON array of capability strings.
    capabilities TEXT NOT NULL DEFAULT '[]',
    max_concurrency INTEGER NOT NULL,
    in_flight INTEGER NOT NULL DEFAULT 0,
    available_capacity INTEGER NOT NULL DEFAULT 0,
    registered_at TIMESTAMP NOT NULL,
    last_heartbeat_at TIMESTAMP NOT NULL
);

-- Same-tenant selection + capacity views, recency-filtered.
CREATE INDEX idx_fleet_agents_tenant_heartbeat
    ON fleet_agents (tenant_id, last_heartbeat_at);

-- Dead-agent sweep: stale rows by heartbeat age.
CREATE INDEX idx_fleet_agents_last_heartbeat
    ON fleet_agents (last_heartbeat_at);
