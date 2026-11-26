-- Fleet result inbox (CLOACINA-T-1015).
--
-- An agent posts its report to any server replica. The replica that
-- dispatched the task holds the wait for it in memory. A replica that gets a
-- report it has no wait for stores it here, and the replica that waits takes
-- it (deletes the row) and resolves its wait.
--
-- `task_execution_id` is the id the wait is keyed by: a task execution id, or
-- the firing id of a graph firing. `dispatch_id` names one dispatch of a task
-- (CLOACINA-A-0016); NULL for a graph firing or an agent from before the field.
-- `payload` is the report (`AgentResultRequest`) as JSON.
--
-- A row nobody takes is a report nobody waits for. It changes no state
-- (CLOACINA-A-0017) and the server deletes it by age.

CREATE TABLE fleet_result_inbox (
    id UUID PRIMARY KEY,
    task_execution_id UUID NOT NULL,
    dispatch_id TEXT NULL,
    payload BYTEA NOT NULL,
    created_at TIMESTAMP NOT NULL
);

CREATE INDEX idx_fleet_result_inbox_task ON fleet_result_inbox (task_execution_id);
CREATE INDEX idx_fleet_result_inbox_age ON fleet_result_inbox (created_at);

-- The wake across replicas (CLOACINA-A-0006): the replica that waits listens
-- on this channel. The notification carries only the id; a missed one is
-- covered by the periodic check of the waiting replica.
CREATE OR REPLACE FUNCTION fleet_result_inbox_notify() RETURNS trigger AS $$
BEGIN
    PERFORM pg_notify('fleet_result_inbox', NEW.task_execution_id::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER fleet_result_inbox_notify_trg
    AFTER INSERT ON fleet_result_inbox
    FOR EACH ROW EXECUTE FUNCTION fleet_result_inbox_notify();
