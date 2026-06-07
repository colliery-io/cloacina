-- Delivery outbox (CLOACI-I-0115 / spec S-0012 / ADR A-0006).
--
-- A durable, ack-tracked, recipient-addressed, multi-kind push-delivery outbox
-- for the interservice communication substrate. This is the system of record
-- for substrate delivery: a row is written in the same transaction as the
-- state change that produced it, retained until acked, and woken for delivery
-- by LISTEN/NOTIFY (added in T-0626).
--
-- Distinct from `task_outbox`, which is the transient, competing-consumer
-- scheduler->executor claim queue (deleted on claim). Rows here are addressed
-- to a specific recipient and carry a payload.

CREATE TABLE delivery_outbox (
    id BIGSERIAL PRIMARY KEY,
    recipient TEXT NOT NULL,
    kind TEXT NOT NULL,
    tenant_id TEXT NULL,
    payload BYTEA NOT NULL,
    delivery_state TEXT NOT NULL DEFAULT 'pending',
    delivery_attempts INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL,
    delivered_at TIMESTAMP NULL,
    acked_at TIMESTAMP NULL
);

-- Open (un-acked) rows for a recipient, ordered by id for deterministic replay
-- (T-0626 relay drain, T-0627 reconnect resync). Partial so acked rows leave
-- the hot path.
CREATE INDEX idx_delivery_outbox_recipient_open
    ON delivery_outbox (recipient, id)
    WHERE delivery_state <> 'acked';

-- Stuck-row scan for the safety-net sweeper (T-0628): open rows by age.
CREATE INDEX idx_delivery_outbox_open_age
    ON delivery_outbox (delivery_state, created_at)
    WHERE delivery_state <> 'acked';
