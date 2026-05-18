# Audit — workflows/ docs (CLOACI-I-0112 Phase 2)

> Produced by parallel audit agent (general-purpose). Per-doc design.md entries; preserved verbatim for Phase 3 reference. Synthesis lives in [design.md](./design.md).

### docs/content/workflows/_index.md (status: existing)
- **Category:** Index
- **Audience:** Any reader landing on the workflows section
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `_index.md:25` references `cloacinactl serve` and `cloacinactl daemon` — these are now noun-form `cloacinactl server start` / `cloacinactl daemon start` per `crates/cloacinactl/src/main.rs:120-138` (I-0098/T-0538).
  - Otherwise accurate but bare; doesn't surface the May 2026 surfaces (signature verification, multi-tenant teardown, packaging shell, observability).
- **Coverage (May 2026 batch):** n/a (overview)
- **Sources:** `crates/cloacinactl/src/main.rs:120-160`, `.metis/initiatives/CLOACI-I-0112/initiative.md`
- **Key topics to cover/preserve:** library vs service mode framing, navigation map
- **Depends on (cross-link from):** none (top of section)
- **Cross-links (this doc points at):** tutorials, how-to-guides, reference, explanation indexes
- **Effort:** S

## workflows/explanation

### docs/content/workflows/explanation/_index.md (status: existing)
- **Category:** Index
- **Audience:** Reader navigating explanation docs
- **Status delta:** verify-no-changes
- **Drift / gaps found:** none — content is `{{< toc-tree >}}` only.
- **Coverage (May 2026 batch):** n/a
- **Effort:** S

### docs/content/workflows/explanation/architecture-overview.md (status: existing)
- **Category:** Explanation
- **Audience:** Developer/architect orienting to the system
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `architecture-overview.md:36` — `cloacinactl daemon --watch-dir /opt/workflows --poll-interval 500` uses old top-level flag syntax; current CLI is `cloacinactl daemon start --watch-dir ... --poll-interval ...` (verb-based per I-0098).
  - `architecture-overview.md:52` — `cloacinactl serve --bind 0.0.0.0:8080` is stale; current verb is `cloacinactl server start --bind ...` (`crates/cloacinactl/src/main.rs:116`).
  - `architecture-overview.md:241-244` claim a `CronRecoveryService` is "Started when both cron scheduling and cron recovery are enabled." Confirmed still extant (`crates/cloacina/src/cron_recovery.rs:87`), but the doc never mentions T-0502 — that `RecoveryManager` was removed and heartbeat sweeper is now the sole recovery path for task-level recovery. The current explanation conflates the two.
  - `architecture-overview.md:244-246` — `StaleClaimSweeper` listed but doc doesn't explain its post-T-0502 elevated role.
  - `architecture-overview.md:209-212` lists `cloacinactl` subcommands incompletely; missing the post-T-0538 nouns (`compiler`, `daemon`, `execution`, `graph`, `key`, `package`, `server`, `tenant`, `trigger`, `workflow`).
  - `architecture-overview.md:127` mentions `Unified Scheduler` driving cron + trigger — accurate but lacks reference to I-0100 DB-backed reactor → workflow subscription fan-out.
- **Coverage (May 2026 batch):** Should expand to cover T-0487 (cooperative task cancellation on claim loss), T-0502 (heartbeat sweeper as sole recovery), I-0096 (inventory unification — no `#[ctor]`, no `global_*_registry`), I-0100 (reactor → workflow subscription pattern at the architecture level), I-0106 multi-tenant search_path coverage.
- **Sources:** `crates/cloacina/src/runner/default_runner/mod.rs`, `crates/cloacina/src/runner/default_runner/service_manager.rs:32-409`, `crates/cloacina/src/cron_recovery.rs`, `crates/cloacina/src/runtime.rs:98-152`, `crates/cloacinactl/src/main.rs:115-160`, `.metis/archived/initiatives/CLOACI-I-0096/`, `.metis/archived/tasks/CLOACI-T-0487/`, `.metis/archived/tasks/CLOACI-T-0502/`
- **Key topics to cover/preserve:** three deployment modes, component map, data flow, mode-by-component table, crate map, background services
- **Depends on (cross-link from):** workflows/_index.md, all how-to guides
- **Cross-links (this doc points at):** task-execution-sequence, dispatcher-architecture, guaranteed-execution-architecture; should add cross-link to platform/explanation/multi-tenancy for I-0106 and to platform/reference/cli for current CLI verb surface
- **Effort:** M

### docs/content/workflows/explanation/context-management.md (status: existing)
- **Category:** Explanation
- **Audience:** Developer learning the Context API
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `context-management.md:60-103` ERD references `recovery_attempts` and `last_recovery_at` columns on `pipeline_executions` and `task_executions` — verify post-T-0502 (RecoveryManager removed) whether those columns are still in the schema. Spot check shows they remain in code (`crates/cloacina/src/cron_recovery.rs:96` etc.) but the semantics changed: pipeline-level recovery is heartbeat-driven now, not RecoveryManager-driven. Doc still implies the old model.
  - No mention of I-0110 atomic `complete_task_transaction` for context persistence — this is the load-bearing change to context durability semantics.
  - No mention of I-0110 typed JSON parse/merge errors (`ContextError::Serialization` was the old umbrella; I-0110 adds typed variants with counter).
  - `context-management.md:144-171` describes "latest wins" merge with reverse-order semantics — verify against post-I-0110 deterministic tiebreaker by completion timestamp (`crates/cloacina/src/executor/thread_task_executor.rs:532-985`). The user's current description is older than the I-0110 spec — doc says "later dependencies override earlier ones" but I-0110 requires `final_context` tiebreaker by completion timestamp, which is different.
- **Coverage (May 2026 batch):** Needs I-0110 (atomic `complete_task_transaction`, typed JSON parse/merge errors, deterministic tiebreaker by completion timestamp). Should cross-link to platform/reference/metrics-catalog for the JSON-merge error counter.
- **Sources:** `crates/cloacina/src/executor/thread_task_executor.rs:532-985`, `crates/cloacina/src/context/`, `crates/cloacina/src/error.rs` (`ContextError`), `.metis/archived/initiatives/CLOACI-I-0110/`
- **Key topics to cover/preserve:** Context fundamentals, schema, dependency-order merge, performance guidance
- **Depends on (cross-link from):** tutorials 01-04, workflows/reference/errors
- **Cross-links (this doc points at):** workflows/reference/errors (ContextError variants), guaranteed-execution-architecture
- **Effort:** M

