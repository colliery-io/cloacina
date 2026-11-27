---
title: "API Error Envelope"
description: "Standardized error response shape, request-ID propagation, full code enumeration by route, and client retry guidance."
sidebar:
  order: 11
---

Every error response from the Cloacina HTTP API (`/v1/*` endpoints) uses the same `ApiError` envelope. This is the canonical place to look up:

- The response body shape.
- The `x-request-id` header that ties responses to server-side log lines.
- Every error `code` string emitted by the routes, the HTTP status it maps to, and the conditions under which it appears.
- Client-side retry guidance.

The implementation lives at `crates/cloacina-server/src/routes/error.rs`.

## Envelope shape

Every error response — regardless of route or status code — has this JSON body:

```json
{
  "error": "human-readable message",
  "code": "machine_readable_code"
}
```

Both fields are always present. `error` is operator-facing prose; `code` is the stable, parseable enum value clients should switch on. The HTTP status code is the canonical signal for category (4xx vs 5xx); `code` discriminates within a category.

## Request correlation: `x-request-id`

Every response — successful and erroring alike — carries an `x-request-id` response header set by the `request_id_middleware` (outermost middleware layer, wraps every endpoint including `/health`, `/ready`, `/metrics`, and the `/v1/*` nest). The same ID appears in the server's structured logs as the `request_id` field on every span emitted while processing that request.

**Client guidance:** capture the `x-request-id` header on every non-2xx response. When filing a support ticket or correlating an outage, that ID is the only identifier that ties a client-observed failure to the server's logs.

The middleware honours an inbound `x-request-id` header if the client supplies one; otherwise it generates a UUID. Clients integrating Cloacina behind their own request-tracing system can therefore propagate a parent trace ID end-to-end.

## Error code catalog

Codes are grouped by the surface that emits them. Within a status-code class the `code` field discriminates the specific failure; clients can switch on `code` for programmatic retry/recovery, or fall back to the HTTP status for generic handling.

### 400 Bad Request

| Code | Routes | When it fires |
|---|---|---|
| `invalid_request` | `POST /v1/tenants/{tenant_id}/workflows`, `GET /v1/tenants/{tenant_id}/executions/{exec_id}`, `GET /v1/tenants/{tenant_id}/executions/{exec_id}/events` | Malformed body, empty multipart payload, or unparsable path parameter (e.g., non-UUID execution ID). |
| `invalid_key_id` | `DELETE /v1/auth/keys/{key_id}` | Path parameter is not a valid UUID. |
| `invalid_pagination` | `GET /v1/tenants/{tenant_id}/executions`, `GET /v1/tenants/{tenant_id}/triggers` | `?limit=` outside `[1, 1000]` or `?offset=` negative. The error message names the violated bound. |
| `tenant_creation_failed` | `POST /v1/tenants` | The name is not a valid tenant name. A name that exists answers `409 tenant_exists`; a database failure answers `500 internal_error`. |
| `tenant_removal_failed` | `DELETE /v1/tenants/{schema_name}` | The name is not a valid tenant name. A failed schema drop answers `500 internal_error`; a retry resumes from the failure point (each step is idempotent). |
| `execution_failed` | `POST /v1/tenants/{tenant_id}/workflows/{name}/execute` | Tenant runner refused the execution (workflow not registered for the tenant, validation error in submitted context, etc.). |
| `upload_failed` | `POST /v1/tenants/{tenant_id}/workflows` | Package registration via the tenant's `WorkflowRegistry` failed (manifest invalid, duplicate version, etc.). |
| `workflow_input_invalid` | `POST /v1/tenants/{tenant_id}/workflows/{name}/execute` | A declared workflow param is missing or its top-level type doesn't match the declared schema; the message names the offending field(s). (CLOACI-T-0757) |
| `reactor_input_invalid` | `POST /v1/health/reactors/{name}/fire` | The injected event doesn't match the reactor's declared input interface. (CLOACI-T-0758) |
| `accumulator_input_invalid` | `POST /v1/health/accumulators/{name}/inject` | The injected event doesn't match the accumulator's declared input interface. (CLOACI-T-0759) |
| `invalid_callback` | `GET /v1/auth/callback` | The OIDC callback is missing `code` or `state`. |
| `invalid_public_key` | `POST /v1/tenants/{tenant_id}/trusted-keys` | The PEM is not an Ed25519 public key, or `public_key` is not 64 hexadecimal characters. |

