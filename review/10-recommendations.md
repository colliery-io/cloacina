# Cloacina — Recommended Actions

> Companion to `09-report.md`. Each recommendation addresses one or more findings.
> Read 09-report.md first.

## How to Read

Recommendations are **anchored on root causes and cross-cutting clusters**, not individual findings. Where the cross-cutting agent identified a cluster (e.g., "multi-tenant boundary is incomplete" — twelve findings, four lenses), this document presents one recommendation that addresses the cluster rather than twelve recommendations addressing each symptom. Where a single finding stands alone, it gets its own recommendation.

Prioritization is **severity-weighted with cross-cutting amplification**. The Immediate tier collects every Critical and Critical-adjusted finding; the Short-Term tier collects Major findings whose fix is bounded; Structural is for Major/Minor findings that aggregate into a coherent multi-week initiative; Architectural addresses systemic patterns.

Effort estimates are coarse:
- **Hours**: <1 day, mostly mechanical.
- **Days**: 1-5 working days; bounded scope.
- **Weeks**: 1-4 weeks; an initiative-sized chunk per the team's Metis cadence (one PR per initiative).
- **Months**: multi-initiative; sequential phasing required.

The team ships one PR per Metis initiative (per project memory). Recommendations that would naturally bundle into one initiative are noted as such; recommendations whose effort estimate is "Hours" or "Days" can typically be a standalone task or batched into an existing initiative's cleanup phase.

Some recommendations include **dependencies** — REC-A unblocks REC-B. Treat these as ordering constraints, not prerequisites in a calendar sense.

A small number of recommendations carry an **Investigate before action** caveat, where a benchmark or deployment context is needed before committing to the solution shape.

---

## Immediate Actions (must address before further development)

### REC-01: Wire signature verification end-to-end and make `.cloacina` upload admin-only by default

**Addresses**: SEC-01 (Critical), SEC-04 (Critical-adjusted), COR-15 (Critical-adjusted), SEC-13 (Minor), audit-log-never-called gap from SEC-01
**Effort**: Days
**Type**: Bug fix + Process
**Dependencies**: None (this can ship independently)

**What to do**:
- Add `--verification-org-id <UUID>` CLI flag and `CLOACINA_VERIFICATION_ORG_ID` env var to `cloacina-server`. Refuse to start if `--require-signatures` is set without `--verification-org-id`.
- Move verification into `WorkflowRegistry::register_workflow` (or `RegistryReconciler::load_package`) so every register-package code path enforces it, not just the upload route. The `cloacinactl daemon` path currently never verifies — this closes that hole.
- Wire the existing `audit::log_package_load_success` and `audit::log_package_load_failure` calls into the load path on both success and failure. They are defined in `crates/cloacina/src/security/audit.rs:188,207` and are currently unreferenced from production code.
- Until sandboxing lands (REC-02), restrict the `POST /v1/tenants/.../workflows` route to keys with `is_admin = true`. Document this as the interim posture.
- Add `org_id` to `package_signatures` rows so signature storage is org-scoped (SEC-13). Migration on each backend.
- Un-gate the six signing trust-chain tests under `crates/cloacina/tests/integration/signing/` from `#[ignore = "Requires database connection"]`. Use the `get_all_fixtures()` pattern that other integration tests use so they auto-skip cleanly when no DB is available but run by default when the fixture is up.

**Why it matters**: Default deployment ships an in-process RCE path. The intended defense exists in code but is unactivatable from the documented config surface. The only tests of the defense don't run by default. This is the project's largest single security debt.

**Suggested approach**:
1. Day 1: Add the CLI flag and the audit-log wiring. Both are small-scoped patches that close immediate gaps without touching the wider verifier logic.
2. Day 2-3: Move verification into the lower-level register path; refactor the upload route to call the canonical path.
3. Day 4-5: Un-gate signing tests; add the org_id migration.

Watch for: the daemon path may have callers that rely on the unverified register path for fixtures. Audit those before flipping default-on.

**Acceptance**:
- `cloacina-server --require-signatures` without `--verification-org-id` exits with a clear error.
- Uploading an unsigned package against a server with `require_signatures = true` fails with a 403 and the failure is logged via `audit::log_package_load_failure`.
- All six signing trust-chain tests pass in `angreal test integration` with no `#[ignore]` annotations.
- A new contract test exercises the full sign → upload → verify → load path against a fixture-trusted key.

---

### REC-02: Sandbox the `cloacina-compiler` build path

**Addresses**: SEC-06 (Critical), OPS-07 (Critical-adjusted), OPS-10 (Minor)
**Effort**: Weeks (initial pass) → Months (full sandbox)
**Type**: Architecture + Bug fix
**Dependencies**: None
**Investigate before action**: Sandboxing strategy depends on deployment target. Linux-only solutions (`landlock`, `bwrap`/bubblewrap, `nsjail`) are simpler than container-spawn-per-build; container-spawn is more portable but heavier. A short discovery is needed before committing.

**What to do**:
- Wrap `cargo build` in `tokio::time::timeout` (default 10 minutes, configurable via `--build-timeout-s`). This is the lowest-cost mitigation and closes the SIGTERM-hangs-on-cargo issue (OPS-10).
- Default cargo flags to `["build", "--release", "--lib", "--frozen", "--offline"]`. Pre-vendor required dependencies into a curated `~/.cargo/registry`. Make off-network the explicit choice; document the fixture/vendor procedure.
- Document the deployment posture: compiler service MUST run as an unprivileged UID, MUST be on a host with no outbound network beyond curated cargo paths, MUST NOT have any Cloacina admin credentials beyond its own build-claim DB user. Ship this in `production-deployment.md`.
- Add resource limits via `setrlimit` wrapper (RLIMIT_CPU, RLIMIT_AS, RLIMIT_NOFILE, RLIMIT_NPROC) before exec.
- Long-cycle: integrate `bwrap` (Linux) or per-build container spawn so each build runs in an unprivileged namespace with a tmpfs filesystem.
- Add a security-relevant audit log when builds start/finish, including the Cargo.toml dep-graph hash, so a forensics path exists.

**Why it matters**: Default config runs `cargo build` on attacker-supplied source. `build.rs` has full process privileges, full network access, full filesystem access. It can exfiltrate the compiler service's `DATABASE_URL`, copy `~/.cargo/credentials.toml`, run `curl` against internal services. With that DATABASE_URL it has full read/write access to all tenant data in the cluster. Treat this as build-side RCE.

**Suggested approach**:
1. Week 1: Discovery — pick sandbox primitive (likely bubblewrap on Linux). Write the deployment hardening doc. Land the cargo timeout + `--frozen --offline` defaults.
2. Week 2-3: Resource-limit wrapper + audit log + container-spawn integration.
3. Months: full namespace sandbox per build with tmpfs, network policy, and forensics path.

Watch for: `--frozen --offline` will break legitimate uploads that haven't been pre-vendored. Roll out behind a feature flag; document the vendor procedure first.

**Acceptance**:
- `cloacina-compiler` rejects packages whose Cargo.toml requires fetching new dependencies (with `--frozen --offline` enabled).
- A build process exceeding `--build-timeout-s` is killed; the build row is reset to `pending` by the sweeper.
- `setrlimit` wrapper verifiably limits the cargo subprocess.
- Production deployment docs explicitly call out the threat model and the operator's responsibility for network/UID isolation.

---

### REC-03: Complete the multi-tenant abstraction across runner, reconciler, observability, and lifecycle

**Addresses**: COR-01 (Critical), EVO-04 (Critical-adjusted), SEC-03 (Major), SEC-02 (Major), SEC-05 (Major), SEC-14 (Minor), SEC-17 (Minor), OPS-03 (Major), OPS-12 (Major), OPS-16 (Minor), EVO-15 (Observation), API-04 (Major)
**Effort**: Weeks (one initiative-sized chunk)
**Type**: Architecture
**Dependencies**: None (but unlocks several follow-ups)

