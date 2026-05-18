-- CLOACI-T-0602 — add CEL predicate to reactor_trigger_subscriptions.
-- Mirror of postgres migration 026. ADD COLUMN is the right shape here:
-- per project convention we avoid DROP+CREATE sqlite migrations, and the
-- column is genuinely additive (NULL preserves existing behavior).
ALTER TABLE reactor_trigger_subscriptions
    ADD COLUMN predicate_expression TEXT NULL;
