-- CLOACI-T-0851 / ADR CLOACI-A-0012 Amendment 3: where to reach the replica
-- that owns a reactor.
--
-- THIS TABLE IS A ROUTING HINT. IT IS NOT A SOURCE OF TRUTH FOR OWNERSHIP.
--
-- Ownership is decided solely by a Postgres session-level advisory lock (see
-- `computation_graph::reactor_lock_key`). That lock proves WHO owns a reactor
-- but is not routable: it yields no address anyone can send a request to. This
-- table carries only that missing piece — the advertised address of whoever
-- currently holds the lock.
--
-- The distinction is load-bearing. A stale row here must only ever cost a
-- WASTED REDIRECT: the request arrives at a replica that no longer owns the
-- reactor, which either redirects again or falls back to the durable delivery
-- outbox. A stale row must NEVER be read as "this replica owns the reactor",
-- because that would be a second, weaker answer to a question the advisory lock
-- already answers authoritatively — and the two can disagree during a takeover.
-- Any future query that treats a row here as ownership is a bug.
--
-- Because it is only a hint, this table is deliberately NOT transactional with
-- the lock and has no lease, expiry, or heartbeat. It is written after a
-- successful claim and deleted on release; if a replica dies between the two,
-- the row is simply stale until the next owner overwrites it, and the fallback
-- path covers the gap.
--
-- Keyed by (tenant_id, reactor_name) to match how the scheduler keys its
-- reactors map. `tenant_id` is nullable for the untenanted/embedded entry, so
-- the uniqueness constraint is expressed as an index over COALESCE rather than
-- a plain UNIQUE — in SQL, NULL <> NULL, and a bare UNIQUE(tenant_id,
-- reactor_name) would happily admit many untenanted rows for one reactor.
CREATE TABLE reactor_owner_addresses (
    tenant_id TEXT NULL,
    reactor_name TEXT NOT NULL,
    address TEXT NOT NULL,
    claimed_at TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX idx_reactor_owner_addresses_key
    ON reactor_owner_addresses (COALESCE(tenant_id, ''), reactor_name);
