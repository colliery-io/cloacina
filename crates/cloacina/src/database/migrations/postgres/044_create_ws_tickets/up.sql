-- CLOACI-T-0916: DB-backed single-use WebSocket auth tickets.
--
-- Tickets were previously held in a per-replica in-memory map, so a ticket
-- minted on replica A failed on replica B behind a non-affine load balancer.
-- Persisting them (mirroring the 035_create_oidc_login_flows precedent for
-- multi-replica login-flow state) makes `POST /auth/ws-ticket` +
-- WS-upgrade-with-`?token=` correct on any replica with no session affinity.
--
-- Single-use is enforced by an atomic compare-and-set:
--   UPDATE ws_tickets SET redeemed_at = now
--   WHERE ticket = $1 AND redeemed_at IS NULL AND expires_at > now
-- (rows_affected == 1 wins; a concurrent second redeem sees 0 rows).
-- Expired rows are pruned opportunistically on issue.
CREATE TABLE ws_tickets (
    ticket TEXT PRIMARY KEY,
    key_id UUID NOT NULL,
    key_name TEXT NOT NULL,
    permissions TEXT NOT NULL,
    tenant_id TEXT NULL,
    is_admin BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    redeemed_at TIMESTAMP NULL
);

-- Prune scan: expired tickets by expiry time.
CREATE INDEX idx_ws_tickets_expires_at ON ws_tickets (expires_at);
