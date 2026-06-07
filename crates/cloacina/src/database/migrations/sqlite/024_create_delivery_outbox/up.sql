-- Delivery outbox (CLOACI-I-0115 / spec S-0012 / ADR A-0006).
-- See sibling postgres migration 027 for the full rationale.
--
-- SQLite uses BLOB for bytes and TEXT for timestamps (RFC3339). The substrate
-- is Postgres-only at runtime (A-0006); this table exists on SQLite solely to
-- keep the unified diesel schema consistent across backends and is never
-- driven on the single-process daemon.

CREATE TABLE delivery_outbox (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    recipient TEXT NOT NULL,
    kind TEXT NOT NULL,
    tenant_id TEXT NULL,
    payload BLOB NOT NULL,
    delivery_state TEXT NOT NULL DEFAULT 'pending',
    delivery_attempts INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    delivered_at TEXT NULL,
    acked_at TEXT NULL
);

CREATE INDEX idx_delivery_outbox_recipient_open
    ON delivery_outbox (recipient, id)
    WHERE delivery_state <> 'acked';

CREATE INDEX idx_delivery_outbox_open_age
    ON delivery_outbox (delivery_state, created_at)
    WHERE delivery_state <> 'acked';
