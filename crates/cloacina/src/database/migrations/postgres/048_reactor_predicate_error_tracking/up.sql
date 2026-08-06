-- CLOACI-T-0922 — CEL predicate errors must not silently black-hole a
-- subscription.
--
-- Before this change an `Err(_)` out of predicate evaluation took the same
-- path as `Ok(false)`: skip the firing AND advance the watermark. That turned
-- a transient or structural fault into permanent, invisible data loss.
--
-- The dispatcher now HOLDS the watermark on an evaluation error and retries
-- the firing on the next poll tick. These columns make the retry bounded and
-- the failure durable, so one poison firing cannot wedge the subscription
-- forever and an operator can see what happened after the fact:
--
--   predicate_error_count      consecutive eval failures on the CURRENT
--                              head-of-line firing (reset when the firing
--                              changes or a later evaluation succeeds)
--   predicate_error_firing_id  the firing the count applies to
--   last_predicate_error       truncated error text (forensic trail; never
--                              cleared, so the evidence survives recovery)
--   last_predicate_error_at    when that error was recorded
--   predicate_degraded         TRUE once a firing was dead-lettered (bound
--                              exceeded, watermark force-advanced); cleared
--                              by the next successful evaluation
ALTER TABLE reactor_trigger_subscriptions
    ADD COLUMN predicate_error_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE reactor_trigger_subscriptions
    ADD COLUMN predicate_error_firing_id UUID;
ALTER TABLE reactor_trigger_subscriptions
    ADD COLUMN last_predicate_error TEXT;
ALTER TABLE reactor_trigger_subscriptions
    ADD COLUMN last_predicate_error_at TIMESTAMP;
ALTER TABLE reactor_trigger_subscriptions
    ADD COLUMN predicate_degraded BOOLEAN NOT NULL DEFAULT FALSE;
