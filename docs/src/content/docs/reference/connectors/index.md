---
title: "Sync Connectors"
description: "The compiled Sync connectors: what each reads and writes, its config, the grant it needs and the credential it takes."
sidebar:
  order: 49
---

The connectors that ship with cloacina. A **compiled** connector is a
`wasm32-wasip2` component in `connectors/`, loaded as a provider package with
a `source` and a `destination` member. A **declarative** connector is a
[connector definition](definitions/) run on the REST runtime (see
[Declarative Connectors](../../explanation/primitives/sync/declarative/)). All of them implement the
[Connector Contract](../connector-contract/) and pass their conformance file
(see [Test a Connector](../../how-to/sync/test-a-connector/)).

| Connector | Package | Reads | Writes | Grant | Credential |
|---|---|---|---|---|---|
| [Postgres](postgres/) | `cloacina-connector-postgres` | full refresh, incremental, CDC | append, overwrite, upsert, CDC | `tcp` to the database | in the config |
| [MSSQL](mssql/) | `cloacina-connector-mssql` | full refresh, incremental | no | `tcp` to the server | in the config |
| [S3](s3/) | `cloacina-connector-s3` | full refresh, incremental | no | `http` to the endpoint | `aws_sigv4` |
| [Snowflake](snowflake/) | `cloacina-connector-snowflake` | full refresh, incremental | append, overwrite, upsert, CDC | `http` to the API host | `snowflake_keypair_jwt` |
| [REST Source](rest/) | `cloacina-connector-rest` | full refresh, incremental, partitioned | no | `http` to the definition's host | the definition's auth |
| [REST Destination](rest-dest/) | `cloacina-connector-rest-dest` | no | create, upsert, CDC | `http` to the definition's host | the definition's auth |

The REST runtime runs the [corpus](corpus/): 36 sources and 2 destinations,
named by `*_definition` rather than `*_provider`.

## In a sync definition

A side of a sync names its connector and how it may reach out:

| Parameter | Value |
|---|---|
| `source_provider`, `destination_provider` | The package name. |
| `source_member`, `destination_member` | `source` or `destination`. |
| `source_config`, `destination_config` | The connector's config, as JSON text. |
| `source_grants`, `destination_grants` | A grant spec, as JSON text: `{"tcp": ["db.example.com:5432"]}` or `{"http": ["s3.amazonaws.com"]}`. Without grants the connector reaches nothing. |
| `source_credential`, `destination_credential` | A credential spec, for the connectors that take one. See [Credential Injection](../../explanation/primitives/sync/credentials/). |
| `source_definition`, `destination_definition` | A connector definition's name, instead of a provider and member. The side's config is then the user's config for the definition. |
| `source_secret`, `destination_secret` | The secret a definition's auth reads. Default: the binding `source_secret` or `destination_secret`. |

A database connector takes its password in the config, so the config is the
place for a `{"$secret": ...}` binding there.

## The catalog

`GET /v1/tenants/{tenant_id}/connectors` and `cloacinactl connector list` show
the connectors a server can run: each compiled connector provider in its
provider search path, and each declarative definition, with its roles, sync
modes and streams.

## Common to all

- An incremental read compares numbers as numbers, so a cursor at `9` still
  reads `10`.
- `check` makes one cheap call to the service, so a wrong address or key
  fails it before a sync runs.
- A record a destination cannot take becomes a dead-letter, and the write
  goes on.
