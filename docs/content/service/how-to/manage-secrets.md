---
title: "Manage Secrets"
description: "Create, rotate, list, and delete tenant secrets with cloacinactl or the web UI — values are write-only and never enter argv."
weight: 26
---

# Manage Secrets

A **secret** is a named, tenant-scoped bundle of named fields (e.g.
`db_prod = { host, user, password }`), encrypted at rest, that a workflow
references by name. This guide covers creating, rotating, listing, and deleting
secrets with `cloacinactl` and the web UI. For what a secret is and how its
plaintext is kept out of logs, see
[Secrets]({{< ref "/service/explanation/secrets" >}}); to declare and read one
from a workflow, see
[Use Secrets in a Workflow]({{< ref "/engine/workflows/how-to/use-secrets" >}}).

## Prerequisites

- `cloacinactl` installed and a reachable server, authenticating with a tenant
  **admin** key (managing secrets is admin-only self-service). Either pass
  `--server`/`--api-key` on each command or save a profile (see
  [Use CLI Profiles]({{< ref "/service/how-to/use-cli-profiles" >}})).
- The server must have a **key-encryption key** configured. Set
  `CLOACINA_SECRET_KEK` on `cloacina-server` to a base64 or hex encoding of
  exactly 32 bytes:

  ```bash
  # base64 (either encoding is accepted)
  export CLOACINA_SECRET_KEK="$(openssl rand -base64 32)"
  # or hex
  export CLOACINA_SECRET_KEK="$(openssl rand -hex 32)"
  ```

  Keep this key stable and backed up: it wraps every tenant's data key, so
  losing it makes existing secrets unrecoverable, and changing it invalidates
  them. If it is unset or malformed, every secret route returns `503` and
  workflow resolution fails closed.

## Create a secret

Values are never passed on the command line — that would leak them into shell
history and the process table. Each `--field` (`-f`) instead names a **source**
for its value:

| Form | Reads the value from |
|------|----------------------|
| `NAME=@path` | the file at `path` (one trailing newline is stripped) |
| `NAME=-` | standard input (only one field may use `-`) |
| `NAME` or `NAME=?` | an interactive prompt (echoed to stderr, value not printed) |

A literal `NAME=value` is **rejected**.

```bash
# password from a file, host/user from prompts
cloacinactl secret create db_prod \
  -f host -f user -f password=@/run/secrets/db_password \
  --server http://127.0.0.1:8080 --api-key "$ADMIN_KEY"

# a single field piped in from stdin
printf '%s' "$STRIPE_KEY" | cloacinactl secret create stripe -f api_key=- \
  --server http://127.0.0.1:8080 --api-key "$ADMIN_KEY"
```

The secret name is unique within the tenant; creating a second `db_prod`
returns a conflict. The response is **metadata only** — the field names and
timestamps, never the values you just set.

## Rotate a secret

Rotation replaces the whole field map in place. The next fire sees the new
value; an in-flight execution keeps the copy it already resolved. There is no
versioning, and no code or instance change is needed — workflows reference the
secret by name.

```bash
cloacinactl secret rotate db_prod -f password=@/run/secrets/db_password_new \
  --server http://127.0.0.1:8080 --api-key "$ADMIN_KEY"
```

## List and inspect

```bash
cloacinactl secret list --server http://127.0.0.1:8080 --api-key "$ADMIN_KEY"
cloacinactl secret get db_prod --server http://127.0.0.1:8080 --api-key "$ADMIN_KEY"
```

Both show only names, field names, and timestamps — **never values**. Add
`--output json` for scripting.

## Delete a secret

```bash
cloacinactl secret delete db_prod --server http://127.0.0.1:8080 --api-key "$ADMIN_KEY"
```

The CLI prompts for confirmation; pass `--force` to skip it in automation.

## From the web UI

The [embedded web UI]({{< ref "/service/embedded-ui" >}}) has a **Secrets** view
(admin-gated) that does the same thing: create a named secret with field rows,
rotate an existing one, and delete. Value inputs are **write-only** — they are
never populated from a read and the rotate form seeds the known field *names*
with empty values, so a value is never shown back after creation.

## REST equivalents

The CLI is a thin wrapper over these tenant-scoped endpoints (full details in
the [HTTP API Reference]({{< ref "/reference/http-api" >}})). All require a
tenant **admin** key; all return `503` when `CLOACINA_SECRET_KEK` is not
configured; reads are metadata-only.

| Action | Method + path |
|--------|---------------|
| Create | `POST /v1/tenants/{tenant_id}/secrets` |
| List | `GET /v1/tenants/{tenant_id}/secrets` |
| Get metadata | `GET /v1/tenants/{tenant_id}/secrets/{name}` |
| Rotate | `PUT /v1/tenants/{tenant_id}/secrets/{name}` |
| Delete | `DELETE /v1/tenants/{tenant_id}/secrets/{name}` |

## See also

- [Secrets]({{< ref "/service/explanation/secrets" >}}) — encryption at rest,
  the no-leak guarantee, fleet delivery, and tenant-scope authorization.
- [Use Secrets in a Workflow]({{< ref "/engine/workflows/how-to/use-secrets" >}})
  — declaring and reading secrets, and binding them on an instance.
- [Environment Variables]({{< ref "/reference/environment-variables" >}}) —
  `CLOACINA_SECRET_KEK` and the rest of the server configuration.
