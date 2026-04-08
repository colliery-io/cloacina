# Security Review

## Summary

Cloacina's security posture is mixed: the cryptographic foundations are solid (Ed25519, AES-256-GCM, proper key management), SQL injection prevention for tenant provisioning is well-tested, and the auth middleware is correctly structured. However, there are several critical and major issues: signature verification is optional by default and not enforced at the API upload path, the `create_key` endpoint lacks authorization checks allowing any authenticated user to mint new keys, tenant data isolation at the query layer is absent (the `tenant_id` path parameter is decorative), and the FFI plugin loading boundary represents an inherent arbitrary code execution surface with no sandboxing. The system has no TLS configuration, no rate limiting, no request body size limits on multipart uploads, and no audit logging for authentication events or API key operations.

## Trust Boundary Map

```
                              UNTRUSTED
                                 |
           +-------- HTTP / WebSocket ---------+
           |                                    |
           v                                    v
    +--- Auth Middleware ---+        +--- WS Token Check ---+
    | (Bearer token hash)  |        | (query param or hdr) |
    +----------+-----------+        +-----------+-----------+
               |                                |
               v                                v
    +--- Authorization ---+          +--- Endpoint Registry ---+
    | (tenant, role)      |          | (per-endpoint ACL)      |
    +----------+----------+          +-----------+--------------+
               |                                |
               +----------+---------------------+
                          |
                          v
              +--- Core Engine Layer ---+
              | Runner, Scheduler, DAL  |
              +----------+--------------+
                         |
           +-------------+-------------+
           |                           |
           v                           v
    +--- PostgreSQL ---+      +--- Plugin (FFI) ---+
    | Diesel ORM       |      | dlopen/dlsym       |
    | (parameterized)  |      | (arbitrary code)   |
    +------------------+      +--------------------+
```

Trust boundaries exist at:
1. **Network edge** -- HTTP requests from untrusted clients
2. **Auth middleware** -- Bearer token validation (hash-based)
3. **Authorization layer** -- Tenant scoping and role checks (incomplete)
4. **Database layer** -- Diesel ORM provides parameterized queries
5. **FFI boundary** -- Loading `.so`/`.dylib` plugins executes arbitrary native code

## Threat Model Observations

The system faces several threat categories:

1. **Privilege escalation via API key minting** -- Any authenticated user can create new keys, including admin keys
2. **Cross-tenant data access** -- The tenant_id in API paths is not enforced at the database query layer
3. **Malicious package execution** -- Signature verification is off by default; uploading a package results in arbitrary code execution via dlopen
4. **Credential exposure** -- Bootstrap key written to disk, database URL contains credentials, tenant passwords returned in HTTP responses
5. **Denial of service** -- No rate limiting, no upload size limits at the HTTP layer, no connection limits

## Findings

---

### SEC-01: `create_key` endpoint has no authorization check -- any authenticated user can mint admin keys

**Severity**: Critical
**Location**: `crates/cloacinactl/src/server/keys.rs:50-84`
**Confidence**: High

#### Description

The `POST /auth/keys` handler (`create_key`) checks only that the request passes the auth middleware (i.e., has a valid API key). It does not check whether the caller has admin permissions, or even write permissions. Any authenticated user -- including one with a `read`-only tenant-scoped key -- can create new API keys with any role, including `admin`.

Furthermore, the `role` field defaults to `"admin"` (line 43), so an unauthenticated omission of the `role` field produces an admin key. The caller can also pass `role: "admin"` explicitly.

The `is_admin` flag is always set to `false` in `create_key` (line 59), but `permissions: "admin"` grants `can_admin()` and `can_write()` (lines 254-256 of auth.rs), which is sufficient for all tenant operations.

#### Evidence

```rust
// keys.rs line 50 -- no auth.can_admin() or auth.is_admin check
pub async fn create_key(
    State(state): State<AppState>,
    Json(body): Json<CreateKeyRequest>,  // No Extension(auth) extracted!
) -> impl IntoResponse {
```

Compare with `revoke_key` (line 120) which correctly checks `auth.can_admin()`, and `create_tenant` (line 55 of tenants.rs) which checks `auth.is_admin`.

#### Suggested Resolution

Add `Extension(auth): Extension<AuthenticatedKey>` and check `auth.can_admin()` before creating keys. Restrict non-admin users to creating keys with equal or lesser permissions.

---

### SEC-02: Tenant data isolation is not enforced at the database query layer (IDOR)

