---
title: "HTTP API Reference"
description: "Complete REST API reference for the Cloacina API server (cloacinactl serve)"
weight: 6
---

# HTTP API Reference

The Cloacina API server (`cloacinactl serve`) exposes a REST API backed by PostgreSQL for managing API keys, tenants, workflows, executions, and triggers.

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

### POST /auth/keys

Create a new API key. The plaintext key is returned exactly once and cannot be retrieved again.

**Request:**

```json
{"name": "ci-deploy"}
```

**Response:** `201 Created`

```json
{
  "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "name": "ci-deploy",
  "key": "clk_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6",
  "permissions": "admin",
  "created_at": "2026-04-02T14:30:00+00:00"
}
```

**Errors:**

| Status | Body |
|---|---|
| `500` | `{"error": "failed to create API key"}` |

### GET /auth/keys

List all API keys. No hashes or plaintext values are returned.

**Response:** `200 OK`

```json
{
  "keys": [
    {
      "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
      "name": "ci-deploy",
      "permissions": "admin",
      "created_at": "2026-04-02T14:30:00+00:00",
      "revoked": false
    }
  ]
}
```

**Errors:**

| Status | Body |
|---|---|
| `500` | `{"error": "failed to list API keys"}` |

### DELETE /auth/keys/{key_id}

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

## Tenant Management

Tenants are isolated PostgreSQL schemas. Each tenant gets its own schema, database user, permissions, and migrations.

> **Note:** The `tenant_id` used in URL paths (e.g., `/tenants/{tenant_id}/workflows`) corresponds to the `schema_name` value used when creating the tenant via `POST /tenants`.

### POST /tenants

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
  "username": "acme_user",
  "password": "auto_generated_secure_password_32c",
  "connection_string": "postgresql://acme_user:auto_generated_secure_password_32c@localhost/cloacina?options=-c search_path=tenant_acme"
}
```

**Errors:**

| Status | Body |
|---|---|
| `400` | `{"error": "<detail>"}` |

### GET /tenants

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

### DELETE /tenants/{schema_name}

Remove a tenant. Drops the schema (CASCADE) and the database user.

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

### POST /tenants/{tenant_id}/workflows

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

### GET /tenants/{tenant_id}/workflows

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

### GET /tenants/{tenant_id}/workflows/{name}

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

### DELETE /tenants/{tenant_id}/workflows/{name}/{version}

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

### POST /tenants/{tenant_id}/workflows/{name}/execute

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

### GET /tenants/{tenant_id}/executions

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

### GET /tenants/{tenant_id}/executions/{exec_id}

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

### GET /tenants/{tenant_id}/executions/{exec_id}/events

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

### GET /tenants/{tenant_id}/triggers

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

### GET /tenants/{tenant_id}/triggers/{name}

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

## See Also

- [CLI Reference]({{< ref "cli" >}}) -- `cloacinactl serve` flags and bootstrap key behavior
- [DatabaseAdmin API]({{< ref "database-admin" >}}) -- tenant provisioning internals
- [Multi-Tenancy Architecture]({{< ref "/explanation/multi-tenancy" >}}) -- schema isolation design
