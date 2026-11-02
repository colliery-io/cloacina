---
title: "Postgres"
description: "The Postgres connector: a source with full refresh, incremental and CDC reads, and a destination with append, overwrite, upsert and CDC writes."
sidebar:
  order: 10
---

`connectors/postgres`, package `cloacina-connector-postgres`. A source and a
destination over a Postgres wire client written for the sandbox, with no
libpq. TLS goes through rustls. Moved from weir (CLOACINA-T-0979).

## Config

| Key | Meaning |
|---|---|
| `url` | `postgres://user:password@host:port/dbname`. Or give the parts: |
| `host`, `port`, `user`, `password`, `dbname` | The connection, when there is no `url`. |
| `table` | The table. Default: the stream name. |
| `sslmode` | `require` (default: encrypt), `verify-full` (encrypt and verify the chain and host name), or `disable`. |
| `sslrootcert` | An inline PEM CA bundle for `verify-full`. Default: the public webpki roots. |
| `schema` | The schema `discover` lists. Default `public`. |
| `typed_columns` | The destination creates typed columns inferred from the records. Default `true`; `false` writes one `data` JSONB column. |
| `on_delete` | For CDC deletes: `hard` (default) deletes the row, `tombstone` sets `tombstone_column`. |
| `tombstone_column` | Default `_deleted_at`. |

**Grant:** `{"tcp": ["<host>:<port>"]}`.

## Source

| Mode | How it reads |
|---|---|
| Full refresh | The whole table. |
| Incremental | Rows past the cursor of `cursor_field`, compared in the column's own type, ordered by it. The cursor is the largest value read. |
| CDC | Changes from a logical replication slot (`test_decoding`, through `pg_logical_slot_get_changes`). The slot is created on the first read and named after the stream. Needs `wal_level=logical`. The cursor is the last LSN. |

`discover` lists the base tables of `schema`, with their primary keys.
A key-shard partition (`{"key": ..., "shard": n, "of": m}`) reads one slice of
a table.

## Destination

| Write mode | What it does |
|---|---|
| Append | Inserts the rows. |
| Overwrite | Empties the table once, then inserts. |
| Upsert | Inserts, or updates the row with the same business keys. Replaying a segment changes nothing. |
| CDC | Applies inserts, updates and deletes in order. |

The table is created from the first record when it does not exist.
