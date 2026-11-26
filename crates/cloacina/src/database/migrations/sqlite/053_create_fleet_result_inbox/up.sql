-- Fleet result inbox (CLOACINA-T-1015). See the Postgres migration of the
-- same number for what the table is for. The fleet runs on Postgres only;
-- this table exists on SQLite for schema parity and for tests. There is no
-- NOTIFY on SQLite.
-- UUID stored as BLOB (16 bytes), TIMESTAMP stored as TEXT (RFC3339 format)

CREATE TABLE fleet_result_inbox (
    id BLOB PRIMARY KEY NOT NULL,
    task_execution_id BLOB NOT NULL,
    dispatch_id TEXT NULL,
    payload BLOB NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_fleet_result_inbox_task ON fleet_result_inbox (task_execution_id);
CREATE INDEX idx_fleet_result_inbox_age ON fleet_result_inbox (created_at);
