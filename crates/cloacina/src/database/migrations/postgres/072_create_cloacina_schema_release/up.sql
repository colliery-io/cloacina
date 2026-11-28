-- CLOACINA-T-1087: the release that last migrated this schema.
-- One row (id = 1). The binary that runs the migrations writes it in the same
-- transaction, under the migration lock, and only raises it. At start a
-- server reads it before it migrates: a schema more than one minor ahead of
-- the binary is refused, one exactly one minor ahead is used without a
-- migration. A new table only. Migration 072: 069 to 071 are taken by other
-- open work.
CREATE TABLE IF NOT EXISTS cloacina_schema_release (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    written_by TEXT NOT NULL,
    written_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
