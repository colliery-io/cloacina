-- CLOACI-T-0923: DB-backed brute-force throttle for local login.
-- See the sibling postgres migration 047 for the full rationale (dual-keyed
-- username + source-IP counters, persisted for multi-replica correctness).
--
-- The API server is Postgres-only at runtime; this table exists on SQLite
-- because the throttle DAL is a *unified* (`interact_on_backend!`) DAL — the
-- same reason `ws_tickets`/`fleet_agents` have sqlite twins — so its
-- compare-and-set/decay semantics are exercised on both backends.
CREATE TABLE login_throttle (
    throttle_key TEXT PRIMARY KEY,
    scope TEXT NOT NULL,
    failure_count INTEGER NOT NULL DEFAULT 0,
    first_failure_at TEXT NOT NULL,
    last_failure_at TEXT NOT NULL,
    locked_until TEXT NULL
);

CREATE INDEX idx_login_throttle_last_failure_at ON login_throttle (last_failure_at);
