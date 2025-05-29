-- SQLite version: contexts table
-- UUID stored as BLOB (16 bytes)
-- TIMESTAMP stored as TEXT (RFC3339 format)

CREATE TABLE contexts (
    id BLOB PRIMARY KEY NOT NULL,
    value TEXT NOT NULL CHECK (json_valid(value)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX contexts_created_at_idx ON contexts(created_at DESC);