### 401 Unauthorized

| Code | Routes | When it fires |
|---|---|---|
| `unauthorized` | All authenticated routes | Missing/malformed `Authorization` header, invalid bearer token, revoked API key, or expired WebSocket ticket. Auth-cache failures (DB unavailable) also map here with a more specific message. |

### 403 Forbidden

| Code | Routes | When it fires |
|---|---|---|
| `admin_required` | `POST /v1/tenants`, `DELETE /v1/tenants/{schema_name}`, `POST /v1/tenants/{tenant_id}/keys` (when caller is not the tenant) | Operation requires an `is_admin = true` key. |
| `tenant_access_denied` | All `/v1/tenants/{tenant_id}/*` routes | Authenticated key exists but does not have access to the requested tenant. |
| `insufficient_permissions` | Tenant-scoped write routes (`execute_workflow`, package upload, etc.) | Authenticated key can read the tenant but lacks write/admin role. |
| `identity_not_mapped` | `GET /v1/auth/callback` | A validated OIDC identity matched no rule in the god-owned tenant allowlist (`CLOACINA_OIDC_MAP`). (CLOACI-I-0118) |
| `signature_verification_failed` | `POST /v1/tenants/{tenant_id}/workflows` | Generic verification failure that doesn't match a more specific code below. |
| `untrusted_signer` | `POST /v1/tenants/{tenant_id}/workflows` | The signer's key is not a live trusted key of the tenant, nor of the server-wide org if one is set: unknown, or revoked. |
| `invalid_signature` | `POST /v1/tenants/{tenant_id}/workflows` | Cryptographic signature check failed against the trusted key. |
| `package_tampered` | `POST /v1/tenants/{tenant_id}/workflows` | The uploaded bytes do not match the hash the signature was made for. |
| `signature_not_found` | `POST /v1/tenants/{tenant_id}/workflows` | No signature row found for this package in the trust database. Operator must sign before upload. |
| `signature_verification_error` | `POST /v1/tenants/{tenant_id}/workflows` | Verification path hit an unexpected error (DAL failure, key-lookup error). Treat as transient; retry with backoff. |
| (custom WS reasons) | `GET /v1/ws/accumulator/{name}`, `GET /v1/ws/reactor/{name}` | WebSocket upgrade rejected; the message names the specific reason (`tenant_mismatch`, `not_authorized`, `ticket_expired`, etc., aligned with the `cloacina_ws_auth_failures_total` metric labels). |

### 404 Not Found

| Code | Routes | When it fires |
|---|---|---|
| `key_not_found` | `DELETE /v1/auth/keys/{key_id}` | Key ID is well-formed but no key exists with that ID, or the key was already revoked. |
| `execution_not_found` | `GET /v1/tenants/{tenant_id}/executions/{exec_id}` | Execution row missing from the tenant's schema. |
| `trigger_not_found` | `GET /v1/tenants/{tenant_id}/triggers/{name}` | No schedule row with that name in the tenant's schema. |
| `workflow_not_found` | `GET /v1/tenants/{tenant_id}/workflows/{name}`, `DELETE /v1/tenants/{tenant_id}/workflows/{name}/{version}` | Tenant registry has no entry for `name` (or `(name, version)` for delete, in any build state). |
| `trusted_key_not_found` | `DELETE /v1/tenants/{tenant_id}/trusted-keys/{key_id}` | No trusted key with that id in this tenant. The id of another tenant's key gives the same answer. |

### 409 Conflict

