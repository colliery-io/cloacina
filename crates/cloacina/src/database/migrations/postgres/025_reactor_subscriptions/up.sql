-- Reactor-triggered workflows — DB-backed subscription fan-out
-- (CLOACI-I-0100 / T-0598). Adds two tables:
--
--   reactor_firings — append-only event log written by the reactor
--     runtime on every fire. Each row is one firing; the `payload`
--     blob carries the same boundary cache the in-process CG
--     traversal consumed.
--
--   reactor_trigger_subscriptions — one row per (reactor, workflow,
--     tenant). The poller advances `last_seen_fired_at` as it
--     dispatches workflows from new firings.

CREATE TABLE reactor_firings (
    id              UUID PRIMARY KEY,
    reactor_name    TEXT NOT NULL,
    tenant_id       TEXT NOT NULL,
    payload         BYTEA,
    fired_at        TIMESTAMP NOT NULL,
    created_at      TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Composite index supports the hot poll path
--   WHERE tenant_id = ? AND reactor_name = ? AND fired_at > ?
--   ORDER BY fired_at
-- plus the TTL prune scan
--   DELETE WHERE fired_at < cutoff.
CREATE INDEX reactor_firings_by_reactor_and_time
    ON reactor_firings (tenant_id, reactor_name, fired_at);

CREATE TABLE reactor_trigger_subscriptions (
    id                    UUID PRIMARY KEY,
    reactor_name          TEXT NOT NULL,
    workflow_name         TEXT NOT NULL,
    tenant_id             TEXT NOT NULL,
    enabled               BOOLEAN NOT NULL DEFAULT TRUE,
    last_seen_fired_at    TIMESTAMP,
    created_at            TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE (reactor_name, workflow_name, tenant_id)
);