**What to do**:
- Resolve the runner-vs-tenant gap. Two paths (per EVO-04's analysis):
  - **(a) Per-tenant runner cache** — extend `TenantDatabaseCache` to a `TenantRunnerCache` keyed by tenant. Each tenant gets a `DefaultRunner` bound to its DB. The registries are inventory-seeded and tenant-agnostic, so they can be shared via Arc.
  - **(b) Database-override on execute** — extend `WorkflowExecutor::execute_async` to accept a `Database` override so a single runner serves all tenants. Lighter but creates more multi-tenant gaps later.
  - Recommend (a) — cleaner long-term, modeled on the existing `TenantDatabaseCache` pattern.
- Fix `triggers.rs::list_triggers` and `get_trigger` to use `state.tenant_databases.resolve(...)` instead of `state.database` (SEC-02). This is a 5-line change once the resolver path is the canonical pattern.
- Filter `/v1/health/accumulators`, `/v1/health/graphs`, `/v1/health/graphs/{name}` by the caller's authorized tenant set (SEC-05). Use the existing `AccumulatorAuthPolicy.allowed_tenants` for the filter; add a `/v1/tenants/{tenant_id}/health/...` route family for explicit tenant-scoped access.
- Make `COR-01` fail-closed: propagate the `SET search_path` error from `get_connection_with_schema` as a connection-pool error so the caller gets a proper failure rather than a silently misrouted query. Add a defense-in-depth guard that asserts `current_schemas()` matches the expected tenant on first query.
- Make `remove_tenant` orchestrate the full teardown: revoke API keys with `tenant_id = schema_name`; stop reactors and accumulators registered for the tenant via `graph_scheduler.unload_for_tenant(tenant_id)`; cancel running workflow executions; evict the tenant's `Database` from `TenantDatabaseCache`; then run `admin.remove_tenant(...)` (SEC-14).
- Bound `TenantDatabaseCache` (LRU, 256 entries) and evict on `remove_tenant` (SEC-17). Surface a metric `cloacina_tenant_cache_size`.
- After auth succeeds, record `tenant_id`, `key_id`, and `role` onto the current request span (`tracing::Span::current().record(...)`). Declare these fields on the span at creation as `tracing::field::Empty` so `record` works (OPS-03, OPS-12).
- Extract `traceparent` from the request via `opentelemetry-http`'s extractor at the outer middleware so server spans become children of upstream cloacinactl traces (OPS-03).
- Add `request_id`, `runner_id`, `tenant_id` columns to `execution_events` (OPS-16). Backfill from the request span at workflow-execution creation; set `runner_id` from the executor's `instance_id` at task-state-transition write time. Migration on each backend.
- Document the auth role matrix (API-04 covered separately under REC-09) — cross-references this initiative.

**Why it matters**: The system pitches multi-tenancy in three places (system overview §6, the README, the `--tenant` CLI flag). The implementation has tenant-scoped storage and admin-schema-bound execution. Workflows execute in the wrong schema, triggers and graph health leak across tenants, tenant deletion is incomplete. Twelve findings across four lenses converge here. This is the project's largest single architectural debt.

**Suggested approach**: Plan as one Metis initiative with phases:
1. **Discovery / Design**: which runner-cache strategy; what the tenant lifecycle teardown order is; what migration is needed on `execution_events`.
2. **Phase 1**: Span enrichment (OPS-03, OPS-12). 30-line patch; ships standalone value.
3. **Phase 2**: Triggers + graph health route fixes (SEC-02, SEC-05). Bounded.
4. **Phase 3**: Per-tenant runner cache; new constructor and cache eviction.
5. **Phase 4**: Tenant deletion teardown + cache eviction (SEC-14, SEC-17).
6. **Phase 5**: COR-01 fail-closed and search_path defense-in-depth.
7. **Phase 6**: `execution_events` correlation columns + migration (OPS-16).

Watch for: the per-tenant runner cache will multiply memory cost per tenant. Bound it via LRU; document the operational sizing. Sharing inventory-seeded registries across cache entries via Arc is essential; a naive copy will explode memory.

**Acceptance**:
- `POST /v1/tenants/foo/workflows/bar/execute` writes execution rows into the `foo` schema, not the admin schema.
- A regression test calls `list_triggers` for tenant A and asserts that schedules created in tenant B's schema do not appear.
- `GET /v1/health/graphs` returns only graphs accessible to the caller's tenant; admin keys see all.
- Deleting a tenant via `DELETE /v1/tenants/foo` revokes all `tenant_id = "foo"` API keys, stops the tenant's reactors, cancels running executions, evicts the cached `Database`, then drops the schema.
- Request spans carry `tenant_id`, `key_id`, `role` fields that propagate to JSON logs.
- `execution_events` rows carry `request_id`, `runner_id`, `tenant_id`.

---

### REC-04: Audit the CLI/server contract end-to-end and add integration tests for every CLI verb

**Addresses**: API-01 (Critical), API-02 (Critical), API-03 (Critical-adjusted), API-05 (Major), API-06 (Major), API-08 (Major), API-10 (Major), API-17 (Minor)
**Effort**: Weeks
**Type**: Bug fix + New tests
**Dependencies**: None

**What to do**:
- Fix `cloacinactl tenant create` (API-01). Pick the resolution: change CLI to send `{schema_name, username, password}`, or rename server's request struct to `{name, description, password?}`. Recommend matching the CLI's user-friendly shape — operators expect "name" and "description". Add `--password` flag accepting `Option<String>` with auto-generation when absent.
- Fix `cloacinactl execution list` filters (API-02). Add `Query<ListExecutionsQuery>` extractor with `workflow_name: Option<String>`, `status: Option<String>`, `limit: u32 = 50`, `offset: u32 = 0`. Switch DAL call to `list_recent` plus a status filter. Either rename the route to `/v1/tenants/{t}/executions/active` if "list active" is the intended semantics, or actually serve all executions with the filters.
- Fix `cloacinactl tenant/trigger/execution list` renderers (API-03). Each broken caller does `let items = body.get("<key>").cloned().unwrap_or(body); render::list(&items, output)`. Make `render::list` accept either an array or an object with a single array-valued field, and warn (or error) if neither matches.
- Decide the fate of `cloacinactl package pack --sign <key>` (API-05). Either implement it (the existing `cloacina::security::package_signer` infrastructure can produce a `<archive>.sig` sidecar; wire `upload`/`publish` to push it to `package_signatures`), or change the silent `eprintln!` to a hard `Err(CliError::UserError("--sign is not yet implemented"))`. Don't accept a flag that doesn't do anything.
- Unify the REST error envelope (API-06). Make `ApiError::into_response` the only way the server emits errors. Replace raw `Json` returns in `auth.rs:158, 165` and `health_graphs.rs:113` with `ApiError::unauthorized(...)` / `ApiError::not_found(...)`. Update CLI's `extract_message` to read `error` as a string (the canonical format) and treat `code` as a hint surfaced separately on `ServerReject` errors.
- Fix the prefix invariant (API-08). Move `/v1/health/*` and `/v1/ws/*` routes into `auth_routes` (or a sibling `Router` that's `.nest("/v1", ...)`'d the same way). The routes lose the literal `/v1/` prefix in their `route(...)` paths but gain consistency. WS routes need their auth handling preserved (auth-in-handler, not auth-in-middleware).
- Plumb pagination on `list_triggers` and `get_trigger` (API-10). Accept `Query<{limit: Option<u32>, offset: Option<u32>}>` (default 50/0, max 1000); include `next_offset` or `total` in the response.
- Resolve `--follow` (API-17). Either implement SSE streaming from `execution_events` (the table exists; Postgres `LISTEN/NOTIFY` is a small lift), or hide the flag with `#[arg(hide = true)]` until implemented.
- **Add an integration test for every CLI verb** that exercises the CLI binary, hits a real (test-fixture-backed) server, and asserts both that the command succeeds and that the rendered output contains expected items. The CLI/server seam is currently untested; this is the load-bearing investment that prevents regression.

**Why it matters**: The CLI is the documented operator surface. Today, `cloacinactl tenant create my_tenant` fails with 400; `cloacinactl execution list --status Failed` returns Pending/Running rows; `cloacinactl tenant list` outputs empty for tenants that exist. New CLI features ship without anyone catching the contract drift. This is a release-blocker for the CLI as a published interface.

**Suggested approach**: Plan as one Metis initiative.
1. Days 1-3: Fix the four broken commands (API-01, API-02, API-03, API-05). Each is a bounded patch; can be parallelized.
2. Days 4-5: Unify the error envelope; reroute graph health routes into the `/v1/` nest.
3. Days 6-10: Build the CLI integration test harness — likely a fixture that spawns a server bound to a temp Postgres, runs CLI commands as subprocesses, asserts outputs. Cover every verb of every noun.
4. Day 10+: pagination plumb-through; `--follow` decision.

Watch for: the integration test fixture is the larger investment. Plan for it to be reusable beyond this initiative — future API additions should automatically get a CLI integration test.

**Acceptance**:
- All four broken CLI commands work end-to-end against a real server.
- `cloacinactl execution list --status Failed --limit 10` returns up to 10 Failed executions.
- `cloacinactl tenant list` shows tenants in the configured output format.
- `--sign` either signs or errors hard.
- Every CLI verb in `cloacinactl tree` has at least one integration test that exercises the command end-to-end.

---

### REC-05: Document the auth model in one place and tighten the implementation gaps

**Addresses**: API-04 (Major), SEC-09 (Minor), SEC-10 (Minor), COR-12 (Minor), SEC-08 (Minor), OPS-20 (Minor)
**Effort**: Days
**Type**: Docs + Bug fix
**Dependencies**: None (but composes well with REC-03)

**What to do**:
- Write a one-page auth model doc in `docs/content/platform/auth-model.md`: define `is_admin` as "cross-tenant override only granted at server bootstrap" and `permissions` as "tenant-scoped role". List which routes require which. Include the four-cell matrix (`is_admin × permissions`).
- Replace open-coded `auth.is_admin` checks in `tenants.rs:56, 97, 123` and `keys.rs:202` with a `require_god_mode()` helper that returns the same `admin_required_response` everywhere. Makes the distinction grep-able.
- Add unit tests for the auth matrix: 4 roles × 3 routes = a small grid of "this combination should/shouldn't pass".
- Tighten migration `019` (SEC-09): change to `UPDATE api_keys SET is_admin = TRUE WHERE name = 'bootstrap-admin' AND tenant_id IS NULL`. Add a follow-up migration that warns about pre-existing `name = 'bootstrap-admin'` rows that don't satisfy the constraint.
- Add anti-lockout guards to `revoke_key` (SEC-10): reject self-revocation; refuse to revoke the last unrevoked god-mode admin key; document recovery via direct DB access in case all god-mode keys are revoked.
- Switch `KeyCache::clear()` calls in `keys.rs:181` to the dead-code `evict(&hash)` for single-server deployments (COR-12, SEC-08). Drop the `#[allow(dead_code)]` annotation.
- For multi-server deployments: implement Postgres `LISTEN/NOTIFY` on a `key_revoked` channel where every server subscribes and evicts on receive (SEC-08). Document the multi-server limitation in the runbook.
- Loud `warn!` line at server bootstrap-key write: "Bootstrap key written to {}. Rotate after first admin login. Delete the file with `cloacinactl key delete-bootstrap` once you have a permanent admin key." (OPS-20)
- Add `cloacinactl key delete-bootstrap` verb that revokes the bootstrap key in DB + deletes the file.
- Document rotation in `production-deployment.md`.

**Why it matters**: The cryptographic primitives are excellent (Ed25519, AES-256-GCM, schema validation, mode-0600 bootstrap key). The compositional doc is missing — eight findings across three lenses are downstream of one missing one-page doc plus six 30-line patches.

**Suggested approach**:
1. Day 1: Write the auth model doc. Run it past one operator-equivalent reviewer.
2. Days 2-3: Implement the helper consolidation, the migration tightening, the revoke guards, the cache eviction.
3. Days 4-5: Multi-server `LISTEN/NOTIFY` for cache invalidation; bootstrap rotation runbook.

**Acceptance**:
- A new operator can read `auth-model.md` and answer "what does `--role admin` do" without reading source.
- Every check that previously open-coded `auth.is_admin` now goes through `require_god_mode()`.
- `revoke_key` rejects self-revocation and last-admin revocation.
- Migration `019` filter includes `tenant_id IS NULL`.
- `cloacinactl key revoke` evicts only that key from cache (single-server), or notifies all servers via LISTEN/NOTIFY (multi-server).

---

### REC-06: Fix the `cloacina_active_tasks` gauge leak and extend the SQL-derived gauge pattern

**Addresses**: OPS-01 (Major), OPS-15 (Minor)
**Effort**: Hours
**Type**: Bug fix
**Dependencies**: None

**What to do**:
- Drop the naked `metrics::gauge!("cloacina_active_tasks").increment(1.0)` / `.decrement(1.0)` calls at `crates/cloacina/src/executor/thread_task_executor.rs:840, 904`.
- In the scheduler tick (next to the existing `cloacina_active_workflows` re-seed at `scheduler_loop.rs:166-168`), add `metrics::gauge!("cloacina_active_tasks").set(running_count as f64)` — count from `task_executions WHERE status = 'Running'` per tick. The `update_workflow_task_readiness` path already scans pending/running tasks; the count is in hand.
- Add `cloacina_*_persist_failures_total{kind}` counters in CG paths where `let _ = persist_*` swallows errors (OPS-15). Specifically:
  - `cloacina_reactor_persist_failures_total{reactor, kind=cache|dirty|seq_queue|checkpoint}`
  - `cloacina_accumulator_persist_failures_total{accumulator, kind=boundary|buffer|health}`
- Add a watchdog: if a reactor logs 5 consecutive persist failures, downgrade `ReactorHealth::Live` → `ReactorHealth::Degraded` and surface via `/v1/health/graphs/{name}`.

**Why it matters**: This is exactly the gauge-leak antipattern T-0534 fixed for `cloacina_active_workflows`. Any panic between lines 840 and 904 of `thread_task_executor.rs` leaks the count permanently. The COR-04 cdylib panic vector is one reachable path. Production deployments observing `cloacina_active_tasks` may see slow drift upward over time.

**Suggested approach**: One short-cycle task; can ship as part of a metric-improvements initiative or standalone.

**Acceptance**:
- After a synthetic panic in `thread_task_executor::execute`, the next scheduler tick re-seeds `cloacina_active_tasks` to the actual SQL-derived value.
- A reactor whose persist fails 5 times in a row reports `ReactorHealth::Degraded` via the health endpoint.

---

### REC-07: Replace `release_runner_claim` ownership-blind release with claim-guarded release

**Addresses**: COR-02 (Major)
**Effort**: Hours
**Type**: Bug fix
**Dependencies**: None

**What to do**:
- Add an optional `runner_id: Option<UniversalUuid>` parameter to `release_runner_claim`, mirror the `mark_completed` shape, and require the executor to pass `self.instance_id`.
- Return a bool indicating whether the release applied so callers can detect & log "claim was stolen mid-execution" cleanly.
- Update the executor's call site at `thread_task_executor.rs:986-998` to pass the runner_id.

**Why it matters**: A runner whose heartbeat stalled and recovered after the sweeper re-dispatched the task can release the new owner's claim, leaving the new owner running with no claim guard. Mitigated by `mark_completed`/`mark_failed` already being claim-guarded, but the bare release is a footgun and is the kind of bug that surfaces only under load.

**Acceptance**:
- A test simulates two runners A and B claiming the same task in sequence (A is sweeper-released, B claims), and asserts that A's late `release_runner_claim` does NOT clear B's claim.

---

### REC-08: Remove dead `generate_packaged_registration` path and update the `package!()` coexistence warning

**Addresses**: LEG-06 (Minor-adjusted), LEG-20 (Observation), EVO-10 (Major — same dead-path family; the unified shell macro and per-macro `_ffi` emission can coexist; deleting the dead path closes EVO-10's unsoundness window)
**Effort**: Hours
**Type**: Refactor
**Dependencies**: None

**What to do**:
- Delete `generate_packaged_registration` and its callers (lines 277, 700-829 of `crates/cloacina-macros/src/workflow_attr.rs`).
- Delete the `let _ = packaged_registration;` line and the migration tracking comment at lines 289-293.
- Update the `cloacina::package!()` macro doc at `crates/cloacina-workflow-plugin/src/lib.rs:103-108`: replace the unconditional coexistence warning with a one-line note that `cloacina::package!()` is the sole producer of a fidius plugin and should appear exactly once at the crate root.

**Why it matters**: 130 lines of token construction emitted per `#[workflow]` invocation, immediately discarded. The coexistence warning currently warns about a problem that no longer exists.

**Acceptance**: `workflow_attr.rs` no longer references `generate_packaged_registration`. `package!()` doc accurately describes T-C state.

---

## Short-Term Actions (next development cycle)

### REC-09: Make connection pool sizes configurable; document the trade-offs

**Addresses**: PERF-01 (Major), PERF-02 (Major), OPS-09 (Major), PERF-17 (Minor)
**Effort**: Days
**Type**: Bug fix + Configuration
**Dependencies**: None
**Investigate before action**: Right pool sizes depend on workload; ship configurable defaults with explicit warnings rather than guessing.

**What to do**:
- Plumb `tenant_pool_size: usize` through `cloacina-server` CLI args → `AppState` → `TenantDatabaseCache::resolve`. Default 8.
- For SQLite: stop overriding `db_pool_size` to 1 silently. If the caller explicitly opts in to pool > 1, keep it (WAL mode supports concurrent readers); if not, default to 4 readers + serialize writes (deadpool already does this). Document the behavior in the rustdoc.
- Move SQLite PRAGMA setup (`journal_mode=WAL`, `busy_timeout=30000`) into a `deadpool_diesel::Manager::create` post-create hook (PERF-17). Run pragmas once per connection lifetime, not per checkout. Each DAL call currently pays for two extra round-trips on SQLite; this is pure overhead.
- Refuse to silently swap caller intent: if `db_pool_size` is set > 1 on SQLite without `--enable-sqlite-readers`, log a `warn!` explaining the override (ideally remove the override entirely).

**Why it matters**: A single tenant under load saturates its 2-connection pool; queue grows; operators see 504s and queue growth and find no config knob that works because the SQLite path silently overrides. Operators cannot tune their way out.

**Suggested approach**:
1. Day 1: Plumb the CLI flag; document the per-tenant pool sizing trade-offs in the runbook.
2. Day 2: PRAGMA-on-create hook; stop running pragmas per checkout.
3. Day 3: SQLite pool size default of 4 readers + tests that confirm WAL mode allows concurrent readers.

**Acceptance**:
- `cloacina-server --tenant-pool-size 16` actually uses 16 connections per tenant.
- SQLite DAL calls no longer issue PRAGMA queries on every checkout (verified by query log).
- Setting `db_pool_size = 32` on SQLite without an opt-in warns clearly.

---

### REC-10: Plumb missing operator verbs (`cancel`, `pause`, `resume`, `rebuild`, `drain`)

**Addresses**: OPS-02 (Major)
**Effort**: Days
**Type**: New code
**Dependencies**: REC-04 (the integration-test harness from REC-04 is what catches the contract; REC-10 should add to it)

**What to do**:
- Add `cloacinactl execution cancel <id>` → `POST /v1/tenants/{t}/executions/{id}/cancel` → `WorkflowExecutor::cancel_execution`. The engine method exists at `crates/cloacina/src/runner/default_runner/workflow_executor_impl.rs:231`.
- Add `cloacinactl package rebuild <id>` → `POST /v1/tenants/{t}/packages/{id}/rebuild` (or DB direct: `UPDATE workflow_packages SET build_status='pending' WHERE id=$1`), gated by tenant + admin permission.
- Add `cloacinactl tenant drain <name>` to mark all running workflows for graceful cancel before delete.
- Add `cloacinactl workflow pause <name>` / `resume <name>` — engine has paused-state support per migration `007_add_pause_support`; just needs CLI + REST plumbing.
- Each CLI verb needs an integration test (REC-04 harness).

**Why it matters**: Operators currently cancel a runaway run, drain a tenant before deletion, or rebuild a failed package by writing SQL by hand. The engine has the primitives; the CLI and HTTP routes don't. These are exactly the verbs an operator wants on incident day.

**Suggested approach**: Each verb is 1-2 days. Can be parallelized across multiple PRs if the team prefers.

**Acceptance**:
- `cloacinactl execution cancel <id>` cancels a running execution; the next status reports `Cancelled`.
- `cloacinactl package rebuild <id>` resets `build_status` to `pending`; the compiler picks it up.
- `cloacinactl tenant drain foo` cancels all running workflows for tenant foo; subsequent `tenant remove foo` succeeds cleanly.

---

### REC-11: Install Prometheus recorders in `cloacina-compiler` and `cloacinactl daemon`

**Addresses**: OPS-04 (Major), OPS-05 (Major), OPS-06 (Minor)
**Effort**: Days
**Type**: New code + Configuration
**Dependencies**: REC-06 (extending the SQL-derived gauge pattern)

**What to do**:
- Add a `/metrics` route to the compiler's HTTP server alongside `/v1/status`, using the same `metrics_exporter_prometheus::PrometheusBuilder` pattern as `cloacina-server`. Emit:
  - `cloacina_compiler_builds_total{status=success|failed}` counter
  - `cloacina_compiler_build_duration_seconds` histogram
  - `cloacina_compiler_queue_depth{state=pending|building}` gauge (re-seeded SQL-derived per loop tick)
  - `cloacina_compiler_sweep_resets_total` counter
  - `cloacina_compiler_heartbeat_failures_total` counter
- Install a Prometheus recorder in `daemon::run` symmetric to the server. Bind a small HTTP endpoint (e.g., `--metrics-bind 127.0.0.1:9091`) for `/metrics` separate from the Unix socket health.
- Add `cloacina-compiler` to `angreal test:metrics-format` so promtool validates compiler exposition output too.
- Add log retention via `tracing_appender::rolling::Builder::max_log_files(N)` (added in 0.2.3+) — wire to a `--log-retention-days` CLI flag on all three deployables. Default 14 days. Closes OPS-06.

**Why it matters**: The engine emits 8 metrics with bounded label vocabulary, validated by `promtool` in CI. None of that reaches the build worker or the daemon, both of which run the same engine code. Self-hosted operators monitoring a SQLite-backed daemon, or a build worker pool, must DB-query for everything.

**Suggested approach**:
1. Day 1: Compiler `/metrics` + counters/histograms.
2. Day 2: Daemon Prometheus recorder + `--metrics-bind` flag.
3. Day 3: Add to `angreal test:metrics-format`; ship retention.

**Acceptance**:
- `curl http://compiler:9000/metrics` returns Prometheus-formatted output validated by `promtool`.
- `curl http://daemon:9091/metrics` returns the same engine-emitted metrics.
- Old log files are pruned per the configured retention.

---

### REC-12: Atomic snapshot+clear pair for reactor `Latest` strategy

**Addresses**: COR-03 (Major), API-15 (Minor)
**Effort**: Days
**Type**: Bug fix
**Dependencies**: None
**Investigate before action**: REC-13 is the larger restructuring; REC-12 is a narrower defect fix within the existing structure. Confirm REC-13's restructure won't subsume this.

**What to do**:
- Take the dirty lock and capture-then-clear flags as a single atomic operation paired with the snapshot in `crates/cloacina/src/computation_graph/reactor.rs:591-593`:
  ```rust
  let (snapshot, _cleared) = {
      let mut dirty = dirty_exec.write().await;
      let cache = cache_exec.read().await;
      let snap = cache.snapshot();
      let cleared = std::mem::replace(&mut *dirty, DirtyFlags::with_sources(&expected));
      (snap, cleared)
  };
  ```
- Add a regression test that interleaves boundaries during graph execution to exercise the dirty-flag clear/cache-update race for `WhenAll`.
- Document the WS protocol for `ReactorCommand` and `ReactorResponse` (API-15) — add `#[serde(tag = "type", rename_all = "snake_case")]` (or document the existing externally-tagged shape) and ship a `ws_protocol.md` doc with example exchanges. Change `cache: HashMap<String, Vec<u8>>` to base64 string values via `serde_bytes` for wire readability.

**Why it matters**: A boundary arriving between the cache snapshot and the dirty-flag clear has its dirty bit cleared and its cache update lost from the snapshot; next signal evaluation sees `any_set()=false`. For `WhenAll`, every source has to re-dirty before the next fire, and one source whose update arrived during the window is permanently masked.

**Acceptance**:
- A test that interleaves `WhenAll` boundaries during graph execution asserts every boundary triggers a re-evaluation.
- `ws_protocol.md` documents the wire format with example exchanges.

---

### REC-13: Restructure reactor / CG hot path locks; switch persistence to bincode

**Addresses**: PERF-05 (Major), PERF-06 (Major), PERF-07 (Major), PERF-15 (Minor), PERF-16 (Minor)
**Effort**: Weeks
**Type**: Refactor
**Dependencies**: REC-12 (fold REC-12's atomic snapshot/clear into this restructure)
**Investigate before action**: Run the existing `examples/performance/computation-graph` benchmark before/after each change to confirm wins. Some of these are measured-value-needed; some are obvious refactors.

**What to do**:
- Replace `Arc<tokio::sync::RwLock<InputCache>>` with `Arc<parking_lot::Mutex<InputCache>>` in the reactor (PERF-05). Access is short and never holds the lock across `.await`. Same for `DirtyFlags` and `SeqQueue`.
- Restructure `EndpointRegistry::send_to_accumulator` to hold the registry-wide lock only for namespace lookup (PERF-06). Store senders behind a per-name `Arc<Mutex<Vec<mpsc::Sender>>>` so the registry-level lock is only taken to add/remove names, not to send. Or use a two-pass: read-lock to send, escalate to write-lock only for pruning.
- Switch reactor state persistence from `serde_json::to_vec` to `bincode` (PERF-07). The data is internal-only; bincode is faster and smaller. `bincode` is already a workspace dep.
- Combine the 3-4 pre-spawn_blocking GIL acquisitions in `crates/cloacina-python/src/task.rs:184-188, 204` into a single `with_gil` block (PERF-15). One refactor, no GIL semantics change.
- Replace 100ms polling in reactor startup gate with `tokio::select!` over each `watch::Receiver::changed()` using `futures::stream::FuturesUnordered` (PERF-16).

**Why it matters**: The CG runtime is the system's intended fast-throughput path. Every accumulator boundary takes a registry-wide write lock AND a per-reactor cache write lock AND a dirty-flag write lock. JSON serialization on every fire is overhead the system wears unnecessarily. GIL acquisitions stack 5+ per Python task invocation.

**Suggested approach**:
1. Week 1: parking_lot swap; benchmark.
2. Week 2: EndpointRegistry restructure; benchmark.
3. Week 3: bincode persistence; reactor startup gate; GIL combine.

**Acceptance**:
- Benchmark shows N% throughput win (TBD; measure first).
- Reactor persistence wire format change is documented in a migration note (existing checkpoint rows must be backward-compatible — likely needs a versioned envelope).

---

### REC-14: Add un-paginated full-table scan limits and N+1 batches; cache constructor outputs in `Runtime`

**Addresses**: PERF-04 (Major), PERF-08 (Major), PERF-11 (Minor)
**Effort**: Days
**Type**: Refactor
**Dependencies**: None

**What to do**:
- Add `.limit(N)` to `get_active_executions` and `get_ready_for_retry` based on capacity (`executor.has_capacity()` / `available_permits()`). Return only what can be dispatched. For Postgres, an `ORDER BY created_at` + `LIMIT N` keeps things fair (PERF-11).
- Add batched DAL methods:
  - `get_metadata_for_tasks_batch(...)` — already exists via `get_dependency_metadata_with_contexts`; reuse with a status filter for `update_execution_final_context` (PERF-04).
  - `release_runner_claims_batch(task_ids)` and `mark_ready_batch(task_ids)` — for `StaleClaimSweeper::sweep` (PERF-04).
- Replace per-task DAL calls in the three N+1 sites with batched calls.
- Cache constructor outputs in `Runtime` registries (PERF-08). For each registry, materialize the constructor's output once into a cache (e.g., `OnceLock<Arc<dyn Task>>` per namespace). The closure is `Fn` not `FnMut`/`FnOnce`, so callers were never promised a fresh value. Same shape applies to `get_workflow`, `get_trigger`, `get_computation_graph`, `get_triggerless_graph`, `get_reactor`. Hoist `runtime.get_workflow(name)` out of the per-task loop in `state_manager` — fetch once per workflow, reuse across tasks.

**Why it matters**: At 100ms scheduler tick + thousands of concurrent workflows, the wire-format payload is megabytes per tick. Sweeping 100 stale claims is 200 transactions. Every dispatched task currently allocates a fresh `Arc<dyn Task>` via the constructor closure even though tasks are stateless. Standard list-detail collapse with batching plus instance caching addresses all three.

**Acceptance**:
- A test with 1000 Ready tasks and `max_concurrent_tasks = 4` returns exactly 4 (or another small bounded number) per tick rather than fanning out 1000 dispatch calls.
- The sweeper test with 100 stale claims uses 1 batched UPDATE per state transition, not 100.
- A flame graph of `runtime.get_task` shows the constructor closure called once per (registry, namespace) tuple, not per dispatch.

---

### REC-15: Quarantine reconciler failures with backoff; mirror tempdir lifecycle to package state

**Addresses**: OPS-11 (Minor), COR-05 (Major), SEC-18 (Minor)
**Effort**: Days
**Type**: New code
**Dependencies**: None

**What to do**:
- Add an in-memory `HashMap<PackageId, PackageFailureState { count, last_failed_at, last_reason }>` on `RegistryReconciler`.
- Skip retry for `T-now < last_failed + backoff(count)`; log `info!` (not `warn`) on the skip.
- Emit `cloacina_reconciler_package_failures_total{package, reason}` counter.
- Quarantine after 5 consecutive failures with a clear "package X is quarantined; remove and re-add to retry" log.
- Replace `std::mem::forget(temp_dir)` at `crates/cloacina/src/registry/reconciler/loading.rs:96` and `crates/cloacina/src/registry/loader/package_loader.rs:581` with a guard struct that owns both the `PluginHandle` and the `TempDir` together, modeled on the existing `LoadedWorkflowPlugin` pattern at `task_registrar/dynamic_task.rs:38` (COR-05). Wire the trigger plugin handles into `PackageState` so they drop with the package's lifecycle.
- Add a metric `cloacina_reconciler_active_temp_dirs` so disk-fill is observable (SEC-18).

**Why it matters**: A single bad `.cloacina` file in a watch directory produces a `warn!` every poll tick (default 1-2s) until manually removed. Logs fill up; operators see noise; no metric. Separately, `mem::forget(temp_dir)` per package load means a daemon hot-reloading versions accumulates `~/tmp/.tmpXXX/trigger_plugin.so` files until process restart — disk-fill DoS on long-running deployments.

**Acceptance**:
- A bad package fails to load 5 times consecutively, then no further attempts log `warn!` until the file is touched/replaced.
- `cloacina_reconciler_package_failures_total` counter increments per failure.

---

### REC-16: Compiler `/health` vs `/ready` split; bound shutdown

**Addresses**: OPS-08 (Minor), OPS-10 (Minor)
**Effort**: Hours
**Type**: Bug fix
**Dependencies**: None

**What to do**:
- Make `/health` a true liveness probe (always 200 if process is alive — fine).
- Add `/ready` returning 200 on healthy stats and 503 on DAL error, mirroring the server's `/health` vs `/ready` split (OPS-08). Update the docker-compose health check to point at `/ready`.
- Wrap the cargo invocation in `tokio::select!` against a shutdown token: spawn cargo via `tokio::process::Command`, then `tokio::select! { res = child.wait() => …, _ = shutdown.cancelled() => { child.kill().await.ok(); … } }` (OPS-10). Add a `--shutdown-timeout-s` flag mirroring the daemon. On hard timeout, log a warn and abandon — the row stays `building`, the sweeper resets it after `stale_threshold_s`.

**Why it matters**: K8s/Nomad probes can't distinguish "queue depth fine" from "DB unreachable" because both return 200. SIGTERM during a cargo build hangs indefinitely on the cargo subprocess.

**Acceptance**:
- Compiler `/ready` returns 503 when DB is unreachable.
- SIGTERM during a 10-minute cargo build kills the cargo subprocess within `--shutdown-timeout-s`.

---

### REC-17: Loose-end correctness items

**Addresses**: COR-06 (Minor), COR-08 (Minor), COR-10 (Major), COR-11 (Major), COR-14 (Observation), COR-16 (Minor), COR-18 (Minor)
**Effort**: Days
**Type**: Bug fix
**Dependencies**: None

**What to do**:
- Replace `chrono::Duration::from_std(...).unwrap()` with `unwrap_or(chrono::Duration::seconds(60))` in `cron_recovery.rs:212` (COR-06).
- Reverse ordering in `complete_task_transaction`: save context first, then `mark_completed` only if context save succeeded (COR-10). Or wrap both writes in a single Diesel transaction.
- Promote silent JSON parse and context-merge failures to errors (COR-11). Failed parses become `ExecutorError::ContextLoadFailed`. Add `cloacina_context_merge_failures_total{kind=...}` counter.
- Add deterministic tiebreaker (task name or task id) on `final_context` selection by completion timestamp (COR-14).
- Add instance-id or build-claim-id column to `workflow_packages`, set it on claim, filter on it in `mark_build_success`/`mark_build_failed` (COR-16). Mirrors task-claim guarding.
- Replace the wildcard `_ => WorkflowStatus::Failed` arm in `get_execution_status` with a fallible parse returning `Err(...)` on unknown values (COR-18). Add a unit test that round-trips every variant.
- Either `let _ = handle.await` after heartbeat abort, or restructure so heartbeat takes a `watch::Receiver<bool>` shutdown signal flipped synchronously before state transition (COR-08).

**Why it matters**: Each is small; collectively they remove "log-and-continue" idioms at the boundaries where data loss happens silently.

**Acceptance**: Each finding's "Acceptance" line from the lens file is satisfied.

---

## Structural Improvements (larger efforts, schedule deliberately)

### REC-18: Decompose the `cloacina::package!()` macro

**Addresses**: EVO-01 (Major), LEG-05 (Major), LEG-14 (Minor), LEG-20 (Observation), API-07 (Major), API-14 (Minor), COR-04 (Major), EVO-13 (Minor), EVO-17 (Observation)
**Effort**: Weeks
**Type**: Refactor + Architecture
**Dependencies**: REC-08 (dead-code cleanup before decomposition)

**What to do**:
- Decompose `cloacina::package!()` (`crates/cloacina-workflow-plugin/src/lib.rs:110-672`) into per-method emitter macros: `__cloacina_emit_get_task_metadata!()`, `__cloacina_emit_execute_graph!()`, etc. The shell becomes the orchestrator that calls each helper.
- Add a single shared `cdylib_runtime!()` macro (or `pub fn cdylib_runtime(thread_name: &'static str) -> &'static Runtime` helper in `cloacina-workflow-plugin`) that all four execute-* methods reuse. The four duplicated `OnceLock<Runtime>` blocks (LEG-14) become a single helper call.
- Replace `.expect("Failed to create cdylib tokio runtime")` with `OnceLock<Result<Runtime, String>>` so init failures don't panic across FFI; on subsequent calls, return cached error as `PluginError` (COR-04).
- Add `cloacina::package!(includes = [tasks, triggers, ...])` for explicit shape selection (API-14). Default = all (current behavior).
- Rewrite the `package!()` macro doc and the `CloacinaPlugin` trait `## Methods` section to enumerate all 9 methods with one-line semantics (LEG-05, API-07). Give every `METHOD_*` constant its own one-line doc that names the method, not "/// See [`METHOD_GET_TASK_METADATA`]."
- Plan the next leaf-crate phase: relocate `Workflow` (or a thinner `WorkflowSpec`) to `cloacina-workflow` so packaged cdylibs can include the full constructor in their inventory rather than just metadata (EVO-17). Unify `TriggerlessGraphRegistration` with `ComputationGraphRegistration` via `Option<TriggerSource>` (EVO-13).
- Author an ADR or section in `cloacina-workflow-plugin`'s `lib.rs` covering: ABI version field, optional method bit policy, wire-format type evolution rules (add fields with `#[serde(default)]`, never remove), deprecation timeline (EVO-20).

**Why it matters**: The 560-line monolithic macro is the single high-cost surface for plugin-ABI evolution. Adding a tenth method, opting tasks-only crates out of unused shapes, fixing the `.expect()` panic, or sharing a single tokio runtime — all require touching the same macro body. Twelve findings cluster around this.

**Suggested approach**: Plan as one Metis initiative.
1. Week 1: Decompose into per-method emitters; ship `cdylib_runtime!()` helper.
2. Week 2: `OnceLock<Result>` for runtime init; add `includes = [...]` selector.
3. Week 3: Doc rewrite (macro, trait, METHOD_* constants).
4. Week 4: ABI evolution policy ADR.

The `Workflow` relocation and `TriggerlessGraph` unification are separate follow-up initiatives — call them out at decomposition time so they're scheduled.

**Acceptance**:
- Adding a 10th method to `CloacinaPlugin` requires writing one helper macro and adding one line to the shell.
- A tasks-only cdylib emitted with `cloacina::package!(includes = [tasks])` does not initialize tokio runtimes for trigger/CG/triggerless-CG.
- The `package!()` macro doc enumerates 9 methods.
- The `CloacinaPlugin::Methods` section documents every method.
- Cdylib runtime init failures return a `PluginError` instead of aborting the host.

---

### REC-19: Test architecture: invest in failure-path, contract, and concurrency coverage

**Addresses**: COR-09 (Major), COR-13 (Minor), COR-15 (Critical-adjusted, partly addressed in REC-01), EVO-07 (Major), API-01/02/03 (already in REC-04), SEC-01/04 (already in REC-01)
**Effort**: Weeks (multi-phase)
**Type**: New tests + Refactor
**Dependencies**: REC-04 (CLI/server contract test harness)

**What to do**:
- Phase 1 (already partly in REC-01/REC-04): un-gate signing tests; add CLI/server contract harness.
- Phase 2: Add a double-dispatch race test (COR-09). Spawn two `SchedulerLoop` instances pointing at the same DB, mark N tasks Ready, assert each runs exactly once. Add `.filter(task_executions::claimed_by.is_null())` to `get_ready_for_retry` so the fan-in only sees genuinely-unclaimed work.
- Phase 3: Add tests for `mark_failed`/`mark_completed` returning `false` (claim-stolen path) — the post-T-0474 invariant has no regression test.
- Phase 4: Add an FFI panic test — load a cdylib whose plugin method panics; assert fidius converts to `CallError` and the host survives.
- Phase 5: Add DST/TZ transition tests for the cron evaluator (spring-forward, fall-back).
- Phase 6: Add a reactor `WhenAll`-after-partial-fire test (COR-03 regression coverage).
- Phase 7: Replace real-clock sleeps in `stale_claims.rs:143, 173, 178, 206, 209` with `tokio::time::pause()` + `advance()` (COR-13). Inject the clock as a trait so tests supply a `MockClock`.
- Phase 8: Establish a `cloacina-testing-cg` (or extend `cloacina-testing`) with documented test helpers — `make_test_reactor()`, `make_test_accumulator()`, `make_test_endpoint_registry()` — that take stable arguments and produce ready-to-use values. Refactor existing tests to use the helper surface, freeing internal modules to be refactored without breaking tests (EVO-07).
- Phase 9: Audit Python tests. `test_scenario_11_retry_mechanisms.py` configures a retry policy on a task that doesn't fail and asserts only completion — that's not a retry test. Rewrite to actually exercise retry.
- Phase 10: Replace silent `return` in `fidius_validation.rs:91-98` with `panic!("test requires dylib at {:?} — build with `angreal test integration` or `cargo build -p packaged-workflow-example`", project_path)` so missing fixture surfaces as real failure.

**Why it matters**: Failures don't surface as test failures, they surface as production incidents. Three Critical CLI/server findings shipped without any test catching them. The atomic claim is the only thing preventing duplicate execution — but no test asserts this. The signing trust-chain has six tests that don't run by default. Test architecture is biased toward happy paths.

**Suggested approach**: Phases 1-3 are the urgent ones (release-blockers backed in REC-01 and REC-04). Phases 4-7 are the next correctness wave. Phase 8-10 are continuous investment.

**Acceptance**:
- Every Phase has a corresponding green test in CI.
- `cloacina-testing-cg` (or extended `cloacina-testing`) is documented and used by ≥3 integration tests.
- `cargo test --no-default-features` and `angreal test unit` both run the signing trust-chain tests.

---

### REC-20: Documentation hygiene sweep

**Addresses**: LEG-01, LEG-02, LEG-03, LEG-07, LEG-08, LEG-09, LEG-15, LEG-16, LEG-17, LEG-18, LEG-19, EVO-11 (all Minor); LEG-04 / API-11 (Critical-adjusted) — see REC-21 for that one.
**Effort**: Days
**Type**: Docs
**Dependencies**: None (best done after REC-08, REC-18 settle to avoid double work)

**What to do**:
- Sweep `crates/cloacina/src/computation_graph/scheduler.rs:38, 266` and `crates/cloacina/src/computation_graph/registry.rs:189` for "Reactive Scheduler" / "reactive scheduler" / "reactive computation graph" — replace with the post-CLOACI-S-0011 names. Regenerate rustdoc HTML so the banned phrases stop shipping (LEG-01).
- For `S-0007` spec: rewrite section to use "computation graph scheduler" (it's a published spec, not historical), or add a one-line note pointing readers to S-0011.
- Rename `RunningGraph` → `RunningReactor` (or `ReactorInstance`); rename `AccumulatorSpawnConfig.graph_name` → `reactor_name`; same for `CheckpointHandle.graph_name` if storage is per-reactor. Audit `graph_name`-bound iterator variables in `scheduler.rs` (LEG-02).
- Inline the three re-exports from `crates/cloacina/src/computation_graph/global_registry.rs` and `types.rs` into `computation_graph/mod.rs`; delete the dead modules; regenerate rustdoc (LEG-03).
- Delete the stale paragraph at `crates/cloacina/src/inventory_entries.rs:30-31` (LEG-07).
- Update `Runtime` struct doc to say "seven namespaces" and list all of them; extend the `Debug` impl to include `triggerless_graphs` and `reactors`; extend the test (LEG-08).
- Sweep "with_global_workflows*", "global_registry", "global registry" comments in `execution_planner/mod.rs:178-183, 199, 569-571`; `executor/thread_task_executor.rs:783`; `registry/loader/task_registrar/mod.rs:128, 210, 229-233`; `python_runtime.rs:55`; `runner/default_runner/config.rs:585, 696-697` (LEG-09).
- Refactor `tasks.rs:384-471` and `tasks.rs:485-541` so `parse_trigger_rules_expr` calls `parse_trigger_condition_expr` plus a wrapper (LEG-15).
- Rewrite `#[reactor]` macro doc at `crates/cloacina-macros/src/lib.rs:144-155` to say "by name string" with a `trigger = reactor("risk_signals")` example (LEG-16).
- Delete the orphaned "Helper macro" comment at `crates/cloacina/src/dal/unified/mod.rs:78-83` (LEG-17).
- Decide `WorkflowMetadata.schedules: Vec<String>` fate (LEG-18) — populate from FFI metadata or `#[deprecated]` and document migration.
- Replace "global registries" with "scoped runtime registries" in `reconciler/loading.rs:101-110` (LEG-19).
- Update `WorkflowRegistry` trait doc at `crates/cloacina/src/registry/traits.rs:67-73` to reflect that `register_workflow` only stores; the reconciler handles registration. Consider splitting into `WorkflowMetadataStore` and `WorkflowRegistry` (EVO-11).

**Why it matters**: Refactors landed cleanly in code; doc comments, examples, module names, and rustdoc HTML lag behind. Each finding is small. Collectively they tell newcomers the wrong story about every concept that's been refactored.

**Suggested approach**: Bundle into a single PR or a single initiative's cleanup phase. Most are 1-3 line edits.

**Acceptance**: `grep "Reactive Scheduler\|reactive scheduler\|reactive computation graph"` in `crates/` returns zero hits in production source. Rustdoc HTML regenerated with banned phrases gone.

---

### REC-21: Rewrite the README and engine-crate quick-start to match actual API

**Addresses**: LEG-04 (Critical-adjusted), API-11 (Critical-adjusted)
**Effort**: Hours
**Type**: Docs
**Dependencies**: None

**What to do**:
- Rewrite `README.md:33` version pin from `cloacina = "0.1.0"` to `cloacina = "0.5.1"`.
- Rewrite `README.md:72-77` quick-start example to use the actual `#[workflow(name = "...", description = "...")] pub mod my_workflow { #[task(id = "extract", dependencies = [])] async fn extract(ctx: ...) -> ... { ... } }` syntax.
- Rewrite `crates/cloacina/src/lib.rs:75-105` quick-start the same way. Update the `executor.execute(...)` typo to `runner.execute(...)` (LEG-04).
- Update `crates/cloacina/src/lib.rs:452` prelude doc to say `#[task]` and `#[workflow]` (no bang on `workflow`) consistently.
- Audit `crates/cloacina/src/execution_planner/mod.rs:99-103` for the same `workflow! { ... }` pattern.
- Add a tested example to `examples/tutorials/rust/` that demonstrates the README's quick-start verbatim, so future README changes have a concrete reference.

**Why it matters**: This is the README's first code sample and the engine crate's top-level rustdoc. A new user's first 30 minutes of cloacina is reading these docs and writing code that doesn't compile. Public-face issue.

**Acceptance**: A new user can copy the README quick-start verbatim and `cargo build` succeeds.

---

### REC-21B: Extract per-language drivers from `reconciler/loading.rs::load_package`

**Addresses**: LEG-10 (Major), EVO-03 (Major)
**Effort**: Days
**Type**: Refactor
**Dependencies**: None — but best landed after REC-08 (delete dead path) settles, before any further loader changes

**What to do**:
- Extract three private async methods on `RegistryReconciler`:
  - `load_rust_package(metadata, manifest, work_dir, library_data) -> ...`
  - `load_python_workflow_package(...)`
  - `load_python_cg_package(...)`
- `load_package` becomes a 60-line dispatcher that handles archive write, unpack, manifest load, then dispatches by language.
- The `step_load_*` helpers (cron triggers, custom triggers, reactors, trigger-less CGs, reactor-bound CGs, workflows) at `loading.rs:1407-1885` are already the right abstraction; just stop inlining their callers in the giant `load_package` chain.
- The `build_view_rust` and `build_view_python` are halfway there — make `load_package` itself language-agnostic with a single `let view = build_view(language, archive)?;` call followed by the unchanged precedence-ordered step calls.

**Why it matters**: `loading.rs` is 2,458 lines; `load_package` is 540+ lines of intricate per-language branching. Three rewrites in six months (T-0549 → T-0553 → T-0554 → T-0556) were each cleanup waves removing dead branches uncovered by the previous wave. The function will need at least one more pass to settle. Sub-step refactors today touch the giant function's local variable threads (`rust_reactor_names`, `triggerless_graph_names`, `cron_schedule_ids`, ...) all of which live in the same scope.

**Suggested approach**:
1. Day 1: Extract `load_rust_package` first (largest branch, clearest boundary).
2. Day 2: Extract `load_python_workflow_package` and `load_python_cg_package`.
3. Day 3: `load_package` becomes a thin dispatcher; tests still pass.

Watch for: shared local state across the three branches (e.g., reconciler bookkeeping) needs to either move into the per-language methods or stay in the dispatcher. Keep the precedence-ordered pipeline intact through the move.

**Acceptance**:
- `load_package` is <100 lines.
- Each of the three per-language methods is independently readable.
- Existing reconciler integration tests pass unchanged.

---

### REC-22: Adopt diesel `MultiConnection` to halve DAL maintenance cost

**Addresses**: EVO-02 (Major), COR-07 (Minor)
**Effort**: Weeks
**Type**: Refactor
**Dependencies**: None
**Investigate before action**: `MultiConnection` is mentioned in the module doc at `dal/unified/mod.rs:22-27` but isn't applied. Before committing, confirm with a small spike (e.g., one DAL module) that it does in fact let one body serve both backends without `#[cfg]` divergence.

**What to do**:
- Spike: pick one DAL module (e.g., `recovery_event.rs` — small, clean) and convert it from paired `_postgres`/`_sqlite` functions to `MultiConnection`. Validate test coverage holds.
- Roll out across the 218 paired functions in the unified DAL module-by-module. Each module is ~10-30 paired functions.
- Document the transition in the DAL `mod.rs` rustdoc.
- Document on migrations 006 and 007 that the DROP+CREATE pattern is deprecated and should not be copied (COR-07). For the next column addition, use `ALTER TABLE ... ADD COLUMN`.

**Why it matters**: 218 paired functions, 141 dispatch sites. Every DAL change is two changes and every test must validate both paths. Adding a third backend (MySQL) would be ~110 new functions plus a three-arm dispatcher.

**Suggested approach**:
1. Week 1: Spike on one module; validate.
2. Weeks 2-4: Roll out across all DAL modules; one PR per module or batched per DAL family.

**Acceptance**:
- The 218 paired-function count drops to <50 (some modules genuinely need backend-specific behavior).
- DAL test coverage holds across both backends.

---

### REC-23: Resolve migration drift — port API keys to SQLite or document the auth feature scope

**Addresses**: EVO-09 (Major), COR-07 (Minor — covered in REC-22)
**Effort**: Days
**Type**: Refactor + Docs
**Dependencies**: None

**What to do**:
- Decide: port the API key tables to SQLite (the `Universal*` types should support this), or split the auth migrations into a separate `migrations/postgres-auth/` directory that's only loaded when `feature = "auth"` is on.
- If SQLite-auth is genuinely out of scope, document it in `dal/unified/mod.rs` near where `api_keys` is `#[cfg(feature = "postgres")]`.
- Update the `auth = ["postgres"]` Cargo feature doc to explain why SQLite can't have API keys.

**Why it matters**: Three Postgres-only migrations; numbering offsets diverge by 3 throughout. The `auth = ["postgres"]` Cargo feature gates at compile time, but the migration directory structure makes the divergence implicit and easy to miss.

**Acceptance**: A new contributor can read the DAL doc and answer "does SQLite have API keys" without reading source.

---

### REC-24: Standardize graceful shutdown across the three binaries

**Addresses**: OPS-10 (Minor — covered in REC-16), EVO-18 (Minor), COR-02 (Major — covered in REC-07), COR-08 (Minor — covered in REC-17)
**Effort**: Days
**Type**: Refactor
**Dependencies**: REC-16 (compiler shutdown bound)

**What to do**:
- Document in `DefaultRunner::new` rustdoc that `shutdown()` must be called explicitly; the doc at line 79 doesn't mention it (EVO-18).
- Consider a `DefaultRunner::with_shutdown_token` constructor that returns `(DefaultRunner, ShutdownGuard)` where the guard's Drop blocks on shutdown via `tokio::runtime::Handle::current().block_on(...)`.
- Standardize the shutdown pattern across server / daemon / compiler: each should bound shutdown with `tokio::time::timeout` and document the timeout flag.

**Why it matters**: Async Rust + Drop is an open design space. The three deployables have three different answers; cross-binary consistency makes operator behavior predictable.

**Acceptance**: All three binaries support `--shutdown-timeout-s` (or equivalent); all three log a clear message on timeout-triggered force exit.

---

### REC-25: Add HTTP rate limiter + per-route body limits + WS frame limits

**Addresses**: OPS-19 (Major), OPS-18 (Minor), SEC-12 (Minor)
**Effort**: Days
**Type**: New code
**Dependencies**: None

**What to do**:
- Add `tower-governor` (or hand-roll a per-key rate limiter middleware) with separate buckets for `auth_attempts`, `uploads`, `executes`, `ws_connections`. Default 100 req/min/key.
- Set explicit per-route body limits — e.g., 10KB for JSON routes (overriding the 100MB default), keep 100MB only for `/v1/tenants/{id}/workflows`. Add `--max-package-size-mb` CLI flag.
- Configure WS `with_max_message_size` and `with_max_frame_size` on the upgrade — pin to e.g., 1MB per message.
- Add per-accumulator connection limit (default 16) enforced at upgrade time.
- Add `cloacina_rate_limited_total{route}` counter so operators can see when the limit fires.

**Why it matters**: Project memory cites "rate limiter" as one of 5 major gaps from a soak test. A single misbehaving CI bot can wedge a tenant's pool. WebSocket connections are unlimited; an attacker can open thousands of `/v1/ws/accumulator/{name}` connections to one accumulator.

**Acceptance**:
- 101 requests in one minute from a single API key triggers a 429.
- A 1.5MB WS message is rejected with a clear close code.
- `cloacina_rate_limited_total` counter increments visibly.

---

## Architectural Recommendations (systemic, beyond individual findings)

### REC-26: Adopt a "cleanup-task placeholder" decomposition convention

**Addresses**: EVO-14 (Observation) and the systemic doc-lag pattern across LEG/EVO findings
**Effort**: Process change (no code)
**Type**: Process
**Dependencies**: None

**What to do**:
- At Metis initiative decomposition time, every initiative explicitly lists N "cleanup phase" placeholders alongside the headline tasks. The team is doing this implicitly already (T-0549 has Phase 1/2a/2b/2c/2d) — formalize the convention.
- Sized at ~1 day of doc + dead-code sweep each.
- Scheduled at N+1 from the headline initiative; carried in Metis as visible tasks rather than discovered piecemeal in follow-up initiatives.
- Consider lints as structural guardrails: `cargo deny`-style rules for "no `pub use` chains across more than 2 hops" or "no `_legacy` / `_old` suffix in module names" — surface drift earlier.

**Why it matters**: 275 commits in 6 months, last 50 dominated by I-0102 cleanup waves. T-0509 had to "finish I-0096 cleanup" — meaning a second initiative was needed to clean up after the first. The pattern is healthy in that the team explicitly closes loops; it's a worry in that complete coverage of a refactor takes ~5 follow-up tasks beyond the headline initiative. Pre-1.0 the pace is sustainable; post-1.0 it won't be.

**Acceptance**: The next initiative shipped after this recommendation lists explicit "cleanup phase" tasks at decomposition time.

---

### REC-27: Plan the next phase of the leaf-crate refactor

**Addresses**: EVO-13 (Minor), EVO-16 (Minor), EVO-17 (Observation), EVO-19 (Minor)
**Effort**: Months (a Metis initiative)
**Type**: Architecture
**Dependencies**: REC-18 (`package!()` decomposition)

**What to do**:
- Move `Workflow` (or a thinner `WorkflowSpec`) to `cloacina-workflow` so packaged cdylibs can include the full constructor in their inventory rather than just metadata. The current `Workflow` type is heavy — has dependency-graph computation, task lookup, validation. A minimal authoring `WorkflowSpec` could live in `cloacina-workflow`, with the engine consuming it via `From<WorkflowSpec>` (EVO-17).
- Unify `TriggerlessGraphRegistration` with `ComputationGraphRegistration` via `Option<TriggerSource>` (EVO-13). The reconciler's precedence-ordered loader is the right place to dispatch by source.
- Decide stream-backend extension model (EVO-16): either relocate `StreamBackendEntry` to `cloacina-workflow-plugin` (matching T-0549's pattern for `TaskEntry`) and add a tenth method to `CloacinaPlugin` for stream-backend metadata + factory bridging, or document that custom stream backends are not packaged-cdylib-supported and must be linked into the host. The current state is implicit — the doc says the shell handles "any combination of declared primitives" but stream backends are silently excluded.
- Split `crates/cloacina-macros/src/tasks.rs` into `parse.rs` (attribute parsing), `codegen.rs` (token generation), `validators.rs` (compile-time validation) — matching the internal organization in `computation_graph/`. Consider extracting the retry attribute set into a separate `#[retry(...)]` macro (EVO-19).

**Why it matters**: The leaf-crate refactor is incomplete. `WorkflowEntry`'s constructor type still lives in the engine; `StreamBackendEntry` is silently skipped by the shell macro; the `#[task]` macro is monolithic; CG and trigger-less CG are duplicate surfaces. Each future CG-related feature has to be added in two places.

**Suggested approach**: Plan as a Metis initiative downstream of REC-18 — that initiative's macro decomposition unblocks the leaf-crate moves.

**Acceptance**: `cloacina-workflow`'s dependency tree no longer transitively pulls in the engine's heavy types. Adding a CG-related feature is a single-place change.

---

### REC-28: Decide the multi-tenant audience and align the implementation

**Addresses**: EVO-15 (Observation), the Embedded simplicity vs Multi-tenant safety tension, the architectural framing for REC-03
**Effort**: Architecture decision (followed by implementation per REC-03)
**Type**: Architecture
**Dependencies**: None — but the answer guides REC-03

**What to do**:
- Choose: either (a) declare the embedded shape canonical and explicitly limit multi-tenant to read-only operations until per-tenant runners are built; or (b) build per-tenant runner caching and treat multi-tenant as first-class (REC-03's path).
- Document the choice in an ADR.

**Why it matters**: The product positions itself for both audiences (embedded single-runner and multi-tenant SaaS). The implementation prefers the embedded shape; multi-tenant is shimmed in via `TenantDatabaseCache` for read paths only. The current "implicitly multi-tenant" posture is what creates the security findings. Either is a defensible position; the indecision is what hurts.

**Acceptance**: An ADR exists; the README and runbook reflect the choice.

---

### REC-29: Shrink the `RuntimeMessage` enum in `cloacina-python`

**Addresses**: EVO-05 (Major), API-12 (Major), PERF-19 (Observation)
**Effort**: Weeks
**Type**: Refactor
**Dependencies**: None

**What to do**:
- Two reasonable directions (per EVO-05/API-12):
  - **(a) Shrink to single variant `Execute(BoxedFuture)`** and pass futures through the channel instead of named operations. Let the Python wrapper construct closures that capture engine method calls. This requires the engine methods to all be `Send + Future + 'static`.
  - **(b) Generate the bindings** — a proc-macro on the engine API that emits the message variant + the Python method + the runtime-thread match arm.
- Recommend (a) for maintainability; (b) is the bigger investment but pays off if the Python surface keeps expanding.
- Group related Python methods into sub-objects: `runner.cron`, `runner.triggers`, `runner.executions` (API-12). Each sub-object has 5-6 methods.
- Bound `tx: mpsc::UnboundedSender<RuntimeMessage>` to `mpsc::channel(1024)` and treat backpressure as a real signal (PERF-19).
- Document Python response shapes (`WorkflowResult`, `Schedule`, `ScheduleExecution`) so they're surfaced cleanly from the `cloaca` module's docstring.

**Why it matters**: 28-variant enum, 4-6 file ripple per Python API addition. Asymmetric method shapes; no pagination convention. The Python surface is the public face for non-Rust users — it deserves the same care as the CLI.

**Suggested approach**: Plan as a Metis initiative; the (a) vs (b) choice is the decomposition decision.

**Acceptance**:
- Adding a new engine method exposed to Python is a single-place change.
- `runner.cron.list()`, `runner.triggers.get(name)`, etc. are the natural sub-object hierarchy.

---

## Summary Roadmap

The team ships one PR per Metis initiative. Recommendations group naturally as follows:

```
Phase 1 — Release-blockers (parallel where possible)
├── REC-01: Wire signature verification end-to-end (Days)
├── REC-02: Sandbox compiler build path (Weeks → Months)
├── REC-03: Complete multi-tenant abstraction (Weeks)
└── REC-04: Audit CLI/server contract end-to-end (Weeks)
    │
    └── REC-10: Plumb missing operator verbs (Days; depends on REC-04 harness)

Phase 2 — Critical bug fixes and operability (parallel; small)
├── REC-05: Auth model docs + tightenings (Days)
├── REC-06: Active-tasks gauge + persist counters (Hours)
├── REC-07: release_runner_claim guard (Hours)
├── REC-08: Remove dead generate_packaged_registration (Hours)
├── REC-09: Configurable pool sizes (Days)
├── REC-11: Prometheus in compiler + daemon (Days; depends on REC-06)
├── REC-12: Reactor Latest atomic snapshot (Days)
├── REC-14: Pagination + N+1 batches (Days)
├── REC-15: Reconciler quarantine + backoff (Days)
├── REC-16: Compiler /ready + bounded shutdown (Hours)
└── REC-17: Loose-end correctness items (Days)

Phase 3 — Structural improvements (one initiative each)
├── REC-13: Reactor / CG hot-path restructure + bincode (Weeks; depends on REC-12)
├── REC-18: Decompose package!() macro (Weeks; depends on REC-08)
├── REC-19: Test architecture investment (Weeks; depends on REC-04)
├── REC-20: Documentation hygiene sweep (Days; best after REC-08, REC-18)
├── REC-21: README + engine quick-start rewrite (Hours)
├── REC-21B: Extract per-language drivers from reconciler/loading.rs (Days)
├── REC-22: Diesel MultiConnection adoption (Weeks)
├── REC-23: Resolve migration drift (Days)
├── REC-24: Standardize graceful shutdown (Days; depends on REC-16)
└── REC-25: HTTP rate limiter + body limits (Days)

Phase 4 — Architectural / process (longer arc)
├── REC-26: Cleanup-task placeholder convention (Process)
├── REC-27: Next leaf-crate refactor phase (Months; depends on REC-18)
├── REC-28: Multi-tenant audience decision (Architecture; informs REC-03)
└── REC-29: Shrink Python RuntimeMessage enum (Weeks)
```

**Suggested phasing for the next 6 months**:

- **Sprint 1-2**: REC-01, REC-04, REC-06, REC-07, REC-08, REC-21 (ship release-blockers and the highest-leverage small fixes; CLI works again; signing is on; gauge fixed; quick-start compiles).
- **Sprint 3-4**: REC-02 phase 1 (build timeout + offline-by-default), REC-03 phases 1-2 (span enrichment + triggers/health route fixes), REC-05, REC-10.
- **Sprint 5-6**: REC-03 phases 3-6 (per-tenant runner cache + tenant deletion teardown), REC-09, REC-11, REC-15, REC-16.
- **Sprint 7-8**: REC-12, REC-13, REC-17, REC-23, REC-25.
- **Months 5-6**: REC-18, REC-19, REC-20, REC-22.
- **Continuing**: REC-26 (process), REC-27, REC-28, REC-29 sequenced based on team priorities.

The first three sprints close every Critical-severity finding. Sprints 4-6 close every Major finding tied to multi-tenant, observability, and operability. Sprints 7-8 are correctness investment. Months 5-6 are evolvability investment.

## Open Questions / Investigations Needed

These are points where a benchmark, deployment context, or product decision is needed before committing to the recommendation shape:

- **REC-02 sandbox primitive choice**: Linux-only `landlock`/`bwrap`/`nsjail` is simpler but ties to Linux; container-spawn-per-build is more portable but heavier. Need a discovery spike.
- **REC-09 pool sizing defaults**: The right values for per-tenant Postgres pool size and SQLite reader count depend on workload. Ship configurable; recommend defaults; let operators tune.
- **REC-13 reactor hot-path wins**: PERF-05/06/07 should be measured before/after with the existing `examples/performance/computation-graph` benchmark to confirm the wins justify the refactor.
- **REC-22 `MultiConnection` adoption**: Spike on one DAL module first to confirm the abstraction does in fact let one body serve both backends without `#[cfg]` divergence.
- **REC-28 multi-tenant audience**: Architectural decision — embedded vs SaaS. The answer guides whether REC-03 is "limit multi-tenant to read-only" or "build per-tenant runner caching."
- **PERF-03 (`runner.execute` poll)**: Whether the 500ms poll is a user-visible bottleneck for synchronous workflows needs benchmark before committing to LISTEN/NOTIFY or `tokio::sync::Notify` plumbing.
- **PERF-14 (per-task heartbeat)**: Whether N is high enough for batched-heartbeat to matter needs measurement on a high-concurrency runner.
- **OPS-21 (daemon SIGHUP reload semantics)**: Which config fields should be hot-reloadable (versus warn + require restart) is a product decision.
- **OPS-13 (server `/health` liveness)**: The right `last-tick-staleness` threshold for liveness depends on deployment cadence; coordinate with operations runbook authors.
- **EVO-08 (`PYTHON_RUNTIME` global slot)**: If a third language runtime (Lua, Wasm) is ever added, refactor to a runtime registry keyed by language tag rather than another OnceLock. Watch for divergence.

These should be tracked alongside the recommendations as "investigate before committing" notes.
