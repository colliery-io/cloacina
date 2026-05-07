# Cloacina Code Review — Final Report

> Seven-lens code review of cloacina at branch `i-0102-fidius-and-plugin-shell`. Conducted in five phases.
> Source artifacts: `00-system-overview.md` through `08-cross-cutting.md` in this directory.

## Executive Summary

Cloacina has a sound architectural skeleton and a mature engine core. The Cargo workspace split (authoring crates / engine / binaries) is real and the discipline holds (the leaf authoring crates pull zero diesel/pyo3/kafka; the compiler service is verifiably pyo3-free); the unified DAL accessor pattern is consistent across both backends; the `Runtime` registry shape gives every primitive kind a symmetric register/unregister/get triple; the atomic claim/heartbeat/release primitives that underpin distributed work assignment are correctly built and the post-T-0474 single-finalizer invariant is regression-tested. CLOACI-S-0011 is an exemplary specification — banned terminology, primitive definitions, mapping tables. The credential-logging guard, schema-name validation, request-id middleware, `#[serde(deny_unknown_fields)]` with migration hints, `#[optional(since=N)]` plugin-ABI capability bits, and the CLI's noun-verb file structure are textbook examples of investments that pay off. Where the team has thought carefully about a problem, the work is good.

The damage concentrates at the **outer perimeter and at the seams between what the engine knows about and what the consumer-facing surfaces actually do**. Multi-tenancy is a half-built abstraction — storage is tenant-scoped, but the runner that executes workflows, the reconciler that loads packages, the request span that frames logs, and the lifecycle that tears tenants down are all admin-schema-bound. The plugin-load path is in-process arbitrary code execution by design and the only intended defense (signature verification) cannot be activated from the documented CLI surface; the build path runs `cargo build` on attacker-supplied source with no timeout, no rlimit, no namespace, no jail. Three CLI commands published to operators (`tenant create`, `execution list`, `tenant/trigger/execution list`) are non-functional because no test exercises the CLI/server contract end-to-end. `cloacina-compiler` and `cloacinactl daemon` install no Prometheus recorder, so the engine's metrics die in those processes. The README's quick-start walks new users through a `workflow!` declarative macro that does not exist. These are not subtle bugs — they are foundational gaps that ship in the default configuration.

## Headline Concerns

1. **SEC-01/SEC-04/SEC-06/OPS-07** — Default-config plugin loading is in-process RCE. Any tenant `write` key can upload code that executes as the host UID with full DB credentials; the intended signature defense is unactivatable from the documented CLI; the compiler builds attacker source with no isolation. Release-blocker for any deployment exposing the upload endpoint to untrusted principals.
2. **EVO-04/SEC-03/SEC-02/SEC-05/COR-01** — Multi-tenant boundary is incomplete. The shared `DefaultRunner` executes every tenant's workflow in the admin schema; `triggers.rs` reads from the admin DB regardless of URL tenant; `/v1/health/*` leaks every tenant's reactor names; `SET search_path` failures fall through silently to public. Release-blocker for any multi-tenant-claimed deployment.
3. **API-01/API-02/API-03** — Three CLI commands ship non-functional. `tenant create` posts a body the server cannot deserialize; `execution list` ignores all filters and only returns active workflows; `tenant/trigger/execution list` hand wrapping objects to an array renderer and silently emit empty output. No integration test covers the CLI/server seam.
4. **OPS-01** — `cloacina_active_tasks` gauge uses naked increment/decrement — exact antipattern T-0534 fixed for `cloacina_active_workflows`. Any panic between lines 840 and 904 of `thread_task_executor.rs` leaks the count permanently.
5. **OPS-04/OPS-05** — `cloacina-compiler` and `cloacinactl daemon` emit zero Prometheus metrics despite running long-lived claim-based queues that operators absolutely need to watch. The build queue is invisible to monitoring.
6. **LEG-04/API-11** — README first code sample and engine `lib.rs` quick-start use a `workflow!` declarative macro that does not exist. Newcomer's first 30 minutes fail to compile.
7. **OPS-02** — No `execution cancel`, `package rebuild`, `tenant drain`, or `workflow pause` CLI verbs. The engine has the primitives; operators have to write SQL.

## Summary Table

| Lens | Critical | Major | Minor | Observation | Total |
|---|---|---|---|---|---|
| Legibility (LEG) | 0 | 7 | 12 | 1 | 20 |
| Correctness (COR) | 1 | 9 | 7 | 1 | 18 |
| Evolvability (EVO) | 0 | 8 | 6 | 6 | 20 |
| Performance (PERF) | 0 | 8 | 10 | 2 | 20 |
| API Design (API) | 2 | 11 | 7 | 1 | 21 |
| Operability (OPS) | 0 | 7 | 13 | 1 | 21 |
| Security (SEC) | 2 | 4 | 12 | 0 | 18 |
| **Totals** | **5** | **54** | **67** | **12** | **138** |

After cross-cutting severity adjustments (see below): 11 Critical, 50 Major.

## Findings by Lens

