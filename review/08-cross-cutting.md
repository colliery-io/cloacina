# Cross-Cutting Analysis

## Summary

The seven lenses converge on a single, recognizable picture of cloacina: an architecturally sound system in the middle of a large, deliberate refactor (CLOACI-I-0102) whose engine internals and authoring crates are getting cleaner with every cycle, while the **outer perimeter — the CLI/server contract, multi-tenant execution path, plugin-load trust gate, packaging RCE surface, and observability of the non-server binaries — has not been audited end-to-end** since the engine started moving. Findings cluster tightly: where the system has thought about a problem, it has thought well (atomic claim, universal types, `#[optional(since=N)]` ABI evolution, credential-logging lint, schema validation, request-id middleware). Where it hasn't thought about a problem yet, it is uniformly silent — silent search-path fallback, silent CLI/server contract drift, silent best-effort persistence, silent plugin-load trust gate, silent test skip on missing dylibs, silent migration-promotion on key name match.

A few patterns recur across nearly every lens. The first is **multi-tenancy is a half-built abstraction**: storage-side scoping exists (`TenantDatabaseCache`, schema search_path, per-tenant Postgres pool), but the **execution side and the read-side audit/observability/health surface are still admin-schema-bound**. This is COR-01, EVO-04, EVO-15, OPS-12, SEC-02, SEC-03, SEC-05, SEC-14 — eight findings, four lenses, one structural gap. The second is **the I-0102 plugin-shell migration left footprints across several lenses**: dead code (LEG-06), stale doc warnings (LEG-20), a 560-line monolithic macro (EVO-01, LEG-14, API-14), missing helpers (LEG-14), the four duplicate `OnceLock<Runtime>` blocks that COR-04 flags as a panic vector and PERF flags as wasted thread-overhead. The third is **the test architecture is happy-path-first**: 138 findings include a striking number of the same shape — `let _ =` swallowing failure, tests sleeping on absolute durations, signing tests `#[ignore]`-gated, fidius tests silently `return`ing on missing fixtures, no double-dispatch race test, no FFI-panic test, no DST cron test, no end-to-end CLI/server contract test. Failure paths are simply not exercised.

The synthesis agent should not group these findings by lens. The natural shape of the recommendations is **by structural concern**: tenant boundary, plugin trust boundary, observability symmetry across the three deployables, the I-0102 cleanup surface, and the test-architecture bias. Each of those structural concerns is multi-lens, and addressing each of them resolves a fan-out of 5–10 findings simultaneously.

## Cross-Lens Findings

### Cluster: Multi-tenant isolation is half-built — storage scopes, execution doesn't, observability doesn't

**Lenses affected**: Correctness, Evolvability, Operability, Security, API Design

**Related findings**: COR-01, EVO-04, EVO-15, OPS-03, OPS-12, OPS-16, SEC-02, SEC-03, SEC-05, SEC-14, SEC-17, API-04

**Relationship**: Common root — the tenant abstraction was added to the storage layer (`TenantDatabaseCache`, schema search_path, per-tenant pools) but the runner, scheduler, executor, reconciler, and several read paths still use the admin/shared `state.database`. Each finding is the same architectural gap manifest in a different surface.

**Adjusted severity**: COR-01 stays Critical; EVO-04 should be **upgraded** from Major to Critical when read alongside SEC-03 — the same gap is a "config issue" through the evolvability lens but a "tenant boundary breach" through the security lens. The realistic interpretation is that the multi-tenant execute path is **load-bearing for the security model** and currently broken, which makes EVO-04 a release blocker for any multi-tenant-claimed deployment.

**Description**

The system pitches multi-tenancy in three places (system overview §6, the README, the `--tenant` CLI flag). The implementation has:

- A working per-tenant `Database` cache for read paths — but only some read paths use it (`executions::list_executions` does; `triggers.rs` does not — SEC-02).
- A single shared `DefaultRunner` constructed at server startup with the admin schema — used unconditionally by `execute_workflow` (SEC-03, EVO-04).
- A `RegistryReconciler` bound to the admin runner's DB — packages uploaded to tenant schemas are never reconciled (SEC-03).
- Graph health endpoints with no tenant scoping at all (SEC-05) — every authenticated key sees every tenant's reactor and accumulator names.
- An auth model with two undocumented orthogonal axes (`is_admin` and `permissions`) that produces four near-identical helper methods plus open-coded checks (API-04).
- A request-tracing span that doesn't carry `tenant_id` (OPS-03, OPS-12), so log filtering for incident triage requires payload string matching across non-tenant-scoped log lines.
- A tenant-deletion path that drops the schema but leaves API keys, cached `Database` instances (SEC-17), reactor instances, and running workflow executions live (SEC-14).
- The silent search_path failure (COR-01) — in the rare case that a `SET search_path` interaction fails on connection acquisition, the connection silently falls through to `public`. After tenant deletion (SEC-14) this becomes a deterministic data-leak path: the tenant schema is gone, every connection acquisition fails the SET, and queries hit the admin schema instead.

The finding density tells the story: multi-tenancy was implemented as a storage-layer feature and never propagated to the execution-layer, observability-layer, or lifecycle-layer. The execution-side gap (EVO-04 / SEC-03) is the keystone — every other tenant finding either depends on or is amplified by the fact that the runner is shared.

**Implication for synthesis**

Treat all twelve findings as **one recommendation**: "Complete the multi-tenant abstraction across runner, reconciler, observability, and lifecycle." The synthesis report should not list these separately; it should present a single section ("Multi-tenant boundary is incomplete") with the eight symptoms and one structural prescription. EVO-04's two suggested resolutions (per-tenant runner cache vs. database-override on execute) are the lever.

---

### Cluster: Plugin loading is in-process arbitrary code execution with the trust gate unwired

**Lenses affected**: Correctness, Operability, Security

**Related findings**: COR-04, COR-05, COR-15, COR-17, OPS-07, OPS-14, SEC-01, SEC-04, SEC-06, SEC-07, SEC-13, SEC-18

**Relationship**: Same root cause — `cloacina-compiler` and the reconciler form an end-to-end chain that takes attacker-controlled source bytes and produces in-process arbitrary native code. Each lens names a different point on the chain.

**Adjusted severity**: SEC-01 and SEC-06 stay Critical. OPS-07 should be **upgraded** from Major to Critical given SEC-04 (the only intended defense, signature verification, is provably unactivatable from the supplied CLI). COR-15 (signing tests `#[ignore]`-gated) is **upgraded** from Major to Critical when read alongside SEC-01 — these are the only tests of the only defense for the only RCE path, and they don't run by default.

**Description**

The chain:

1. **Upload**: any tenant `write` key can post a `.cloacina` archive to `POST /v1/tenants/{t}/workflows`. There is no signature verification because (a) the server defaults to `require_signatures=false` (SEC-01), and (b) even with `require_signatures=true`, `verification_org_id` cannot be set from the CLI/config — the verification fails closed and rejects all uploads (SEC-04). The lower-level `register_workflow` path used by `cloacinactl daemon` never verifies (SEC-01).
2. **Compile**: `cloacina-compiler` runs `cargo build` on the attacker-supplied source with no timeout, no rlimit, no network restriction, no namespace, no jail (SEC-06, OPS-07). `build.rs` is arbitrary code; cargo can fetch dependencies from arbitrary git URLs by default. The compiler service's `DATABASE_URL` is exfiltratable via `std::env::var`. The compiler shutdown does not bound running cargo subprocesses (OPS-10) — SIGTERM hangs on misbehaving builds.
3. **Load**: `RegistryReconciler.load_package` writes the cdylib bytes to a temp file, dlopens via `fidius_host::loader::load_library`, leaks the tempdir with `mem::forget` (COR-05, SEC-18). The cdylib runs as the host UID with full filesystem and DB access. Audit functions (`audit::log_package_load_*`) exist but are never called (SEC-01).
4. **Execute**: cdylibs initialize per-shape `OnceLock<Runtime>` blocks with `.expect()` panics (COR-04) — a build that exhausts resource limits at runtime panics across the FFI boundary, poisons the OnceLock, and the package is permanently broken until restart. There is no tracing-context propagation across the FFI boundary (OPS-14) — packaged-task logs are detached from runner logs.
5. **Test the defense**: signing trust-chain tests are six `todo!()` stubs marked `#[ignore = "Requires database connection"]` (COR-15); a fidius validation test silently `return`s if the example dylib isn't built. The end-to-end "sign → upload → verify → load" path has no green-CI signal.

