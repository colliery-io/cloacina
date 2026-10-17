---
title: "Snowflake"
description: "The Snowflake connector: a source and a destination over the SQL API, authenticated with a key-pair JWT the host signs."
sidebar:
  order: 40
---

`connectors/snowflake`, package `cloacina-connector-snowflake`. A source and
a destination over the Snowflake SQL API (`/api/v2/statements`). Moved from
weir (CLOACINA-T-0986).

## Config

| Key | Meaning |
|---|---|
| `account`, `database`, `schema`, `warehouse` | Required. |
| `role` | Optional. |
| `table` | Default: the stream name. |
| `query` | A SELECT for the source to read instead of the table. |
| `base_url` | Default `https://<account>.snowflakecomputing.com`. Set it for PrivateLink, or for a test mock. |
| `batch_rows` | Rows per write statement. Default 500. |
| `max_retries` | Retries for 429, 5xx and transport failures. Default 4. |

**Grant:** `{"http": ["<account>.snowflakecomputing.com"]}`.

**Credential:** `snowflake_keypair_jwt`, with `account`, `user` and a secret
holding `private_key`, a PKCS#8 PEM. The user must have the matching public
key (`ALTER USER ... SET RSA_PUBLIC_KEY`). The host signs the JWT; the guest
never sees the key. See [Credential Injection](../../../explanation/primitives/sync/credentials/).

## Source

| Mode | How it reads |
|---|---|
| Full refresh | The table or `query`, as one statement. |
| Incremental | The table or `query` wrapped as `SELECT * FROM (...) WHERE <cursor_field> > ? ORDER BY <cursor_field>`. The cursor is the largest value read. |

Results across every partition are read. Rows become JSON objects with
lowercased field names, and every value is a string, as the SQL API returns
it. `discover` offers one stream, `records`.

## Destination

The table is created from the first record's fields (string `VARCHAR`,
integer `NUMBER`, float `DOUBLE`, boolean `BOOLEAN`, anything else its JSON
text). Then:

| Write mode | What it does |
|---|---|
| Append | Multi-row `INSERT`, `batch_rows` at a time. |
| Overwrite | `TRUNCATE` once, then insert. |
| Upsert | `MERGE` by the business keys. Within one statement the last row for a key wins; replaying a segment changes nothing. |
| CDC | Inserts and updates by `MERGE`, deletes by key, in order. |

A record that is not a JSON object is a dead-letter. A statement Snowflake
rejects fails the write. A `202` answer is polled until it settles.

`check` runs `SELECT 1`, so a wrong account or key fails it.

## Testing

The tests run against a mock SQL API, `cloacina_testing::connectors::snowflake_mock`,
which checks the JWT against the key's fingerprint:

```bash
cargo test -p cloacina-sync --test snowflake_connector -- --include-ignored
```

The two syncs in that target also need the dev Postgres. A run against a real
Snowflake account is opt-in.