### Legibility

**Posture**: Strong skeleton (CLOACI-S-0011, noun-verb CLI, unified DAL); damage concentrates in CG runtime source comments and the in-flight I-0102 migration tail. Newcomer-facing docs lag the code by 1-2 cleanup waves consistently.

**Themes**:
- Banned spec-prohibited terminology persists in CG runtime source comments and ships in published rustdoc HTML.
- Reactor/graph naming conflation in scheduler internals leaks the very distinction the spec works hardest to enforce.
- Doc/example/macro-doc drift after aggressive refactors — module names, examples, macro counts all stale.
- The 2,458-line `reconciler/loading.rs` carries six load steps, three language branches, and FFI plumbing in one function.
- Three different things called "scheduler"; one of them named just `Scheduler`.
- Packaged-cdylib runtime initialization replicated four times in the unified shell macro.

**Headline findings**:
- LEG-01 — Banned "Reactive Scheduler" terminology in three production source comments (CLOACI-S-0011 R1 violation).
- LEG-02 — `RunningGraph` and `AccumulatorSpawnConfig.graph_name` carry reactor names; reactor/graph distinction leaks.
- LEG-03 — `pub mod global_registry` exists at crate boundary with module doc saying registry was deleted.
- LEG-04 — README and engine `lib.rs` quick-start use a `workflow!` macro that doesn't exist.
- LEG-05 — `cloacina::package!()` doc says "Six methods"; the macro emits nine.
- LEG-06 — `workflow_attr.rs` builds 130 lines of FFI tokens then discards via `let _ = packaged_registration;`.
- LEG-10 — `reconciler/loading.rs` is 2,458 lines; `load_package` carries dispatch logic inline.
- LEG-14 — `package!()` macro inlines tokio-runtime construction four times.

**Positive patterns**:
- Unified `DAL` accessor pattern (`dal.context()`, `dal.task_execution()`, etc.) is symmetric and consistent.
- `cloacinactl` noun-verb file structure: every noun is a directory; every verb is one file.
- CLOACI-S-0011 itself is exemplary — primitives, bans with rationale, mapping tables.
- `dispatcher::traits.rs` trait docs with full Implementation Requirements and `# Example` blocks.
- Universal-types pattern (`UniversalUuid`, `UniversalTimestamp`, `UniversalBool`, `UniversalBinary`) for cross-backend portability.

**Reference**: full detail in `01-legibility.md` (20 findings).

### Correctness

**Posture**: Atomic primitives are solid; partial-failure surface is mostly invisible. Happy-path tests are real; concurrent-contention and FFI-failure tests are absent. The `let _ =` "best-effort" idiom at scale lets data loss happen silently.

**Themes**:
- Multi-tenant schema search_path failure silently routes queries to public — the load-bearing primitive of tenant isolation has fail-open behavior.
- `release_runner_claim` is unguarded; can release another runner's claim post-stale-sweep.
- FFI panic vectors in the unified shell macro `.expect()` paths.
- Best-effort persistence (`let _ = persist_*`) drops crash-recovery state without metrics.
- Test architecture covers happy paths; double-dispatch, FFI panic, DST, claim-stolen scenarios uncovered.
- Signing trust-chain tests are `#[ignore]`-gated and skip in default test runs.

**Headline findings**:
- COR-01 — Multi-tenant `SET search_path` failure silently routes queries to public schema (`let _ = conn.interact(...)`).
- COR-02 — `release_runner_claim` is unguarded; releases any task's claim regardless of ownership.
- COR-03 — Reactor `Latest` strategy clear/snapshot race; `WhenAll` fires can be skipped.
- COR-04 — `cloacina::package!()` shell macro `.expect()` aborts host on tokio Runtime init failure.
- COR-05 — `mem::forget(temp_dir)` per package load leaks tempdirs in long-running daemon.
- COR-09 — Zero double-dispatch race tests; atomic claim is the only thing preventing duplicate execution.
- COR-10 — `mark_completed` succeeds but `save_task_context` fails leaves partial state, downstream tasks see empty context.
- COR-15 — Six signing trust-chain tests `#[ignore]`-gated; the only RCE defense has no green-CI signal.

**Positive patterns**:
- Atomic claim-with-CAS in `claim_for_runner` and `claim_pending_tasks` — true exactly-once semantics.
- `event_dedup.rs` regression test pinning the post-T-0474 single-finalizer invariant.
- Workflow-completion race guard with re-check after the completion check.
- Dual-layer cancellation (heartbeat-driven cancel channel + `TaskHandle::cancelled()` cooperative observation).
- Manifest `#[serde(deny_unknown_fields)]` with friendly migration hints for `package_type`/`[[triggers]]`.

**Reference**: full detail in `02-correctness.md` (18 findings).

### Evolvability

**Posture**: Strong structural posture — crate split discipline holds, recent history shows healthy refactor velocity. Three high-coupling shapes dominate change cost: paired-DAL functions, the monolithic `package!()` macro, and the 28-variant Python `RuntimeMessage` enum.