**Severity**: Critical
**Location**: `crates/cloacinactl/src/server/executions.rs:99-142`, `crates/cloacinactl/src/server/workflows.rs:111-163`
**Confidence**: High

#### Description

Tenant-scoped API endpoints accept a `tenant_id` path parameter and check `auth.can_access_tenant(&tenant_id)`, but the underlying database queries do not filter by tenant. The `tenant_id` is purely decorative -- the DAL queries operate on the entire database regardless of which tenant was specified in the URL.

For example, `list_executions` calls `dal.pipeline_execution().get_active_executions()` which returns all active executions across all tenants. `list_workflows` calls `registry.list_workflows()` which returns all workflows globally. A tenant-scoped key for tenant A can query `GET /tenants/tenant_a/executions` and see executions belonging to tenant B, C, etc.

The architectural intent is PostgreSQL schema-based isolation (each tenant gets its own schema), but the server code uses a single shared `Database` connection for all requests and never sets `search_path` to the requesting tenant's schema.

#### Evidence

```rust
// executions.rs line 110 -- queries all executions, not filtered by tenant
match dal.pipeline_execution().get_active_executions().await {
    Ok(executions) => {
        let items: Vec<_> = executions.into_iter().map(|e| { ... }).collect();
        // tenant_id only appears in the response JSON, not in the query
        Json(serde_json::json!({ "tenant_id": tenant_id, "executions": items }))
    }
}
```

Similarly in `workflows.rs` line 133: `registry.list_workflows().await` returns all workflows without tenant filtering.

#### Suggested Resolution

Each tenant-scoped request must either: (a) set the PostgreSQL `search_path` to the tenant's schema before executing queries, or (b) filter queries by tenant at the DAL layer. Option (a) aligns with the existing multi-tenant schema architecture.

---

### SEC-03: Package signature verification is off by default and not enforced on API upload

**Severity**: Critical
**Location**: `crates/cloacina/src/security/verification.rs:36-71`, `crates/cloacinactl/src/server/workflows.rs:82`
**Confidence**: High

#### Description

The `SecurityConfig` defaults to `require_signatures: false` (line 43). The `SecurityConfig::development()` constructor also defaults to no signatures. The API workflow upload handler (`upload_workflow`) does not perform any signature verification -- it passes raw bytes directly to `registry.register_workflow_package()`.

Since packages are compiled shared libraries loaded via `dlopen`, uploading a malicious package results in arbitrary code execution on the server with the full privileges of the server process. There is no sandboxing, no capability restriction, and no verification that the uploaded code is from a trusted source.

The reconciler (daemon mode) also loads packages from watched directories with no signature verification by default.

#### Evidence

```rust
// SecurityConfig line 43 -- off by default
pub require_signatures: bool,  // default: false

// workflows.rs line 82 -- no verification at upload time
match registry.register_workflow_package(package_data).await {
```

#### Suggested Resolution

1. In server mode (`cloacinactl serve`), signature verification should be mandatory by default (not opt-in).
2. The upload endpoint should verify package signatures before passing to the registry.
3. Consider adding a separate compilation step so the server compiles trusted source rather than loading pre-compiled binaries.

---

### SEC-04: `list_tenants` endpoint has no authorization check

**Severity**: Major
**Location**: `crates/cloacinactl/src/server/tenants.rs:122-143`
**Confidence**: High

#### Description

The `GET /tenants` handler (`list_tenants`) does not extract `AuthenticatedKey` from the request extensions and performs no authorization check. While it is behind the auth middleware (so authentication is required), any authenticated user -- including a read-only tenant-scoped key -- can enumerate all tenant schemas in the database.

This leaks the tenant directory, which in a multi-tenant system is sensitive organizational information.

#### Evidence

```rust
// tenants.rs line 122 -- no Extension(auth) parameter, no auth check
pub async fn list_tenants(State(state): State<AppState>) -> impl IntoResponse {
    let admin = DatabaseAdmin::new(state.database.clone());
    match admin.list_tenant_schemas().await { ... }
}
```

Compare with `create_tenant` (line 50) and `remove_tenant` (line 93) which both check `auth.is_admin`.

#### Suggested Resolution

Add admin-only authorization: extract `Extension(auth)` and check `auth.is_admin` before returning the tenant list.

---

### SEC-05: WebSocket auth token accepted via query parameter -- exposed in logs and browser history