Two ancillary findings amplify: COR-17 (Python loader's stdlib deny-list is shallow — even if it weren't, Python tasks are arbitrary code anyway), and SEC-13 (signatures are not org-scoped at storage time).

**Implication for synthesis**

Treat as **two recommendations** that share an initiative-level frame:
- (a) "Wire the existing signature verification end-to-end and make it default-on for non-trivial deployments" — covers SEC-01, SEC-04, SEC-13, COR-15.
- (b) "Sandbox the build path" — covers SEC-06, OPS-07, OPS-10, with COR-05/SEC-18 as the cleanup tail.

Together these are the project's single largest pre-1.0 security commitment. They should be the first or second item in 09-report.md / 10-recommendations.md.

---

### Cluster: I-0102 plugin-shell migration tail — same code seen four ways

**Lenses affected**: Legibility, Correctness, Evolvability, Performance, API Design

**Related findings**: LEG-05, LEG-06, LEG-14, LEG-20, COR-04, EVO-01, EVO-10, EVO-13, EVO-17, PERF-08 (constructor-per-call has same shape), API-07, API-14

**Relationship**: All point at the same surface — `crates/cloacina-workflow-plugin/src/lib.rs:110-672` plus `crates/cloacina-macros/src/workflow_attr.rs:277-293`. The lenses describe different facets of one monolithic macro mid-migration.

**Adjusted severity**: Most stay Major individually; the cluster aggregates to a structural concern but no single finding upgrades. LEG-06 (the dead-code `let _ = packaged_registration` path) and LEG-20 (the unconditional coexistence warning that no longer applies) are essentially the same finding viewed from opposite directions, and both should resolve in a single cleanup commit.

**Description**

The unified shell macro `cloacina::package!()` is the centerpiece of CLOACI-I-0102. The lenses see:

- **Legibility (LEG-05, LEG-14, LEG-20)**: the doc says "Six methods" while emitting nine; four near-identical `OnceLock<Runtime>` blocks; an unconditional coexistence warning that contradicts the actual T-C state.
- **Correctness (COR-04)**: the `.expect()` on tokio Runtime init aborts the host on resource exhaustion; OnceLock poisoning persists the failure across restarts.
- **Evolvability (EVO-01, EVO-13, EVO-17)**: the macro is a 560-line monolith that doesn't compose; adding a 10th method costs another 30-60 lines in the same body; `TriggerlessGraphRegistration` and `ComputationGraphRegistration` are duplicated surfaces; `WorkflowEntry`'s constructor return type still lives in the engine crate, so the leaf-crate refactor isn't done.
- **Performance**: PERF-08 (constructor-per-call) shows the same shape elsewhere — `runtime.get_task` invokes `ctor()` per dispatched task; in the macro, four separate Runtime initializations duplicate the same Builder body (LEG-14 from the legibility lens, EVO-01 and PERF cross-note from the evolvability lens).
- **API Design (API-07, API-14)**: package! is all-or-nothing — there's no `cloacina::package!(only = [tasks])`; the macro emits all nine methods even for a tasks-only crate. Methods 4-8 are `#[optional(since = 2)]` so unused emissions return empty vectors, but the cdylib still pays for 4 separate tokio runtimes.
- **Migration debt (LEG-06)**: `workflow_attr.rs` calls `generate_packaged_registration` and immediately discards the result (`let _ = packaged_registration;`) — 130 lines of dead code emission per `#[workflow]` invocation.

**Implication for synthesis**

One section: "Finalize and decompose the package! macro." The actions are:
1. Delete the dead `generate_packaged_registration` path (LEG-06).
2. Rewrite the doc to enumerate all 9 methods (LEG-05, API-07).
3. Replace the unconditional coexistence warning with the actual T-C state (LEG-20).
4. Decompose into per-method emitter macros + a single shared `cdylib_runtime!()` helper (EVO-01, LEG-14, API-14, PERF-cross).
5. Replace `.expect()` with `OnceLock<Result<Runtime, _>>` (COR-04).
6. Plan the next leaf-crate phase: relocate `Workflow` constructor type to `cloacina-workflow` (EVO-17); unify `TriggerlessGraphRegistration` with `ComputationGraphRegistration` (EVO-13).

This is a 1-2 week initiative that resolves ~12 findings.

---

### Cluster: Doc/example/macro-doc drift — newcomer's first hour fails

**Lenses affected**: Legibility, API Design, Evolvability

**Related findings**: LEG-01, LEG-02, LEG-03, LEG-04, LEG-07, LEG-08, LEG-09, LEG-15, LEG-16, LEG-17, LEG-18, LEG-19, EVO-11, EVO-20, API-07, API-11, API-15

**Relationship**: All same shape — refactors landed, terminology / examples / module names did not follow. CLOACI-S-0011 banned terminology (LEG-01); T-0509 deleted process-globals (LEG-03, LEG-09, LEG-19); T-0506 wired inventory (LEG-07); T-0544 fanned out reactors but field names stayed graph-shaped (LEG-02); the actual `#[workflow]` attribute macro is two years old but the README and engine docs still walk users through a fictional `workflow!` declarative macro (LEG-04, API-11). EVO-11 (workflow registry trait doc claims responsibilities the trait no longer has) and EVO-20 (FFI/wire-format evolution policy undocumented) are the same shape at the trait level. API-15 (reactor command WS protocol undocumented) is the same shape at the wire-format level.

**Adjusted severity**: LEG-04/API-11 should be **upgraded** from Major to Critical when read together — this is the README's first code sample and the engine crate's top-level rustdoc. A new user's first 30 minutes of cloacina is reading docs that don't compile.

**Description**

The docs lag the code by 1-2 cleanup waves consistently. The examples in the README and `lib.rs` quick-start use `workflow! { name: …, tasks: [...] }` syntax that has never existed; the prelude doc advertises that macro; `executor.execute(...)` typos a variable that should be `runner` three lines up. Three production source comments still say "Reactive Scheduler" after CLOACI-S-0011 explicitly banned the term — the rustdoc HTML in `docs/public/api/` has shipped the banned phrases publicly. `pub mod global_registry` lives at the crate boundary, with module-level doc explicitly stating "the registry was deleted" — the page exists in published rustdoc. `#[reactor]` macro doc says graphs bind "by type path"; they bind by name string (the type-path form was removed in I-0102). The `cloacina::package!()` macro doc says "Six methods" while emitting nine. The Runtime struct doc says "five namespaces"; it has seven, and the Debug impl reports five.

Each individual finding is small. Collectively they tell newcomers the wrong story about every concept that's been refactored — which, on this branch, is most of them.

**Implication for synthesis**

