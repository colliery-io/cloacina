-- CLOACI-I-0100 / T-0598 — SQLite mirror of postgres migration 025.
-- Reactor-triggered workflow subscriptions via DB-backed event log.

CREATE TABLE reactor_firings (
    id              TEXT PRIMARY KEY,           -- UUID stored as text
    reactor_name    TEXT NOT NULL,
    tenant_id       TEXT NOT NULL,
    payload         BLOB,
    fired_at        TIMESTAMP NOT NULL,
    created_at      TIMESTAMP NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX reactor_firings_by_reactor_and_time
    ON reactor_firings (tenant_id, reactor_name, fired_at);

CREATE TABLE reactor_trigger_subscriptions (
    id                    TEXT PRIMARY KEY,
    reactor_name          TEXT NOT NULL,
    workflow_name         TEXT NOT NULL,
    tenant_id             TEXT NOT NULL,
    enabled               BOOLEAN NOT NULL DEFAULT TRUE,
    last_seen_fired_at    TIMESTAMP,
    created_at            TIMESTAMP NOT NULL DEFAULT (datetime('now')),
    updated_at            TIMESTAMP NOT NULL DEFAULT (datetime('now')),
    UNIQUE (reactor_name, workflow_name, tenant_id)
);