**Severity**: Major
**Location**: `crates/cloacinactl/src/server/ws.rs:47-64`
**Confidence**: High

#### Description

WebSocket endpoints accept the auth token as a query parameter (`?token=<pak>`). This is documented as a browser compatibility measure (browsers cannot set custom headers on WebSocket upgrade). However, query parameters are:

1. Logged in web server access logs and reverse proxy logs
2. Stored in browser history
3. Visible in the `Referer` header if the page navigates
4. Cached in network infrastructure (proxies, CDN edge logs)

The token is a full API key (`clk_...`) with all the permissions of the authenticated principal.

#### Evidence

```rust
// ws.rs line 47-49
pub struct WsAuthQuery {
    pub token: Option<String>,
}
```

```rust
// ws.rs line 62-63 -- falls back to query param
query.token.clone()
```

#### Suggested Resolution

Consider using a short-lived WebSocket ticket pattern: the client first calls a REST endpoint to exchange their API key for a single-use, time-limited ticket, which is then passed as the query parameter. The ticket should expire within seconds and be single-use.

---

### SEC-06: No TLS configuration -- all traffic including API keys and tenant credentials sent in cleartext

**Severity**: Major
**Location**: `crates/cloacinactl/src/commands/serve.rs:62-63`, `crates/cloacinactl/src/main.rs:62`
**Confidence**: High

#### Description

The server binds to a plain TCP socket with no TLS support. All API traffic -- including `Authorization: Bearer <key>` headers, tenant credentials in `POST /tenants` responses, WebSocket auth tokens, workflow package uploads, and execution data -- is transmitted in cleartext.

The default bind address is `0.0.0.0:8080`, meaning the server listens on all interfaces.

There is no `--tls-cert` / `--tls-key` option, no rustls dependency, and no documentation suggesting a TLS-terminating reverse proxy.

#### Evidence

```rust
// serve.rs line 122 -- plain TCP
let listener = tokio::net::TcpListener::bind(bind).await?;
```

No TLS/SSL/HTTPS/rustls references exist anywhere in the cloacinactl crate.

#### Suggested Resolution

At minimum, document that a TLS-terminating reverse proxy (nginx, Caddy, etc.) is required for production. Better: add native TLS support via `axum-server` with `rustls` or `openssl`, with `--tls-cert` and `--tls-key` CLI options.

---

### SEC-07: No rate limiting on any endpoint -- auth brute-force and DoS possible

**Severity**: Major
**Location**: `crates/cloacinactl/src/commands/serve.rs:169-282`
**Confidence**: High

#### Description

There is no rate limiting on any endpoint. Key attack vectors:

1. **Auth brute-force**: The `/auth/keys`, `/tenants`, and all authenticated endpoints can be hit at unlimited rate. The API key space is large (256 bits), but there are no exponential backoff or lockout mechanisms.
2. **Upload abuse**: `POST /tenants/{id}/workflows` accepts multipart uploads with no body size limit at the HTTP layer. An attacker could upload very large files to exhaust memory or disk.
3. **Execution flooding**: `POST /tenants/{id}/workflows/{name}/execute` can be called repeatedly to flood the task queue.
4. **WebSocket abuse**: No connection limit on WebSocket endpoints; an attacker could open thousands of connections.

The `KeyCache` with 30s TTL means a valid key is checked against the database at most once per 30 seconds, but there is no rate limiting on invalid keys (each miss hits the database).

#### Evidence

No `tower::limit`, `tower_governor`, or any rate-limiting middleware is used. The `build_router` function (line 173) has no `ServiceBuilder` with rate limits.

#### Suggested Resolution

Add `tower::limit::RateLimitLayer` or `tower_governor` to the router. Apply stricter limits to auth endpoints and upload endpoints. Add connection limits for WebSocket endpoints. Consider adding per-IP rate limiting for unauthenticated requests to the health/ready endpoints.

---

### SEC-08: Tenant credentials returned in HTTP response body and logged

**Severity**: Major
**Location**: `crates/cloacinactl/src/server/tenants.rs:66-78`
**Confidence**: High

#### Description

When a tenant is created via `POST /tenants`, the response includes the tenant's database password in the JSON body:

```json
{
    "password": "auto_generated_or_provided_password",
    "connection_string": "postgresql://user:password@localhost:5432/cloacina"
}
```