One sweep recommendation: "Run a documentation regeneration after the I-0102 cleanup wave settles." Most are 1-3 line edits. The synthesis agent should bundle these into a single "Documentation hygiene sweep" item with a mechanical task list rather than 17 separate items. The notable exception is API-11/LEG-04 (the README quick-start) which should be its own line item with priority — that's the public face of the project.

---

### Cluster: Active-* gauges, persist failures, and log-and-continue idioms — telemetry asymmetry across deployables

**Lenses affected**: Operability, Performance, Correctness

**Related findings**: OPS-01, OPS-04, OPS-05, OPS-06, OPS-11, OPS-15, OPS-16, COR-11, COR-13, PERF-13 (eager Vec allocation in info!), PERF-19, PERF-20

**Relationship**: Common root — observability is engine-shaped, not system-shaped. The engine emits metrics, but only `cloacina-server` installs a recorder; `cloacina-compiler` and `cloacinactl daemon` carry no metrics export despite running long-lived claim-based queues. Inside the engine, `cloacina_active_workflows` was correctly rebuilt as SQL-derived (T-0534 fix) but `cloacina_active_tasks` retains the antipattern (OPS-01). Persistence-failure metrics don't exist (OPS-15). Best-effort `let _ =` and the `release_runner_claim` ordering (COR-08) make partial-failure surfaces invisible.

**Adjusted severity**: OPS-01 stays Major. OPS-04 (compiler emits zero metrics) and OPS-05 (daemon emits zero metrics) should be considered as a pair — the asymmetry is the issue, not either one in isolation.

**Description**

Three concrete patterns:

1. **The non-server binaries are blind.** Every metric the engine emits dies inside daemon/compiler because no Prometheus recorder is installed. Operators monitoring a self-hosted SQLite-backed daemon, or a build worker pool, must DB-query for everything.
2. **Engine-internal gauge antipattern recurs.** `cloacina_active_workflows` got a SQL re-seed fix; `cloacina_active_tasks` did not. Any panic between `increment(1.0)` and `decrement(1.0)` on `thread_task_executor.rs:840-904` leaks the count permanently. The COR-04 cdylib panic vector is one of several reachable paths.
3. **Failures don't surface.** `let _ = persist_*` in CG paths drops crash-recovery state without a metric (OPS-15); accumulator persist failures degrade silently from `Live` toward unknown; reactor JSON serialization or DB write failures pass through (PERF-07 also flags JSON inefficiency); `update_workflow_task_readiness` swallows context-merge failures (COR-11). No `cloacina_*_failures_total` counter for any of these. Reconciler retries failed packages forever with no quarantine (OPS-11). The composite `cloacinactl status` is a serial chain with no aggregation or per-call timeout (OPS-17).

**Implication for synthesis**

Group into three recommendations:
- "Install Prometheus recorders in `cloacina-compiler` and `cloacinactl daemon`" — addresses OPS-04, OPS-05.
- "Re-seed `cloacina_active_tasks` from SQL each tick" — extends T-0534's pattern (OPS-01).
- "Add failure-mode metrics for persist, reconcile, and dispatcher back-pressure" — covers OPS-11, OPS-15, the implicit "no `cloacina_dispatch_no_capacity_total`" gap from the operability failure-mode list.

A metric-asymmetry initiative would unify these.

---

### Cluster: Tests cover happy paths; partial-failure and concurrency are invisible

**Lenses affected**: Correctness, Evolvability, Security, Operability

**Related findings**: COR-09, COR-13, COR-15, COR-17 (less directly), EVO-07 (tests reach into engine internals), API-01 (no contract test), API-03 (no integration test), SEC-01/SEC-04 (no end-to-end signing test), OPS-07 (no sandbox = no test of sandbox)

**Relationship**: Common root — the test suite is structured to confirm the happy path works, with thinner coverage of (a) concurrent contention, (b) FFI failure modes, (c) partial-write states, (d) end-to-end CLI/server contracts, (e) security boundaries. The shape is "no test for the negative case."

**Adjusted severity**: COR-09 (no double-dispatch race test) and COR-15 (signing tests `#[ignore]`-gated) both stay Major individually. As a cluster, the absence of contract tests for the CLI/server seam (API-01 et al.) is the highest-leverage gap — three CLI commands have shipped non-functional because no test exercises the boundary.

**Description**

Specific gaps named across lenses:
- **No double-dispatch race test** (COR-09). The atomic claim is the only thing preventing duplicate execution; no test asserts this.
- **No `mark_failed`/`mark_completed` `false`-return test** (COR-09). The post-T-0474 invariant has no regression.
- **No DST cron tests** despite shipping with `chrono-tz` (COR-09 implicit).
- **No FFI panic test** (COR-04 implicit). We don't know whether fidius converts to `CallError` or aborts the host.
- **No `WhenAll`-after-partial-fire reactor test** (COR-03).
- **No CLI/server contract test** (API-01, API-02, API-03 — three Critical/Major findings, none caught by tests).
- **Stale-claim sweeper tests rely on real wall-clock sleeps** (COR-13).
- **fidius_validation tests silently `return`** when the example dylib isn't built (COR-09 / COR-15 sibling).
- **Signing trust-chain tests are `#[ignore]`-gated** — six tests covering the only RCE defense, skipped in `cargo test` and `angreal test unit` (COR-15).
- **Python retry test doesn't actually retry** (`test_scenario_11_retry_mechanisms`).
- **Tests reach into engine internals** (EVO-07) — `cloacina::computation_graph::accumulator::*`, `EndpointRegistry`, `ManualCommand` — locking abstractions in place. 108 `#[serial]` annotations indicate substantial shared global state.

**Implication for synthesis**

One section: "Test architecture: invest in failure-path, contract, and concurrency coverage." This is a multi-phase initiative; the synthesis report should call out three priorities: (1) CLI/server contract tests for API-01/02/03 immediately, (2) signing trust-chain tests un-gated and run by default (COR-15), (3) double-dispatch and FFI-panic tests as the next correctness wave (COR-04, COR-09).

---

### Cluster: DAL surface is symmetric across backends, asymmetric across migrations and pool sizing

**Lenses affected**: Evolvability, Performance, Operability, Correctness

**Related findings**: EVO-02, EVO-09, COR-07, PERF-01, PERF-02, PERF-04, PERF-11, PERF-17, OPS-09

**Relationship**: Common root — the unified DAL chose runtime-dispatch over `MultiConnection` / type-level dispatch, producing 218 paired Postgres+SQLite functions and 141 `dispatch_backend!` invocations. Adjacent to that, migrations diverge between backends (Postgres has 22, SQLite 19; auth migrations are postgres-only by feature gate, but the directory structure makes the divergence implicit), and connection-pool sizing is hardcoded silently (SQLite to 1, per-tenant Postgres to 2) regardless of caller config.

**Adjusted severity**: EVO-02 stays Major. PERF-01/02 should be considered together with OPS-09 — they're the same finding from two angles (perf says "throughput cliff", ops says "operators cannot tune their way out"). Together they're a release blocker for any non-trivial deployment.

**Description**

The DAL has the right shape but the wrong implementation strategy. Each pattern:

- **Paired-function pattern** (EVO-02): 218 `_postgres`/`_sqlite` functions, 90% identical. Every DAL change is two changes. Adding a third backend is ~110 new functions plus a three-arm dispatcher.
- **Migration drift** (EVO-09): three Postgres-only migrations (`003_standardize_uuid_generation`, `016_create_api_keys`, `019_add_tenant_and_admin_to_api_keys`); SQLite never gets API keys. The `auth = ["postgres"]` Cargo feature is the implicit gate. Migrations 006 and 007 use DROP+CREATE despite project memory explicitly forbidding it (COR-07).
- **Hardcoded pool sizes** (PERF-01, PERF-02, OPS-09): SQLite override to 1; per-tenant Postgres set to 2 with comment "small pool per tenant"; both ignore caller config; no CLI flag to tune. PRAGMA setup runs on every SQLite checkout (PERF-17), adding two extra round-trips per DAL call.
- **N+1 patterns** (PERF-04): scheduler-hot loops do per-task DAL calls in three places where batched queries already exist for the same shape elsewhere.
- **Un-paginated full-table scans** (PERF-11): `get_active_executions` and `get_ready_for_retry` return all rows every scheduler tick.

**Implication for synthesis**

Three recommendations, distinct but related:
- "Adopt diesel `MultiConnection` to halve DAL maintenance cost" (EVO-02 long-term).
- "Make pool sizing configurable across CLI/config; document the trade-offs" (PERF-01, PERF-02, OPS-09 short-term).
- "Resolve migration drift — port API keys to SQLite or document the auth feature scope explicitly" (EVO-09).

Adding a `LIMIT` clause to the unbounded scheduler queries (PERF-11) is a 2-line change.

---

### Cluster: Async runtime and lock contention in the reactor / CG hot path

**Lenses affected**: Performance, Correctness, API Design

**Related findings**: PERF-05, PERF-06, PERF-07, PERF-15, PERF-16, COR-03, API-15

**Relationship**: Common root — the reactor / CG hot path was structured around `tokio::sync::RwLock` and `tokio::sync::Mutex` for coordinating between accumulator producer tasks and the executor consumer task. The locks are held briefly but unnecessarily (no genuine reader-writer parallelism), and the wire format chose JSON for internal-only persistence. COR-03 (Latest-strategy clear/snapshot race) is a correctness consequence of the dual-lock-acquisition pattern; PERF-05/06 are the throughput consequence.

**Adjusted severity**: COR-03 stays Major. PERF-05/06 stay Major individually but as a cluster they amplify each other — every accumulator boundary takes a registry-wide write lock (PERF-06) AND a per-reactor cache write lock AND a dirty-flag write lock; `WhenAll` semantics are then weakened by the dirty-flag clear race (COR-03).

**Description**

The CG runtime is the system's intended fast-throughput path (per system overview §10 "fast-throughput path is the computation graph reactor"). The implementation:

- `EndpointRegistry::send_to_accumulator` takes a write lock per message (PERF-06) — multi-producer Kafka workloads serialize globally on this lock.
- Reactor `cache` and `dirty` are `Arc<tokio::sync::RwLock<...>>` with single-producer/single-consumer access patterns (PERF-05) — `parking_lot::Mutex` would be cheaper.
- The Latest-strategy executor reads cache snapshot, then separately writes `clear_all()` on dirty flags (COR-03) — between those, a boundary may arrive, set dirty, get cleared. `WhenAll` never re-fires for that source until another boundary arrives.
- Reactor persists state via `serde_json::to_vec` of cache + dirty + queue on every fire (PERF-07) — bincode would be faster and the data is internal-only.
- Python task wrapper acquires GIL 5+ times per task invocation (PERF-15) — three pre-spawn_blocking acquisitions could be one.
- Reactor startup gate polls every 100ms instead of awaiting watch::changed() (PERF-16).
- WS protocol for ReactorCommand uses default serde encoding without a `#[serde(tag)]` marker (API-15) — wire format is undocumented; `Vec<u8>` serializes as JSON array of integers, not base64.

**Implication for synthesis**

One section: "Reactor / CG hot path performance and correctness." Sub-actions:
1. Replace `tokio::sync::RwLock` with `parking_lot::Mutex` where access is brief and never holds across `.await` (PERF-05).
2. Restructure `EndpointRegistry::send_to_accumulator` to hold the registry lock only for namespace lookup, not for the send itself (PERF-06).
3. Atomic snapshot+clear pair for Latest-strategy fire (COR-03).
4. Switch reactor persistence to bincode (PERF-07).
5. Document the WS protocol with explicit serde tags and base64 envelope for binary fields (API-15).

This cluster is a correctness fix paired with a performance refactor.

---

### Cluster: Macro-emitted code and inventory entries split across two crates with leaky boundaries

**Lenses affected**: Legibility, Evolvability

**Related findings**: LEG-14, EVO-01, EVO-13, EVO-16, EVO-17, EVO-19

**Relationship**: Common root — the I-0102 leaf-crate refactor moved `TaskEntry`, `ReactorEntry`, `TriggerEntry` from the engine crate to `cloacina-workflow-plugin`, but `WorkflowEntry` (with `fn() -> Workflow` constructor type) and `StreamBackendEntry` couldn't move because their constructor return types live in the engine. The shell macro then walks 6 inventory entry types but skips `StreamBackendEntry` silently — packaged stream backends don't work as a result.

**Description**

EVO-17 names the underlying constraint: `WorkflowEntry`'s constructor returns `Workflow`, defined in the engine crate (1,642 LOC). To compile a `WorkflowEntry`, you need the engine. So the leaf-crate refactor is incomplete — packaged cdylibs that declare a workflow today must use either `WorkflowDescriptorEntry` (metadata only) or pull the engine. EVO-16 names the consequence: `StreamBackendEntry` lives in the engine and the unified shell can't reach it; a packaged Redis stream backend silently doesn't ship. EVO-13 (TriggerlessGraph duplicates ComputationGraph) is the same family — surface duplication that one more refactor wave would consolidate. EVO-19 (`#[task]` macro emits 100+ lines per task) and LEG-14 (the four duplicate Runtime initializations) are the cosmetic surfaces of the same shape.

**Implication for synthesis**

This is the next phase of the leaf-crate refactor. Plan it as a follow-up to the I-0102 cleanup wave (the macro decomposition cluster above). The synthesis report should call this out as "Phase 2 of leaf-crate refactor," not a stand-alone item.

---

### Cluster: API contracts at the CLI/server seam are unverified and partially broken

**Lenses affected**: API Design, Operability, Correctness (test coverage)

**Related findings**: API-01, API-02, API-03, API-05, API-06, API-08, API-10, API-17, OPS-02 (operator-tooling gap), COR-09 (test coverage)

**Relationship**: Common root — the CLI client and the server route handlers were authored separately and the contract is not exercised by any test. Three CLI commands ship non-functional (API-01 tenant create posts wrong body shape; API-02 execution list filters silently ignored; API-03 list commands hand wrapping objects to an array renderer). One CLI flag is documented but does nothing (API-05 `pack --sign`). One CLI flag errors immediately (API-17 `events --follow`). The error envelope has three different shapes server-side and the CLI's `extract_message` matches none of them precisely (API-06). Operators have no `cancel`/`pause`/`rebuild` verbs (OPS-02) for runtime ops the engine already supports.

**Adjusted severity**: API-01 stays Critical (tenant creation is broken); API-02 stays Critical (execution list is non-functional). API-03 should be considered Critical when grouped with the others — three list commands silently emit empty output. The cluster as a whole upgrades from "API design quality" to "release-blocker for the CLI."

**Description**

The CLI is the documented operator surface. Its current state:

