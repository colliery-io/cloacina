-- CLOACINA-T-1087: the release that last migrated this database.
-- One row (id = 1). The binary that runs the migrations writes it and only
-- raises it. At open a binary reads it before it migrates: a database more
-- than one minor ahead of the binary is refused, one exactly one minor ahead
-- is used without a migration. A new table only. Migration 074, as Postgres.
CREATE TABLE IF NOT EXISTS cloacina_schema_release (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    written_by TEXT NOT NULL,
    written_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
