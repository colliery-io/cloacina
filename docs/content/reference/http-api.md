---
title: "HTTP API Reference"
description: "Complete REST API reference for the Cloacina API server (cloacinactl server start)"
weight: 6
aliases:
  - "/platform/reference/http-api/"

---

# HTTP API Reference

The Cloacina API server (`cloacinactl server start`) exposes a REST
API backed by PostgreSQL or SQLite for managing API keys, tenants,
workflows, executions, triggers, and computation-graph health.

All authenticated routes are mounted under the `/v1/` prefix. The
unauthenticated probes `/health`, `/ready`, and `/metrics` are at the
root. The server subcommand was renamed from `cloacinactl serve` to
`cloacinactl server start` in an earlier release; older docs may
still mention the old name.

## Universal response invariants

Every response from every route in this document carries these two
invariants. They are not repeated per-endpoint below.

### Error envelope (`ApiError`)

Every non-2xx response — regardless of endpoint or status code — has
this JSON body:

```json
{
  "error": "human-readable message",
  "code": "machine_readable_code"
}
```

The error-body examples shown later in this document omit the `code`
field for brevity. **In every case the real response body also
includes a stable `code` field** clients can switch on. The full
catalog of codes by route and status, plus client retry guidance,
lives in [API Error Envelope]({{< ref "api-error-envelope" >}}).

### Request correlation: `x-request-id`

Every response (successful and error alike) carries an
`x-request-id` response header set by the outermost middleware. The
same ID appears in the server's structured logs as the `request_id`
span field. Capture this header on every non-2xx response — it's the
only identifier that ties a client-observed failure to the server's
logs. If the client supplies an inbound `x-request-id` header the
middleware honours it (enables end-to-end trace propagation).

### Live event streaming (WebSocket)

Live execution-event streaming is delivered over the WebSocket delivery
substrate, not SSE. The CLI's `execution events --follow` mints a
single-use WebSocket ticket (`POST /v1/auth/ws-ticket`), connects to the
delivery endpoint addressed at `exec_events:<execution_id>`, and tails events
live until interrupted. For a historical snapshot, poll
`GET /v1/tenants/{tenant_id}/executions/{exec_id}/events?since=…`; `--since` and
`--follow` cannot be combined yet (cursor support is future work). See the
[WebSocket Protocol]({{< ref "/reference/websocket-protocol" >}}) for the
envelope and ticket flow.

## Authentication

All endpoints except health checks require a valid API key passed as a Bearer token:

```
Authorization: Bearer clk_a1b2c3d4e5f6...
```

**Key format:** API keys use the `clk_` prefix followed by a cryptographically random string.

**Validation flow:**

1. Extract the `Authorization: Bearer <key>` header
2. Hash the key with SHA-256
3. Check the LRU cache (256 entries, 30-second TTL)
4. On cache miss, validate against the database
5. On success, insert `AuthenticatedKey` into request extensions

**Error responses:**

| Status | Body | Cause |
|---|---|---|
| `401` | `{"error": "missing or malformed Authorization header"}` | No `Authorization` header or not `Bearer` scheme |
| `401` | `{"error": "invalid or revoked API key"}` | Key not found or has been revoked |
| `500` | `{"error": "internal error during authentication"}` | Database error during validation |

## Public Endpoints

These endpoints require no authentication.

### GET /health

Liveness check. Always returns 200.

**Response:** `200 OK`

```json
{"status": "ok"}
```

### GET /ready

Readiness check. Verifies two things: the database connection pool
is healthy, and no loaded computation graphs have crashed.

**Response:** `200 OK`

```json
{"status": "ready"}
```

**Response:** `503 Service Unavailable` — database path

```json
{"status": "not ready", "reason": "database unreachable"}
```

**Response:** `503 Service Unavailable` — crashed-graph path

```json
{
  "status": "not ready",
  "reason": "crashed computation graphs",
  "crashed_graphs": ["pricing_graph", "alerts_graph"]
}
```

The `crashed_graphs` array names every loaded graph whose reactor
task is no longer running. The graph scheduler's supervision loop
attempts to restart crashed graphs every 5 seconds; if a graph stays
crashed past `MAX_RECOVERY_ATTEMPTS` (5 consecutive failures) it is
permanently abandoned and remains in this list until the package is
reloaded.

### GET /metrics

Prometheus metrics endpoint.

**Response:** `200 OK` with `Content-Type: text/plain; version=0.0.4`

```
# HELP cloacina_up Server is running
# TYPE cloacina_up gauge
cloacina_up 1
```

### GET /openapi.json

Machine-readable OpenAPI contract for the server. This is the same document the `cloacinactl server emit-openapi` subcommand writes to disk.

**Response:** `200 OK` with `Content-Type: application/json` — the OpenAPI spec.

## Login and Sessions

These endpoints establish and manage an interactive login session. The
**login** endpoints are **public** (the caller has no bearer key yet — they
mint one), while the **session-lifecycle** endpoints act on the caller's own
key and require authentication.

| Method | Path | Auth | Purpose |
|--------|------|------|---------|
| `POST` | `/v1/auth/local/login` | Public | Exchange local-account credentials for API key(s). Returns the membership set (one scoped key per tenant the account belongs to). |
| `GET`  | `/v1/auth/oidc/login` | Public | Begin the OIDC authorization-code flow — redirects the browser to the configured identity provider. Only mounted when OIDC is configured (`CLOACINA_OIDC_ISSUER` et al.). |
| `GET`  | `/v1/auth/callback` | Public | OIDC redirect target. Validates the ID token, maps claims to principals via `CLOACINA_OIDC_MAP`, and mints one scoped key per matched tenant. On success it returns the membership set as JSON, or (when `CLOACINA_OIDC_SUCCESS_REDIRECT` is set) redirects to the SPA with the memberships in the URL fragment. |
| `POST` | `/v1/auth/refresh` | Authenticated (any role) | Refresh the caller's own login session. |
| `POST` | `/v1/auth/logout` | Authenticated (any role) | End the caller's own login session. |
| `GET`  | `/v1/auth/whoami` | Authenticated (any role) | Return the caller's own tenant scope and role, so a UI can gate write/admin controls. |

See [Configure OIDC / SSO login]({{< ref "/service/how-to/configure-oidc-sso" >}})
for the identity-provider setup and claim-mapping policy.

## Key Management

### POST /v1/auth/keys

Create a new API key. The plaintext key is returned exactly once and cannot be retrieved again. Requires admin role.

**Request:**

