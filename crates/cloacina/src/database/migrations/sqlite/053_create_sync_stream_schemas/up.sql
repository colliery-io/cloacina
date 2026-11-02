-- COLLIERY-I-0635 / CLOACINA-T-1003: the captured schema of each sync stream,
-- as weir kept it. Written only inside the write task's completion
-- transaction, with the cursor. `broken` holds the reasons when the records'
-- shape changed a type the schema holds; NULL when the schema stands.
CREATE TABLE sync_stream_schemas (
    id BLOB PRIMARY KEY NOT NULL,
    sync_name TEXT NOT NULL,
    stream TEXT NOT NULL,
    schema TEXT NOT NULL,
    broken TEXT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_sync_stream_schemas_key
    ON sync_stream_schemas (sync_name, stream);