- `tenant create my_tenant` fails with 400 because body shape mismatches (API-01).
- `execution list --status Failed --limit 10` returns Pending/Running rows for the entire tenant with no filter applied (API-02).
- `tenant list`, `trigger list`, `execution list` all output empty regardless of actual data (API-03).
- `package pack --sign <key>` accepts the flag and ignores it (API-05).
- `execution events --follow` errors immediately (API-17).
- Error responses shaped three different ways across `auth.rs`, `health_graphs.rs`, and `error.rs::ApiError` (API-06); the CLI matches none of them.
- `triggers.rs` hardcodes `LIMIT 100` regardless of caller request (API-10); CLI doesn't surface a `--limit` flag.
- WS auth has overlapping channels with no error on mixed credentials (API-09).
- Graph health routes are merged at router root, breaking the `/v1/` prefix invariant (API-08).
- No `execution cancel`, `package rebuild`, `tenant drain`, `workflow pause` verbs (OPS-02), even though `WorkflowExecutor::cancel_execution` is fully implemented in the engine.

**Implication for synthesis**

Two recommendations:
1. **"Audit the CLI/server contract end-to-end and add integration tests"** — covers API-01, API-02, API-03, API-05, API-06, API-08, API-10, API-17. This is the highest-priority operator-experience initiative. The fixes are short (most are 5-30 lines); the test infrastructure to prevent regression is the bigger investment.
2. **"Plumb missing operator verbs"** — covers OPS-02 specifically. `cancel`, `rebuild`, `drain`, `pause` are 1-2 days each, plus the corresponding REST routes and authorization checks.

---

### Cluster: Auth model is dual-axis and undocumented

**Lenses affected**: API Design, Security, Operability

**Related findings**: API-04, OPS-03, OPS-12, OPS-20, SEC-09, SEC-10, COR-12, SEC-08

**Relationship**: Common root — the auth model encodes authorization across three orthogonal axes (`is_admin` "god mode" boolean, `permissions` enum string, `tenant_id` scope) with four near-identical helper methods plus open-coded `auth.is_admin` checks in routes. The axis-vs-helper inconsistency is the core problem; the cache, audit, and bootstrap consequences fan out from there.

**Description**

The matrix:
- `is_admin` — DB column, "god mode" — bypasses tenant scoping. Granted only via bootstrap key (per the comment at `keys.rs:80-89`, the only inline doc).
- `permissions` — string column with values `admin`/`write`/`read` — tenant-scoped role. CLI exposes `Role::Admin/Write/Read`.
- `tenant_id` — Optional scope. `tenant_id == None && route_tenant == "public"` is the implicit single-tenant fallback.
- Helpers: `can_access_tenant`, `can_write`, `can_admin`. Plus open-coded `auth.is_admin` in `tenants.rs:56,97,123` and `keys.rs:202`.

Resulting consequences across lenses:
- **API-04**: undocumented matrix; user can't see why `permissions=admin` doesn't give cross-tenant access.
- **OPS-03/OPS-12**: tenant_id is never recorded onto the request span; log filtering for tenant `acme` requires payload string matching.
- **OPS-20**: bootstrap key plaintext written 0600 with no rotation guidance.
- **SEC-09**: migration 019 promotes any key named `bootstrap-admin` to god-mode by name match alone.
- **SEC-10**: `revoke_key` has no last-admin-guard, no self-revocation guard.
- **COR-12 / SEC-08**: KeyCache uses `clear()` (blunt) instead of the dead-code `evict()`; multi-server deployments propagate revocation lazily over 30s; the Mutex serializes all auth checks.

**Implication for synthesis**

One recommendation: "Document the auth model in one place and tighten the implementation gaps." The doc is one page. The implementation tightening is half a dozen 30-line patches: span enrichment (OPS-03/12), evict-by-hash on revoke (COR-12), last-admin guard (SEC-10), bootstrap-by-tenant_id filter (SEC-09), bootstrap-key rotation runbook (OPS-20).

---

### Cluster: Async lifecycle and Drop semantics — graceful shutdown is per-binary-different

**Lenses affected**: Operability, Correctness, Evolvability

**Related findings**: OPS-10, EVO-18, COR-02, COR-08

**Relationship**: Common root — async Rust + Drop is an unsolved problem and the project has three different answers. Server bounds shutdown to 30s; daemon races shutdown vs. configurable timeout vs. second-SIGINT force-exit; compiler does not bound running cargo subprocesses. Inside the engine, `release_runner_claim` is unguarded (COR-02) and the heartbeat-task abort happens BEFORE the state transition (COR-08), so shutdown ordering is asymmetric in subtle ways.

**Description**

Three concrete inconsistencies:
- Compiler shutdown does not bound running cargo (OPS-10) — a build started just before SIGTERM keeps running until cargo finishes.
- `DefaultRunner::Drop` cannot run async shutdown (EVO-18) — relies on caller calling `shutdown()` explicitly.
- `release_runner_claim` is unguarded; can release another runner's claim post-stale-sweep (COR-02).
- Heartbeat task abort fires before `complete_task_transaction` returns (COR-08) — benign today but indicates the lifecycle hasn't been thought through holistically.

**Implication for synthesis**

This is a cleanup item rather than a structural one. Recommendation: "Standardize graceful shutdown across the three binaries; tighten claim-release ordering."

---

## Root Causes

### Root Cause: Multi-tenant abstraction is storage-only

**Symptom IDs**: COR-01, EVO-04, EVO-15, OPS-03, OPS-12, OPS-16, SEC-02, SEC-03, SEC-05, SEC-14, SEC-17, API-04