This password is a database credential that grants direct access to the tenant's schema. Returning it in an HTTP response means it traverses all layers of the network stack (especially problematic without TLS -- see SEC-06), and may be logged by reverse proxies, load balancers, or application logs.

The connection string also embeds the password, doubling the exposure surface.

#### Evidence

```rust
// tenants.rs lines 70-76
Json(serde_json::json!({
    "schema_name": credentials.schema_name,
    "username": credentials.username,
    "password": credentials.password,
    "connection_string": credentials.connection_string,
}))
```

#### Suggested Resolution

Consider a sealed-envelope pattern: return a one-time retrieval token for the credentials, or encrypt them with the caller's public key. At minimum, never embed the password in the connection_string field and use a separate secure channel for credential distribution.

---

### SEC-09: FFI plugin loading executes arbitrary native code with no sandboxing

**Severity**: Major
**Location**: `crates/cloacina/src/registry/loader/package_loader.rs:207-243`
**Confidence**: High

#### Description

When a package (compiled shared library) is loaded, `fidius_host::loader::load_library` calls the OS `dlopen` which executes all `__attribute__((constructor))` functions in the library. This means the loaded code runs with the full privileges of the server process, including:

- Access to the database connection pool
- Access to the filesystem
- Ability to spawn processes
- Access to environment variables (including `DATABASE_URL` with credentials)
- Network access

The security validator (`assess_security`) performs only surface-level heuristic byte-pattern scanning (searching for strings like `/bin/sh`, `system(`, `curl`, etc.), which is trivially bypassable. The suspicious pattern check generates warnings but does not block loading.

The `validate_package_symbols` function (line 376) calls `std::mem::forget(loaded)` on the loaded library, which means the library code has already executed its constructors by the time validation is "complete."

#### Evidence

```rust
// package_loader.rs line 208 -- dlopen executes constructors immediately
let loaded = fidius_host::loader::load_library(library_path).map_err(...)?;
```

```rust
// package_loader.rs line 401 -- library has already executed constructors
std::mem::forget(loaded);
```

#### Suggested Resolution

This is an inherent limitation of the native-plugin architecture. Mitigations include:
1. Making signature verification mandatory (SEC-03) so only trusted code is loaded
2. Running plugin loading in a separate process with reduced privileges (seccomp/pledge/sandbox)
3. Using WASM as the plugin format instead of native code (longer-term)
4. At minimum, validating signatures before calling `load_library`, not after

---

### SEC-10: Revoked key remains valid for up to 30 seconds due to cache TTL

**Severity**: Minor
**Location**: `crates/cloacinactl/src/server/keys.rs:141-143`, `crates/cloacinactl/src/server/auth.rs:57-59`
**Confidence**: High

#### Description

When a key is revoked, the `revoke_key` handler calls `state.key_cache.clear()` to flush the entire cache. This is the correct approach -- however, there is a race condition window: if a request using the revoked key arrives between the database revocation and the cache clear, or if the cache clear on one instance does not propagate to other instances (in a multi-instance deployment), the revoked key continues to work.

More importantly, the cache eviction strategy for a single instance is aggressive (full clear), which means revoking any key invalidates the cache for all keys, causing a thundering herd of database lookups.

For multi-instance deployments, there is no mechanism to propagate key revocation across instances -- each instance has its own in-memory LRU cache.

#### Evidence

```rust
// keys.rs line 143 -- clears ALL cached keys when ANY key is revoked
state.key_cache.clear().await;
```

#### Suggested Resolution

For single-instance: the current approach is acceptable with the 30s TTL. For multi-instance: add a revocation notification mechanism (e.g., PostgreSQL LISTEN/NOTIFY already used by the work distributor). Consider evicting only the specific key hash rather than clearing the entire cache.

---

### SEC-11: Bootstrap key written to disk in plaintext

**Severity**: Minor
**Location**: `crates/cloacinactl/src/commands/serve.rs:400-416`
**Confidence**: High

#### Description

The bootstrap admin key is written to `~/.cloacina/bootstrap-key` as plaintext. The file permissions are correctly set to `0600` on Unix, which is good. However:

1. The file persists indefinitely -- there is no rotation or expiry mechanism
2. The file is not encrypted at rest
3. The plaintext key is never overwritten or zeroed from memory
4. If `~/.cloacina/` is backed up, the key is backed up in cleartext
5. On non-Unix systems (Windows), the `#[cfg(unix)]` guard means no file permissions are set at all

The bootstrap key also has `is_admin: true`, making it the most privileged credential in the system.

