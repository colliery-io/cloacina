# Security Review

## Summary

Cloacina demonstrates a deliberately security-conscious design across its critical boundaries: SQL injection prevention through strict identifier validation, Ed25519 package signing with AES-256-GCM key encryption at rest, proper API key hashing (SHA-256 of random 256-bit tokens), and schema-based multi-tenant isolation. However, several gaps weaken the overall posture: the server defaults to no TLS and no signature verification, the WebSocket endpoint accepts long-lived API keys in URL query parameters (bypassing the single-use ticket system it implements), and the key cache creates a 30-second window where revoked keys remain valid.

## Trust Boundary Map

```
 [External Clients]  -- HTTP/WS (no TLS by default) -->  [cloacinactl serve]
                                                               |
                                                    API key auth middleware
                                                               |
                                                    +----- [AppState] ------+
                                                    |    (auth, routing)    |
                                                    +-----------+-----------+
                                                                |
                          +------------------+------------------+------------------+
                          |                  |                  |                  |
                   [WorkflowRegistry]  [DefaultRunner]  [ReactiveScheduler]  [DatabaseAdmin]
                          |                  |                  |                  |
                   .cloacina FFI load   Task execution    WS event ingest    Schema DDL
                          |                  |                  |                  |
                          +------------------+------------------+------------------+
                                                    |
                                         [PostgreSQL / SQLite]
                                      (schema-isolated per tenant)
```

**Trust boundaries:**
1. Network edge -> HTTP server (authentication, authorization)
2. API server -> Database (connection string, schema isolation)
3. Package upload -> FFI dynamic loading (native code execution)
4. Client -> WebSocket (token in query param or header)
5. Configuration files / environment variables -> runtime secrets

## Threat Model Observations

Cloacina's primary risk surface stems from two high-value capabilities: executing arbitrary native code via FFI (packaged workflows loaded with `libloading`) and issuing DDL statements against a shared PostgreSQL database (tenant provisioning). An attacker who can upload an unsigned workflow package gains arbitrary code execution in the server process. An attacker who compromises an admin API key can create/destroy tenant schemas and access any tenant's data.

## Findings

## SEC-001: WebSocket endpoints accept long-lived API keys in URL query parameters
**Severity**: Major
**Location**: `crates/cloacinactl/src/server/ws.rs` lines 41-63, `crates/cloacinactl/src/server/keys.rs` lines 239-253
**Confidence**: High

### Description
The system implements a single-use WebSocket ticket mechanism (`WsTicketStore`) and exposes a `POST /auth/ws-ticket` endpoint for exchanging a Bearer token for a short-lived ticket. However, the actual WebSocket handlers (`accumulator_ws`, `reactor_ws`) never use the ticket store. Instead, they accept the raw API key directly via a `token` query parameter:

```
ws://host/v1/ws/accumulator/alpha?token=<long_lived_api_key>
```

The `extract_ws_token` function falls back to `query.token.clone()` which is the full API key, not a ticket. URL query parameters are logged by reverse proxies, browser histories, and network monitoring tools, leaking long-lived credentials. The ticket system exists but is entirely dead code on the consumption side.

### Evidence
- `ws.rs` lines 47-49: `WsAuthQuery` accepts `token: Option<String>`
- `ws.rs` line 84: `validate_token(&state, &token)` validates the raw API key hash, not a ticket
- `auth.rs` lines 297-308: `WsTicketStore::consume()` is never called anywhere in `ws.rs`
- `keys.rs` line 247: `state.ws_tickets.issue(auth)` creates tickets that are never consumed

### Suggested Resolution
Change `extract_ws_token` to call `state.ws_tickets.consume(&token)` instead of `validate_token` when the token comes from a query parameter. Reject raw API keys in query parameters entirely. Only accept tickets in query params and Bearer tokens in headers.

---

## SEC-002: Package signature verification disabled by default in server mode
**Severity**: Major
**Location**: `crates/cloacinactl/src/commands/serve.rs` line 181, `crates/cloacina/src/security/verification.rs` lines 36-44
**Confidence**: High

### Description
`SecurityConfig::default()` sets `require_signatures: false`. The server hardcodes `SecurityConfig::default()` with no configuration path to enable it:

```rust
security_config: SecurityConfig::default(),  // require_signatures = false
```