**What it is**: When multi-tenancy was added, the storage layer got `TenantDatabaseCache` + per-tenant pools + schema search_path. The runner was not made tenant-scoped (one shared `Arc<DefaultRunner>` connected to admin), the reconciler was not made tenant-scoped (it loads packages into the runner's admin runtime), the request span was not enriched with tenant_id, the audit/health endpoints were not tenant-filtered, and tenant deletion was implemented as a schema drop without lifecycle cleanup. The resulting state is "storage knows about tenants; everything else doesn't."

**What it would take to address**: A multi-week initiative — likely a new spec — to (a) introduce per-tenant runner instances or a database-override on `WorkflowExecutor::execute_async`, (b) propagate `tenant_id` into request spans and execution_event rows, (c) enforce tenant scoping on `triggers.rs` and `health_graphs.rs` consistently with `executions.rs`, (d) make tenant deletion orchestrate runner unload + accumulator/reactor stop + cache eviction + key revocation + execution cancel. Each piece is small; the combined initiative is the project's biggest single architectural debt.

---

### Root Cause: Plugin-load trust gate exists in code but not in deployable configuration

**Symptom IDs**: SEC-01, SEC-04, SEC-06, SEC-13, OPS-07, OPS-10, OPS-14, COR-04, COR-15, COR-17

**What it is**: The signing infrastructure (Ed25519 keys, AES-256-GCM at-rest encryption, package_signer, verifier) is well-built. But the wiring between the verifier and the upload/load paths is incomplete: `verification_org_id` cannot be set from the CLI; the lower-level `register_workflow` path in the daemon does not verify; audit logs are defined but never called; and the build path runs cargo on attacker-supplied source with no isolation. The defense's primitives are correct; the defense is not turned on, cannot be turned on from the documented config surface, and the only tests of it are `#[ignore]`-gated.

**What it would take to address**: One short-cycle fix (CLI flag for `--verification-org-id`, audit-log call sites, default `--frozen --offline` for compiler) plus one long-cycle fix (sandboxing the build path with bubblewrap/nsjail/Linux namespaces, plus per-tenant signing-org binding). The short-cycle fix is the release-blocker.

---

### Root Cause: I-0102 macro decomposition deferred — monolithic `package!()` is the single high-cost surface for plugin-ABI evolution

**Symptom IDs**: LEG-05, LEG-06, LEG-14, LEG-20, COR-04, EVO-01, EVO-13, EVO-17, API-07, API-14, PERF (cdylib runtime overhead in macro)

**What it is**: The unified shell macro emits 560 lines of indivisible token-construction per cdylib and the `workflow_attr.rs` migration tail still emits and discards 130 lines per `#[workflow]` invocation. Adding a tenth method, opting tasks-only crates out of trigger/CG runtime initialization, fixing the `.expect()` panic, or sharing a single tokio runtime across shapes — all require touching the same monolithic macro body. The `WorkflowEntry` constructor type still lives in the engine crate, so the leaf-crate refactor isn't done.

**What it would take to address**: Decompose `package!()` into per-method emitter macros + a shared `cdylib_runtime!()` helper; delete `generate_packaged_registration` + the `let _ = packaged_registration;` line; relocate `Workflow` (or a thinner `WorkflowSpec`) to `cloacina-workflow`; document the wire-format and ABI evolution policy in an ADR. ~1-2 weeks; resolves 12 findings.

---

### Root Cause: Telemetry is engine-shaped; outer binaries are blind

**Symptom IDs**: OPS-04, OPS-05, OPS-15, OPS-17, COR-11 (silent merge failures), PERF-19

**What it is**: The metric primitives, label vocabulary, validation pipeline (`promtool check metrics`), and credential-logging guard are excellent — inside `cloacina-server`. Neither `cloacina-compiler` nor `cloacinactl daemon` install a Prometheus recorder, so all the engine's instrumentation dies in those processes. Inside the engine, persist-failure paths (`let _ = persist_*`) drop crash-recovery state without a metric. Composite operations (`cloacinactl status`) have no aggregation, no timeout, no JSON output.

**What it would take to address**: Install a Prometheus recorder + `/metrics` endpoint in compiler and daemon; add `cloacina_*_persist_failures_total` counters; convert `cloacinactl status` to a parallel fan-out with JSON output. ~2-3 days for the metric expansion; the daemon's metric-bind needs a CLI flag. Total resolves ~6 findings.

---

### Root Cause: Test suite exercises the happy path; partial-failure and contract surfaces are uncovered

**Symptom IDs**: COR-09, COR-13, COR-15, EVO-07, API-01/02/03 (no contract tests), SEC-01/04 (no end-to-end signing test), OPS-07 (no sandbox = no test of sandbox)

**What it is**: There are zero integration tests for: double-dispatch race, FFI panic propagation, DST cron transitions, `mark_failed` returning false, reactor `WhenAll` after partial fire. Three CLI commands ship non-functional because no test exercises the CLI/server boundary. Six signing trust-chain tests are `#[ignore]`-gated and don't run in `cargo test` or `angreal test unit`. Tests reach into engine internals, locking abstractions in place. 108 `#[serial]` annotations indicate substantial shared global state.

**What it would take to address**: Multi-phase initiative. Phase 1: CLI/server contract tests for the three broken commands (1 week). Phase 2: un-gate the signing tests, give them a fixture (0.5 week). Phase 3: double-dispatch, FFI-panic, time-mocked reactor tests (1 week). Phase 4: factor `cloacina-testing-cg` from internal-reach test code (1-2 weeks). The first two phases close release-blockers; the third is correctness investment.

---

### Root Cause: Documentation lags refactor by 1-2 cleanup waves consistently

**Symptom IDs**: LEG-01, LEG-02, LEG-03, LEG-04, LEG-05, LEG-07, LEG-08, LEG-09, LEG-15, LEG-16, LEG-17, LEG-18, LEG-19, EVO-11, EVO-20, API-07, API-11, API-15

**What it is**: Refactors land cleanly in code; doc comments, examples, module names, and rustdoc HTML lag behind. Banned terminology lives in production source comments and ships in rustdoc. Module names point to deleted concepts. Tutorial-style comments reference removed APIs. Macro doc says "Six methods" while emitting nine. The README and engine crate quick-starts walk users through a `workflow!` macro that doesn't exist. Each individual finding is small. Collectively they are the new user's first impression.

**What it would take to address**: A documentation hygiene sweep — most are 1-3 line edits. The notable exception is the README/engine quick-start (LEG-04, API-11), which needs a real rewrite to use the actual `#[workflow]` attribute syntax. ~3-5 days total, mechanical work.

---

### Root Cause: Post-refactor cleanup is per-task, not per-initiative

**Symptom IDs**: EVO-14, plus the refactor-tail consequence pattern across LEG/EVO findings

**What it is**: EVO-14 names the meta-pattern: T-0509 was needed to "finish I-0096 cleanup." T-0549 had Phase 1/2a/2b/2c/2d. The team is excellent at refactoring and at cleanup, but cleanup is added piecemeal as later tasks discover it. There's no convention of "every initiative ships with N cleanup-task placeholders at decomposition time."

**What it would take to address**: A planning convention — every initiative gets a "cleanup phase" placeholder budget at decomposition. This is process, not code. EVO-14 itself suggests a `cargo deny`-style lint for `_legacy`/`_old` suffixes and `pub use` chain depth as a structural guardrail.

---

## Tensions

### Tension: Plugin ergonomics vs. ABI stability vs. macro composability

**Findings on each side**:
- **Ergonomics**: API-14 (one-line `cloacina::package!()` is great DX), API-07 (the documented contract should match emitted code), LEG-05 (six-vs-nine).
- **ABI stability**: EVO-01 (decomposing the macro changes the cdylib output and might break in-flight migrations), EVO-20 (no documented ABI evolution policy).
- **Macro composability**: LEG-14 (four duplicate runtime blocks), API-14 (no opt-out for unused shapes), EVO-13 (TriggerlessGraph duplicates ComputationGraph).

**Assessment**: Drifting. The current shape was the right first move (single macro for symmetry); it is now the bottleneck for every plugin-ABI evolution. The team has implicitly chosen ergonomics over composability; this pays off pre-1.0 (the team owns every cdylib in existence) but the moment a third-party cdylib ships, the macro becomes load-bearing for backward compatibility. **Recommendation**: decompose in the next cycle, document the ABI evolution policy as part of the decomposition, before any third-party cdylibs ship.

---

### Tension: Performance vs. Multi-backend support

**Findings on each side**:
- **Performance**: PERF-01 (per-tenant pool of 2 is a throughput cliff), PERF-02 (SQLite hardcoded to 1), PERF-04 (N+1 in workflow finalization), PERF-05/06/07 (CG hot path).
- **Multi-backend**: EVO-02 (218 paired functions), EVO-09 (migration drift), API-13 (Universal types pattern is the abstraction).

**Assessment**: Healthy at the abstraction layer (`Universal*` types are well-named and consistent), drifting at the implementation layer. The choice to dispatch at runtime rather than adopt diesel `MultiConnection` was a pragmatic early call that has accumulated debt. The hardcoded pool sizes are a separate issue — they're not really about backend support, they're about not having plumbed config through. **Recommendation**: separate the two concerns. Tune pool sizing as configurable now (urgent); plan `MultiConnection` adoption as a longer initiative.

---

### Tension: Embedded simplicity vs. Multi-tenant safety

**Findings on each side**:
- **Embedded simplicity**: One `DefaultRunner`, one `Database`, one `Runtime`, one process-global `python_runtime` slot (EVO-08). The README pitches `let runner = DefaultRunner::new("sqlite://./local.db").await?` as the canonical use case.
- **Multi-tenant safety**: COR-01, EVO-04, SEC-02, SEC-03, SEC-05, SEC-14, OPS-03/12.

**Assessment**: Drifting badly. The embedded use case is trivially simple; the multi-tenant server use case has a runner-shape mismatch (admin schema only, despite tenant URLs). The product positions itself for both audiences. The implementation prefers the embedded shape, with multi-tenant shimmed in via `TenantDatabaseCache` for read paths only. **Recommendation**: choose. Either (a) declare the embedded shape canonical and explicitly limit multi-tenant to read-only operations until per-tenant runners are built, or (b) build per-tenant runner caching and treat multi-tenant as first-class. The current "implicitly multi-tenant" posture creates the security findings.

---

### Tension: Plugin loading flexibility vs. Trust boundary

**Findings on each side**:
- **Flexibility**: The `.cloacina` packaging system is a deliberate hot-reload primitive. Daemons watch directories for new packages; servers accept multipart uploads; the compiler does the heavy build lift. This makes "ship a workflow without restarting the runner" possible.
- **Trust boundary**: SEC-01, SEC-04, SEC-06, OPS-07, OPS-14, COR-15.

**Assessment**: Unconsidered. The flexibility is well-built; the trust boundary is acknowledged in code comments and a security audit log function definition, but the wiring stops there. The default deployment is "anyone with a tenant write key gets in-process arbitrary code execution as the host UID." For the target audience ("trusted operators inside a trusted network", per security review summary), this is acceptable. For the multi-tenant-SaaS audience the README also targets, it is not. **Recommendation**: an explicit threat-model statement in deployment docs, plus the SEC-01/SEC-04/SEC-06 wiring fixes. The team should pick which audience the default deployment serves.

---

### Tension: Best-effort persistence vs. Operational visibility

**Findings on each side**:
- **Best-effort**: 16 `let _ =` sites in `reactor.rs`; CG persistence calls wrapped in `let _ =`; rationale is "a fire that already produced output shouldn't fail because the checkpoint write didn't persist."
- **Visibility**: OPS-15 (no metric for persist failures), COR-11 (silent JSON parse and context-merge errors).

**Assessment**: Healthy in intent, broken in instrumentation. The "log and continue" idiom is the right design choice for the reactor's correctness model. But "log and continue" without a counter, without a watchdog, and without a degraded-health surface means operators can't observe the failure mode. **Recommendation**: keep the `let _ =` pattern; add `cloacina_*_persist_failures_total` counters and a watchdog that downgrades reactor health on N consecutive failures.

---

### Tension: API ergonomics vs. Strict contract

**Findings on each side**:
- **Ergonomics**: API-14 (one-line `cloacina::package!()`), API-13 (30+ builder knobs), API-16 (`Context<T>` parametric API).
- **Strict contract**: API-04 (auth model not documented), API-06 (three error envelope shapes), API-15 (WS protocol undocumented), API-19 (bootstrap-key behavior undocumented).

**Assessment**: Asymmetric. The ergonomic surface is well-designed (`#[task]`, `#[workflow]`, builder patterns, prelude). The contract surface is uneven — some routes use `ApiError`, some return raw JSON; the auth model has three axes nobody has documented; the WS protocol has no schema doc. **Recommendation**: a documentation initiative focused on contracts (the auth matrix, the error envelope, the WS protocol, the wire-format evolution policy), separate from the README/quick-start cleanup.

---

### Tension: Refactor velocity vs. Cleanup overhead

**Findings on each side**:
- **Velocity**: 275 commits in 6 months, last 50 dominated by I-0102 cleanup waves. T-0509, T-0529, T-0483, T-0528, T-0549/51/53/54/55/56/61/63/65 — the team refactors aggressively at the architectural layer.
- **Cleanup overhead**: EVO-14 names the meta-pattern; LEG-01 through LEG-20 are mostly downstream of recent refactors; doc lag everywhere.

**Assessment**: Healthy with a process gap. The team is genuinely good at refactoring, and the closing-of-loops pattern is real. The gap is that cleanup is added piecemeal as later tasks discover what the earlier tasks missed. **Recommendation**: decomposition convention — every initiative ships with N cleanup-task placeholders sized at "1 day of doc + dead-code sweep" each, scheduled at N+1.

---

## Systemic Patterns

### Pattern: "Recently shipped, doc didn't catch up"

Examples by ID: LEG-01 (banned terms in source comments), LEG-02 (RunningGraph still graph-shaped post-T-0544), LEG-03 (deleted global_registry module still public), LEG-04 (README workflow! macro), LEG-05 (package! says six methods), LEG-07 (inventory_entries.rs says T-0506 hasn't happened), LEG-08 (Runtime says five namespaces), LEG-09 (with_global_workflows references), LEG-16 (#[reactor] doc says type-path), LEG-18 (WorkflowMetadata.schedules unpopulated), LEG-19 (load_package doc says global), LEG-20 (coexistence warning unconditional), API-07 (macro doc says six methods), API-11 (prelude advertises workflow!), API-15 (WS protocol undocumented), EVO-11 (workflow registry trait doc claims responsibilities trait no longer has).

This is the dominant pattern in the legibility lens and recurs in API design and evolvability. It is mechanical to fix (~3-5 days of sweep) but the sheer volume signals the cleanup convention is missing.

---

### Pattern: "Test the happy path; partial-failure is invisible"

Examples by ID: COR-09 (no double-dispatch race), COR-13 (sweeper sleeps on real clock), COR-15 (signing tests `#[ignore]`-gated), COR-17 (deny-list shallow check), API-01/02/03 (no contract tests caught broken commands), SEC-01/04 (no end-to-end signing test), the COR section's gap inventory (no DST cron, no FFI panic, no `mark_failed` false-return, no WhenAll-after-partial-fire).

Operationally: failures don't surface as test failures, they surface as production incidents. The CLI/server contract has shipped three Critical/Major findings with no automated detection.

---

### Pattern: "Telemetry is engine-shaped; outer binaries are blind"

(Operability called this out as an open observation; it extends across COR and PERF.)

Examples by ID: OPS-04 (compiler emits zero metrics), OPS-05 (daemon emits zero metrics), OPS-15 (CG persist failures invisible), OPS-17 (cloacinactl status is serial), COR-11 (silent context-merge failures), PERF-19 (unbounded mpsc to runtime thread).

The engine has good metric vocabulary and validates exposition with promtool in CI. None of that reaches the build worker or the daemon, both of which run the same engine code with the same gauge antipatterns.

---

### Pattern: "Auth model is dual-axis but undocumented"

Examples by ID: API-04 (matrix not documented), OPS-03 (tenant_id not in span), OPS-12 (tenant filtering requires payload string match), OPS-20 (bootstrap rotation not documented), SEC-09 (bootstrap-by-name match), SEC-10 (no last-admin guard), SEC-13 (signatures not org-scoped), COR-12 / SEC-08 (KeyCache uses clear() not evict()).

The implementation has correct primitives (Ed25519, AES-256-GCM, schema validation, mode-0600 bootstrap key). The compositional doc is missing. Eight findings, three lenses, one missing one-page doc plus six 30-line implementation tightenings.

---

### Pattern: "Per-task post-refactor cleanup" as a process signal

Examples by ID: EVO-14 (the meta-finding); the LEG-01 through LEG-20 sweep that this implies; the macro-decomposition cluster.

The team's refactor velocity is real and healthy. The cleanup pattern is "ship the headline change, accept that subsequent waves will clean up the remainder." 5 cleanup tasks per headline initiative is the observed rate. This works pre-1.0; the rate will need to drop to 0-1 post-1.0.

---

### Pattern: "Multi-tenancy is a half-built abstraction"

Examples by ID: COR-01, EVO-04, EVO-15, OPS-03, OPS-12, OPS-16, SEC-02, SEC-03, SEC-05, SEC-14, SEC-17, API-04 (twelve findings, four lenses).

The abstraction was added to storage; it stops there. Everything tenant-shaped above the storage layer (runner, reconciler, request span, audit, health, lifecycle) is still admin-schema-bound. This is the project's largest single architectural debt by finding count.

---

### Pattern: "Plugin loading is in-process arbitrary code; the trust gate is unwired"

Examples by ID: SEC-01, SEC-04, SEC-06, SEC-13, COR-04, COR-05, COR-15, COR-17, OPS-07, OPS-14, SEC-18 (ten findings, three lenses).

The signing primitives are sound; the wiring is absent. The compiler builds attacker source with no isolation. The reconciler dlopens cdylibs without verifying. Audit functions exist but are never called. This is the project's largest single security debt by finding count.

---

## Severity Adjustments

| ID | Original | Adjusted | Rationale |
|---|---|---|---|
| EVO-04 | Major | **Critical** | Read alongside SEC-03, this is a tenant-boundary breach that ships in the default config. The "evolvability gap" framing understates the security consequence. Release-blocker for any multi-tenant-claimed deployment. |
| OPS-07 | Major | **Critical** | The intended defense (signature verification, SEC-01/SEC-04) is unactivatable from the supplied CLI. Sandbox is not "additional defense in depth" — it is the only deployable defense. |
| COR-15 | Major | **Critical** | Six signing tests `#[ignore]`-gated covering the only RCE defense for the only RCE path; they don't run in `cargo test` or `angreal test unit`. Combined with SEC-01/SEC-04, the security model has no test signal. |
| LEG-04 / API-11 | Major | **Critical** | This is the README's first code sample and the engine crate's top-level rustdoc, walking the reader through a macro that doesn't exist. New user's first 30 minutes fail. Public-face issue. |
| API-01 | Critical | Critical (held) | Stays Critical — published CLI command non-functional since release. |
| API-02 | Critical | Critical (held) | Same shape as API-01. Filters silently ignored. |
| API-03 | Major | **Critical** | Three CLI list commands silently emit empty output. As a cluster with API-01/02 this is a Critical CLI-regression that ships green. |
| OPS-01 | Major | Major (held) | Active-tasks gauge leak — the same shape as the T-0534 fix already shipped for active-workflows. Major because it's been shipped without the fix. |
| LEG-06 | Major | Minor | After integration: this is dead-code cleanup paired with LEG-20. The stand-alone cost is low; the cluster cost is real but the cluster recommendation is what matters. (But: keep at Major if synthesis treats it stand-alone.) |
| EVO-01 | Major | Major (held) | The 560-line monolithic macro is a real evolvability cliff but the cluster recommendation captures it. |
| SEC-01 | Critical | Critical (held) | Default deployment ships an in-process RCE path. Release-blocker. |
| SEC-04 | Major | **Critical** | The intended defense for SEC-01 is unactivatable from the documented config surface. Together they're one Critical. |
| SEC-06 | Critical | Critical (held) | Build-side RCE on attacker-supplied source. Release-blocker. |
| OPS-04, OPS-05 | Major | Major (held) | Asymmetric metric coverage; the cluster recommendation captures it. |
| PERF-01, PERF-02 | Major | Major (held) | Throughput cliffs but not security/correctness. The cluster recommendation captures it. |
| COR-04 | Major | Major (held) | FFI panic vector but mitigated by host catching some panics. The macro decomposition cluster will fix this naturally. |
| COR-09 | Major | Major (held) | No double-dispatch race test — important gap but the atomic claim does protect today. |

---

## Handoff to Synthesis

The synthesis report (09-report.md / 10-recommendations.md) should organize around **structural concerns**, not lenses or finding IDs. The structural concerns, in priority order:

1. **Multi-tenant boundary is incomplete.** Twelve findings, four lenses. The runner is admin-schema-bound while the URL is tenant-scoped. Workflows execute in the wrong schema. Triggers are read from the wrong DB. Health endpoints leak every tenant's reactor names. Tenant deletion drops the schema but leaves runners, reactors, API keys, and cached pools alive. Single recommendation: complete the multi-tenant abstraction across runner, reconciler, observability, and lifecycle. **Release-blocker for any multi-tenant deployment.**

2. **Plugin-load trust gate is unwired AND the build path is unsandboxed.** Ten findings, three lenses. Default deployment ships an in-process RCE path: any tenant write key can upload code that executes as the host UID with full DB access. The intended defense (signature verification) cannot be activated from the documented CLI. The compiler runs cargo on attacker source with no isolation. Audit logs are defined but never called. The signing tests are `#[ignore]`-gated. Two recommendations:
   - Wire signature verification end-to-end and make it default-on for non-trivial deployments. (Short cycle: CLI flag, audit log calls, default-on policy.)
   - Sandbox the build path. (Long cycle: bubblewrap/nsjail, default `--frozen --offline`, build timeout.)
   **Release-blocker for any deployment exposing the upload endpoint to untrusted principals.**

3. **CLI/server contract has Critical-severity bugs that shipped untested.** Three commands non-functional, two flags silently ignored, error envelope inconsistent. Ten findings, three lenses. The CLI is the documented operator surface; today it does not work. Single recommendation: audit the CLI/server contract end-to-end, add integration tests, fix the broken commands. The fixes are short; the test infrastructure is the larger investment. **Release-blocker for the CLI.**

4. **Operator runbook is missing key verbs.** No `execution cancel`, no `package rebuild`, no `tenant drain`. The engine has the primitives; the CLI and HTTP routes don't. Two days each.

5. **Telemetry asymmetry across the three deployables.** `cloacina-compiler` and `cloacinactl daemon` install no Prometheus recorder. The `active_tasks` gauge in the engine has the leak antipattern that `active_workflows` was fixed for. Persist failures in CG paths are invisible. One recommendation: install recorders, extend the SQL-derived gauge pattern, add failure-mode counters.

6. **I-0102 macro decomposition is the right next refactor.** Twelve findings cluster around the monolithic `package!()` macro. Decompose into per-method emitters; share a single `cdylib_runtime!()` helper; relocate `Workflow` to the leaf crate; document the ABI evolution policy. ~1-2 weeks; resolves a fan-out.

7. **Documentation hygiene sweep.** ~17 findings about doc/example/macro-doc drift; mostly mechanical 1-3 line edits. The README/engine quick-start needs a real rewrite. ~3-5 days.

8. **Test architecture investment.** Failure-path coverage is thin. Specific gaps: CLI/server contract tests, signing trust-chain un-gating, double-dispatch race, FFI panic, DST cron, time-mocked reactor tests. Multi-phase initiative.

9. **Auth model: document and tighten.** One-page doc for the matrix; six 30-line implementation patches (span enrichment, evict-by-hash, last-admin guard, bootstrap-by-tenant_id, etc.).

10. **Hot-path performance.** CG runtime lock contention, reactor JSON persistence, GIL acquisitions, scheduler poll interval. Lower priority than the boundary work but a coherent cluster.

The first three items are release-blockers (Critical-severity, multi-finding, multi-lens). Items 4-7 are the next initiative wave. Items 8-10 are continuous investment.

**Closing observation**: cloacina is a system that has done good architectural thinking and is in the middle of the cleanup. The findings cluster tightly around three structural gaps (multi-tenant, plugin trust, observability symmetry) and a process gap (cleanup convention). The team can refactor at scale; the question for synthesis is whether the next initiative budgets enough cleanup phases to close those gaps in one cycle rather than discovering them across five.
