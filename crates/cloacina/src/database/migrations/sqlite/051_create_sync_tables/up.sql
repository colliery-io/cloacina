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
CREATE TABLE sync_stream_state (
    id BLOB PRIMARY KEY NOT NULL,
    sync_name TEXT NOT NULL,
    stream TEXT NOT NULL,
    partition_id TEXT NOT NULL DEFAULT '',
    cursor TEXT NULL,
    opaque BLOB NOT NULL,
    exhausted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_sync_stream_state_key
    ON sync_stream_state (sync_name, stream, partition_id);

CREATE TABLE sync_dead_letters (
    id BLOB PRIMARY KEY NOT NULL,
    sync_name TEXT NOT NULL,
    stream TEXT NOT NULL,
    task_execution_id BLOB NULL,
    workflow_execution_id BLOB NULL,
    record TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_sync_dead_letters_sync
    ON sync_dead_letters (sync_name, stream, created_at);