When signature verification is off, any authenticated user with `write` permission can upload arbitrary native code via the workflow package endpoint. The uploaded `.cloacina` package is then compiled, loaded via `fidius_host::loader::load_library()` (which calls `libloading::Library::new()`), and executed in the server process. This means an attacker with a compromised write-scoped API key gains arbitrary code execution.

Furthermore, the `upload_workflow` handler at `workflows.rs` lines 59-75 only blocks uploads when `require_signatures` is true but does not actually verify signatures -- it just rejects all uploads with a TODO comment:

```rust
// TODO: implement full signature verification at upload time.
// For now, reject all uploads when signatures are required
```

So there is currently no mode where packages are both accepted AND verified.

### Evidence
- `serve.rs` line 181: `security_config: SecurityConfig::default()`
- `verification.rs` line 43: `pub require_signatures: bool` defaults to `false`
- `workflows.rs` lines 63-75: TODO comment, blanket rejection when enabled
- No CLI flag or config file key to set `require_signatures = true` at startup

### Suggested Resolution
1. Add a `--require-signatures` CLI flag and config file option for the server
2. Implement actual signature verification at upload time (the verification pipeline exists; it needs to be wired in)
3. Consider defaulting to `require_signatures: true` for `serve` mode and `false` for `daemon` mode (local-only)

---

## SEC-003: API key cache creates revocation delay window
**Severity**: Major
**Location**: `crates/cloacinactl/src/server/auth.rs` lines 57-117
**Confidence**: High

### Description
The `KeyCache` has a 30-second TTL. When a key is revoked via `DELETE /auth/keys/:key_id`, the handler calls `state.key_cache.clear()` to flush the entire cache. However, this only clears the cache on the server instance that processed the revocation request. In any deployment with multiple server instances behind a load balancer, other instances will continue to accept the revoked key for up to 30 seconds.

More critically, the eviction only happens on the exact server that processes the DELETE. If a client sends requests to a different server instance, the revoked key remains valid in that instance's cache until TTL expiry.

### Evidence
- `auth.rs` line 84: `entry.inserted_at.elapsed() < self.ttl` -- entries valid for 30s
- `keys.rs` line 174: `state.key_cache.clear().await` -- local-only cache clear
- No cross-instance invalidation mechanism

### Suggested Resolution
For single-instance deployments, the current approach is adequate. For multi-instance deployments, consider either:
1. Reducing TTL to 5 seconds (acceptable latency tradeoff)
2. Adding a cache-busting mechanism via database polling (check revocation timestamps)
3. Documenting that multi-instance deployments have a revocation delay equal to the TTL

---

## ~~SEC-004~~: Rate limiting — intentionally removed
**Severity**: N/A (withdrawn)

> **Note**: Rate limiting was evaluated and intentionally removed because it killed normal throughput. The `tower_governor` dependency and `TOO_MANY_REQUESTS` error variant are vestigial code from that experiment. API keys are 256-bit random values making brute-force impractical. This is a conscious design decision, not a gap.

---

## SEC-005: Server runs without TLS by default
**Severity**: Major
**Location**: `crates/cloacinactl/src/commands/serve.rs` line 126, `crates/cloacinactl/src/main.rs` line 62
**Confidence**: High

### Description
The server binds to `0.0.0.0:8080` by default and runs plain HTTP. API keys, WebSocket tokens, and tenant passwords are transmitted in cleartext. The code includes a warning log:

```rust
warn!("Server running without TLS -- use a TLS-terminating reverse proxy (nginx, Caddy, Envoy) in production");
```

While the documentation delegates TLS to a reverse proxy, the default bind address of `0.0.0.0` exposes the server on all interfaces. Combined with WebSocket tokens in query parameters (SEC-001), this creates a credential-sniffing surface.

### Evidence
- `main.rs` line 62: `#[arg(long, default_value = "0.0.0.0:8080")]`
- `serve.rs` line 126: warning about no TLS
- No `--tls-cert` / `--tls-key` CLI options exist
- Docker example at `00-system-overview.md` line 683 shows `0.0.0.0:8080` without TLS

### Suggested Resolution
1. Default bind address should be `127.0.0.1:8080` (localhost-only) to prevent accidental external exposure
2. Add optional native TLS support via `axum-server` with `rustls`
3. Refuse to start on `0.0.0.0` without either TLS configured or an explicit `--allow-plaintext` flag

---

