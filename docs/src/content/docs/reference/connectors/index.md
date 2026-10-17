---
title: "Sync Connectors"
description: "The compiled Sync connectors: what each reads and writes, its config, the grant it needs and the credential it takes."
sidebar:
  order: 49
---

The compiled connectors that ship with cloacina. Each is a `wasm32-wasip2`
component in `connectors/`, loaded as a provider package with a `source` and
a `destination` member. All of them implement the
[Connector Contract](../connector-contract/) and pass their conformance file
(see [Test a Connector](../../how-to/sync/test-a-connector/)).

| Connector | Package | Reads | Writes | Grant | Credential |
|---|---|---|---|---|---|
| [Postgres](postgres/) | `cloacina-connector-postgres` | full refresh, incremental, CDC | append, overwrite, upsert, CDC | `tcp` to the database | in the config |
| [MSSQL](mssql/) | `cloacina-connector-mssql` | full refresh, incremental | no | `tcp` to the server | in the config |
| [S3](s3/) | `cloacina-connector-s3` | full refresh, incremental | no | `http` to the endpoint | `aws_sigv4` |
| [Snowflake](snowflake/) | `cloacina-connector-snowflake` | full refresh, incremental | append, overwrite, upsert, CDC | `http` to the API host | `snowflake_keypair_jwt` |

## In a sync definition

A side of a sync names its connector and how it may reach out:

| Parameter | Value |
|---|---|
| `source_provider`, `destination_provider` | The package name. |
| `source_member`, `destination_member` | `source` or `destination`. |
| `source_config`, `destination_config` | The connector's config, as JSON text. |
| `source_grants`, `destination_grants` | A grant spec, as JSON text: `{"tcp": ["db.example.com:5432"]}` or `{"http": ["s3.amazonaws.com"]}`. Without grants the connector reaches nothing. |
| `source_credential`, `destination_credential` | A credential spec, for the connectors that take one. See [Credential Injection](../../explanation/primitives/sync/credentials/). |

A database connector takes its password in the config, so the config is the
place for a `{"$secret": ...}` binding there.

## Common to all

- An incremental read compares numbers as numbers, so a cursor at `9` still
  reads `10`.
- `check` makes one cheap call to the service, so a wrong address or key
  fails it before a sync runs.
- A record a destination cannot take becomes a dead-letter, and the write
  goes on.