```json
{
  "name": "ci-deploy",
  "role": "admin"
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | yes | Human-readable name for the key |
| `role` | string | no | Key role. Lowercase string: `"admin"`, `"write"`, or `"read"`. Defaults to `"admin"`. |

> **Naming note:** the request field is `role`; the response field
> is `permissions`. They carry the same value (e.g., `"admin"`).
> The split is a historical artifact and intentional for backward
> compatibility — clients should send `role` in requests and read
> `permissions` from responses.

**Response:** `201 Created`

```json
{
  "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "name": "ci-deploy",
  "key": "clk_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6",
  "permissions": "admin",
  "tenant_id": null,
  "is_admin": false,
  "created_at": "2026-04-02T14:30:00+00:00"
}
```

**Errors:**

| Status | Body | Cause |
|---|---|---|
| `403` | `{"error": "insufficient permissions"}` | Caller does not have admin role |
| `500` | `{"error": "failed to create API key"}` | Database error |

### POST /v1/auth/ws-ticket

Exchange a Bearer token for a single-use WebSocket ticket. The ticket can be passed as a query parameter on WebSocket upgrade requests, avoiding long-lived API keys in URLs.

**Capacity & lifecycle:** Tickets are stored in an in-memory store
bounded to **1024 unconsumed tickets** with a **60-second TTL**.
Tickets are single-use — the first WebSocket upgrade consuming the
ticket invalidates it. If the store reaches capacity, the ticket
nearest to expiry is evicted; rapid `/v1/auth/ws-ticket` calls
without consumption can silently exhaust capacity. Plan ticket
issuance to match your client connection rate; if you hold a
ticket but disconnect without upgrading, the ticket is wasted and
you must request a new one.

**Request:** Bearer token in `Authorization` header. No request body.

**Response:** `200 OK`

```json
{
  "ticket": "a3f8c1d2-b4e5-6789-0abc-def123456789",
  "expires_in_seconds": 60
}
```

The ticket is single-use and expires after 60 seconds. See the [WebSocket Protocol]({{< ref "websocket-protocol" >}}) reference for how tickets are used during connection upgrade.

### GET /v1/auth/keys

List all API keys. No hashes or plaintext values are returned. Requires admin role.

**Response:** `200 OK`

```json
{
  "keys": [
    {
      "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
      "name": "ci-deploy",
      "permissions": "admin",
      "tenant_id": null,
      "is_admin": false,
      "created_at": "2026-04-02T14:30:00+00:00",
      "revoked": false
    }
  ]
}
```

**Errors:**

| Status | Body | Cause |
|---|---|---|
| `403` | `{"error": "insufficient permissions"}` | Caller does not have admin role |
| `500` | `{"error": "failed to list API keys"}` | Database error |

### DELETE /v1/auth/keys/{key_id}

Revoke an API key. The key is immediately invalidated (the cache is cleared).

**Path parameters:**

| Parameter | Type | Description |
|---|---|---|
| `key_id` | UUID | The key's unique identifier |

**Response:** `200 OK`

```json
{"status": "revoked", "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479"}
```

**Errors:**

| Status | Body | Cause |
|---|---|---|
| `400` | `{"error": "invalid key ID format"}` | `key_id` is not a valid UUID |
| `404` | `{"error": "key not found or already revoked"}` | Key does not exist or was already revoked |
| `500` | `{"error": "failed to revoke API key"}` | Database error |

### POST /v1/tenants/{tenant_id}/keys

Create an API key scoped to a specific tenant. Only `is_admin` (god-mode) keys can create tenant-scoped keys.

**Path parameters:**

| Parameter | Type | Description |
|---|---|---|
| `tenant_id` | string | Tenant identifier (schema name) |

**Request:**

```json
{
  "name": "acme-worker",
  "role": "write"
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | yes | Human-readable name for the key |
| `role` | string | no | Key role: `"admin"`, `"write"`, or `"read"`. Defaults to `"admin"`. |

**Response:** `201 Created`

```json
{
  "id": "b58cc437-2a56-70e0-2b2c-3d479f47ac10",
  "name": "acme-worker",
  "key": "clk_x9y8z7w6v5u4t3s2r1q0p9o8n7m6l5k4",
  "permissions": "write",
  "tenant_id": "tenant_acme",
  "is_admin": false,
  "created_at": "2026-04-02T15:00:00+00:00"
}
```

**Errors:**

| Status | Body | Cause |
|---|---|---|
| `403` | `{"error": "admin access required"}` | Caller is not an `is_admin` (god-mode) key |
| `500` | `{"error": "failed to create API key"}` | Database error |

### GET /v1/tenants/{tenant_id}/keys

List the API keys scoped to a tenant. **Tenant-admin** — a caller with the
`admin` role within `{tenant_id}` may list its own tenant's keys; a god-mode
(`is_admin`) key may list any tenant's. No hashes or plaintext are returned.

**Path parameters:**

| Parameter | Type | Description |
|---|---|---|
| `tenant_id` | string | Tenant identifier (schema name) |

**Response:** `200 OK` — same key-listing shape as `GET /v1/auth/keys`, filtered to the tenant.

**Errors:**

| Status | Body | Cause |
|---|---|---|
| `403` | `{"error": "insufficient permissions"}` | Caller is not a tenant-admin for `{tenant_id}` (and not god-mode) |
| `500` | `{"error": "failed to list API keys"}` | Database error |

### DELETE /v1/tenants/{tenant_id}/keys/{key_id}

Revoke a tenant-scoped API key. **Tenant-admin.** The handler additionally
verifies the target key belongs to `{tenant_id}`, so a tenant-admin cannot
revoke another tenant's keys.

**Path parameters:**

| Parameter | Type | Description |
|---|---|---|
| `tenant_id` | string | Tenant identifier (schema name) |
| `key_id` | UUID | The key's unique identifier |

**Response:** `200 OK`

```json
{"status": "revoked", "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479"}
```

**Errors:**

| Status | Body | Cause |
|---|---|---|
| `403` | `{"error": "insufficient permissions"}` | Caller is not a tenant-admin for `{tenant_id}`, or the key belongs to a different tenant |
| `404` | `{"error": "key not found or already revoked"}` | Key does not exist or was already revoked |

## Local Accounts

Local accounts are username/password identities scoped to a tenant. They log in
via `POST /v1/auth/local/login` (see [Login and Sessions](#login-and-sessions)) to
mint API keys. All management endpoints are **tenant-admin** — a caller with the
`admin` role within `{tenant_id}` manages that tenant's accounts; god-mode
(`is_admin`) may manage any tenant's.

| Method | Path | Auth | Purpose |
|--------|------|------|---------|
| `POST` | `/v1/tenants/{tenant_id}/accounts` | Tenant-admin | Create a local account in the tenant. |
| `GET`  | `/v1/tenants/{tenant_id}/accounts` | Tenant-admin | List the tenant's local accounts. |
| `DELETE` | `/v1/tenants/{tenant_id}/accounts/{account_id}` | Tenant-admin | Disable a local account. |
| `POST` | `/v1/tenants/{tenant_id}/accounts/{account_id}/password` | Tenant-admin | Reset an account's password. |

**Errors** (all four): `403` `{"error": "insufficient permissions"}` when the
caller is not a tenant-admin for `{tenant_id}` (and not god-mode).

## Tenant Management

Tenants are isolated PostgreSQL schemas. Each tenant gets its own schema, database user, permissions, and migrations.

> **Note:** The `tenant_id` used in URL paths (e.g., `/v1/tenants/{tenant_id}/workflows`) corresponds to the `schema_name` value used when creating the tenant via `POST /v1/tenants`.

### POST /v1/tenants

Create a new tenant. **Admin-only** (requires an `is_admin` key).

**Request:**

```json
{
  "name": "tenant_acme",
  "description": "ACME Corp production tenant",
  "password": null
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | yes | Tenant identifier — doubles as the Postgres schema name **and** the database username. Must be alphanumeric + underscore (validated server-side). |
| `description` | string | no | Operator-facing metadata; surfaced in audit logs and listing responses. |
| `password` | string | no | Optional password. Omit or set `null` to trigger auto-generation (32 chars, ~202 bits entropy). |

> **Breaking change:** the previous body
> shape `{schema_name, username, password}` is no longer accepted.
> Direct API consumers must migrate to `{name, description?, password?}`.
> The schema name and database username are both derived from `name`
> to keep the public API ergonomic.

**Response:** `201 Created`

```json
{
  "name": "tenant_acme",
  "username": "tenant_acme",
  "description": "ACME Corp production tenant"
}
```

> **Security:** The tenant password is **never returned** in the response, even when auto-generated. The password is set during provisioning and is not surfaced over the API. Operators who need the password must capture it at provisioning time via the database admin tooling, not via this endpoint.

**Errors** (envelope per [API Error Envelope]({{< ref "api-error-envelope" >}})):

| Status | `code` | Cause |
|---|---|---|
| `400` | `tenant_creation_failed` | `DatabaseAdmin::create_tenant` rejected (invalid name, schema exists, Postgres permission denied). |
| `403` | `admin_required` | Caller is not an `is_admin` key. |

### GET /v1/tenants

List all tenant schemas.

**Response:** `200 OK`

```json
{
  "tenants": [
    {"schema_name": "tenant_acme"},
    {"schema_name": "tenant_globex"}
  ]
}
```

**Errors:**

| Status | Body |
|---|---|
| `500` | `{"error": "<detail>"}` |

### DELETE /v1/tenants/{schema_name}

Remove a tenant via the **4-step teardown orchestration**. **Admin-only** (requires an `is_admin` key).

The steps are top-down and each emits a structured audit event with
duration:

1. **Revoke API keys** for the tenant (closes the auth surface — new
   requests against the tenant start failing immediately).
2. **Evict the tenant's `DefaultRunner`** from `TenantRunnerCache`,
   awaiting a bounded graceful drain (`--tenant-deletion-drain-timeout-s`
   server flag, default 30s). Past the timeout the runner is
   **hard-evicted** — any task that ignored cooperative cancellation
   will error on its next DB write once step 4 drops the schema.
3. **Evict the tenant's `Database`** from `TenantDatabaseCache`
   (releases the per-tenant connection pool).
4. **Drop the schema + user** via `DatabaseAdmin::remove_tenant`
   (CASCADE).

Per-step failures bail out; earlier steps stay committed (each step
is idempotent), so a retry resumes from the failure point. The
caller sees a single `200` on overall success or a `400` /
`500` with the failing step's error if any step fails.

**Path parameters:**

| Parameter | Type | Description |
|---|---|---|
| `schema_name` | string | The tenant's schema name |

**Response:** `200 OK`

```json
{
  "status": "removed",
  "schema_name": "tenant_acme",
  "revoked_keys": 3,
  "runner_evicted": true,
  "db_cache_evicted": true
}
```

The `revoked_keys` field counts API keys revoked in step 1.
`runner_evicted` / `db_cache_evicted` are `true` if the cache had a
live entry at teardown time (false if the cache was cold — still a
successful teardown).

**Errors** (envelope per [API Error Envelope]({{< ref "api-error-envelope" >}})):

| Status | `code` | Cause |
|---|---|---|
| `400` | `tenant_removal_failed` | Step 4 (schema drop) failed. Steps 1-3 may have committed. Retry is idempotent. |
| `500` | `internal_error` | Step 1 (key revocation) failed. Steps 2-4 not attempted. |
| `403` | `admin_required` | Caller is not an `is_admin` key. |

## Tenant Secrets

Tenant-scoped, encrypted [secrets]({{< ref "/service/explanation/secrets" >}}) —
named bundles of named fields a workflow references by name. **All five routes
require a tenant `admin` key** (the caller is confined to `{tenant_id}`), and all
return `503 secrets_not_configured` when the server has no `CLOACINA_SECRET_KEK`
configured. **Reads are metadata-only** — list and get return names, field
names, and timestamps, never a plaintext or ciphertext value.

| Method + path | Purpose |
|---|---|
| `POST /v1/tenants/{tenant_id}/secrets` | Create a secret from a `{ "name": ..., "fields": { field: value, ... } }` body. Returns `201` with metadata only. `409 secret_exists` if the name is taken. |
| `GET /v1/tenants/{tenant_id}/secrets` | List secret metadata (names, field names, timestamps). No values. |
| `GET /v1/tenants/{tenant_id}/secrets/{name}` | One secret's metadata. No values. `404 secret_not_found` if absent. |
| `PUT /v1/tenants/{tenant_id}/secrets/{name}` | Rotate values in place from a `{ "fields": { ... } }` body. Returns metadata only; the next fire sees the new value. `404 secret_not_found` if absent. |
| `DELETE /v1/tenants/{tenant_id}/secrets/{name}` | Delete a secret. `404 secret_not_found` if absent. |

Metadata responses carry `id`, `name`, `field_names`, `created_at`, and
`updated_at`. Managing secrets is documented in
[Manage Secrets]({{< ref "/service/how-to/manage-secrets" >}}).

## Workflow Packages

Workflow packages are `.cloacina` archives uploaded via multipart form data, scoped to a tenant.

### POST /v1/tenants/{tenant_id}/workflows

Upload a workflow package.

**Content-Type:** `multipart/form-data`

**Form fields:**

| Field | Type | Description |
|---|---|---|
| `file` | binary | The `.cloacina` package archive |

**Example (curl):**

```bash
curl -X POST http://localhost:8080/tenants/tenant_acme/workflows \
  -H "Authorization: Bearer clk_a1b2c3d4..." \
  -F "file=@my_workflow.cloacina"
```

**Response:** `201 Created`

```json
{
  "package_id": "d290f1ee-6c54-4b01-90e6-d701748f0851",
  "tenant_id": "tenant_acme"
}
```

**Errors:**

| Status | Body | Cause |
|---|---|---|
| `400` | `{"error": "no 'file' field in multipart request"}` | Missing file field |
| `400` | `{"error": "empty package file"}` | Zero-byte file |
| `400` | `{"error": "<detail>"}` | Package validation or registration failure |
| `500` | `{"error": "internal registry error"}` | Registry initialization failure |

### GET /v1/tenants/{tenant_id}/workflows

List all registered workflows for a tenant.

**Response:** `200 OK`

```json
{
  "tenant_id": "tenant_acme",
  "workflows": [
    {
      "id": "d290f1ee-6c54-4b01-90e6-d701748f0851",
      "package_name": "etl_pipeline",
      "version": "1.2.0",
      "description": "Extract, transform, and load data",
      "tasks": ["extract", "transform", "load"],
      "created_at": "2026-04-01T10:00:00+00:00"
    }
  ]
}
```

### GET /v1/tenants/{tenant_id}/workflows/{name}

Get details for a specific workflow by package name.

**Path parameters:**

| Parameter | Type | Description |
|---|---|---|
| `tenant_id` | string | Tenant identifier |
| `name` | string | Workflow package name |

**Response:** `200 OK`

```json
{
  "tenant_id": "tenant_acme",
  "id": "d290f1ee-6c54-4b01-90e6-d701748f0851",
  "package_name": "etl_pipeline",
  "workflow_name": "etl_pipeline",
  "version": "1.2.0",
  "description": "Extract, transform, and load data",
  "tasks": ["extract", "transform", "load"],
  "created_at": "2026-04-01T10:00:00+00:00",
  "build_status": "success",
  "build_error": null,
  "paused": false,
  "declared_params": [
    {
      "name": "source_url",
      "schema": {"type": "string"},
      "required": true
    }
  ],
  "task_graph": []
}
```

| Field | Type | Description |
|---|---|---|
| `workflow_name` | string | Executable workflow name (the identifier to execute by). Differs from `package_name` under the standard convention (package `demo-slow-rust` → workflow `demo_slow_workflow`); falls back to `package_name` for packages predating workflow-name persistence. |
| `build_status` | string | Real build state: `pending` / `building` / `failed` / `success`. |
| `build_error` | string \| null | Build failure detail when `build_status` is `failed`. |
| `paused` | boolean | Whether this workflow is paused. A paused workflow refuses new executions until resumed. |
| `declared_params` | array | Declared input params (named, JSON-Schema-typed slots) the workflow accepts at execute time. Empty when undeclared; same slot shape as the `/interface` surfaces. The execute endpoint validates the submitted `context` against these. |
| `task_graph` | array | Task dependency graph (nodes + upstream deps) for rendering the DAG. Empty for packages predating task-graph persistence. |

**Errors:**

| Status | Body |
|---|---|
| `404` | `{"error": "workflow 'etl_pipeline' not found"}` |

### GET /v1/tenants/{tenant_id}/workflows/{name}/source

Return the original source files retained in the package's `.cloacina`
archive, surfaced **read-only** for display. Source is
independent of build state, so it's available even while a package is
building or after a failed build. `name` may be a package name or a
package UUID (matching `GET .../workflows/{name}`).

**Path parameters:**

| Parameter | Type | Description |
|---|---|---|
| `tenant_id` | string | Tenant identifier |
| `name` | string | Workflow package name or package UUID |

**Response:** `200 OK`

```json
{
  "tenant_id": "tenant_acme",
  "id": "d290f1ee-6c54-4b01-90e6-d701748f0851",
  "package_name": "etl_pipeline",
  "workflow_name": "etl_pipeline",
  "version": "1.2.0",
  "files": [
    {
      "path": "package.toml",
      "language": "toml",
      "contents": "[package]\nname = \"etl_pipeline\"\n…"
    },
    {
      "path": "src/lib.rs",
      "language": "rust",
      "contents": "use cloacina::*;\n…"
    }
  ]
}
```

| Field | Type | Description |
|---|---|---|
| `files[].path` | string | Path relative to the package source root, forward slashes (e.g. `src/lib.rs`, `package.toml`). |
| `files[].language` | string \| null | Best-effort language id from the file extension (`rust`, `python`, `toml`, …) for syntax highlighting; `null` when unknown. |
| `files[].contents` | string | UTF-8 file contents. Binary and oversized files are omitted from the list. |

**Errors:**

| Status | Body |
|---|---|
| `404` | `{"error": "workflow 'etl_pipeline' not found"}` |

### POST /v1/tenants/{tenant_id}/workflows/{name}/pause

Pause a workflow. Blocks new executions of the workflow
(manual **and** triggered) until resumed. In-flight executions are
unaffected. `name` may be the workflow name or package name. Requires a
`write`-or-better key.

**Response:** `200 OK`

```json
{
  "tenant_id": "tenant_acme",
  "id": "d290f1ee-6c54-4b01-90e6-d701748f0851",
  "name": "etl_pipeline",
  "status": "paused",
  "paused": true
}
```

The `status` field is `"paused"` here and `"resumed"` on the resume
route; `paused` reflects the resulting state. The same `paused` flag
surfaces on `GET .../workflows/{name}`.

**Errors:**

| Status | Body |
|---|---|
| `404` | `{"error": "workflow 'etl_pipeline' not found"}` |

### POST /v1/tenants/{tenant_id}/workflows/{name}/resume

Resume a paused workflow. New executions are accepted
again. Same response shape as `/pause` with `status: "resumed"` and
`paused: false`. Requires a `write`-or-better key.

### DELETE /v1/tenants/{tenant_id}/workflows/{name}/{version}

Unregister a specific workflow version.

**Path parameters:**

| Parameter | Type | Description |
|---|---|---|
| `tenant_id` | string | Tenant identifier |
| `name` | string | Workflow package name |
| `version` | string | Semantic version |

**Response:** `200 OK`

```json
{
  "status": "deleted",
  "package_name": "etl_pipeline",
  "version": "1.2.0"
}
```

**Errors:**

| Status | Body |
|---|---|
| `404` | `{"error": "<detail>"}` |

## Executions

### POST /v1/tenants/{tenant_id}/workflows/{name}/execute

Execute a workflow. Returns immediately with a scheduled execution ID.

**Request:**

```json
{
  "context": {
    "source_url": "s3://bucket/data.csv",
    "batch_size": 1000
  }
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `context` | object | no | JSON key-value pairs to inject into the workflow context |

**Response:** `202 Accepted`

```json
{
  "execution_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "workflow_name": "etl_pipeline",
  "tenant_id": "tenant_acme",
  "status": "scheduled"
}
```

**Errors** (envelope per [API Error Envelope]({{< ref "api-error-envelope" >}})):

| Status | `code` | Cause |
|---|---|---|
| `400` | `workflow_input_invalid` | The submitted `context` failed validation against the workflow's `declared_params`. Undeclared workflows accept free-form context and never raise this. |
| `400` | (other) | Generic execution failure (`{"error": "<detail>"}`). |
| `409` | `workflow_paused` | The workflow is paused; resume it before executing. |

### GET /v1/tenants/{tenant_id}/executions

List workflow executions for a tenant. Supports filtering and
pagination (previously these query params
were silently discarded).

**Query parameters:**

| Param | Type | Default | Description |
|---|---|---|---|
| `status` | string | (none) | Filter by execution status (e.g., `Pending`, `Running`, `Completed`, `Failed`). |
| `workflow` | string | (none) | Filter by workflow name (exact match). |
| `limit` | integer | `100` | Page size. Min `1`, max `1000`. |
| `offset` | integer | `0` | Page offset. Must be ≥ 0. |

**Errors specific to this endpoint:**

| Status | `code` | Cause |
|---|---|---|
| `400` | `invalid_pagination` | `limit` outside `[1, 1000]` or `offset` negative. |

**Response:** `200 OK`

```json
{
  "tenant_id": "tenant_acme",
  "executions": [
    {
      "id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
      "pipeline_name": "etl_pipeline",
      "status": "running",
      "started_at": "2026-04-02T14:35:00+00:00",
      "completed_at": null
    }
  ]
}
```

### GET /v1/tenants/{tenant_id}/executions/{exec_id}

Get execution status.

**Path parameters:**

| Parameter | Type | Description |
|---|---|---|
| `tenant_id` | string | Tenant identifier |
| `exec_id` | UUID | Execution identifier |

**Response:** `200 OK`

```json
{
  "tenant_id": "tenant_acme",
  "execution_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "status": "Completed"
}
```

**Errors:**

| Status | Body | Cause |
|---|---|---|
| `400` | `{"error": "invalid execution ID"}` | `exec_id` is not a valid UUID |
| `404` | `{"error": "<detail>"}` | Execution not found |

### GET /v1/tenants/{tenant_id}/executions/{exec_id}/events

Get the execution event log for a specific execution.

**Response:** `200 OK`

```json
{
  "tenant_id": "tenant_acme",
  "execution_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "events": [
    {
      "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
      "event_type": "task_started",
      "event_data": "{\"task_id\": \"extract\"}",
      "created_at": "2026-04-02T14:35:01+00:00",
      "sequence_num": 1
    },
    {
      "id": "b2c3d4e5-f6a7-8901-bcde-f12345678901",
      "event_type": "task_completed",
      "event_data": "{\"task_id\": \"extract\"}",
      "created_at": "2026-04-02T14:35:05+00:00",
      "sequence_num": 2
    }
  ]
}
```

**Errors:**

| Status | Body |
|---|---|
| `400` | `{"error": "invalid execution ID"}` |
| `500` | `{"error": "<detail>"}` |

### GET /v1/tenants/{tenant_id}/executions/{exec_id}/tasks

List the per-task status rows for a single execution — the task-level breakdown
of an execution's progress. **Tenant-scoped read.**

**Path parameters:**

| Parameter | Type | Description |
|---|---|---|
| `tenant_id` | string | Tenant identifier |
| `exec_id` | UUID | Execution identifier |

**Response:** `200 OK` — the execution's tasks with their individual statuses.

**Errors:**

| Status | Body |
|---|---|
| `400` | `{"error": "invalid execution ID"}` |
| `404` | `{"error": "<detail>"}` |

## Triggers

Read-only listing of cron and trigger schedules.

### GET /v1/tenants/{tenant_id}/triggers

List all schedules (cron and trigger) for a tenant. Supports
pagination.

**Query parameters:**

| Param | Type | Default | Description |
|---|---|---|---|
| `limit` | integer | `100` | Page size. Min `1`, max `1000`. |
| `offset` | integer | `0` | Page offset. Must be ≥ 0. |

**Errors specific to this endpoint:**

| Status | `code` | Cause |
|---|---|---|
| `400` | `invalid_pagination` | `limit` outside `[1, 1000]` or `offset` negative. |

**Response:** `200 OK`

```json
{
  "tenant_id": "tenant_acme",
  "schedules": [
    {
      "id": "c3d4e5f6-a7b8-9012-cdef-234567890123",
      "schedule_type": "cron",
      "workflow_name": "etl_pipeline",
      "enabled": true,
      "cron_expression": "0 2 * * *",
      "trigger_name": null,
      "poll_interval_ms": null,
      "next_run_at": "2026-04-03T02:00:00+00:00",
      "last_run_at": "2026-04-02T02:00:00+00:00",
      "created_at": "2026-03-01T10:00:00+00:00"
    },
    {
      "id": "d4e5f6a7-b8c9-0123-def0-345678901234",
      "schedule_type": "trigger",
      "workflow_name": "inbox_processor",
      "enabled": true,
      "cron_expression": null,
      "trigger_name": "check_inbox",
      "poll_interval_ms": 5000,
      "next_run_at": null,
      "last_run_at": "2026-04-02T14:30:00+00:00",
      "created_at": "2026-03-15T12:00:00+00:00"
    }
  ]
}
```

### GET /v1/tenants/{tenant_id}/triggers/{name}

Get trigger details and recent executions. Matches by trigger name or workflow name.

**Path parameters:**

| Parameter | Type | Description |
|---|---|---|
| `tenant_id` | string | Tenant identifier |
| `name` | string | Trigger name or workflow name |

**Response:** `200 OK`

```json
{
  "tenant_id": "tenant_acme",
  "schedule": {
    "id": "c3d4e5f6-a7b8-9012-cdef-234567890123",
    "schedule_type": "cron",
    "workflow_name": "etl_pipeline",
    "enabled": true,
    "cron_expression": "0 2 * * *",
    "trigger_name": null
  },
  "recent_executions": [
    {
      "id": "e5f6a7b8-c9d0-1234-ef01-456789012345",
      "scheduled_time": "2026-04-02T02:00:00+00:00",
      "started_at": "2026-04-02T02:00:01+00:00",
      "completed_at": "2026-04-02T02:05:30+00:00"
    }
  ]
}
```

**Errors:**

| Status | Body |
|---|---|
| `404` | `{"error": "trigger 'my_trigger' not found"}` |

### POST /v1/tenants/{tenant_id}/triggers/{name}/fire

Manually fire a trigger, **fanning out to every subscribed workflow**
. One operator action instead of running each workflow by
hand. An optional `event` is merged into each fired workflow's context
(alongside the trigger metadata) and validated against the trigger's
declared pass-through schema (see `/interface` below). The started
executions are marked `manual`. `name` resolves by
trigger name or workflow name.

**Request:**

```json
{
  "event": {
    "symbol": "ABC",
    "price": 12.5
  }
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `event` | any JSON | no | Typed event merged into each fired workflow's context. Omit to fire with just the trigger metadata. |

**Response:** `200 OK`

```json
{
  "tenant_id": "tenant_acme",
  "trigger": "check_inbox",
  "fired": 2,
  "executions": [
    {"workflow_name": "inbox_processor", "execution_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7"},
    {"workflow_name": "inbox_archiver", "execution_id": "8d0f7780-8536-51ef-a55c-f18gd2g01bf8"}
  ]
}
```

`fired` is the fan-out count (how many subscribed workflows were
started); `executions` lists each `(workflow_name, execution_id)`.

**Errors:**

| Status | Body | Cause |
|---|---|---|
| `404` | `{"error": "<detail>"}` | No enabled subscribers for this trigger. |

### GET /v1/tenants/{tenant_id}/triggers/{name}/interface

The trigger's declared pass-through schema: the union of
the declared params of every workflow subscribed to this trigger. Empty
`slots` means an untyped trigger (free-form event). Read-only discovery
— the same slots back the validation in `/fire`, and the web UI builds a
typed fire form from them.

**Response:** `200 OK`

```json
{
  "kind": "trigger",
  "name": "check_inbox",
  "slots": [
    {
      "name": "symbol",
      "schema": {"type": "string"},
      "required": false
    }
  ]
}
```

The `slots` shape matches the [declared input interface](#get-v1healthreactorsaccumulatorsnameinterface)
used by the computation-graph surfaces (`name` / `schema` / `required`,
with an optional `default`).

### POST /v1/tenants/{tenant_id}/triggers/{name}/pause

Pause a schedule. Resolves the schedule by trigger name
or workflow name (same as `GET .../triggers/{name}`) and sets it paused
so the scheduler stops firing it. Works for **both** `trigger` and
`cron` schedules. In-flight executions are unaffected; this only gates
new ones. Requires a `write`-or-better key.

**Response:** `200 OK`

```json
{
  "tenant_id": "tenant_acme",
  "id": "d4e5f6a7-b8c9-0123-def0-345678901234",
  "name": "check_inbox",
  "status": "paused",
  "paused": true
}
```

`id` is the schedule UUID; `name` echoes the name the schedule was
addressed by; `paused` reflects the resulting state.

**Errors:**

| Status | Body |
|---|---|
| `404` | `{"error": "trigger 'my_trigger' not found"}` |

### POST /v1/tenants/{tenant_id}/triggers/{name}/resume

Resume a paused schedule. Re-arms it on the normal
schedule; missed fires are **not** caught up (skip policy). Same
response shape as `/pause` with `status: "resumed"` and `paused: false`.
Requires a `write`-or-better key.

## Computation Graph Health

Health endpoints for the computation graph system. These endpoints require authentication.

### GET /v1/health/accumulators

List accumulators visible to the caller, with health **and freshness**
. Results are filtered by each accumulator's authorization policy.

**Response:** `200 OK` — unified `{items, total}` envelope.

```json
{
  "items": [
    {
      "name": "market_data",
      "reactor": "pricing_reactor",
      "tenant_id": "public",
      "state": "live",
      "last_event_at": "2026-06-21T20:21:41.283+00:00",
      "events_total": 9861,
      "error": null,
      "status": "live"
    }
  ],
  "total": 1
}
```

| Field | Type | Description |
|---|---|---|
| `state` | string | Health label: `starting` / `connecting` / `live` / `socket_only` / `disconnected`. |
| `last_event_at` | string \| null | RFC 3339 wall-clock of the last boundary the accumulator emitted; `null` if it hasn't emitted yet. |
| `events_total` | integer \| null | Monotonic count of boundaries emitted since load. The UI derives an events/min rate from the delta across polls. |
| `error` | string \| null | Degradation detail when the source is unhealthy. |
| `reactor` / `tenant_id` | string \| null | The reactor this accumulator feeds and its owning tenant (self-registered at load). |
| `status` | object/string | Free-form health value, retained for back-compat; `state` is the typed form. |

> Staleness is derived client-side from `last_event_at` age (the web UI flags a
> source as degraded when it hasn't emitted within its freshness window).

### GET /v1/health/graphs

List loaded computation graphs with their health status. `paused` reports the pause state of the graph's reactor.

**Response:** `200 OK`

```json
{
  "graphs": [
    {
      "name": "pricing_graph",
      "health": {"state": "running"},
      "accumulators": ["market_data", "risk_params"],
      "paused": false
    }
  ]
}
```

### GET /v1/health/graphs/{name}

Get health details for a specific computation graph.

**Path parameters:**

| Parameter | Type | Description |
|---|---|---|
| `name` | string | Graph name |

**Response:** `200 OK`

```json
{
  "name": "pricing_graph",
  "health": {"state": "running"},
  "accumulators": ["market_data", "risk_params"],
  "paused": false
}
```

**Errors:**

| Status | Body |
|---|---|
| `404` | `{"error": "graph 'pricing_graph' not found"}` |

### GET /v1/health/reactors

List loaded reactors visible to the caller.
**Reactor-first:** reactors are standalone (a graph binds to a reactor,
not vice versa), so a reactor with no graph bound appears here even
though `GET /v1/health/graphs` omits it. Visibility reuses the same
tenant gate as graphs.

**Response:** `200 OK` — unified `{items, total}` envelope.

```json
{
  "items": [
    {
      "name": "pricing_reactor",
      "health": {"state": "running"},
      "accumulators": ["market_data", "risk_params"],
      "bound_graphs": ["pricing_graph"],
      "paused": false,
      "fires": 9861,
      "last_fired_at": "2026-06-21T20:21:51.300+00:00",
      "reaction_mode": "when_any",
      "input_strategy": "latest"
    }
  ],
  "total": 1
}
```

| Field | Type | Description |
|---|---|---|
| `accumulators` | array | Accumulators this reactor consumes (its inputs). |
| `bound_graphs` | array | Graphs bound to this reactor; empty when the reactor has no graph yet. |
| `paused` | boolean | Pause state of the reactor. |
| `fires` | integer | Total fires since load (the reactor's live fire counter). |
| `last_fired_at` | string \| null | RFC 3339 timestamp of the last fire; `null` if it hasn't fired yet. |
| `reaction_mode` | string \| null | Firing criteria: `when_any` / `when_all`. |
| `input_strategy` | string \| null | Input strategy: `latest` / `sequential`. |

### GET /v1/health/reactors/{name}/fires

Recent fires for a reactor, newest first. Makes the reactive
layer observable: what fired, whether it completed, how long it took, and why it
failed.

**Query parameters:** `limit` (optional, default `50`, max `200`).

**Response:** `200 OK`

```json
{
  "items": [
    { "fired_at": "2026-06-21T20:21:51.300+00:00", "ok": true,  "error": null, "duration_ms": 1 },
    { "fired_at": "2026-06-21T20:21:49.297+00:00", "ok": false, "error": "node 'evaluate' failed: …", "duration_ms": 4 }
  ],
  "total": 2
}
```

### GET /v1/health/reactors/{name}/fires/timeseries

Fire counts per minute for the last 60 minutes, oldest → newest,
gaps filled with `0`; the last entry is the current minute. Backs the
fire-activity heatmap in the web UI.

**Response:** `200 OK`

```json
{ "buckets": [0, 0, 0, 2870, 6906, 26] }
```

> Both fires endpoints are tenant-gated by the same visibility check as
> `GET /v1/health/graphs`.

### POST /v1/health/reactors/{name}/fire

Manually fire a reactor. `force_fire` fires with the reactor's
current cache; `fire_with` injects typed per-source `inputs` first, then fires.

**Request body:**

```json
{ "mode": "fire_with", "inputs": { "orderbook": { "best_bid": 100.1, "best_ask": 100.4 } } }
```

`mode` defaults to `force_fire` (where `inputs` is ignored). Each value in
`inputs` is validated against that source's declared slot schema (see
`/interface` below) and serialized to the boundary wire encoding server-side.

### POST /v1/health/accumulators/{name}/inject

Push a single typed event into a running accumulator — the REST
analogue of the accumulator WebSocket push.

**Request body:** `{ "event": { "best_bid": 100.1, "best_ask": 100.4 } }`

### GET /v1/health/{reactors|accumulators}/{name}/interface

The declared input interface: the typed slots an
operator supplies to `fire_with` / `inject`. Slot schemas are derived from the
boundary type **when it derives `schemars::JsonSchema`** — otherwise `schema` is
an empty/permissive `{}`. The web UI renders these as typed forms.

**Response:** `200 OK`

```json
{
  "kind": "accumulator",
  "name": "orderbook",
  "slots": [
    { "name": "orderbook", "schema": { "type": "object", "properties": { "best_bid": {"type":"number"}, "best_ask": {"type":"number"} } }, "required": false }
  ]
}
```

## Tenant agent fleet

These endpoints manage a tenant's **agent-capacity limit** and **self-service
fleet scaling** — the per-tenant control plane. They
set a `desired_count` (the operational target the
[fleet actuator]({{< ref "/service/explanation/execution-agent-fleet" >}}#pluggable-actuators--substrate-guard)
and autoscaler reconcile toward); they do **not** themselves start containers.

Authorization is enforced server-side by the route authz table:

- **Reading** a tenant's limit or fleet view is **tenant-scoped read** — a caller
  may read only its own tenant. Cross-tenant access is denied (`403`,
  `tenant_access_denied`); a god-mode (`is_admin`) key may read any tenant.
- **Provisioning / deprovisioning** is **tenant-admin** — a tenant self-services
  its OWN fleet (god-mode bypasses; cross-tenant is denied).
- **Setting / clearing** a tenant's limit is **platform-admin only** (`is_admin`
  god-mode); a tenant cannot raise its own ceiling (NFR-004).

### GET /v1/tenants/{tenant_id}/limits

The tenant's effective agent-capacity limit. Tenant-scoped read.

**Path parameters:**

| Parameter | Type | Description |
|---|---|---|
| `tenant_id` | string | Tenant identifier |

**Response:** `200 OK`

```json
{
  "tenant_id": "tenant_acme",
  "default_max_agents": 4,
  "tenant_override": 6,
  "effective_limit": 6
}
```

| Field | Type | Description |
|---|---|---|
| `default_max_agents` | integer | Platform-wide default (`CLOACINA_DEFAULT_MAX_AGENTS`). |
| `tenant_override` | integer \| null | Per-tenant exception if an admin set one; `null` when none. |
| `effective_limit` | integer | The limit actually enforced: `tenant_override` if set, else `default_max_agents`. |

**Errors** (envelope per [API Error Envelope]({{< ref "api-error-envelope" >}})):

| Status | `code` | Cause |
|---|---|---|
| `401` | (auth) | Missing or invalid API key. |
| `403` | `tenant_access_denied` | Caller's key is not scoped to `tenant_id` (and is not god-mode). |
| `500` | `internal_error` | Failed to read the limit. |

### POST /v1/tenants/{tenant_id}/limits

Set (or replace) a tenant's agent-capacity exception. **Platform-admin only.**

**Request:**

```json
{ "max_agents": 6 }
```

| Field | Type | Required | Description |
|---|---|---|---|
| `max_agents` | integer | yes | The new per-tenant ceiling. Becomes `tenant_override`. |

**Response:** `200 OK` — the resulting `TenantAgentLimitInfo` (same shape as `GET`).

**Errors:**

| Status | `code` | Cause |
|---|---|---|
| `401` | (auth) | Missing or invalid API key. |
| `403` | `admin_required` | Caller is not an `is_admin` (god-mode) key. |
| `500` | `internal_error` | Failed to set the limit. |

### DELETE /v1/tenants/{tenant_id}/limits

Remove a tenant's exception (revert to the platform default).
**Platform-admin only.**

**Response:** `200 OK` — the resulting `TenantAgentLimitInfo`, now with
`tenant_override: null` and `effective_limit` equal to `default_max_agents`.

**Errors:** same as `POST .../limits`.

### GET /v1/tenants/{tenant_id}/fleet

The tenant's fleet-scaling view. Tenant-scoped read.

**Response:** `200 OK`

```json
{
  "tenant_id": "tenant_acme",
  "desired_count": 2,
  "actual_count": 2,
  "effective_limit": 6,
  "default_max_agents": 4
}
```

| Field | Type | Description |
|---|---|---|
| `desired_count` | integer | The tenant's requested agent count (the target the actuator/autoscaler drive toward). |
| `actual_count` | integer | Agents currently registered for the tenant **in this server replica's** roster (a per-replica local view — the in-memory registry is not shared across replicas). |
| `effective_limit` | integer | The hard ceiling provisioning clamps to (override if set, else default). |
| `default_max_agents` | integer | Platform-wide default. |

**Errors:** same shape as `GET .../limits` (`401` / `403 tenant_access_denied` / `500`).

### POST /v1/tenants/{tenant_id}/fleet/provision

Request one more agent. Tenant-admin. Increments `desired_count` by 1 while it is
under the effective limit.

**Response:** `200 OK` — the updated `FleetScaleInfo`.

**Errors:**

| Status | `code` | Cause |
|---|---|---|
| `401` | (auth) | Missing or invalid API key. |
| `403` | `tenant_access_denied` / `insufficient_permissions` | Caller's key isn't scoped to `tenant_id` (cross-tenant), or is scoped to it but lacks the `admin` role. God-mode bypasses both. |
| `409` | `at_capacity` | `desired_count` is already at the effective limit. Raise the limit (admin) or deprovision first. |
| `500` | `internal_error` | Failed to persist the new desired count. |

### POST /v1/tenants/{tenant_id}/fleet/deprovision

Release one agent. Tenant-admin. Decrements `desired_count` by 1 with a floor of
0 (deprovisioning at 0 is a no-op, not an error).

**Response:** `200 OK` — the updated `FleetScaleInfo`.

**Errors:** `401` / `403` (`tenant_access_denied` cross-tenant, or
`insufficient_permissions` for an in-tenant non-admin key) / `500` (same as
`provision`, without the `409`).

> **Auto-provision on tenant create.** `POST /v1/tenants` seeds the new tenant's
> `desired_count` to `min(CLOACINA_INITIAL_AGENTS, CLOACINA_DEFAULT_MAX_AGENTS)`
> (default `min(1, 4) = 1`; `0` disables). It is best-effort — a failure logs a
> warning but the tenant is still created — so a freshly created tenant's first
> `GET .../fleet` typically already shows `desired_count: 1`.

## Execution-Agent Fleet

When the server runs an execution-agent fleet, agents call a dedicated set of
endpoints to register, heartbeat, and return results. These are consumed by the
agents themselves, not by typical API clients. (For the tenant-facing
limit/provision surface that decides how many agents a tenant runs, see
[Tenant agent fleet](#tenant-agent-fleet) above.)

| Method | Path | Purpose |
|--------|------|---------|
| `POST` | `/v1/agent/register` | An agent announces itself and its advertised heartbeat interval. |
| `POST` | `/v1/agent/heartbeat` | Liveness heartbeat; missing heartbeats let the sweeper evict the agent and reassign its in-flight work. |
| `POST` | `/v1/agent/result` | An agent returns the outcome of a dispatched unit of work. |
| `GET`  | `/v1/agent/artifact/{digest}` | An agent fetches a content-addressed package artifact by digest. |
| `GET`  | `/v1/agent/source/{digest}` | An agent fetches a content-addressed **source** bundle by digest (for build-on-agent flows). |
| `GET`  | `/v1/agent/providers/{digest}` | An agent fetches a content-addressed **provider** bundle by digest. |

All of these require an authenticated key (any role); the roster/liveness
handlers scope work to the caller's tenant.

See [Execution-Agent Fleet]({{< ref "/service/explanation/execution-agent-fleet" >}})
for how the fleet coordinates, and [Deploy an Execution-Agent Fleet]({{< ref "/service/how-to/deploy-an-execution-agent-fleet" >}}) to run one.

## Admin & Operations

Operator-facing read endpoints for observing the fleet and build pipeline.
These are distinct from the agent-consumed `/v1/agent/*` endpoints above.

### GET /v1/agents

The execution-agent roster: the agents currently registered with this server
replica, with their advertised capacity and liveness. **Tenant-admin** — the
handler filters the roster to the caller's own tenant; a god-mode (`is_admin`)
key sees every tenant's agents.

**Response:** `200 OK` — the list of registered agents.

**Errors:**

| Status | `code` | Cause |
|---|---|---|
| `403` | `insufficient_permissions` | Caller lacks the `admin` role. |

### GET /v1/compiler/status

Build-pipeline status from the compiler service (queue depth, recent build
outcomes). **Platform-admin only** (god-mode `is_admin` key).

**Response:** `200 OK` — the compiler / build-pipeline status.

**Errors:**

| Status | `code` | Cause |
|---|---|---|
| `403` | `admin_required` | Caller is not an `is_admin` (god-mode) key. |

## WebSocket Endpoints

The API server also exposes WebSocket endpoints for real-time interaction with computation graphs and event delivery:

- **`/v1/ws/accumulator/{name}`** -- push events into a graph accumulator
- **`/v1/ws/reactor/{name}`** -- send commands (force-fire, pause, resume) and query reactor state
- **`/v1/ws/delivery/{recipient}`** -- subscribe to at-least-once outbox deliveries (how execution events reach `cloacinactl execution follow` and SDK subscribers)

WebSocket connections authenticate via a single-use ticket obtained from `POST /v1/auth/ws-ticket`. See the [WebSocket Protocol]({{< ref "websocket-protocol" >}}) reference for connection details and message formats.

## Common Error Format

All error responses use a consistent JSON format:

```json
{"error": "<human-readable message>"}
```

Unmatched routes return:

```
404 Not Found
{"error": "not found"}
```

## Operational Caveats

These are non-obvious failure modes and invariants surfaced from the
implementation. Operators deploying cloacina-server should be aware of
them.

### Authentication cache

- The auth cache is an LRU with **256 entries and a 30-second TTL**.
  Key updates (rare) are not visible until the TTL expires.
- Revoking a key via `DELETE /v1/auth/keys/{id}` clears the **entire**
  cache, not just the revoked key. This makes revocation immediate but
  causes a brief spike in database validation queries as subsequent
  requests rewarm the cache.

### WebSocket tickets

- Tickets issued by `POST /v1/auth/ws-ticket` are **single-use** with
  a 60-second TTL. A client that holds a ticket but disconnects
  without upgrading wastes the ticket; retries require a fresh ticket.
- The `WsTicketStore` is bounded to **1024 unconsumed tickets**. If
  capacity is reached, the ticket nearest to expiry is evicted.
  Rapid `/v1/auth/ws-ticket` calls without consumption can exhaust
  capacity; there is no backpressure signal, just silent eviction.

### Tenant database isolation

The following caveats were **closed by the multi-tenant
abstraction** (T-0579, T-0580, T-0581) — they are described here in
their current resolved state. The earlier "isolation gap" framing has
been removed.

- **Per-tenant runner instances.** Each tenant has its own
  `DefaultRunner` (with its own scheduler loop, executor pool, and
  per-tenant DB pool), cached in `TenantRunnerCache` up to the
  `--tenant-runner-cache-size` cap (default 256).
  Workflow execution lands in the tenant's schema, not in `public`.
- **Per-tenant trigger filtering.** `GET /v1/tenants/{id}/triggers`
  routes through the tenant-scoped `Database` from
  `TenantDatabaseCache`, so the underlying SQL hits
  the tenant's `schedules` table, not a shared global table.
- **Cache eviction on tenant delete**. The
  `DELETE /v1/tenants/{name}` route runs the 4-step teardown
  orchestration (revoke keys → evict runner cache → evict DB cache →
  drop schema); both `TenantRunnerCache` and `TenantDatabaseCache`
  are evicted as part of the teardown. **The "restart the server" workaround
  is no longer required** — subsequent requests to a deleted tenant
  return `404`, not stale-pool errors.
- **Fail-closed `SET search_path`.** Per-tenant connection
  acquisition sets `search_path` strictly to the
  tenant's schema; a failed `SET search_path` is a hard, fail-closed
  error rather than a silent fall-through to `public`. This closes
  the cross-tenant data-leak risk that existed pre-I-0106.

### Request handling

- All routes share a global **100 MB body limit** via
  `DefaultBodyLimit::max(100 * 1024 * 1024)`. Package uploads consume
  this; there's no per-route override.
- **No request timeout** is enforced by the server itself; rely on
  OS / reverse-proxy timeouts. Long-running executions block the
  handler thread; `/ready` may stall if computation graphs are
  wedged.
- **Database admin operations are synchronous.** Creating or deleting
  tenants blocks the request handler. Large schemas or slow databases
  can cause client timeouts; there is no async background-task path
  for provisioning.

### Bootstrap key

- On first startup with no API keys in the database, the server
  generates an admin key (or uses `--bootstrap-key` /
  `CLOACINA_BOOTSTRAP_KEY` if supplied) and writes the plaintext to
  `~/.cloacina/bootstrap-key` with mode `0600`. **The plaintext is
  written exactly once and never logged.** Capture it from the file;
  there is no way to retrieve it later.
- On subsequent startups, the bootstrap path is skipped if any keys
  exist. There is no automatic re-bootstrap.

### Signature verification

- When `--require-signatures` (or `CLOACINA_REQUIRE_SIGNATURES=true`)
  is set, the server verifies package signatures at upload via
  `cloacina::security::verify_package_bytes()`. The verification
  requires a signature row in the `package_signatures` table — the
  server does **not** sign packages, only verifies. Signing is done
  offline (e.g., `cloacinactl pack --sign <key>` once the side-car
  generation is wired up).
- A signature row keyed by the configured `verification_org_id` must
  exist before upload. Missing signature → `403 Forbidden`.

### Metrics endpoint

- `/metrics` is **public** — no authentication is enforced. Operators
  who need to restrict access should reverse-proxy the endpoint and
  enforce auth at that layer.

### Graph supervision

- The graph scheduler restarts crashed accumulator/reactor tasks on
  a **5-second supervision cadence**. If an accumulator crashes
  outside the window, it stays dead until the next check. There is
  no active health check or alerting; the only signal is the
  `paused: true` field returned by `GET /v1/health/graphs/{name}`.

### Unmatched routes

- The fallback handler returns `{"error": "not found"}` as JSON, not
  HTML. Clients that expect HTML error pages need to handle the JSON
  shape.

## See Also

- [CLI Reference]({{< ref "cli" >}}) — `cloacinactl server start` flags and bootstrap-key behavior.
- [DatabaseAdmin API]({{< ref "database-admin" >}}) — tenant provisioning internals.
- [Multi-Tenancy Architecture]({{< ref "/service/explanation/multi-tenancy" >}}) — schema isolation design.
- [WebSocket Protocol]({{< ref "websocket-protocol" >}}) — real-time WebSocket endpoints and message formats.
- [Reconciler Pipeline]({{< ref "/service/explanation/reconciler-pipeline" >}}) — how the server loads and unloads packaged workflows.
