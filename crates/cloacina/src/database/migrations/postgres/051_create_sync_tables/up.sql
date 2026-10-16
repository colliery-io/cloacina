-- COLLIERY-I-0632 / CLOACINA-T-0974: the durable state of a sync.
--
-- sync_stream_state holds, per (sync, stream, partition), the cursor the last
-- completed write task committed and the connector's opaque resume bytes. The
-- row is written only inside a task's completion transaction, after the
-- destination has durably written the segment (CLOACINA-A-0013 amendment 2),
-- so the cursor never passes an unwritten record. `exhausted` says the source
-- reported its end at this state: the pass is done.
--
-- sync_dead_letters holds the records a sync could not process, with the
-- reason, stamped with the task and workflow execution that produced them.
-- Written in the same completion transaction.
--
-- Tenant scoping comes from the schema the connection's search_path selects,
-- as for every other table here; there is no tenant column.
CREATE TABLE sync_stream_state (
    id UUID PRIMARY KEY,
    sync_name TEXT NOT NULL,
    stream TEXT NOT NULL,
    partition_id TEXT NOT NULL DEFAULT '',
    cursor TEXT NULL,
    opaque BYTEA NOT NULL,
    exhausted BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX idx_sync_stream_state_key
    ON sync_stream_state (sync_name, stream, partition_id);

CREATE TABLE sync_dead_letters (
    id UUID PRIMARY KEY,
    sync_name TEXT NOT NULL,
    stream TEXT NOT NULL,
    task_execution_id UUID NULL,
    workflow_execution_id UUID NULL,
    record TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL
);

CREATE INDEX idx_sync_dead_letters_sync
    ON sync_dead_letters (sync_name, stream, created_at);
