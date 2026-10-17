# cloacina-connector-postgres

The Postgres connector of Sync: a source (full refresh, incremental by cursor,
CDC through `pg_logical_slot_get_changes`) and a destination (append,
overwrite, upsert) as one `wasm32-wasip2` component. A sync wire client over
fidius's TCP egress and `postgres-protocol`, no libpq; TLS through rustls.

Config: `url` (`postgres://user:pw@host:port/db`) or `host`, `port`, `user`,
`password`, `dbname`; `table` (default: the stream name); `sslmode` (`require`,
`verify-full`, `disable`); `sslrootcert`; `schema`; `typed_columns`.
Capability: `tcp` to the database host.

Moved from weir (COLLIERY-I-0632 / CLOACINA-T-0979).