## SEC-006: Tenant data access not enforced at the DAL layer
**Severity**: Major
**Location**: `crates/cloacinactl/src/server/executions.rs` lines 96-134, `crates/cloacinactl/src/server/workflows.rs` lines 112-154
**Confidence**: High

### Description
The tenant access check `auth.can_access_tenant(&tenant_id)` is performed in every handler, but the DAL queries do not use the `tenant_id` as a filter. For example, `list_executions` checks tenant access, then calls `dal.workflow_execution().get_active_executions()` which returns executions from the current database schema -- not filtered by the tenant path parameter.

If the server is connected to the `public` schema (which it is by default), all handlers return global data regardless of which tenant ID is in the URL path. A tenant-scoped key for "tenant_a" calling `/tenants/tenant_a/executions` would see all executions in the public schema, not just tenant_a's.

The schema isolation is intended to be enforced by connecting to a per-tenant schema, but the server creates a single `DefaultRunner` with the admin database connection and shares it across all tenant-scoped routes.

### Evidence
- `executions.rs` line 107: `dal.workflow_execution().get_active_executions().await` -- no tenant filter
- `workflows.rs` line 128: `registry.list_workflows().await` -- no tenant filter
- `serve.rs` line 157: single `DefaultRunner` for all requests
- `auth.rs` line 222: `can_access_tenant` checks permission but does not set schema context

### Suggested Resolution
Either:
1. Create per-tenant `DefaultRunner` instances (or per-request schema-scoped DAL connections) so queries are schema-isolated
2. Add tenant filtering to all DAL queries used in tenant-scoped endpoints
3. Use PostgreSQL `SET search_path` at the connection level before each tenant-scoped request

---

## SEC-007: Unsafe FFI Send/Sync implementations without memory safety audit
**Severity**: Minor
**Location**: `crates/cloacina/src/computation_graph/packaging_bridge.rs` lines 51-54, `crates/cloacina/src/registry/loader/task_registrar/dynamic_task.rs` lines 41-44, `crates/cloacina/src/python/task.rs` lines 129-130
**Confidence**: Medium

### Description
Multiple types implement `unsafe impl Send` and `unsafe impl Sync` with brief comments like "Safety: fidius PluginHandle wraps a libloading::Library which is Send." These unsafe impls vouch for the thread-safety of FFI-loaded native code, which the compiler cannot verify.

If a loaded plugin contains thread-unsafe global state, data races could occur. The `LoadedWorkflowPlugin` and `LoadedGraphPlugin` types wrap `fidius_host::PluginHandle` which itself wraps `libloading::Library`, but the actual plugin code (user-provided) may not be thread-safe.

### Evidence
- `packaging_bridge.rs` lines 51-54: `unsafe impl Send for LoadedGraphPlugin {}` / `unsafe impl Sync`
- `dynamic_task.rs` lines 41-44: same pattern for `LoadedWorkflowPlugin`
- `python/task.rs` lines 129-130: `unsafe impl Send for PythonTaskWrapper {}`
- `python/computation_graph.rs` lines 491-492: `unsafe impl Send for PythonGraphExecutor {}`