**Themes**:
- Plugin ABI evolution is monolithic — `package!()` macro body is the single high-cost surface.
- DAL paired-function pattern doubles every backend operation (218 paired functions, 141 dispatch sites).
- Reconciler loading pipeline is a known refactor target (three rewrites in six months).
- Multi-tenant execution scoping is structural gap — runner is admin-schema-bound by construction.
- Python runner bridge through 28-variant `RuntimeMessage` enum.
- Migration drift between Postgres (22) and SQLite (19) is structural, not just feature-gated.
- Post-refactor cleanup is per-task, not per-initiative — every initiative needs ~5 cleanup tasks.

**Headline findings**:
- EVO-01 — Plugin ABI evolution is monolithic (560-line indivisible macro body).
- EVO-02 — DAL paired-function pattern (218 `_postgres`/`_sqlite` pairs, 141 dispatch sites).
- EVO-03 — Reconciler loading pipeline 2,458 LOC, three rewrites in six months.
- EVO-04 — Multi-tenant execution scoping is a structural gap (one shared runner across all tenants).
- EVO-05 — Python `RuntimeMessage` enum: 28 variants; every API addition is 4-6 file ripple.
- EVO-07 — Tests reach into engine internals (`accumulator::*`, `EndpointRegistry`, `ManualCommand`).
- EVO-09 — Migration drift between Postgres and SQLite is structural.
- EVO-10 — Per-macro `_ffi` and unified shell can coexist on the I-0102 transition branch.

**Positive patterns**:
- Crate split discipline holds: `cloacina-compiler` is verifiably pyo3-free; `cloacina-workflow` is diesel/kafka-free.
- `Runtime` registry shape: seven uniform register/unregister/get triples; clean +1 for new primitive types.
- `#[serde(deny_unknown_fields)]` + migration-hint wrapping at manifest load boundary.
- Service manager extraction (T-0483) — single shutdown call, reusable for `PyDefaultRunner`.
- fidius `#[optional(since = N)]` capability bits applied correctly across methods 4-8.

**Reference**: full detail in `03-evolvability.md` (20 findings).

### Performance

**Posture**: System is structured for correctness and operability first; performance is secondary but with real, addressable hot-path liabilities. Most findings need measurement before action; a handful are obviously addressable refactors.

**Themes**:
- Connection pool sizing is hardcoded and silently overrides caller config (per-tenant Postgres = 2; SQLite = 1).
- Scheduler loop polls every 100ms with un-paginated table scans on `workflow_executions` and `task_executions`.
- Reactor in-memory hot path uses `tokio::sync::RwLock` for tight read/write patterns; `parking_lot::Mutex` would be cheaper.
- Reactor persists state via JSON serialization on every fire (bincode would be faster and smaller).
- N+1 DAL patterns in `update_execution_final_context`, `merge_dependency_contexts`, `StaleClaimSweeper::sweep`.
- Constructor-per-call pattern in `runtime.get_task` — fresh `Arc<dyn Task>` allocation per dispatch.
- Python task wrapper acquires GIL 5+ times per task invocation; could be 1-2.

**Headline findings**:
- PERF-01 — Per-tenant Postgres pool hardcoded to 2 connections.
- PERF-02 — SQLite pool fixed at size 1; defeats `db_pool_size = 10` config.
- PERF-03 — `runner.execute()` polls DB every 500ms instead of using a notification.
- PERF-04 — N+1 DAL queries in workflow finalization, dependency context merge, sweeper.
- PERF-05 — Reactor in-memory cache uses async RwLock for tight read/write hot path.
- PERF-06 — `EndpointRegistry::send_to_accumulator` takes write lock per message.
- PERF-07 — Reactor persists state via JSON serialization on every fire.
- PERF-08 — `runtime.get_task` invokes constructor closure per call (no instance caching).

**Positive patterns**:
- SQL-derived gauge for `cloacina_active_workflows` (re-seeded every tick); avoids gauge-drift bug class.
- Push-based dispatch via `Dispatcher` + `TaskReadyEvent` (replaces older polling on `task_outbox`).
- Content-hash artifact reuse in compiler service (`find_success_by_hash`); fresh upload with matching hash skips cargo build.
- Atomic DB-row claiming for cron schedules and tasks (`UPDATE ... WHERE claimed_by IS NULL` returning rows-affected).
- Batched DAL queries on the scheduler critical path (`get_pending_tasks_batch`, `get_task_statuses_batch`).
- Bounded reason labels for `cloacina_tasks_total{reason}` enforced by positive test.

**Reference**: full detail in `04-performance.md` (20 findings).

### API Design

**Posture**: Inner contracts are good (CLI noun-verb shape, `ApiError` envelope, plugin-ABI versioning, Universal types); the seams between surfaces are uneven. Three CLI commands ship non-functional.

