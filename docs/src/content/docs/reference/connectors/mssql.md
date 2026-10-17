---
title: "MSSQL"
description: "The Microsoft SQL Server connector: a source with full refresh and incremental reads."
sidebar:
  order: 20
---

`connectors/mssql`, package `cloacina-connector-mssql`. A source over TDS,
the SQL Server protocol, through `vendor/tiberius-wasm`: a fork of tiberius
0.12.3 (MIT or Apache-2.0) that builds for `wasm32-wasip2`. Reads use
`FOR JSON PATH`, so the server produces the JSON rows. TLS goes through
rustls. Moved from weir (CLOACINA-T-0984).

## Config

| Key | Meaning |
|---|---|
| `host`, `database`, `user`, `password` | Required. |
| `port` | Default 1433. |
| `table` | The table, `schema.table` allowed. Default: the stream name. |
| `tls_mode` | `require` (default), `prefer`, or `disable` for a plaintext dev server. |
| `tls_ca_pem` | An inline PEM CA bundle. Default: the public webpki roots. |
| `tls_insecure_skip_verify` | Encrypt without verifying the certificate. Default `false`. |

**Grant:** `{"tcp": ["<host>:<port>"]}`.

## Source

| Mode | How it reads |
|---|---|
| Full refresh | The whole table. |
| Incremental | Rows past the cursor of `cursor_field`, ordered by it. SQL Server converts the bound cursor to the column's type, so an integer cursor at `9` reads `10`, and a datetime cursor compares as a datetime. |

`discover` lists the base tables as `schema.table`. There is no CDC and no
destination.

## Testing

SQL Server runs under the compose profile `mssql`, so `angreal services up`
does not pull its image (about 1.5 GB, amd64 only):

```bash
docker compose -f .angreal/docker-compose.yaml --profile mssql up -d --wait mssql
cargo test -p cloacina-sync --test mssql_connector -- --ignored
```

The Postgres integration lane in CI runs these.
