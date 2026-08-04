-- CLOACI-T-0916: DB-backed single-use WebSocket auth tickets.
-- See sibling postgres migration 044 for the full rationale.
--
-- The API server is Postgres-only at runtime; this table exists on SQLite to
-- keep the unified diesel schema consistent across backends and to let the
-- unified DAL's compare-and-set redemption be tested on both.
CREATE TABLE ws_tickets (
    ticket TEXT PRIMARY KEY,
    key_id BLOB NOT NULL,
    key_name TEXT NOT NULL,
    permissions TEXT NOT NULL,
    tenant_id TEXT NULL,
    is_admin INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    redeemed_at TEXT NULL
);

CREATE INDEX idx_ws_tickets_expires_at ON ws_tickets (expires_at);
