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

### SSE / live-follow (NOT in v1)

The CLI's `execution events --follow` flag exists for forward
compatibility but is **not implemented in v1**. There is no SSE
endpoint, no WebSocket subscription for execution events, and no
long-poll alternative. Clients that need live event streaming should
poll `GET /v1/tenants/{tenant_id}/executions/{exec_id}/events?since=…`
on an interval until the upstream SSE work lands. (Planned but
unscheduled; tracked outside CLOACI-I-0107.)

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

> **Breaking change (CLOACI-T-0594 / API-01):** the previous body
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

Remove a tenant via the **4-step teardown orchestration** introduced
in CLOACI-T-0581. **Admin-only** (requires an `is_admin` key).

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

List workflow executions for a tenant. Supports filtering and
pagination (CLOACI-T-0594 / API-02; previously these query params
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

## Triggers

Read-only listing of cron and trigger schedules.

### GET /v1/tenants/{tenant_id}/triggers

List all schedules (cron and trigger) for a tenant. Supports
pagination per CLOACI-T-0596 / API-10.

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

The following caveats were **closed by the CLOACI-I-0106 multi-tenant
abstraction** (T-0579, T-0580, T-0581) — they are described here in
their current resolved state. The earlier "isolation gap" framing has
been removed.

- **Per-tenant runner instances.** Each tenant has its own
  `DefaultRunner` (with its own scheduler loop, executor pool, and
  per-tenant DB pool), cached in `TenantRunnerCache` up to the
  `--tenant-runner-cache-size` cap (default 256, CLOACI-T-0580).
  Workflow execution lands in the tenant's schema, not in `public`.
- **Per-tenant trigger filtering.** `GET /v1/tenants/{id}/triggers`
  routes through the tenant-scoped `Database` from
  `TenantDatabaseCache` (CLOACI-T-0579), so the underlying SQL hits
  the tenant's `schedules` table, not a shared global table.
- **Cache eviction on tenant delete** (CLOACI-T-0581). The
  `DELETE /v1/tenants/{name}` route runs the 4-step teardown
  orchestration (revoke keys → evict runner cache → evict DB cache →
  drop schema); both `TenantRunnerCache` and `TenantDatabaseCache`
  are evicted as part of the teardown. **The "restart the server" workaround
  is no longer required** — subsequent requests to a deleted tenant
  return `404`, not stale-pool errors.
- **Fail-closed `SET search_path`.** Per-tenant connection
  acquisition (CLOACI-I-0106) sets `search_path` strictly to the
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
- [Multi-Tenancy Architecture]({{< ref "/platform/explanation/multi-tenancy" >}}) — schema isolation design.
- [WebSocket Protocol]({{< ref "websocket-protocol" >}}) — real-time WebSocket endpoints and message formats.
- [Reconciler Pipeline]({{< ref "/platform/explanation/reconciler-pipeline" >}}) — how the server loads and unloads packaged workflows.
