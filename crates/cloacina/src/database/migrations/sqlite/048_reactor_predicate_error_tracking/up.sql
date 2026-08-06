-- CLOACI-T-0922 — SQLite mirror of postgres migration 048. See the postgres
-- twin for the full rationale: the reactor dispatcher now holds the watermark
-- when a CEL predicate errors (instead of skipping + advancing, which silently
-- destroyed the firing) and needs a durable, bounded retry counter plus a
-- dead-letter marker.
--
-- ADD COLUMN only — no DROP+CREATE table rewrite.
ALTER TABLE reactor_trigger_subscriptions
    ADD COLUMN predicate_error_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE reactor_trigger_subscriptions
    ADD COLUMN predicate_error_firing_id TEXT;
ALTER TABLE reactor_trigger_subscriptions
    ADD COLUMN last_predicate_error TEXT;
ALTER TABLE reactor_trigger_subscriptions
    ADD COLUMN last_predicate_error_at TIMESTAMP;
ALTER TABLE reactor_trigger_subscriptions
    ADD COLUMN predicate_degraded BOOLEAN NOT NULL DEFAULT FALSE;