**Themes**:
- CLI/server contract drift: tenant create body shape wrong; execution list filters silently ignored; list-renderer mismatched.
- Documented CLI flags silently ignored or always-error (`pack --sign`, `events --follow`).
- Auth role model has three orthogonal axes (`is_admin` / `permissions` / `tenant_id`) with no central documentation.
- REST error envelope has three different shapes; CLI's `extract_message` matches none precisely.
- Hardcoded pagination limits in route handlers ignore client requests.
- Public Rust API quick-starts use a `workflow!` macro that doesn't exist.

**Headline findings**:
- API-01 (Critical) — `tenant create` posts wrong body shape; every tenant-create from CLI fails.
- API-02 (Critical) — `execution list` silently ignores `--workflow`/`--status`/`--limit` filters.
- API-03 — `tenant/trigger/execution list` pass wrapping object to array renderer; silent empty output.
- API-04 — Auth role model is three orthogonal axes; matrix not documented anywhere.
- API-05 — `package pack --sign <key>` accepts flag and silently does nothing.
- API-06 — REST error envelope has three different shapes server-side.
- API-07 — `package!()` macro doc says "Six methods"; emits nine; trait docs only two.
- API-08 — Graph health routes merged at router root; breaks `/v1/` prefix invariant.
- API-10 — Hardcoded `LIMIT 100` in `list_triggers`; CLI `--limit` ignored.
- API-11 — Rust prelude advertises `workflow!` macro; users copy-pasting fail to compile.

**Positive patterns**:
- `cloacinactl` noun-verb file structure with consistent verb palette and typed exit codes.
- `ApiError` with `{error, code}` envelope and dedicated constructors (`ApiError::bad_request(code, message)`).
- `#[optional(since = N)]` plugin-ABI versioning correctly applied across methods 4-8.
- `#[serde(deny_unknown_fields)]` on `CloacinaMetadata` with friendly migration hints.
- Universal-types pattern naming communicates intent; backend-agnostic public surface.
- `DefaultRunnerBuilder` with `#[non_exhaustive]` config; `#[must_use]` on the runner.
- Per-tenant route namespace `/v1/tenants/{tenant_id}/...` is unambiguous and audit-friendly.

**Reference**: full detail in `05-api-design.md` (21 findings).

### Operability

**Posture**: Bones of an operable system (three deployables, structured tracing, Prometheus metrics, credential-logging guard, multi-stage Dockerfile, TLS-via-proxy runbook). The shape that breaks operability is asymmetry across the three deployables and a runbook gap for runtime ops.

**Themes**:
- Telemetry asymmetry: only `cloacina-server` installs a Prometheus recorder; compiler and daemon emit nothing.
- Operator runbook gap: no `cancel`/`pause`/`resume`/`rerun`/`rebuild`/`drain` CLI verbs.
- Tenant-id never reaches the request span; log filtering for incident triage requires payload string matching.
- `cloacina_active_tasks` gauge uses the antipattern T-0534 already fixed for `cloacina_active_workflows`.
- Compiler runs `cargo build` with no sandbox, no timeout, no rlimit.
- Reconciler retries failed packages forever with no quarantine or backoff.
- Body limits, rate limits, log retention are missing or hardcoded.

**Headline findings**:
- OPS-01 — `cloacina_active_tasks` gauge uses naked increment/decrement (T-0534 antipattern).
- OPS-02 — No `cancel`/`pause`/`rebuild`/`drain` CLI verbs; engine has primitives.
- OPS-03 — Server's `request_id` span has no `tenant_id`/`key_name`/downstream context fields.
- OPS-04 — `cloacina-compiler` emits zero Prometheus metrics; build queue invisible.
- OPS-05 — `cloacinactl daemon` emits zero Prometheus metrics; engine instrumentation dies.
- OPS-07 — Compiler runs `cargo build` on user-uploaded source with zero sandboxing.
- OPS-09 — Pool sizing hardcoded; SQLite override to 1, per-tenant Postgres to 2; ignore config.
- OPS-12 — `tenant_id` never reaches request span; log filtering requires payload match.
- OPS-19 — No HTTP rate limiter despite project memory citing it as a soak gap.

**Positive patterns**:
- Credential-logging guard (`scripts/check_credential_logging.py`) enforced as `angreal lint:credential-logging`.
- `/metrics` validated by `promtool check metrics` in CI via `angreal test:metrics-format`.
- SQL-derived `cloacina_active_workflows` gauge re-seeded every scheduler tick (T-0534 fix pattern).
- Server `/ready` checks DB connectivity AND crashed-graph status with structured 503 payload.
- Graceful shutdown ordering on the server is correct (graph scheduler drained first, runner second).
- Daemon health pulse + Unix socket for single-host local observability.

**Reference**: full detail in `06-operability.md` (21 findings).

### Security

**Posture**: Cryptographic primitives are sound; validation routines for primary identifiers exist and are well-tested. The surface area between "we have a verifier" and "the verifier runs on every dangerous operation" is wide open. System is "trustworthy operators inside a trusted network."

