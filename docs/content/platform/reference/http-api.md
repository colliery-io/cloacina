---
title: "HTTP API Reference"
description: "Complete REST API reference for the Cloacina API server (cloacinactl server start)"
weight: 6
---

# HTTP API Reference

The Cloacina API server (`cloacinactl server start`) exposes a REST
API backed by PostgreSQL or SQLite for managing API keys, tenants,
workflows, executions, triggers, and computation-graph health.

All authenticated routes are mounted under the `/v1/` prefix. The
unauthenticated probes `/health`, `/ready`, and `/metrics` are at the
root. The server was renamed from `cloacinactl serve` in T-0511.

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

Readiness check. Verifies the database connection pool is healthy.

**Response:** `200 OK`

```json
{"status": "ready"}
```

**Response:** `503 Service Unavailable`

```json
{"status": "not ready", "reason": "database unreachable"}
```

### GET /metrics

Prometheus metrics endpoint.

**Response:** `200 OK` with `Content-Type: text/plain; version=0.0.4`

```
# HELP cloacina_up Server is running
# TYPE cloacina_up gauge
cloacina_up 1
```

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
| `role` | string | no | Key role: `"admin"`, `"write"`, or `"read"`. Defaults to `"admin"`. |

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

## Tenant Management

Tenants are isolated PostgreSQL schemas. Each tenant gets its own schema, database user, permissions, and migrations.

> **Note:** The `tenant_id` used in URL paths (e.g., `/v1/tenants/{tenant_id}/workflows`) corresponds to the `schema_name` value used when creating the tenant via `POST /v1/tenants`.

### POST /v1/tenants

Create a new tenant.

**Request:**

```json
{
  "schema_name": "tenant_acme",
  "username": "acme_user",
  "password": ""
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `schema_name` | string | yes | Schema name (alphanumeric + underscore) |
| `username` | string | yes | Database username for this tenant |
| `password` | string | no | Password. Empty string triggers auto-generation (32 chars, ~202 bits entropy). |

**Response:** `201 Created`

```json
{
  "schema_name": "tenant_acme",
  "username": "acme_user"
}
```

> **Security (SEC-08, T-0557 Bug 2 fix):** The tenant password is **never returned** in the response, even when auto-generated. The password is set during provisioning and not surfaced over the API. Operators who need the password must capture it at provisioning time via the database admin tooling, not via this endpoint.

**Errors:**

| Status | Body |
|---|---|
| `400` | `{"error": "<detail>"}` |

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

Remove a tenant. Drops the schema (CASCADE) and the database user.

> **Operational caveat:** The server's `TenantDatabaseCache` does **not** evict its connection pool when a tenant is deleted. Subsequent requests to the deleted tenant will fail with stale-pool errors. Restart `cloacina-server` to reclaim the cache. See [Operational Caveats](#operational-caveats) below.

**Path parameters:**

| Parameter | Type | Description |
|---|---|---|
| `schema_name` | string | The tenant's schema name |

**Response:** `200 OK`

```json
{"status": "removed", "schema_name": "tenant_acme"}
```

**Errors:**

| Status | Body |
|---|---|
| `400` | `{"error": "<detail>"}` |

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
  "version": "1.2.0",
  "description": "Extract, transform, and load data",
  "tasks": ["extract", "transform", "load"],
  "created_at": "2026-04-01T10:00:00+00:00"
}
```

**Errors:**

| Status | Body |
|---|---|
| `404` | `{"error": "workflow 'etl_pipeline' not found"}` |

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

**Errors:**

| Status | Body |
|---|---|
| `400` | `{"error": "<detail>"}` |

### GET /v1/tenants/{tenant_id}/executions

List pipeline executions for a tenant. Returns all recent executions (including running and completed).

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

## Triggers

Read-only listing of cron and trigger schedules.

### GET /v1/tenants/{tenant_id}/triggers

List all schedules (cron and trigger) for a tenant.

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

## Computation Graph Health

Health endpoints for the computation graph system. These endpoints require authentication.

### GET /v1/health/accumulators

List all registered accumulators with their health status.

**Response:** `200 OK`

```json
{
  "accumulators": [
    {
      "name": "market_data",
      "status": "healthy"
    }
  ]
}
```

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

## WebSocket Endpoints

The API server also exposes WebSocket endpoints for real-time interaction with computation graphs:

- **`/v1/ws/accumulator/{name}`** -- push events into a graph accumulator
- **`/v1/ws/reactor/{name}`** -- send commands (force-fire, pause, resume) and query reactor state

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
  causes a one-time validation thunder on subsequent requests as the
  cache rewarms.

### WebSocket tickets

- Tickets issued by `POST /v1/auth/ws-ticket` are **single-use** with
  a 60-second TTL. A client that holds a ticket but disconnects
  without upgrading wastes the ticket; retries require a fresh ticket.
- The `WsTicketStore` is bounded to **1024 unconsumed tickets**. If
  capacity is reached, the ticket nearest to expiry is evicted.
  Rapid `/v1/auth/ws-ticket` calls without consumption can exhaust
  capacity; there is no backpressure signal, just silent eviction.

### Tenant database isolation

- **Workflow execution scheduling is NOT tenant-scoped.** The
  `DefaultRunner` that backs `POST /v1/tenants/{id}/workflows/{name}/execute`
  is a single global instance; executions land in the **runner's
  schema** (typically `public`), not the tenant's schema. In
  multi-tenant deployments this is a known isolation gap. Operators
  who need true per-tenant execution isolation must run a separate
  `cloacina-server` instance per tenant or wait for per-tenant runner
  support to ship.
- The trigger list endpoint `GET /v1/tenants/{id}/triggers` returns
  schedules from the **global** schedule table and filters
  client-side by name. It is not a true per-tenant audit; the same
  schedule will appear regardless of which tenant ID is in the path
  if it matches the filter.
- The `TenantDatabaseCache` lazily creates per-tenant connection
  pools but **never evicts**. Deleting a tenant via
  `DELETE /v1/tenants/{name}` drops the schema but leaves the cached
  pool. Subsequent requests to the deleted tenant fail with stale-
  pool errors. **Restart the server** to reclaim the cache.

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
- [Multi-Tenancy Architecture]({{< ref "/platform/explanation/multi-tenancy" >}}) — schema isolation design.
- [WebSocket Protocol]({{< ref "websocket-protocol" >}}) — real-time WebSocket endpoints and message formats.
- [Reconciler Pipeline]({{< ref "/platform/explanation/reconciler-pipeline" >}}) — how the server loads and unloads packaged workflows.