| Code | Routes | When it fires |
|---|---|---|
| `tenant_exists` | `POST /v1/tenants` | A tenant of this name exists. The message names the tenant. |
| `workflow_paused` | `POST /v1/tenants/{tenant_id}/workflows/{name}/execute` | The workflow (or its schedule) is paused; resume it before executing. (CLOACI-T-0749) |
| `package_building` | `DELETE /v1/tenants/{tenant_id}/workflows/{name}/{version}` | The compiler is building the package version. Nothing is removed; delete it when the build ends. |
| `trusted_key_exists` | `POST /v1/tenants/{tenant_id}/trusted-keys` | The tenant already has this key, live or revoked. |

### 500 Internal Server Error

| Code | Routes | When it fires |
|---|---|---|
| `internal_error` | All routes | Catch-all for failures the server cannot turn into a more specific code: tenant database resolve failure, registry I/O error, deserialization panic caught by the framework, etc. The `error` message field holds a fixed text that names the operation, never the text of the underlying error. The server log holds the cause, under the `request_id` that the `x-request-id` response header gives. |

### 501 Not Implemented

| Code | Routes | When it fires |
|---|---|---|
| `oidc_disabled` | `GET /v1/auth/oidc/login`, `GET /v1/auth/callback` | OIDC single sign-on is not configured on this server (no `CLOACINA_OIDC_ISSUER`, or discovery failed at startup). (CLOACI-I-0118) |

## Client retry guidance

| HTTP status | `code` examples | Retry? |
|---|---|---|
| 400 | `invalid_request`, `invalid_pagination`, `tenant_creation_failed` | **No.** Fix the request and retry only after correction. |
| 401 `unauthorized` | (same) | **No.** Rotate or refresh credentials; the request is broken without new auth material. |
| 403 (admin/tenant/permissions) | `admin_required`, `tenant_access_denied`, `insufficient_permissions` | **No.** The key lacks the role; talk to your operator. |
| 403 (signature) | `invalid_signature`, `untrusted_signer`, `package_tampered`, `signature_not_found` | **No.** Fix the signing pipeline; the package will fail identically on retry. |
| 403 `signature_verification_error` | | **Yes, with backoff.** Indicates a transient DAL/key-lookup error. |
| 404 | `*_not_found` | **No.** Verify the resource exists at the path you requested; check tenant scoping. |
| 500 `internal_error` | | **Yes, with exponential backoff** (start 500ms, max 30s, give up after ~5 attempts). Log the `x-request-id` on every retry. |

## WebSocket auth failures

WebSocket upgrades (`/v1/ws/accumulator/{name}`, `/v1/ws/reactor/{name}`) reject with the same `ApiError` envelope **before** the upgrade completes; the response is an ordinary HTTP `401`/`403` with the JSON body above. Post-upgrade frame-level errors are emitted as JSON messages inside the WebSocket stream — see [WebSocket Protocol](../websocket-protocol/) for that shape.

The `reason` field in the message text on `403`s aligns with the labels on `cloacina_ws_auth_failures_total` so operator dashboards and audit logs use the same vocabulary:

- `ticket_expired` — the single-use ticket from `POST /v1/auth/ws-ticket` expired before the upgrade.
- `invalid_signature` — the ticket's signature didn't verify.
- `tenant_mismatch` — the caller's key isn't authorized for the tenant scope embedded in the requested resource.
- `not_authorized` — the caller's key lacks the role required for this WS endpoint.

## See also

- [HTTP API Reference](../http-api/) — endpoint surface; every error response shown there is rendered in this envelope.
- [WebSocket Protocol](../websocket-protocol/) — frame format and post-upgrade error shape.
- [CLI Reference](../cli/) — CLI exit codes derived from these HTTP statuses (1 for user error, 2 for transport, 3 for not-found, 4 for auth, 5 for other server reject).
- [Metrics Catalog](../metrics-catalog/) — `cloacina_api_requests_total{status=…}` for tracking the rate of each HTTP status.