**Themes**:
- Plugin loading is in-process arbitrary code execution; signature verification is off by default and unactivatable from CLI.
- Multi-tenant boundary breach: triggers and execute_workflow use admin DB; graph health leaks all tenants.
- Compiler `cargo build` on attacker source has no sandbox/timeout/rlimit/network restriction.
- Audit functions defined but never called from production code paths.
- Auth model has dual axes with no last-admin guard, no self-revocation guard, key-name-based migration promotion.
- HTTP and WebSocket have no rate limiter; 100MB body limit applies to all routes globally.

**Headline findings**:
- SEC-01 (Critical) — Plugin loading executes arbitrary native code; signature verification off by default; daemon path never verifies.
- SEC-02 — `triggers.rs` uses admin DB pool regardless of tenant; cross-tenant data exposure.
- SEC-03 — `execute_workflow` runs every tenant's workflow on shared admin runner.
- SEC-04 — `verification_org_id` unreachable from CLI; `--require-signatures` always fails-closed.
- SEC-05 — Graph health endpoints leak every tenant's accumulator and reactor names.
- SEC-06 (Critical) — Compiler runs `cargo build` on attacker source with no isolation.
- SEC-13 — Package signatures are not org-scoped at storage time.
- SEC-14 — Tenant deletion drops schema but leaves API keys, runners, registries, cached pools alive.
- SEC-15 — `cargo audit` runs with `continue-on-error: true`; `cargo deny` not configured.

**Positive patterns**:
- PostgreSQL identifier validation has dedicated module with SQL-injection-attempt fixtures.
- Bootstrap admin key is mode-0600 and never logged (only file path logged).
- `scripts/check_credential_logging.py` lints log/print macros for sensitive identifiers.
- WebSocket auth ticket store: single-use, 60s TTL, bounded capacity, proper unit tests.
- Ed25519 signing with AES-256-GCM at-rest encryption; modern primitives correctly applied.
- Loud `RUNNING_WITHOUT_TLS` warning at server startup; correct operator-facing message.

**Reference**: full detail in `07-security.md` (18 findings).

## Cross-Cutting Concerns

The cross-cutting agent identified **seven structural concerns** that span multiple lenses. Each cluster represents one root cause with multiple symptoms; addressing the cluster resolves a fan-out of 5-12 findings simultaneously.

### Multi-tenant boundary is incomplete

Storage scopes by tenant; runner, reconciler, observability, and lifecycle do not. Twelve findings, four lenses.
- **Lenses affected**: Correctness, Evolvability, Operability, Security, API Design.
- **Findings**: COR-01, EVO-04, EVO-15, OPS-03, OPS-12, OPS-16, SEC-02, SEC-03, SEC-05, SEC-14, SEC-17, API-04.
- **Severity-adjusted note**: EVO-04 upgrades from Major to **Critical** when read alongside SEC-03 — the same gap is a "config issue" through evolvability but a "tenant boundary breach" through security. Release-blocker for any multi-tenant-claimed deployment.

### Plugin-load trust gate is unwired AND build path is unsandboxed

The defense's primitives are correct; the defense is not turned on, cannot be turned on from the documented config surface, and the only tests of it are `#[ignore]`-gated. Twelve findings, three lenses.
- **Lenses affected**: Correctness, Operability, Security.
- **Findings**: COR-04, COR-05, COR-15, COR-17, OPS-07, OPS-14, SEC-01, SEC-04, SEC-06, SEC-07, SEC-13, SEC-18.
- **Severity-adjusted note**: OPS-07 upgrades from Major to **Critical** given SEC-04 (the only intended defense is unactivatable). COR-15 upgrades from Major to **Critical** — the only tests of the only defense for the only RCE path don't run by default.

### I-0102 macro decomposition is the next refactor target

The unified shell macro `cloacina::package!()` is a 560-line monolith that emits four duplicate `OnceLock<Runtime>` blocks and is the single high-cost surface for plugin-ABI evolution. Twelve findings, five lenses.
- **Lenses affected**: Legibility, Correctness, Evolvability, Performance, API Design.
- **Findings**: LEG-05, LEG-06, LEG-14, LEG-20, COR-04, EVO-01, EVO-13, EVO-17, API-07, API-14, plus PERF cdylib-runtime cross-note.

### Doc/example/macro-doc drift — newcomer's first hour fails

Refactors landed cleanly in code; doc comments, examples, module names, and rustdoc HTML lag by 1-2 cleanup waves. Seventeen findings, three lenses.
- **Lenses affected**: Legibility, API Design, Evolvability.
- **Findings**: LEG-01, LEG-02, LEG-03, LEG-04, LEG-07, LEG-08, LEG-09, LEG-15, LEG-16, LEG-17, LEG-18, LEG-19, EVO-11, EVO-20, API-07, API-11, API-15.
- **Severity-adjusted note**: LEG-04 / API-11 upgrade from Major to **Critical** when read together — README's first code sample and engine crate's top-level rustdoc walk users through a macro that doesn't exist.

### Telemetry asymmetry across the three deployables

