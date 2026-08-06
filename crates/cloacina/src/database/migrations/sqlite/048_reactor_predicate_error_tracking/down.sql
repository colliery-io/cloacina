-- SQLite 3.35+ supports DROP COLUMN (the workspace already requires it).
ALTER TABLE reactor_trigger_subscriptions DROP COLUMN predicate_degraded;
ALTER TABLE reactor_trigger_subscriptions DROP COLUMN last_predicate_error_at;
ALTER TABLE reactor_trigger_subscriptions DROP COLUMN last_predicate_error;
ALTER TABLE reactor_trigger_subscriptions DROP COLUMN predicate_error_firing_id;
ALTER TABLE reactor_trigger_subscriptions DROP COLUMN predicate_error_count;