#### Evidence

```rust
// serve.rs line 402
std::fs::write(&key_path, &plaintext)?;

// serve.rs lines 406-411 -- Unix only
#[cfg(unix)]
{
    std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))?;
}
```

#### Suggested Resolution

1. Add a warning on startup if the bootstrap-key file is older than N days
2. Log a recommendation to create a new admin key and revoke the bootstrap key after initial setup
3. On Windows, use platform-specific ACLs to restrict file access
4. Consider using the `CLOACINA_BOOTSTRAP_KEY` environment variable as the primary mechanism (it already exists) and make the file write opt-in

---

### SEC-12: API key hash uses SHA-256 without salt -- vulnerable to rainbow table precomputation

**Severity**: Minor
**Location**: `crates/cloacina/src/security/api_keys.rs:40-44`
**Confidence**: Medium

#### Description

API keys are hashed with SHA-256 and stored as lowercase hex. The hash function uses no salt, no pepper, and no key derivation function. While the input space is large (256 bits of randomness), the lack of salting means:

1. If the `api_keys` database table is compromised, an attacker can build a rainbow table to reverse hashes of known key prefixes (`clk_` + base64url)
2. Two API keys that happen to generate the same random bytes (negligible probability with 256-bit keys) would produce the same hash, which is a theoretical uniqueness issue
3. SHA-256 is fast, which favors brute-force attacks (though the 256-bit input space makes this infeasible)

The practical risk is low because the 256-bit input space makes precomputation infeasible. However, using a proper KDF (bcrypt, scrypt, argon2) or at minimum HMAC-SHA256 with a server-side secret would be the standard approach.

#### Evidence

