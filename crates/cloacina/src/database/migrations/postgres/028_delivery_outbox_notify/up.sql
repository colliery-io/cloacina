-- Delivery-outbox NOTIFY trigger (CLOACI-I-0115 / T-0626).
--
-- The load-bearing cross-replica wake of the substrate (ADR A-0006). An
-- AFTER INSERT trigger emits a NOTIFY on the `delivery_outbox` channel carrying
-- only the new row's id (well under the 8 KB NOTIFY cap — OQ-F; the payload
-- itself lives only in the table). Postgres queues the notification and
-- delivers it on COMMIT, so a relay LISTENing on any replica is woken without
-- polling. Producer-agnostic: fires for every insert path.
--
-- Postgres-only: SQLite has no LISTEN/NOTIFY and the substrate is not driven
-- on the single-process daemon, so there is no sibling SQLite migration.

CREATE OR REPLACE FUNCTION delivery_outbox_notify() RETURNS trigger AS $$
BEGIN
    PERFORM pg_notify('delivery_outbox', NEW.id::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER delivery_outbox_notify_trg
    AFTER INSERT ON delivery_outbox
    FOR EACH ROW EXECUTE FUNCTION delivery_outbox_notify();