The metric primitives, label vocabulary, and validation pipeline are excellent — inside `cloacina-server`. Neither `cloacina-compiler` nor `cloacinactl daemon` install a Prometheus recorder. Eleven findings, three lenses.
- **Lenses affected**: Operability, Performance, Correctness.
- **Findings**: OPS-01, OPS-04, OPS-05, OPS-06, OPS-11, OPS-15, OPS-16, COR-11, COR-13, PERF-13, PERF-19, PERF-20.

### Test architecture: happy paths first; partial-failure invisible

Zero integration tests for: double-dispatch race, FFI panic propagation, DST cron, `mark_failed` returning false, reactor `WhenAll` after partial fire, CLI/server contract. Six signing trust-chain tests `#[ignore]`-gated. Eight findings, four lenses.
- **Lenses affected**: Correctness, Evolvability, Security, Operability.
- **Findings**: COR-09, COR-13, COR-15, COR-17, EVO-07, API-01, API-02, API-03, SEC-01, SEC-04, OPS-07.

### CLI/server contract has Critical-severity bugs that shipped untested

Three commands non-functional, two flags silently ignored, error envelope inconsistent. Ten findings, three lenses.
- **Lenses affected**: API Design, Operability, Correctness.
- **Findings**: API-01, API-02, API-03, API-05, API-06, API-08, API-10, API-17, OPS-02, COR-09.
- **Severity-adjusted note**: API-03 upgrades from Major to **Critical** when grouped with API-01/API-02 — three CLI list commands silently emit empty output as a cluster.

### DAL surface symmetric across backends, asymmetric across migrations and pool sizing

The unified DAL chose runtime-dispatch over `MultiConnection`, producing 218 paired functions. Migrations diverge; pool sizing is silently hardcoded. Nine findings, four lenses.
- **Lenses affected**: Evolvability, Performance, Operability, Correctness.
- **Findings**: EVO-02, EVO-09, COR-07, PERF-01, PERF-02, PERF-04, PERF-11, PERF-17, OPS-09.

### Async runtime and lock contention in the reactor / CG hot path

The reactor / CG hot path uses `tokio::sync::RwLock` and `tokio::sync::Mutex` for coordination patterns where briefer locks (`parking_lot`) would suffice. JSON for internal-only persistence. Seven findings, three lenses.
- **Lenses affected**: Performance, Correctness, API Design.
- **Findings**: PERF-05, PERF-06, PERF-07, PERF-15, PERF-16, COR-03, API-15.

### Auth model is dual-axis and undocumented

Three orthogonal axes (`is_admin` / `permissions` / `tenant_id`) with four near-identical helpers plus open-coded `auth.is_admin` checks. Eight findings, three lenses.
- **Lenses affected**: API Design, Security, Operability.
- **Findings**: API-04, OPS-03, OPS-12, OPS-20, SEC-09, SEC-10, COR-12, SEC-08.

### Async lifecycle and Drop semantics — graceful shutdown is per-binary-different

Server bounds shutdown to 30s; daemon races to second-SIGINT force-exit; compiler does not bound running cargo. Four findings, three lenses.
- **Lenses affected**: Operability, Correctness, Evolvability.
- **Findings**: OPS-10, EVO-18, COR-02, COR-08.

### Macro-emitted code and inventory entries split across two crates with leaky boundaries

The leaf-crate refactor moved `TaskEntry`, `ReactorEntry`, `TriggerEntry` to `cloacina-workflow-plugin`, but `WorkflowEntry` and `StreamBackendEntry` could not move because their constructor return types live in the engine. Six findings, two lenses.
- **Lenses affected**: Legibility, Evolvability.
- **Findings**: LEG-14, EVO-01, EVO-13, EVO-16, EVO-17, EVO-19.

## Severity Adjustments

Applied by the cross-cutting agent based on multi-lens reading.

| ID | Original | Adjusted | Rationale |
|---|---|---|---|
| EVO-04 | Major | **Critical** | Read alongside SEC-03 — tenant boundary breach in default config. Release-blocker for multi-tenant deployments. |
| OPS-07 | Major | **Critical** | The only deployable defense for plugin RCE; signature verification (SEC-01/SEC-04) is unactivatable from CLI. |
| COR-15 | Major | **Critical** | Six signing tests `#[ignore]`-gated; the only tests of the only defense for the only RCE path don't run by default. |
| LEG-04 / API-11 | Major | **Critical** | README first code sample and engine crate top-level rustdoc walk through a macro that doesn't exist. New user's first 30 minutes fail. |
| API-01 | Critical | Critical (held) | Stays Critical — published CLI command non-functional since release. |
| API-02 | Critical | Critical (held) | Filters silently ignored. |
| API-03 | Major | **Critical** | Three CLI list commands silently emit empty output as a cluster. CLI-regression that ships green. |
| OPS-01 | Major | Major (held) | Active-tasks gauge leak — same shape as already-shipped T-0534 fix. |
| LEG-06 | Major | Minor | Dead-code cleanup paired with LEG-20; cluster cost is real but cluster recommendation captures it. |
| EVO-01 | Major | Major (held) | 560-line monolithic macro is real evolvability cliff but cluster recommendation captures it. |
| SEC-01 | Critical | Critical (held) | Default deployment ships in-process RCE path. |
| SEC-04 | Major | **Critical** | Intended defense for SEC-01 unactivatable from documented config surface. |
| SEC-06 | Critical | Critical (held) | Build-side RCE on attacker source. |
| OPS-04, OPS-05 | Major | Major (held) | Asymmetric metric coverage; cluster recommendation captures it. |
| PERF-01, PERF-02 | Major | Major (held) | Throughput cliffs but not security/correctness. |
| COR-04 | Major | Major (held) | FFI panic vector mitigated by host catching some panics. |
| COR-09 | Major | Major (held) | Atomic claim does protect today; double-dispatch test still important gap. |