```rust
// api_keys.rs line 40-44
pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

#### Suggested Resolution

Given the 256-bit input space, the practical risk is minimal. If addressing this, consider HMAC-SHA256 with a server-side secret (faster than bcrypt for high-entropy inputs while adding a secret binding).

---

### SEC-13: No request body size limit on multipart upload endpoint

**Severity**: Minor
**Location**: `crates/cloacinactl/src/server/workflows.rs:35-59`
**Confidence**: High

#### Description

The `POST /tenants/{id}/workflows` endpoint accepts multipart file uploads with no configured body size limit. Axum's default multipart limit is generous (several MB), but there is no explicit `DefaultBodyLimit` configured in the router.

The `PackageValidator.max_package_size` limit (100MB) is only checked after the full package data has been read into memory. An attacker could send a request with a body larger than available memory to cause an OOM crash.

#### Evidence

No `DefaultBodyLimit::max()` or `RequestBodyLimit::max()` in the `build_router` function (serve.rs lines 173-282).

The `extract_file_field` function (workflows.rs line 272) reads the entire field into memory with `field.bytes().await`.

#### Suggested Resolution

Add `axum::extract::DefaultBodyLimit::max(100 * 1024 * 1024)` to the router, matching the `PackageValidator` limit. Consider streaming the upload to disk rather than buffering entirely in memory.

---

### SEC-14: No dependency vulnerability auditing in CI pipeline

**Severity**: Minor
**Location**: `.github/workflows/ci.yml`, `Cargo.toml`
**Confidence**: High

#### Description

The CI pipeline does not run `cargo audit` or `cargo deny` to check for known vulnerabilities in dependencies. The project depends on many external crates including security-critical ones (`ed25519-dalek`, `aes-gcm`, `sha2`, `rand`). There is no `deny.toml` configuration and no evidence of any automated dependency vulnerability scanning.

#### Evidence

No files matching `cargo-audit`, `cargo.deny`, or `advisory` found in any `.yml`, `.yaml`, or `.toml` files.

#### Suggested Resolution

Add `cargo audit` to the CI pipeline (can be a non-blocking step initially). Consider adding `cargo deny` for license and vulnerability auditing. Run `cargo audit` in the nightly CI workflow at minimum.

---

### SEC-15: Database URL with credentials exposed via CLI argument and environment variable

**Severity**: Observation
**Location**: `crates/cloacinactl/src/main.rs:65-66`
**Confidence**: High

#### Description

The database URL (which contains the PostgreSQL username and password) is accepted via both a CLI argument (`--database-url`) and an environment variable (`DATABASE_URL`). CLI arguments are visible to all users on the system via `ps` or `/proc/<pid>/cmdline`. Environment variables are more restricted but still visible to the same user.

The `mask_db_url` function (serve.rs line 421) correctly masks the password in log output, which is a good practice.

#### Evidence

```rust
#[arg(long, env = "DATABASE_URL")]
database_url: Option<String>,
```

#### Suggested Resolution

Document that the environment variable approach is preferred over CLI arguments. Consider supporting a credentials file as an alternative source.

---

### SEC-16: Security heuristic scan of packages is trivially bypassable

**Severity**: Observation
**Location**: `crates/cloacina/src/registry/loader/validator/security.rs:62-83`
**Confidence**: High

#### Description

The security assessment scans package binaries for byte patterns like `/bin/sh`, `system(`, `exec`, `curl`, `wget`, and `nc `. These patterns can be trivially avoided by obfuscation (e.g., constructing strings at runtime, using `execvp` instead of `exec`, XOR-encoding strings, etc.). The check produces warnings but does not block loading.

This gives a false sense of security -- the heuristic scan cannot meaningfully detect malicious intent in compiled native code.

#### Evidence

```rust
let suspicious_patterns: [&[u8]; 6] = [b"/bin/sh", b"system(", b"exec", b"curl", b"wget", b"nc "];
```

Note: the string `exec` appears in virtually all legitimate shared libraries (as part of symbol names, ELF section names, etc.), causing frequent false positives.

#### Suggested Resolution

Do not present this as a security mechanism. Either remove it entirely or clearly document that it is a best-effort heuristic providing no security guarantee. The real security mechanism should be mandatory signature verification (SEC-03).

---

### SEC-17: Password escaping for tenant creation is necessary but fragile

**Severity**: Observation
**Location**: `crates/cloacina/src/database/connection/schema_validation.rs:236-238`, `crates/cloacina/src/database/admin.rs:165-168`
**Confidence**: Medium

#### Description

The `CREATE USER ... WITH PASSWORD '...'` SQL statement uses a manually escaped password (single-quote doubling). While this is the correct PostgreSQL escape sequence for string literals, and the schema name and username are validated with strict allowlists, the password escaping approach is fragile:

1. It handles only single-quote injection. Backslash escaping (`standard_conforming_strings = off`) is not considered.
2. The test (line 468 of schema_validation.rs) asserts that `'; DROP TABLE users; --` becomes `''; DROP TABLE users; --`, which demonstrates the escaping works for the literal-termination attack but does not test other escape sequences.
3. PostgreSQL's `CREATE USER` supports parameterized alternatives (`ALTER USER ... PASSWORD ...` with `$1` binding through Diesel), which would eliminate the need for manual escaping entirely.

The practical risk is low because the attacker would need admin API access to call `POST /tenants` (correctly gated behind `auth.is_admin`), and the password value does not allow identifier injection (it is inside a string literal).

#### Evidence

```rust
// admin.rs line 165
let sql = format!("CREATE USER {} WITH PASSWORD '{}'", username, escaped_password_clone);
```

#### Suggested Resolution

Use parameterized queries for the password portion of `CREATE USER` if Diesel supports it. Alternatively, use `ALTER ROLE ... PASSWORD ...` after creating the user, which can be parameterized.

---

## Positive Security Patterns

- **SQL injection prevention**: Schema name and username validation uses strict ASCII-alphanumeric-underscore allowlists with comprehensive test coverage including Unicode rejection and SQL injection attempts
- **Cryptographic choices**: Ed25519 for signing and AES-256-GCM for key encryption are modern, well-regarded algorithms. Key lengths are validated.
- **Audit logging**: The `security::audit` module provides structured SIEM-compatible logging for all key management and verification operations with proper event types and field naming
- **API key design**: The `clk_` prefix enables easy key identification and rotation tracking. 256-bit entropy is excellent. Plaintext is returned only once at creation time.
- **Database URL masking**: The `mask_db_url` function correctly redacts passwords from log output
- **Diesel ORM usage**: All regular data operations use Diesel's parameterized query builder, which prevents SQL injection at the application data layer
- **File permissions**: The bootstrap key file correctly sets `0600` permissions on Unix
- **Key revocation**: Revoking a key immediately clears the entire LRU cache to prevent stale entries
- **WebSocket pre-upgrade auth**: Authentication is checked before the WebSocket upgrade, not after, preventing unauthenticated connection establishment
- **Per-endpoint WebSocket authZ**: Individual accumulator/reactor endpoints have fine-grained ACL checks via the EndpointRegistry