### Suggested Resolution
Document the thread-safety requirements for packaged workflows in the packaging guide. Consider adding a runtime guard (e.g., executing each plugin's tasks on a dedicated thread) to contain potential thread-safety issues from user-provided code. At minimum, add comments that explain exactly which invariants must hold for these unsafe impls to be sound.

---

## SEC-008: WsTicketStore does not bound ticket count or expire old tickets
**Severity**: Minor
**Location**: `crates/cloacinactl/src/server/auth.rs` lines 269-308
**Confidence**: High

### Description
The `WsTicketStore` uses an unbounded `HashMap<String, WsTicket>`. Tickets have a 60-second TTL, but expired tickets are only removed when consumed (lazy eviction). An attacker who repeatedly calls `POST /auth/ws-ticket` without consuming the tickets can grow the HashMap indefinitely, causing memory exhaustion.

There is no periodic cleanup, no capacity limit, and no per-key rate limit on ticket issuance.

### Evidence
- `auth.rs` line 271: `tickets: Mutex<HashMap<String, WsTicket>>` -- unbounded
- `auth.rs` lines 286-294: `issue()` inserts without checking capacity
- `auth.rs` lines 299-307: `consume()` only removes the consumed ticket, not other expired ones
- No background cleanup task for expired tickets

### Suggested Resolution
Replace `HashMap` with an `LruCache` (like the `KeyCache`) with a fixed capacity (e.g., 1024 tickets). Add a periodic eviction of expired tickets, or check capacity in `issue()` and evict the oldest expired tickets before inserting.

---

## SEC-009: Bootstrap key written to disk without cleanup mechanism
**Severity**: Minor
**Location**: `crates/cloacinactl/src/commands/serve.rs` lines 498-550
**Confidence**: High

### Description
On first server startup, a bootstrap admin API key is generated and written to `~/.cloacina/bootstrap-key` with mode 0600. The plaintext key persists on disk indefinitely. There is no mechanism to:
1. Rotate the bootstrap key
2. Delete the file after the admin retrieves it
3. Warn if the file still exists after initial setup

If the server host is compromised, the bootstrap-key file provides persistent god-mode access. The key is never logged (good), but it remains on the filesystem.

### Evidence
- `serve.rs` line 532: `let key_path = home.join("bootstrap-key");`
- `serve.rs` line 533: `std::fs::write(&key_path, &plaintext)` -- writes plaintext to file
- `serve.rs` lines 537-542: sets 0600 permissions (Unix only)
- No subsequent cleanup or rotation logic

### Suggested Resolution
1. Print the key to stderr on first startup and do not write it to a file, OR
2. Add a `--rotate-bootstrap-key` command that generates a new key and revokes the old one
3. Log a warning on subsequent startups if the bootstrap-key file still exists
4. Consider a one-time-read mechanism (e.g., delete file after first read)

---

## SEC-010: CLOACINA_VAR_* environment variables may contain secrets without protection
**Severity**: Minor
**Location**: `crates/cloacina/src/var.rs`
**Confidence**: Medium

### Description
The `CLOACINA_VAR_*` variable system is documented as the mechanism for passing external connections and secrets (API keys, database URLs, etc.) to tasks. These values are resolved at runtime from environment variables and can be embedded in package metadata templates using `{{ VAR_NAME }}` syntax.

There is no distinction between sensitive and non-sensitive variables. Variables like `CLOACINA_VAR_API_KEY` are treated identically to `CLOACINA_VAR_MODEL_THRESHOLD`. This means:
1. Debug logging could expose secret values
2. Template resolution results could be logged
3. Error messages from `VarNotFound` include the variable name (revealing which secrets are expected)

### Evidence
- `var.rs` lines 27-29: `CLOACINA_VAR_API_KEY=abc123` shown as example usage
- `var.rs` line 54: `const PREFIX: &str = "CLOACINA_VAR_";` -- flat namespace, no secret marker
- No `CLOACINA_SECRET_*` or `CLOACINA_SENSITIVE_*` convention exists

### Suggested Resolution
Consider adding a `CLOACINA_SECRET_*` prefix for sensitive variables that:
1. Are never included in log output
2. Are masked (like `mask_db_url`) when displayed
3. Are not expanded in template resolution error messages

---

## SEC-011: Metrics endpoint exposed without authentication
**Severity**: Minor
**Location**: `crates/cloacinactl/src/commands/serve.rs` lines 399-402, 452-463
**Confidence**: High

### Description
The `/metrics` endpoint is in the public (unauthenticated) route group alongside `/health` and `/ready`. Prometheus metrics can expose operational details including:
- Pipeline execution counts and durations
- Active pipeline and task counts
- API request patterns (method, path, status)

This information leakage could help an attacker enumerate workflows, identify peak usage times, or discover error patterns.

### Evidence
- `serve.rs` line 402: `.route("/metrics", get(metrics))` in the public Router, not behind auth
- `serve.rs` lines 135-149: metric descriptions include pipeline counts, task counts, API request patterns
- Prometheus handle renders all registered metrics

### Suggested Resolution
Move the `/metrics` endpoint behind authentication, or add a separate `--metrics-bind` flag to expose metrics on a different port/interface (common pattern for internal-only metrics).

---

## SEC-012: Database URL accessible via environment variable and config file
**Severity**: Observation
**Location**: `crates/cloacinactl/src/main.rs` lines 65-67, `crates/cloacinactl/src/commands/config.rs` lines 35, 315-327
**Confidence**: High

### Description
The database URL (which contains credentials) can be provided via `DATABASE_URL` environment variable or `~/.cloacina/config.toml`. The `mask_db_url()` function properly masks credentials in log output. However, the config file has no permission enforcement, and the `cloacinactl config get database_url` command would output the full URL including password to stdout.

### Evidence
- `main.rs` line 66: `#[arg(long, env = "DATABASE_URL")]`
- `serve.rs` line 124: `info!("  Database: {}", mask_db_url(&database_url))` -- properly masked in logs
- `config.rs` line 366: test shows `database_url = "postgres://localhost/test"` in plain config
- No permission check on config file

### Suggested Resolution
Apply `mask_db_url()` to the output of `cloacinactl config get database_url`. Recommend 0600 permissions for the config file in documentation.

---

## SEC-013: Password-only escaping for tenant SQL (no parameterized queries for DDL)
**Severity**: Observation
**Location**: `crates/cloacina/src/database/admin.rs` lines 108-235, `crates/cloacina/src/database/connection/schema_validation.rs`
**Confidence**: Medium

### Description
Tenant provisioning uses `format!()` to construct DDL statements (`CREATE SCHEMA`, `CREATE USER`, `GRANT`, `SET search_path`). Input validation through `validate_schema_name()` and `validate_username()` is thorough (alphanumeric + underscore only, reserved name checks), and password escaping doubles single quotes. This is a defense-in-depth approach that works because PostgreSQL DDL does not support parameterized identifiers.

The validation is sound, but the pattern of string-formatting SQL is inherently fragile. A future developer adding a new DDL operation might forget to call the validation functions.

### Evidence
- `admin.rs` line 152: `format!("CREATE SCHEMA IF NOT EXISTS {}", schema_name)` -- validated
- `admin.rs` line 166: `format!("CREATE USER {} WITH PASSWORD '{}'", username, escaped_password)` -- validated + escaped
- `connection/mod.rs` line 485: `format!("CREATE SCHEMA IF NOT EXISTS {}", schema_name)` -- validated at entry
- `schema_validation.rs`: comprehensive validation with SQL injection tests

### Suggested Resolution
The current approach is acceptable given PostgreSQL's limitation on parameterized DDL. Consider centralizing all DDL generation into a single module with a `#[must_use]` guard on validation, so that it is impossible to reach `format!()` without first passing validation.

---

## SEC-014: Security audit logging is comprehensive (positive)
**Severity**: Observation
**Location**: `crates/cloacina/src/security/audit.rs`
**Confidence**: High

### Description
The audit module provides structured SIEM-compatible logging for all security-sensitive operations: key creation/revocation, trust grants/revocations, package signing, package verification success/failure, and package loads. Events use dot-notation types (`key.signing.created`, `verification.failure`), include relevant context (org_id, fingerprint, package_hash), and are logged at appropriate severity levels (info for success, warn for revocations and failures, error for errors).

### Evidence
- 14 audit functions covering the complete key and package lifecycle
- Structured fields (event_type, org_id, key_fingerprint) for SIEM parsing
- Verification failures logged with failure_reason for forensic analysis

### Suggested Resolution
No change needed. Consider extending audit logging to API key creation/revocation in the HTTP layer (currently only key management operations are audited, not HTTP-level auth events).

---

## SEC-015: Cryptographic implementation uses standard, modern algorithms (positive)
**Severity**: Observation
**Location**: `crates/cloacina/src/crypto/signing.rs`, `crates/cloacina/src/crypto/key_encryption.rs`
**Confidence**: High

### Description
The cryptographic choices are sound:
- **Package signing**: Ed25519 via `ed25519-dalek` (modern, standard)
- **Key encryption at rest**: AES-256-GCM via `aes-gcm` crate (AEAD, standard)
- **API key hashing**: SHA-256 (appropriate for high-entropy inputs)
- **API key generation**: 256-bit random via `rand::thread_rng()` + base64url encoding
- **Nonces**: 96-bit random for AES-GCM (per-operation)
- **Key fingerprints**: SHA-256 of public key bytes

The implementation correctly:
- Validates key/signature lengths before use
- Generates random nonces per encryption operation
- Includes tamper detection tests
- Documents the encrypted format (`nonce || ciphertext || tag`)

### Evidence
- `signing.rs` line 25: `use ed25519_dalek`
- `key_encryption.rs` line 27: `use aes_gcm`
- `api_keys.rs` line 22: `use sha2::Sha256`
- `key_encryption.rs` line 80-81: random nonce generation per operation

### Suggested Resolution
No change needed. The cryptographic implementation follows best practices.