## Top-Level Risks

- **Multi-tenant deployments are not safe today.** The runner executes every tenant's workflow in the admin schema; triggers and graph health endpoints leak across tenants; tenant deletion is incomplete. Any deployment claiming multi-tenant isolation is misconfigured by construction. Recommend disabling multi-tenant routes (or marking them experimental) until per-tenant runner caching lands.
- **A single malicious `.cloacina` upload is full host compromise.** Default config has no signature verification; the build path runs `cargo build` with no isolation. Any deployment with the upload endpoint reachable by untrusted principals is one upload away from RCE-as-host-UID with full DB credentials. Recommend network-isolating the compiler host, enforcing signature verification end-to-end, and treating the `.cloacina` upload endpoint as admin-only until sandboxing lands.
- **CLI is the documented operator surface and three commands don't work.** Operators relying on `cloacinactl tenant create`, `execution list`, or any `list` verb (tenants/triggers/executions) experience either failures or silent empty output. The unverified CLI/server seam is also a regression vector — without integration tests, future PRs can introduce more drift.
- **Compiler and daemon are operationally blind.** Self-hosted operators monitoring a SQLite-backed daemon, or a build-worker pool, must DB-query for everything. The build queue grows unobserved; reconciler failures stack invisibly; `cloacina_active_workflows` never reaches a Prometheus scraper.
- **Cleanup-task convention is missing.** Every initiative the team ships requires ~5 follow-up cleanup tasks discovered piecemeal. The current cadence (T-0509 finishing I-0096; T-0549 with phases 1/2a/2b/2c/2d) is healthy pre-1.0 but unsustainable post-1.0. Without an explicit "every initiative ships with N cleanup placeholders" convention, future architectural moves will keep accumulating doc lag, dead code, and stale module names.
- **Test architecture is biased toward happy paths.** Failures don't surface as test failures, they surface as production incidents. Three Critical CLI/server findings shipped without any test catching them. Without investment in CLI contract tests, double-dispatch race tests, FFI-panic tests, and un-gating the signing tests, similar regressions will keep landing.

## Appendix A: System Overview

Cloacina is a Rust workflow orchestration system with first-class Python bindings, packaged as a Cargo workspace under `crates/`. Workspace version 0.5.1 on branch `i-0102-fidius-and-plugin-shell`. 482 source files indexed.

### Crates and binaries

The workspace has 12 members organized into four families:
- **Authoring leaf crates** (`cloacina-workflow`, `cloacina-computation-graph`, `cloacina-workflow-plugin`) — minimal-dependency surfaces a packaged cdylib can compile against; deliberately diesel/pyo3/kafka-free.
- **Engine** (`cloacina`) — runtime, DAL, executor, scheduler, registries, computation graph, packaging, security. ~63K LOC.
- **Binaries** (`cloacina-server`, `cloacina-compiler`, `cloacinactl`) — Axum HTTP+WS API service, DB-queue-driven build worker, CLI noun-verb dispatcher.
- **Ancillaries** (`cloacina-macros`, `cloacina-build`, `cloacina-python`, `cloacina-testing`).

### Modes of operation

- **Fully embedded**: `DefaultRunner::new(...)` linked into your binary; tasks compiled in via `inventory::submit!` from macros; SQLite or Postgres.
- **Daemon mode**: `cloacinactl daemon start` runs SQLite-backed long-lived process that watches a directory for `.cloacina` packages.
- **Server mode**: `cloacina-server` runs Postgres-backed HTTP service with API key auth, multipart workflow upload, multi-tenant support, WebSocket endpoints.
- **Compiler mode**: `cloacina-compiler` is a stateless build worker — multiple instances coordinate via DB-row claiming.
- **Python authoring**: `cloaca` PyO3 module with `@task`/`@workflow`/`@reactor` decorators.

### Key concepts

Per CLOACI-S-0011 (the published nomenclature spec):
- **Workflow**: DAG of `#[task]`-decorated async functions; quantum is the task. Persists every state transition.
- **Computation graph**: Parallel execution model where the quantum is the graph traversal; once triggered, all nodes run in-process with in-memory channels.
- **Reactor**: Specialized trigger that consumes accumulator boundary events and fires a downstream computation graph.
- **Accumulator**: The reactor's stream-input adapter (passthrough, polling, batch, state, stream-backed/Kafka).
- **Trigger**: Schedules workflow execution by cron or custom-poll.
- **Package**: bzip2-compressed `.cloacina` source archive that the compiler builds into a cdylib via a fidius-based plugin ABI.