### docs/content/workflows/explanation/cron-scheduling.md (status: existing)
- **Category:** Explanation
- **Audience:** Operator running scheduled workflows
- **Status delta:** rewrite
- **Drift / gaps found:**
  - `cron-scheduling.md:181-244` claims `Pythonista`-style snippets in a Rust explanation doc (`@cloaca.task` mid-Rust narrative — Diataxis-leaky).
  - `cron-scheduling.md:319-352` `MissedExecutionPolicy::Execute / Skip / ExecuteWithDelay` — code search confirms **this type does not exist** (`grep MissedExecutionPolicy` returns no matches). The real type is `CatchupPolicy::Skip / RunAll` (see `crates/cloacina/src/models/schedule.rs` via `crates/cloacina/src/cron_trigger_scheduler.rs:471-484`). The whole section is invented.
  - `cron-scheduling.md:413-432` `CronSchedule::new_with_timezone` constructor — not in current code (timezone is part of `Schedule` / `NewSchedule` row, set via DAL).
  - `cron-scheduling.md:464-531` "Distributed Execution / Leader Election" section invents `try_acquire_leader_lease`, `DistributedCronScheduler`, `DistributedCronExecutor` — no such types exist. Real coordination is via atomic `claim_and_update` on `cron_schedules` rows (the doc itself describes this elsewhere, contradicting itself).
  - `cron-scheduling.md:560-573` SQL `CREATE INDEX CONCURRENTLY ... idx_cron_executions_*` are illustrative; verify they match current migrations.
  - `cron-scheduling.md:580-642` `CronMetrics`/`HealthStatus` types invented — actual metric surface is `cloacina_*` namespace per I-0099 (`crates/cloacina-server/src/lib.rs:301-321`). Doc doesn't mention `cloacina_*` metrics or the I-0108 `cloacina_active_tasks` re-seed.
  - No mention of S-0011 boundary: this is a workflow doc and should not discuss reactor-side concepts (it doesn't, but the CG auditor should know the cross-link `cron-scheduling.md:716-722` references workflows/multi-tenant-setup which is fine).
- **Coverage (May 2026 batch):** Needs I-0099 (metrics surface) and I-0108 (cloacina_active_tasks re-seed); cross-link to platform/reference/metrics-catalog (new).
- **Sources:** `crates/cloacina/src/cron_trigger_scheduler.rs:471-984`, `crates/cloacina/src/cron_recovery.rs:87-410`, `crates/cloacina/src/models/schedule.rs`, `.metis/archived/initiatives/CLOACI-I-0099/`, `crates/cloacina-server/src/lib.rs:301-321`
- **Key topics to cover/preserve:** CronScheduler vs CronExecutor split, at-least-once guarantee, recovery service, cron expression parsing, timezone+DST
- **Depends on (cross-link from):** tutorial 05, how-to monitoring-executions
- **Cross-links (this doc points at):** guaranteed-execution-architecture, tutorial 05, multi-tenant-setup. Should add platform/reference/metrics-catalog (new).
- **Effort:** L (significant rewrite — multiple sections are fabricated)

### docs/content/workflows/explanation/dispatcher-architecture.md (status: existing)
- **Category:** Explanation
- **Audience:** Developer implementing a custom executor backend
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `dispatcher-architecture.md:61-75` `Dispatcher` trait signatures — verify against `crates/cloacina/src/dispatcher/mod.rs:63` (DefaultDispatcher confirmed at `crates/cloacina/src/dispatcher/default.rs:35`). Code paths exist; signatures should be confirmed exactly.
  - `dispatcher-architecture.md:212-222` `RoutingConfig::new("default").with_rule(RoutingRule::new(...))` — matches code per `how-to-guides/custom-task-routing.md` cross-reference.
  - `dispatcher-architecture.md:237-253` `DefaultRunner::builder().database_url(...).routing_config(...)` — verify builder method exists. Per `crates/cloacina/src/runner/default_runner/config.rs:490` and `:788`, `routing_config` exists but takes `Option<RoutingConfig>` on `DefaultRunnerConfigBuilder` (`.routing_config(Some(config))`). The doc's snippet shows it bare on `DefaultRunner::builder()` — needs verifying that `DefaultRunner::builder()` returns a builder with `.routing_config()` accepting `RoutingConfig` (per `config.rs:788` it does, ungated).
  - No mention of T-0487 (cooperative cancellation on claim loss) which affects dispatcher behavior — when claim is lost, dispatcher should propagate cancellation to in-flight executor.
- **Coverage (May 2026 batch):** Should add T-0487 cancellation note. No reference yet.
- **Sources:** `crates/cloacina/src/dispatcher/mod.rs`, `crates/cloacina/src/dispatcher/default.rs:35-87`, `crates/cloacina/src/runner/default_runner/config.rs:488-790`, `.metis/archived/tasks/CLOACI-T-0487/`
- **Key topics to cover/preserve:** Dispatcher/Executor trait separation, TaskReadyEvent shape, routing patterns, custom executor template
- **Depends on (cross-link from):** how-to custom-task-routing, task-execution-sequence
- **Cross-links (this doc points at):** task-execution-sequence, guaranteed-execution-architecture, performance-characteristics (note: `performance-characteristics.md` is referenced but the file does not exist under workflows/explanation — broken link IA-04)
- **Effort:** M

### docs/content/workflows/explanation/guaranteed-execution-architecture.md (status: existing)
- **Category:** Explanation
- **Audience:** Operator/architect needing to trust the durability story
- **Status delta:** rewrite
- **Drift / gaps found:**
  - Title says "Guaranteed Cron Scheduling" but file is named `guaranteed-execution-architecture.md` — content covers cron audit/saga pattern, not the broader "guaranteed task execution" architecture. Title/filename misalignment.
  - `guaranteed-execution-architecture.md:6` description "Understanding the differences between PostgreSQL and SQLite backends" is inaccurate — content is about two-phase commit for cron.
  - No coverage of I-0110: atomic `complete_task_transaction` (confirmed exists at `crates/cloacina/src/executor/thread_task_executor.rs:532`), typed JSON parse/merge errors with counter, deterministic `final_context` tiebreaker by completion timestamp. This doc is the natural home for that material.
  - No coverage of T-0487 (cooperative cancellation on claim loss) — affects "what happens if the claim is lost mid-task".
  - No coverage of T-0502 (`RecoveryManager` removed — heartbeat sweeper is sole recovery path). Doc still implies a separate recovery service for tasks (only the cron recovery service remains per current code; task-level recovery is now heartbeat-driven via StaleClaimSweeper).
  - `guaranteed-execution-architecture.md:124-152` SQL schema diff between PG and SQLite — verify accurate.
  - `guaranteed-execution-architecture.md:225-251` `find_lost_executions` function — confirm signature against `crates/cloacina/src/dal/`.
- **Coverage (May 2026 batch):** I-0110, T-0487, T-0502 are the dominant signals. Must be added.
- **Sources:** `crates/cloacina/src/executor/thread_task_executor.rs:412-985`, `crates/cloacina/src/cron_recovery.rs:87-410`, `crates/cloacina/src/execution_planner/stale_claim_sweeper.rs`, `.metis/archived/initiatives/CLOACI-I-0110/`, `.metis/archived/tasks/CLOACI-T-0487/`, `.metis/archived/tasks/CLOACI-T-0502/`
- **Key topics to cover/preserve:** two-phase commit, audit trail, recovery detection, cross-DB schema, saga pattern
- **Depends on (cross-link from):** cron-scheduling, multi-tenant-setup, monitoring-executions
- **Cross-links (this doc points at):** cron-scheduling, task-execution-sequence; add platform/reference/metrics-catalog and workflows/reference/errors (typed JSON parse/merge variants)
- **Effort:** L

### docs/content/workflows/explanation/macro-system.md (status: existing)
- **Category:** Explanation
- **Audience:** Developer writing workflows in Rust
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `macro-system.md:110-118` Cargo snippet pins `cloacina = "0.1.0"` (VER drift; crates are 0.6.1).
  - `macro-system.md:120` correctly notes inventory replacing `#[ctor]` (I-0096 covered). Good.
  - `macro-system.md:121` says "Task and workflow registration is handled automatically by `inventory::submit!`" — accurate (`crates/cloacina/src/lib.rs:527`, `runtime.rs:98-152`).
  - `macro-system.md:122` `once_cell` and `Mutex` for compile-time registry — confirm; the runtime registry is `RwLock<HashMap>` (`crates/cloacina/src/runtime.rs:80-82`) not Mutex.
  - No mention of I-0102 (unified `cloacina::package!()` shell, `package_type` removed, macro-only triggers/reactors/graphs). For Rust packaged workflows, the `package!()` shell is now the singular FFI entry point.
  - No mention of I-0101: `#[computation_graph]` macro split with `invokes = computation_graph("name")` workflow-task variant. (S-0011 awareness: workflows can invoke a CG as a single task — this is a workflow-side concern, not just CG.)
  - "Task fingerprints calculated by `calculate_function_fingerprint`" snippet (`macro-system.md:78-99`) — verify against `crates/cloacina-macros/src/tasks.rs` (likely the actual fingerprint impl moved post-I-0096).
- **Coverage (May 2026 batch):** I-0102 (`cloacina::package!()`), I-0101 (`invokes = computation_graph("name")` workflow task variant), I-0096 (already partially covered).
- **Sources:** `crates/cloacina-macros/src/tasks.rs`, `crates/cloacina-macros/src/workflow_attr.rs:241-309`, `crates/cloacina-macros/src/packaged_workflow.rs:24`, `crates/cloacina/src/runtime.rs:80-152`, `crates/cloacina/src/lib.rs:527`, `.metis/archived/initiatives/CLOACI-I-0101/`, `.metis/archived/initiatives/CLOACI-I-0102/`
- **Key topics to cover/preserve:** task/workflow macros, fingerprinting, compile-time validation
- **Depends on (cross-link from):** tutorials 01-04, reference/macros, migrating-to-service-mode
- **Cross-links (this doc points at):** workflow-versioning, reference/macros, platform/reference/package-shell-macro (existing or new)
- **Effort:** M

### docs/content/workflows/explanation/task-deferral.md (status: existing)
- **Category:** Explanation
- **Audience:** Developer authoring tasks that wait on external state
- **Status delta:** verify-no-changes
- **Drift / gaps found:**
  - `task-deferral.md:99-117` `SlotToken` struct shape: confirmed against `crates/cloacina/src/executor/task_handle.rs` (the SlotToken pattern matches the executor's use).
  - `task-deferral.md:148-164` task-local storage protocol matches `crates/cloacina/src/executor/task_handle.rs:60-100`.
  - `task-deferral.md:139` claims `task_execution_id` and `is_slot_held` methods on TaskHandle — confirmed at `crates/cloacina/src/executor/task_handle.rs:238,243`.
  - No drift. T-0487 cancellation channel referenced obliquely via "if heartbeat detects ClaimLost"; could be expanded but accurate.
- **Coverage (May 2026 batch):** Touches T-0487 (claim-loss cancellation propagation) tangentially; could cross-link to T-0487 work.
- **Sources:** `crates/cloacina/src/executor/task_handle.rs:60-300`, `crates/cloacina/src/executor/thread_task_executor.rs:412-450`
- **Key topics to cover/preserve:** slot release/reclaim, defer_until lifecycle, sub_status tracking, comparison vs alternatives, limitations
- **Depends on (cross-link from):** tutorial 10, dispatcher-architecture, task-execution-sequence
- **Cross-links (this doc points at):** tutorial 10, reference/macros, task-execution-sequence, dispatcher-architecture, python/api-reference/task
- **Effort:** S

### docs/content/workflows/explanation/task-execution-sequence.md (status: existing)
- **Category:** Explanation
- **Audience:** Developer/operator following a task through the system
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `task-execution-sequence.md:111-124` `claim_ready_task` SQL with `FOR UPDATE SKIP LOCKED` — confirmed PG, but no mention of SQLite path (which uses a different claiming strategy).
  - `task-execution-sequence.md:218-251` `claim_task_for_execution`/`claim_task_within_tx` — verify these names against the current DAL (`crates/cloacina/src/dal/task_execution.rs` or similar).
  - `task-execution-sequence.md:277-280` `notice warning` Hugo shortcode uses `{{%/* */%}}` escape syntax — odd, may be a rendering bug to preserve or fix.
  - `task-execution-sequence.md:301-326` `handle_task_result` references `complete_task_transaction` — confirmed in code at `crates/cloacina/src/executor/thread_task_executor.rs:532`. But the doc doesn't surface I-0110's atomicity guarantees (the whole point of post-I-0110 `complete_task_transaction`).
  - `task-execution-sequence.md:332-340` `RetryPolicy` struct fields don't exactly match `cloacina-workflow::RetryPolicy` (the real one is simpler — `retry_attempts`, `retry_delay_ms`, `retry_backoff`, `retry_condition`, `retry_max_delay_ms`, `retry_jitter` per `reference/macros.md`).
  - No T-0487 (claim-loss → cooperative cancellation) note in the lifecycle diagram. The state diagram should show a `Running → Ready` arc on claim loss.
  - No T-0502 (`RecoveryManager` removed) — task-execution-sequence.md:383-401 "Recovery Mechanisms" section discusses "Orphaned Task Detection" generically; should be updated to say heartbeat sweeper is sole path.
- **Coverage (May 2026 batch):** I-0110 (atomic complete_task_transaction), T-0487 (cooperative cancellation), T-0502 (heartbeat-only recovery).
- **Sources:** `crates/cloacina/src/executor/thread_task_executor.rs:412-985`, `crates/cloacina/src/dal/`, `crates/cloacina-workflow/src/retry.rs`, `.metis/archived/initiatives/CLOACI-I-0110/`, `.metis/archived/tasks/CLOACI-T-0487/`, `.metis/archived/tasks/CLOACI-T-0502/`
- **Key topics to cover/preserve:** Task lifecycle states, push-based dispatch, atomic claiming, isolation/threading, retry semantics, recovery
- **Depends on (cross-link from):** tutorials 04, dispatcher-architecture, guaranteed-execution-architecture
- **Cross-links (this doc points at):** dispatcher-architecture, guaranteed-execution-architecture; add reference/errors (TaskError variants), platform/reference/metrics-catalog
- **Effort:** M

### docs/content/workflows/explanation/trigger-rules.md (status: existing)
- **Category:** Explanation
- **Audience:** Workflow author building conditional pipelines
- **Status delta:** verify-no-changes
- **Drift / gaps found:**
  - `trigger-rules.md:185-225` retry interaction matches code; `retry_attempts`/`retry_delay_ms`/`retry_backoff` syntax matches `reference/macros.md:43-72`.
  - `trigger-rules.md:314-316` api-link shortcodes pointing to `cloacina::execution_planner::*` — verify those modules still exist post-refactor (`crates/cloacina/src/execution_planner/` is referenced at `task-execution-sequence.md` so likely fine).
  - No S-0011 drift, no ver drift. The doc is tightly scoped.
- **Coverage (May 2026 batch):** n/a (trigger-rules are stable pre-batch)
- **Sources:** `crates/cloacina/src/execution_planner/`, `crates/cloacina-macros/src/tasks.rs:83-138`
- **Key topics to cover/preserve:** conditions (task_*/context_value), combinators (all/any/none), evaluation timing, skip propagation, retry interaction, common patterns
- **Depends on (cross-link from):** tutorial 04, reference/macros
- **Cross-links (this doc points at):** tutorial 04, reference/macros
- **Effort:** S

### docs/content/workflows/explanation/workflow-versioning.md (status: existing)
- **Category:** Explanation
- **Audience:** Developer reasoning about workflow change detection
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `workflow-versioning.md:172-190` SQL schema for `pipeline_executions` includes `recovery_attempts` and `last_recovery_at` columns — verify still in migrations post-T-0502.
  - Doc does not mention I-0096 inventory unification — but the fingerprint mechanism is independent of registration mechanism, so this is benign.
  - Hash construction snippets (`hash_topology`, `hash_task_definitions`, `hash_configuration`) — verify against `crates/cloacina/src/workflow/` or wherever `calculate_version` lives now.
  - "Task fingerprints" call out `calculate_function_fingerprint` — same code-path as macro-system.md, should be verified once and cross-cited.
- **Coverage (May 2026 batch):** n/a (versioning is stable pre-batch)
- **Sources:** `crates/cloacina/src/workflow/mod.rs` (or wherever `calculate_version` lives), `crates/cloacina-macros/src/tasks.rs`
- **Key topics to cover/preserve:** content hashing, what is/isn't included, consumer pattern for change detection
- **Depends on (cross-link from):** macro-system, tutorial 07/08 (packaged versioning)
- **Cross-links (this doc points at):** macro-system, packaged-workflow-architecture
- **Effort:** S

## workflows/how-to-guides

### docs/content/workflows/how-to-guides/_index.md (status: existing)
- **Category:** Index
- **Audience:** Reader scanning recipe list
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `_index.md:13-30` no entry for `observe-execution-state.md` or `conditional-retries.md` (both exist as files). IA inconsistency.
- **Coverage (May 2026 batch):** n/a (just a list)
- **Effort:** S

### docs/content/workflows/how-to-guides/cleaning-up-events.md (status: existing)
- **Category:** How-to
- **Audience:** Operator managing DB growth
- **Status delta:** verify-no-changes
- **Drift / gaps found:**
  - Confirmed `cloacinactl admin cleanup-events` exists (`crates/cloacinactl/src/main.rs:209,304`, `crates/cloacinactl/src/commands/cleanup_events.rs`). Note `Admin` is a top-level command, not a `noun` per I-0098/T-0538; doc usage is correct.
  - No version drift, no S-0011 drift.
  - I-0109 added `--log-retention-days` on compiler/server/daemon — orthogonal to event cleanup but worth a cross-link.
- **Coverage (May 2026 batch):** Tangential to I-0109 (log retention); cross-link to platform/how-to-guides/running-the-compiler or running-the-server for `--log-retention-days`.
- **Sources:** `crates/cloacinactl/src/main.rs:149-219`, `crates/cloacinactl/src/commands/cleanup_events.rs`
- **Key topics to cover/preserve:** dry-run pattern, duration format, retention guidance, cron automation
- **Depends on (cross-link from):** monitoring-executions, multi-tenant-recovery
- **Cross-links (this doc points at):** platform/reference/cli; add platform/how-to-guides/running-the-server for `--log-retention-days` after I-0109 doc lands
- **Effort:** S

### docs/content/workflows/how-to-guides/conditional-retries.md (status: existing)
- **Category:** How-to
- **Audience:** Workflow author tuning retry behavior
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `conditional-retries.md:23-30` — `retry_condition = "all" | "never" | "transient" | "foo,bar"` matches `reference/macros.md:67-72`. Accurate.
  - `conditional-retries.md:128-130` references `examples/features/workflows/conditional-retries` — confirmed exists.
  - `conditional-retries.md:128` cross-link to `../reference/` is a directory link; should be `reference/macros#retry-conditions` deep link.
- **Coverage (May 2026 batch):** n/a
- **Sources:** `crates/cloacina-workflow/src/retry.rs`, `crates/cloacina-macros/src/tasks.rs`, `examples/features/workflows/conditional-retries`
- **Effort:** S

### docs/content/workflows/how-to-guides/custom-task-routing.md (status: existing)
- **Category:** How-to
- **Audience:** Operator routing tasks to backends
- **Status delta:** verify-no-changes
- **Drift / gaps found:**
  - `custom-task-routing.md:27` `RoutingConfig::new("default").with_rule(RoutingRule::new("ml::*", "gpu"))` — matches code per `crates/cloacina/src/dispatcher/`.
  - `custom-task-routing.md:48-51` `DefaultRunnerConfig::builder().routing_config(Some(config)).build()` — matches `crates/cloacina/src/runner/default_runner/config.rs:490` (builder method exists, takes `Option<RoutingConfig>`).
  - `custom-task-routing.md:116-122` `cloacina::dispatcher::router::Router; let mut router = Router::new(config); router.add_rule(...)` — verify public exposure (likely available but worth a code spot-check during write phase).
- **Coverage (May 2026 batch):** n/a
- **Effort:** S

### docs/content/workflows/how-to-guides/migrating-to-service-mode.md (status: existing)
- **Category:** How-to
- **Audience:** Developer porting a library workflow to a packaged workflow
- **Status delta:** rewrite
- **Drift / gaps found:**
  - VER pins: `migrating-to-service-mode.md:60` `cloacina = "0.6.1"`, `:81-88` `cloacina-macros = "0.6.1"`, etc. — currently correct but tracked for future bumps.
  - `migrating-to-service-mode.md:23` "Registration | FFI vtable exports (9 methods, indices 0–8) loaded dynamically; the unified `cloacina::package!()` shell macro emits the entry points" — confirmed by I-0102. Good.
  - However, the migration steps do NOT show adding the `cloacina::package!()` invocation at the cdylib crate root — this is now required per I-0102, but Step 4 doesn't include it. The "before/after" Cargo.toml has `cloacina-workflow-plugin` but the rewritten `lib.rs` (Step 4 `After`) shows no `cloacina::package!();` line.
  - `migrating-to-service-mode.md:73-77` `[features] default = ["packaged"] packaged = []` — this user-defined feature gate may be incorrect; the `packaged` feature should come from `cloacina-workflow` directly, not be re-declared in the user crate. Verify against `examples/features/workflows/simple-packaged/Cargo.toml`.
- **Coverage (May 2026 batch):** I-0102 (unified `cloacina::package!();` per cdylib, `package_type` removed, macro-only triggers/reactors/graphs).
- **Sources:** `examples/features/workflows/simple-packaged/Cargo.toml` (canonical packaged crate Cargo.toml), `crates/cloacina-macros/src/workflow_attr.rs:241-309`, `crates/cloacina-macros/src/packaged_workflow.rs`, `.metis/archived/initiatives/CLOACI-I-0102/`
- **Key topics to cover/preserve:** library vs service crate type, dependency swap, cloacina-build/build.rs, deploy steps
- **Depends on (cross-link from):** tutorial 07, reference/macros
- **Cross-links (this doc points at):** tutorial 07, tutorial 08, platform/explanation/ffi-system, platform/explanation/packaged-workflow-architecture, platform/reference/package-shell-macro
- **Effort:** M

### docs/content/workflows/how-to-guides/monitoring-executions.md (status: existing)
- **Category:** How-to
- **Audience:** Operator wiring up monitoring
- **Status delta:** rewrite
- **Drift / gaps found:**
  - **Critical IA-05 — route drift**: `monitoring-executions.md:22, 55, 74, 119, 162` use `http://localhost:8080/tenants/...` — the actual server mounts `/v1` prefix (`crates/cloacina-server/src/lib.rs:777`). All sample URLs should be `/v1/tenants/...`.
  - `monitoring-executions.md:21-48` `GET /tenants/tenant_a/executions` response shape — needs to be checked against the I-0107 unified `ApiError` envelope and list pagination (`crates/cloacina-server/src/routes/executions.rs`). Pagination metadata fields are missing from the example response.
  - `monitoring-executions.md:113-156` `GET /tenants/tenant_a/triggers` — verify pagination per I-0107 (`list_triggers`/`get_trigger`). Sample response has no pagination envelope.
  - `monitoring-executions.md:115-156` covers cron AND trigger schedules at the same endpoint — verify the post-I-0107 route shape (may have split into `/triggers` for trigger schedules and a separate `/cron-schedules`, or unified).
  - `monitoring-executions.md:206-242` Python API — `cloacina.Runner` reference is stale; the actual Python class is `cloaca.DefaultRunner` (per `tutorial 09 line 502`). The `cloacina` Python import path is wrong.
  - No SSE `--follow` coverage for execution streaming (I-0107).
  - No reference to unified `ApiError` envelope (`crates/cloacina-server/src/routes/error.rs:41`).
- **Coverage (May 2026 batch):** I-0107 (unified `ApiError`, pagination on list endpoints, SSE `--follow`), I-0099/I-0108 (metrics surface — but observe-execution-state.md covers this), I-0100 (durable event log for reactor → workflow subscriptions — relevant to event APIs).
- **Sources:** `crates/cloacina-server/src/lib.rs:680-778`, `crates/cloacina-server/src/routes/executions.rs`, `crates/cloacina-server/src/routes/triggers.rs`, `crates/cloacina-server/src/routes/error.rs:41-80`, `.metis/archived/initiatives/CLOACI-I-0107/`
- **Key topics to cover/preserve:** listing executions, drilling into single execution, event log, schedule status, Python alternative, monitoring script template
- **Depends on (cross-link from):** observe-execution-state, multi-tenant-recovery
- **Cross-links (this doc points at):** platform/reference/http-api (for full route+ApiError surface), platform/how-to-guides/deploying-the-api-server, observe-execution-state
- **Effort:** L

### docs/content/workflows/how-to-guides/multi-tenant-recovery.md (status: existing)
- **Category:** How-to
- **Audience:** Operator handling tenant outages
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `multi-tenant-recovery.md:20-22` `DefaultRunner::with_schema(db_url, "tenant_acme")` — confirmed at `crates/cloacina/src/runner/default_runner/mod.rs:91`.
  - `multi-tenant-recovery.md:54` `get_cron_execution_stats(...)` — confirmed at `crates/cloacina/src/runner/default_runner/cron_api.rs:314`.
  - No mention of I-0106 multi-tenant `remove_tenant` orchestration order or drain timeout — relevant to "decommissioning a tenant" but this is more of a "recovery" doc. Cross-link to a new "Decommission a tenant safely" how-to (per I-0112 plan) would help.
  - `multi-tenant-recovery.md:122` mentions "Recovery queries are automatically scoped to the tenant's schema via `SET search_path`" — needs update for I-0106 fail-closed semantics (`crates/cloacina/src/database/connection/mod.rs:113-157`).
- **Coverage (May 2026 batch):** I-0106 (fail-closed `SET search_path` semantics, `remove_tenant` orchestration order, drain timeout).
- **Sources:** `crates/cloacina/src/database/connection/mod.rs:113-160`, `crates/cloacina/src/database/admin.rs:206-241`, `.metis/archived/initiatives/CLOACI-I-0106/`
- **Effort:** M

### docs/content/workflows/how-to-guides/multi-tenant-setup.md (status: existing)
- **Category:** How-to
- **Audience:** Operator provisioning multi-tenant deployments
- **Status delta:** edit-section
- **Drift / gaps found:**
  - **VER-02**: `multi-tenant-setup.md:15` `cloacina = "0.1.0"` — VER drift; should be `0.6.1`.
  - `multi-tenant-setup.md:48-68` `Database::new(..., "cloacina", 10); DatabaseAdmin::new(admin_db); admin.create_tenant(TenantConfig {...})` — confirmed at `crates/cloacina/src/database/admin.rs:206-241`.
  - `multi-tenant-setup.md:69-77` `admin.remove_tenant(&creds.schema_name, &creds.username)?` — confirmed exists at `crates/cloacina/src/database/admin.rs:241`. Does NOT cover I-0106's full orchestration (reactors → executions → cache → keys → schema teardown order, drain timeout). The user signature is `remove_tenant(schema, username)`, but I-0106 enriched this with orchestration steps that should be surfaced.
  - No mention of fail-closed `SET search_path` per I-0106 (the search_path setup happens in connection pool init, not at tenant creation, but the security implications should be cross-linked).
  - Trailing cross-link to `performance-tuning` (`:108`) is broken — that file doesn't exist under workflows/how-to-guides; likely lives in platform/.
- **Coverage (May 2026 batch):** I-0106 (multi-tenant abstraction: fail-closed search_path, `remove_tenant` orchestration).
- **Effort:** M

### docs/content/workflows/how-to-guides/observe-execution-state.md (status: existing)
- **Category:** How-to
- **Audience:** Operator wiring up observability
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `observe-execution-state.md:39-67` metric names match I-0099/I-0108 (`cloacina_workflows_total`, `cloacina_tasks_total`, `cloacina_active_tasks`) — confirmed at `crates/cloacina-server/src/lib.rs:301-321`.
  - `observe-execution-state.md:124-130` `RUST_LOG=cloacina=debug,cloacina_server=debug,axum=info cloacinactl server start ...` — uses post-I-0098 `server start` verb form. Good.
  - `observe-execution-state.md:217-231` `cloacinactl daemon status` output — verify against current daemon Unix-socket health pulse implementation.
  - `observe-execution-state.md:254` links to "https://github.com/colliery-io/cloacina/blob/main/docs/operations/metrics.md" — this is the orphaned ops file (IA-01) being folded into platform/reference/metrics-catalog per the initiative plan. After the fold the cross-link should point at the in-tree catalog.
  - Doesn't cover the `cloacina_active_tasks` re-seed (I-0108) behavior or the persist-failure counter for reactor health (also I-0108) — but those are CG concerns more than workflow concerns.
- **Coverage (May 2026 batch):** I-0099/I-0108 (metrics surface, active_tasks re-seed). Doc is the canonical observability landing page on the workflow side.
- **Effort:** S

### docs/content/workflows/how-to-guides/sequential-strategy.md (status: existing, candidate for move)
- **Category:** How-to (mis-quadrant)
- **Audience:** Reactor pipeline author
- **Status delta:** move (flag for CG auditor; this is wholly CG content)
- **Drift / gaps found:**
  - **IA-06**: This file is in `workflows/how-to-guides/` but its entire subject is `Reactor`, `InputStrategy`, `Accumulator`, `Boundary` — all computation-graph primitives. Should live under `computation-graphs/how-to-guides/`. Flagged for CG auditor.
  - `sequential-strategy.md:13` cross-link to `/computation-graphs/tutorials/library/09-full-pipeline` — confirms cross-section drift.
  - `sequential-strategy.md:111-117` carries a `TODO(I-0101)` comment block in the live doc — the input_strategy macro field is unresolved (I-0101 split status). This is a known live in-doc TODO that the workflow auditor should flag for the CG auditor.
  - `sequential-strategy.md:27` "Latest is the correct choice for most reactive pipelines" — uses the word "reactive" but as an English adjective for a pipeline, not in the banned S-0011 sense ("reactive scheduler / subsystem / execution"). Borderline — Diataxis reviewer should call.
- **Coverage (May 2026 batch):** I-0101 (CG macro split), T-0602 (filter coverage for reactor-triggered workflows could be referenced but is the CG auditor's call).
- **Effort:** S (move only; content correctness is the CG auditor's call)

### docs/content/workflows/how-to-guides/testing-workflows.md (status: existing)
- **Category:** How-to
- **Audience:** Workflow author writing unit tests
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `testing-workflows.md:21-24` `cloacina-testing = { path = "../crates/cloacina-testing" }` — uses path dep; the doc should also document the crates.io version (`cloacina-testing = "0.6.1"`) for users not vendoring the repo. Mirrors VER-03 across tutorials.
  - `testing-workflows.md:178` "If you're testing continuous/reactive tasks" — "reactive" used as English adjective; benign vs S-0011 banned phrases but Diataxis reviewer should sweep.
  - `testing-workflows.md:181-203` `BoundaryEmitter` example references CG concepts (boundaries, time-range/offset-range) — this section is borderline CG-leakage in a workflow doc. Could be moved to `cloacina-testing API reference` (already exists at `workflows/reference/testing-crate.md`) for the CG-specific functionality.
- **Coverage (May 2026 batch):** n/a
- **Effort:** S

### docs/content/workflows/how-to-guides/variable-registry.md (status: existing)
- **Category:** How-to
- **Audience:** Workflow author externalizing config
- **Status delta:** verify-no-changes
- **Drift / gaps found:**
  - `variable-registry.md:33-42` `use cloacina::var; var("KAFKA_BROKER")?` — verify `var()` and `var_or()` are top-level exports of the `cloacina` crate.
  - `variable-registry.md:53-58` `use cloacina::var::resolve_template; resolve_template("postgres://...{{ HOST }}:{{ PORT }}/mydb")` — confirm path.
  - No drift on S-0011 or version.
- **Coverage (May 2026 batch):** n/a
- **Effort:** S

## workflows/reference

### docs/content/workflows/reference/_index.md (status: existing)
- **Category:** Index
- **Status delta:** verify-no-changes
- **Drift / gaps found:** none — `{{< toc-tree >}}` only.
- **Effort:** S

### docs/content/workflows/reference/errors.md (status: existing)
- **Category:** Reference
- **Audience:** Developer pattern-matching errors
- **Status delta:** edit-section
- **Drift / gaps found:**
  - Doc is comprehensive and well-structured. Likely accurate but needs spot-check for added variants from I-0110 (typed JSON parse/merge errors). Specifically: `ContextError::Serialization(serde_json::Error)` is the legacy bucket; I-0110 likely added a typed parse error variant and a typed merge error variant with counters. Verify against current `crates/cloacina/src/error.rs`.
  - No mention of I-0107 unified `ApiError` envelope (that's an HTTP-side error, but the workflows error reference should at least cross-link).
- **Coverage (May 2026 batch):** I-0110 (typed JSON parse/merge errors). I-0107 cross-link to ApiError envelope.
- **Effort:** M

### docs/content/workflows/reference/macros.md (status: existing)
- **Category:** Reference
- **Audience:** Workflow author looking up macro syntax
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `macros.md:43-56` `#[task]` attribute table is comprehensive and matches `crates/cloacina-macros/src/tasks.rs:83-138`.
  - `macros.md:194-200` `#[workflow]` Delivery Modes table notes inventory + `cloacina::package!()` per I-0096/I-0102. Good.
  - `macros.md:217-263` `#[trigger]` macro: `on`, `poll_interval`, `cron`, `timezone`, `allow_concurrent`, `name` — matches `crates/cloacina-macros/src/trigger_attr.rs:49-70`.
  - `macros.md:115-145` TaskHandle parameter detection + `defer_until`, `is_slot_held`, `task_execution_id` — matches `crates/cloacina/src/executor/task_handle.rs:238-243`.
  - Missing: post-I-0101 `#[computation_graph]` macro reference. The doc covers `#[task]`, `#[workflow]`, `#[trigger]` but not `#[computation_graph]` or `#[reactor]` — those should live in `computation-graphs/reference/macros.md`. Flag IA-07: cross-link to that CG ref needs to exist (and `computation-graphs/reference/` may need to mirror this structure).
  - Missing: post-I-0101 workflow task with `invokes = computation_graph("name")` — this is a `#[task]` attribute that lets a workflow task delegate to a CG. Reference must document it under `#[task]`. Verify against `crates/cloacina-macros/src/tasks.rs`.
  - Missing: post-I-0100 reactor subscription (`upstream = reactor("name")` on a workflow trigger). Should be documented under `#[trigger]` or in a new "workflow subscriptions" subsection.
- **Coverage (May 2026 batch):** I-0100 (`upstream = reactor("name")` declaration on workflow trigger), I-0101 (`#[task]` with `invokes = computation_graph("name")`), I-0102 (`cloacina::package!()` ref already covered).
- **Effort:** M

### docs/content/workflows/reference/testing-crate.md (status: existing)
- **Category:** Reference
- **Audience:** Developer using `cloacina-testing`
- **Status delta:** verify-no-changes
- **Drift / gaps found:**
  - `testing-crate.md:21-24` path-dep example only; should also document crates.io version pin (VER-04 parallel).
  - `testing-crate.md:32-48` `TestRunner` API matches `crates/cloacina-testing/src/`.
  - `testing-crate.md:142-202` `BoundaryEmitter`, `MockDataConnection`, `ComputationBoundary`, `ConnectionDescriptor` — these are CG-adjacent (boundary/connection are CG concepts) but live in the workflow testing crate. Acceptable for now; CG auditor should confirm.
- **Effort:** S

## workflows/tutorials

### docs/content/workflows/tutorials/_index.md (status: existing)
- **Category:** Index
- **Status delta:** verify-no-changes
- **Drift / gaps found:**
  - `_index.md:23-29` numbering 5-10 for service tutorials reflects the IA-03 collision flagged in the initiative — out of scope to fix but worth re-noting.
- **Effort:** S

### docs/content/workflows/tutorials/library/_index.md (status: existing)
- **Category:** Index
- **Status delta:** verify-no-changes
- **Effort:** S

### docs/content/workflows/tutorials/library/01-first-workflow.md (status: existing)
- **Category:** Tutorial
- **Audience:** Application developer writing first Rust workflow
- **Status delta:** rewrite
- **Drift / gaps found:**
  - **VER-05**: `01-first-workflow.md:58` `cloacina = "0.1.0"` in the warning block — should be `0.6.1`.
  - `01-first-workflow.md:48` `cloacina = { path = "../../cloacina" }` — recommends path-dep against the repo; this is a "tutorial mode" reasonable choice but the version hint should be 0.6.1.
  - `01-first-workflow.md:67-70` "Cloacina supports both PostgreSQL and SQLite ... no feature flags needed" — INCORRECT. Per `crates/cloacina/Cargo.toml:19-22`, `default = ["macros", "postgres", "sqlite", "kafka"]` — these ARE feature flags, just on by default. The user can disable them. The doc-claim that "no feature flags are needed" is misleading; the accurate framing is "default features include both backends, so for most users no feature management is required at the call site."
  - `01-first-workflow.md:75-77` lists `ctor` as a dependency that "enables static initialization before `main()`" — I-0096 removed `#[ctor]`. The post-T-E story is `inventory::submit!` + `seed_from_inventory()` (`crates/cloacina/src/runtime.rs:98-127`). The dependency list is **stale by ~6 weeks**.
  - `01-first-workflow.md:243` "`angreal demos tutorials rust 01`" — confirmed angreal pattern exists (`.angreal/demos/tutorials/rust.py:30-37`); preserved with tutorial directory `01-basic-workflow`. ✓
  - `01-first-workflow.md:282` GitHub link `examples/tutorials/workflows/library/01-basic-workflow` — confirmed exists.
- **Coverage (May 2026 batch):** I-0096 (inventory unification — must remove the `ctor` claim).
- **Effort:** M

### docs/content/workflows/tutorials/library/02-context-handling.md (status: existing)
- **Category:** Tutorial
- **Audience:** Application developer composing multi-task workflows
- **Status delta:** edit-section
- **Drift / gaps found:**
  - **VER-06**: `02-context-handling.md:60` same `cloacina = "0.1.0"` mention as 01.
  - `02-context-handling.md:69` same incorrect "no feature flags" claim as 01.
- **Effort:** S

### docs/content/workflows/tutorials/library/03-complex-workflows.md (status: existing)
- **Category:** Tutorial
- **Audience:** Developer building parallel pipelines
- **Status delta:** edit-section
- **Drift / gaps found:**
  - **VER-07**: `03-complex-workflows.md:60` same `cloacina = "0.1.0"` line.
  - `03-complex-workflows.md:69` same feature-flag inaccuracy.
  - `03-complex-workflows.md` GitHub example link missing — the doc has no "Download the Example" block (compared with tutorial 04 which does).
- **Effort:** S

### docs/content/workflows/tutorials/library/04-error-handling.md (status: existing)
- **Category:** Tutorial
- **Audience:** Developer building resilient pipelines
- **Status delta:** edit-section
- **Drift / gaps found:**
  - **VER-08**: `04-error-handling.md:60` same `cloacina = "0.1.0"` line.
  - `04-error-handling.md:69` same feature-flag inaccuracy.
  - `04-error-handling.md:665-670` "Option 2: Manual Setup" section has a stray instruction starting at `:668` ("2. A database named 'cloacina' created / 3. A user 'cloacina' with password ...") that appears to be left over from a postgres-specific copy-paste; the workflow is SQLite-based per `:395`. Formatting/content bug.
  - `04-error-handling.md:683-705` Sample output references log lines `cloacina::scheduler > Task ready` — verify the actual scheduler module path is still `cloacina::scheduler` (per code paths, the scheduler module is at `crates/cloacina/src/execution_planner/` and the runner at `crates/cloacina/src/runner/`).
- **Effort:** S

## workflows/tutorials/service

### docs/content/workflows/tutorials/service/_index.md (status: existing)
- **Category:** Index
- **Status delta:** verify-no-changes
- **Effort:** S

### docs/content/workflows/tutorials/service/05-cron-scheduling.md (status: existing)
- **Category:** Tutorial
- **Audience:** Developer adding scheduled workflows
- **Status delta:** rewrite
- **Drift / gaps found:**
  - `05-cron-scheduling.md:52-62` Cargo.toml includes `ctor = "0.2"` — I-0096 drift. Should be deleted.
  - `05-cron-scheduling.md:64` feature-flag misstatement (same as library tutorials).
  - **IA-08**: This tutorial's example code lives in the doc only — no corresponding directory under `examples/tutorials/workflows/service/`. The `examples/tutorials/workflows/library/` only has 01-06; tutorials 05-10 in service mode have no example directories that angreal can discover.
  - `05-cron-scheduling.md:284-313` `register_cron_workflow(name, cron, tz)` — confirmed at `crates/cloacina/src/runner/default_runner/cron_api.rs:40`.
  - `05-cron-scheduling.md:412` `use cloacina::models::cron_schedule::{ScheduleConfig, CatchupPolicy}` — verify path; modern code uses `crates/cloacina/src/models/schedule.rs` (`CatchupPolicy`). The `cron_schedule` module name may be stale post-I-0098 schema/module reorg.
  - `05-cron-scheduling.md:439` `cron_max_catchup_executions` — confirm DefaultRunnerConfig still has this knob.
- **Coverage (May 2026 batch):** I-0096 (`ctor` removal).
- **Effort:** M

### docs/content/workflows/tutorials/service/06-multi-tenancy.md (status: existing)
- **Category:** Tutorial
- **Audience:** Developer building multi-tenant deployments
- **Status delta:** rewrite
- **Drift / gaps found:**
  - `06-multi-tenancy.md:63-67` Cargo.toml uses `cloacina = { path = "../../cloacina" }` — no `0.1.0` version pin so VER drift not a concern; but the post-I-0098/I-0106 multi-tenant story needs new framing.
  - `06-multi-tenancy.md:86-93` Uses `#[workflow(...)] pub mod customer_processing { ... }` syntax — matches post-I-0102 macro.
  - `06-multi-tenancy.md:147` `DefaultRunner::with_schema(database_url, tenant_id)` — confirmed.
  - `06-multi-tenancy.md:344-357` `DatabaseAdmin::new(...) admin.create_tenant(...)` — confirmed.
  - No mention of I-0106 fail-closed `SET search_path` semantics (`crates/cloacina/src/database/connection/mod.rs:113-160`) — this is the critical security note for multi-tenant deployments.
  - No mention of I-0106 `remove_tenant` orchestration order (reactors → executions → cache → keys → schema). The doc shows tenant creation but not tenant teardown.
- **Coverage (May 2026 batch):** I-0106, T-0529/T-0532.
- **Effort:** M

### docs/content/workflows/tutorials/service/07-packaged-workflows.md (status: existing)
- **Category:** Tutorial
- **Audience:** Developer building distributable workflows
- **Status delta:** rewrite
- **Drift / gaps found:**
  - **VER-09**: `07-packaged-workflows.md:84` `cloacina-workflow = "0.2"` — should be `0.6.1`.
  - `07-packaged-workflows.md:16, 379-405` References `cloacina-ctl` binary — this is the legacy name; the binary was renamed to `cloacinactl` per I-0098/T-0538. All `cloacina-ctl package`, `cloacina-ctl inspect` commands must become `cloacinactl package build`, `cloacinactl package inspect` (the verb form per `crates/cloacinactl/src/nouns/package/`). **Major drift across the whole tutorial.**
  - `07-packaged-workflows.md:100-108` "Important Configuration Differences" warns about `features = ["packaged"]` on `cloacina-workflow` — per I-0102 the unified `cloacina::package!()` shell replaces this; the FFI exports come from `package!()`, not from a `packaged` feature flag on the cdylib crate's dependency.
  - `07-packaged-workflows.md:113-232` Workflow code uses `#[workflow(name = "...", package = "...", description = "...", author = "...")]` — per I-0102 the `package` attribute on `#[workflow]` may have been changed/removed (the unified `cloacina::package!()` shell encodes package metadata). Verify against `crates/cloacina-macros/src/workflow_attr.rs:241-309`.
  - `07-packaged-workflows.md:300-308` `cargo build --release` — produces the cdylib. Doesn't show the required `cloacina::package!();` invocation in `lib.rs` (I-0102 requirement).
- **Coverage (May 2026 batch):** I-0102, I-0098/T-0538.
- **Effort:** L

### docs/content/workflows/tutorials/service/08-workflow-registry.md (status: existing)
- **Category:** Tutorial
- **Audience:** Developer using the registry for dynamic loading
- **Status delta:** rewrite
- **Drift / gaps found:**
  - `08-workflow-registry.md:73` `cloacina-ctl = { path = "../../cloacina-ctl" }` — legacy binary name; should be `cloacinactl` post-T-0538.
  - `08-workflow-registry.md:67-80` Cargo.toml uses `cloacina = { path = "../../cloacina" }`, `cloacina-ctl = { path = "../../cloacina-ctl" }` — second is a stale crate path. The legacy `cloacina-ctl` crate has been renamed to `cloacinactl`.
  - `08-workflow-registry.md:122-149` `WorkflowRegistryImpl::new(storage, database)?` — confirmed.
  - `08-workflow-registry.md:158-180` `DefaultRunnerConfig::default(); config.enable_registry_reconciler = true; config.registry_storage_path = Some(PathBuf::from(...))` — verify these direct-assignment patterns still work post-builder migration (config.rs prefers builder). The doc uses field-mutation style; builder-pattern is preferred per the latest config.
  - No mention of I-0102 `cloacina::package!()` unified shell. Building from source via `cloacina-ctl` (now `cloacinactl package build`) for the registry demo.
  - Doc still uses `cloaca-ctl` legacy nomenclature throughout — major sweep needed.
- **Coverage (May 2026 batch):** I-0098/T-0538, I-0102.
- **Effort:** L

### docs/content/workflows/tutorials/service/09-event-triggers.md (status: existing)
- **Category:** Tutorial
- **Audience:** Developer adding condition-based workflows
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `09-event-triggers.md:107-115` Cargo.toml uses `cloacina = { path = "../cloacina", features = ["sqlite", "macros"] }` — verify against post-I-0096/I-0098 feature flags (`crates/cloacina/Cargo.toml:19-25` confirms `sqlite` and `macros` features still exist).
  - `09-event-triggers.md:53-66` `Trigger` trait definition matches `crates/cloacina-workflow/src/trigger.rs:91`. **Note**: the trait was relocated to `cloacina-workflow` per T-0552/I-0102 followup; the doc imports it from `cloacina::trigger::{Trigger, TriggerError, TriggerResult}` which may still re-export but the canonical location is `cloacina_workflow::Trigger`.
  - `09-event-triggers.md:124-195` `FileWatcherTrigger` example using `Trigger` trait — accurate pattern.
  - `09-event-triggers.md:282-307` `dal.trigger_schedule().upsert(NewTriggerSchedule::new(...))` — verify DAL surface.
  - `09-event-triggers.md:294` `register_trigger(trigger.clone())` — confirmed at `crates/cloacina/src/runtime.rs:144,241`. Good.
  - No mention of T-0602 (CEL predicate filtering for reactor-triggered workflows). Workflow event triggers are distinct from reactor subscriptions, but should cross-link.
  - No mention of I-0100 DB-backed reactor → workflow subscription fan-out. The doc covers in-process triggers (`Trigger` trait) but I-0100 is the durable event-log/subscription path — orthogonal but related.
  - `09-event-triggers.md:441-471` Python `@trigger` decorator example — verify `cloaca` exposes this post-T-0529/T-0532 (Python crate split).
- **Coverage (May 2026 batch):** I-0100, T-0602, T-0529/T-0532.
- **Effort:** M

### docs/content/workflows/tutorials/service/10-task-deferral.md (status: existing)
- **Category:** Tutorial
- **Audience:** Developer handling external-condition waits
- **Status delta:** verify-no-changes
- **Drift / gaps found:**
  - `10-task-deferral.md:56` `use cloacina::{task, workflow, Context, TaskError, TaskHandle};` — verify TaskHandle public path.
  - `10-task-deferral.md:69-72` `pub async fn wait_for_data(context: &mut Context<serde_json::Value>, handle: &mut TaskHandle) -> Result<(), TaskError>` — matches macro detection per `crates/cloacina-macros/src/tasks.rs` (parameters `handle` or `task_handle` detected).
  - `10-task-deferral.md:88-107` `handle.defer_until(|| async move { ... }, Duration::from_millis(500)).await` — matches `crates/cloacina/src/executor/task_handle.rs:170-238`.
  - Tutorial is comprehensive and well-scoped — minimal drift. Could add T-0487 (claim-loss cancellation) note: deferred tasks should be cancellation-aware.
- **Coverage (May 2026 batch):** T-0487.
- **Effort:** S

## New docs

### docs/content/workflows/how-to-guides/decommission-a-tenant.md (status: new)
- **Category:** How-to
- **Audience:** Operator safely tearing down a tenant
- **Status delta:** new-write
- **Coverage (May 2026 batch):** I-0106 (full `remove_tenant` orchestration: reactors → executions → cache → keys → schema, drain timeout).
- **Sources:** `crates/cloacina/src/database/admin.rs:241-300`, `crates/cloacina/src/database/connection/mod.rs:113-160`, `.metis/archived/initiatives/CLOACI-I-0106/`
- **Key topics to cover/preserve:** preparation (drain in-flight), tear-down ordering (4-step: reactors stop, executions complete or abort, cache purge, keys revoke, schema drop), drain timeout knob, verification steps, rollback considerations
- **Effort:** M

### docs/content/workflows/how-to-guides/subscribe-workflow-to-reactor.md (status: new)
- **Category:** How-to
- **Audience:** Workflow author wiring a workflow to fire on reactor output
- **Status delta:** new-write
- **Coverage (May 2026 batch):** I-0100 (DB-backed reactor → workflow subscription fan-out; durable event log; `upstream = reactor("name")` declaration on `#[trigger]`).
- **Sources:** `crates/cloacina-macros/src/trigger_attr.rs:49-70` (verify upstream attribute), `crates/cloacina/src/runtime.rs:241` (`register_trigger`), `.metis/archived/initiatives/CLOACI-I-0100/`
- **Effort:** M

### docs/content/workflows/how-to-guides/invoke-computation-graph-from-workflow.md (status: new)
- **Category:** How-to
- **Audience:** Workflow author embedding a CG as a single workflow task
- **Status delta:** new-write
- **Coverage (May 2026 batch):** I-0101 (`#[task]` with `invokes = computation_graph("name")` — graph runs as one task in workflow quantum).
- **Sources:** `crates/cloacina-macros/src/tasks.rs` (verify invokes attribute), `.metis/archived/initiatives/CLOACI-I-0101/`
- **Effort:** M

---

## Summary

- **Files reviewed:** 41 (`workflows/_index.md` + 11 explanation + 12 how-to-guides + 4 reference + 13 tutorials)
- **NOM:** 0 banned-phrase occurrences in workflows/. The word "reactive" appears 3 times (`sequential-strategy.md:27`, `testing-workflows.md:178`, `testing-crate.md:142`) as English adjective ("reactive pipelines/tasks") — not the banned S-0011 noun form. Diataxis reviewer should sweep for tone.
- **VER:** 9 stale version pins found.
- **IA-04** broken link: `dispatcher-architecture.md` → `performance-characteristics.md` (file doesn't exist under workflows/explanation); `multi-tenant-setup.md:108` → `performance-tuning` (broken under how-to-guides).
- **IA-05** route drift: `monitoring-executions.md:22,55,74,119,162` URL examples omit `/v1/` prefix.
- **IA-06** quadrant leakage: `sequential-strategy.md` lives in workflows/how-to-guides but is entirely about CG primitives.
- **IA-07** cross-section reference gap: `reference/macros.md` covers `#[task]/#[workflow]/#[trigger]` but lacks cross-link to a CG-side reference for `#[computation_graph]/#[reactor]`.
- **IA-08** missing example dirs: `examples/tutorials/workflows/library/` has only 01-06; the `service/` subdir is empty. Tutorials 05-10 in service-mode docs reference example code that has no corresponding angreal-discoverable directory.
- **Code drift (non-tagged):**
  - `cron-scheduling.md:319-352, 464-573, 580-642` contains entirely fabricated APIs.
  - `07-packaged-workflows.md` / `08-workflow-registry.md` extensively use `cloacina-ctl` (legacy binary name).
  - `01-first-workflow.md:75-77` cites `ctor` as required dep — removed by I-0096.
  - `05-cron-scheduling.md:60` Cargo.toml includes `ctor = "0.2"` — removed by I-0096.
  - `_index.md:25, 31`, `architecture-overview.md:36, 52` use the old `cloacinactl serve`/`cloacinactl daemon <flags>` syntax.
- **New-doc proposals:** 3 (decommission-a-tenant, subscribe-workflow-to-reactor, invoke-computation-graph-from-workflow) plus 1 stub (durable-event-log, deferred to CG auditor).
- **May 2026 surface coverage gaps in current workflow docs:**
  - **I-0096:** macro-system.md notes it; tutorials 01/05 still mention `ctor`. Sweep needed.
  - **I-0098/T-0538:** architecture-overview, _index.md, 07/08, cleaning-up-events all need verb form updates.
  - **I-0099/I-0108:** observe-execution-state covers it; cron-scheduling.md contains fabricated metrics types.
  - **I-0100:** **uncovered** in workflow docs — needs new subscribe-workflow-to-reactor.md plus reference/macros update.
  - **I-0101:** **uncovered** in workflow docs — needs new invoke-computation-graph-from-workflow.md plus reference/macros update.
  - **I-0102:** partially covered in migrating-to-service-mode and reference/macros; tutorials 07/08 need rewrite.
  - **I-0106:** partially covered in multi-tenant-setup/recovery and tutorial 06; needs new decommission-a-tenant how-to.
  - **I-0107:** **uncovered** in monitoring-executions; needs rewrite.
  - **I-0110:** **uncovered** in context-management, guaranteed-execution-architecture, task-execution-sequence, errors reference — needs broad edits.
  - **T-0487:** **uncovered** in task-execution-sequence, task-deferral, guaranteed-execution-architecture.
  - **T-0502:** **uncovered** in architecture-overview, task-execution-sequence, guaranteed-execution-architecture.
  - **T-0529/T-0532:** tutorial 06 cross-link to Python tutorial needs verification; tutorial 09 Python section needs verification.
  - **T-0602:** cross-link target for tutorial 09 and new subscribe-workflow-to-reactor.md (primary doc is CG-side).
- **Effort distribution:** 11 S, 13 M, 6 L (of 30 existing-doc entries) plus 2 M and 1 S for new docs and 1 stub.
