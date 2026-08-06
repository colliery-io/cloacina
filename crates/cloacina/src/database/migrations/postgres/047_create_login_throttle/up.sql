-- CLOACI-T-0923: DB-backed brute-force throttle for `/v1/auth/local/login`
-- (I-0118 OQ-13, "required before production").
--
-- Argon2id makes each guess expensive for the SERVER, not for the attacker;
-- nothing previously stopped sustained guessing against a known username.
--
-- State is persisted (NOT a per-replica map) for the T-0916 reason: a counter
-- held in one replica's memory is trivially evaded by spraying attempts across
-- replicas behind a load balancer, and a lockout decided on replica A is
-- invisible to replica B.
--
-- DUAL-KEYED, on purpose. Every failed attempt increments TWO independent
-- counters and a login is refused if EITHER is locked:
--   * `u:<tenant-or-_>/<username>` — an attacker rotating source IPs still
--     accumulates against the account they are guessing. IP-only throttling
--     would miss this entirely.
--   * `ip:<source>` — one host spraying many usernames is caught even though
--     no single username crosses its threshold. Username-only throttling would
--     miss this entirely.
-- A composite `(username, ip)` key — the obvious third option — is the worst of
-- the three: it resets for every new source IP, so it stops neither attack.
-- The IP counter's threshold is deliberately far higher than the username one
-- so a shared-NAT office does not lock itself out over a handful of typos.
--
-- Rows are keyed by that opaque throttle key. Failures for an UNKNOWN username
-- are counted exactly like failures for a real one, so the 429 a locked key
-- returns is not a user-enumeration oracle.
CREATE TABLE login_throttle (
    throttle_key TEXT PRIMARY KEY,
    scope TEXT NOT NULL,
    failure_count INTEGER NOT NULL DEFAULT 0,
    first_failure_at TIMESTAMP NOT NULL,
    last_failure_at TIMESTAMP NOT NULL,
    locked_until TIMESTAMP NULL
);

-- Prune scan: idle counters are dropped by last-failure age.
CREATE INDEX idx_login_throttle_last_failure_at ON login_throttle (last_failure_at);