### Architecture (engine internals)

Runtime owns seven scoped registries (tasks, workflows, triggers, computation_graphs, triggerless_graphs, reactors, stream_backends). Background services managed by `ServiceManager`: `SchedulerLoop`, `StaleClaimSweeper`, `CronRecovery`, unified `Scheduler`, `RegistryReconciler`, `ComputationGraphScheduler`. The unified DAL has per-table accessors (`dal.task_execution()`, `dal.workflow_packages()`, etc.) with paired Postgres+SQLite implementations dispatching on URL scheme. Multi-tenancy uses Postgres schema isolation or per-file SQLite databases.

### Packaging and FFI

Packages export a fidius-based `CloacinaPlugin` v2 trait (9 methods, methods 4-8 `#[optional(since=2)]`). Wire format: debug=JSON, release=bincode. The unified `cloacina::package!()` shell macro emits one fidius plugin per cdylib for any combination of declared primitives. Host loads via `fidius_host::loader::load_library`. The compiler service polls `workflow_packages` for `pending` rows, runs cargo, writes cdylib bytes back; content-hash artifact reuse skips redundant builds.

### Configuration surface

- Server: `--bind`, `--database-url`, `--bootstrap-key`, `--require-signatures`, `--reconcile-interval-s`, `--home`.
- Daemon: `--watch-dir`, `--poll-interval`, plus `~/.cloacina/config.toml`.
- Compiler: `--bind`, `--database-url`, `--poll-interval-ms`, `--heartbeat-interval-s`, `--cargo-flag`, `--cargo-target-dir`.
- Telemetry: `OTEL_EXPORTER_OTLP_ENDPOINT` (server only, gated `feature = "telemetry"`).

### Metrics and events

Eight metrics emitted by the engine: `cloacina_workflows_total{status, reason}`, `cloacina_tasks_total{status, reason}`, `cloacina_api_requests_total`, `cloacina_api_request_duration_seconds`, `cloacina_workflow_duration_seconds`, `cloacina_task_duration_seconds`, `cloacina_active_workflows`, `cloacina_active_tasks`. Bounded reasons: `task_error`, `timeout`, `validation_failed`, `infrastructure`, `task_not_found`, `claim_lost`, `unknown`. Events written to `execution_events` and `recovery_events` tables. Logs structured via tracing + tracing-subscriber, daily-rotated JSON files.

### Project conventions worth respecting

- One PR per Metis initiative (per project memory `feedback_pr_is_initiative.md`).
- angreal is the canonical dev/CI surface (`feedback_use_angreal_testing.md`).
- Postgres + SQLite multi-backend — both compiled in by default; runtime URL-scheme dispatch.
- SQLite migrations: avoid DROP+CREATE; prefer ADD COLUMN + CREATE INDEX (`feedback_sqlite_migration_recreate.md`).
- CLOACI-S-0011 nomenclature is authoritative; banned phrases listed in spec.
- Python is core, not optional — `cloacina-python` extracted to its own crate so pyo3 doesn't leak into compiler/daemon.

For the full system overview, see `00-system-overview.md` (754 lines).

## Appendix B: Methodology

This review followed a five-phase orchestration:

1. **Discovery (Phase 1)**: One agent built the system overview baseline (`00-system-overview.md`) — repository structure, entrypoints, architecture, primary workflows, dependency graph, open questions. Output is the authoritative starting point all subsequent agents read first.
2. **Foundational lenses (Phase 2)**: Four agents in parallel produced the lens reviews most internal to the codebase — Legibility, Correctness, Evolvability, Performance. Each has its own findings file.
3. **External lenses (Phase 3)**: Three agents in parallel produced the lens reviews most consumer-facing — API Design, Operability, Security.
4. **Cross-Cutting (Phase 4)**: One agent integrated all seven lens files, identified clusters and root causes, applied severity adjustments, and prepared the handoff to synthesis (`08-cross-cutting.md`).
5. **Synthesis (Phase 5)**: This phase. One agent produced the report (`09-report.md`) and recommendations (`10-recommendations.md`) anchored on the cross-cutting agent's clusters and root causes rather than per-finding.

Severity taxonomy:
- **Critical**: Security-critical, data-loss, or release-blocker issue; default config is unsafe.
- **Major**: Significant defect; ships in default config and requires action; not immediate safety risk.
- **Minor**: Real but bounded; should be fixed but not release-blocking.
- **Observation**: Notable pattern worth recording; may not require action today.

The lens files (`01-legibility.md` through `07-security.md`) remain authoritative for individual finding detail, evidence citations, and per-finding suggested resolutions. This report summarizes; the recommendations document acts on the summary. ~138 findings total before severity adjustment; 11 Critical / 50 Major after adjustment.
