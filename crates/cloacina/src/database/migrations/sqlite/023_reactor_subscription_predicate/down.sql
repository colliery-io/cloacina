-- SQLite ALTER TABLE DROP COLUMN was added in 3.35 (2021) but our
-- bundled libsqlite3-sys may pre-date that on some platforms. The
-- "rebuild table without the column" dance is the portable form, but
-- since this column is additive and NULL-tolerant, the safest down
-- migration is a no-op that documents the situation rather than
-- risking data on rollback.
SELECT 'down migration is a no-op — predicate_expression column kept';
