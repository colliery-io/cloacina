-- CLOACI-T-0602 — add CEL predicate to reactor_trigger_subscriptions.
-- NULL means "fire on every reactor firing" (existing unfiltered behavior).
-- Non-NULL is a CEL expression evaluated against the firing payload; the
-- scheduler only dispatches when it evaluates to true. Watermark advance
-- happens whether or not the firing dispatches.
ALTER TABLE reactor_trigger_subscriptions
    ADD COLUMN predicate_expression TEXT NULL;
