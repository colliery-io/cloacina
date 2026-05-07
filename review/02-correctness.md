# Correctness Review

## Summary

Cloacina is a system whose correctness story rests on a tight loop: an executor that atomically claims tasks, heartbeats while it owns them, and lets a sweeper recover anything left behind. That loop is, on the whole, well-built — claim/heartbeat/release are guarded by a `claimed_by` filter on the critical paths, the dedup-events test (`event_dedup.rs:54-119`) pins down the post-T-0474 single-finalizer invariant, and the `claim_loss_cancellation.rs` integration test demonstrates two layers of cooperative + forced cancellation. The DAL is symmetric across PG and SQLite with paired `*_postgres`/`*_sqlite` arms and a `dispatch_backend!` switch, which keeps backend drift visible.

That said, the system is leaking correctness in three families. **(1) FFI & `unsafe Send/Sync` for `PluginHandle`** — the `LoadedWorkflowPlugin` and `LoadedGraphPlugin` types claim Send/Sync over a `libloading::Library` plus a serializing `Mutex`, and packaged cdylibs each spin up their own `tokio::Runtime` and call `.block_on()` from inside `tokio::task::spawn_blocking` on the host. That works but is structurally fragile, and `mem::forget(temp_dir)` on every package load (`registry/reconciler/loading.rs:96`, `package_loader.rs:581`) leaks a tempdir per load by design — fine for a one-shot daemon, gradual pollution if a process churns through versions. **(2) Best-effort persistence and silent error swallowing** — `let _ =` is a deliberate idiom in the reactor (16 sites in `reactor.rs`) and accumulator paths, and `context.insert`/`update` failures during dependency context merging are silently dropped (`thread_task_executor.rs:229,286,289`). The `let _ = conn.interact(...)` in `get_connection_with_schema` (`database/connection/mod.rs:550-557`) is the most worrying — a multi-tenant workload can silently fall through to the public schema on any one transient SET failure. **(3) Test smoke vs. behaviour** — most CG, scheduler, and recovery tests rely on absolute `tokio::time::sleep` durations (50ms–1.5s, dozens of sites in `computation_graph.rs`), and the fidius integration tests silently `return` when the prebuilt dylib is missing rather than fail. There are zero integration tests for race-y double-dispatch, no DST cron tests, no test for `mark_failed`/`mark_completed` returning `false` (the claim-lost path), and the Python retry test (`test_scenario_11`) doesn't actually exercise a retry.

The dominant theme: the happy path is well-tested and the obvious atomicity primitives are in place, but the **partial-failure surface** — claim-lost mid-write, persistence failure that rolls forward instead of back, FFI panics, schema-set failures, dispatcher back-pressure, mid-graph crashes for `Latest`-strategy reactors — is mostly invisible to the test suite, and the production code generally chooses "log and continue" over "stop and surface." For a workflow orchestrator advertising guaranteed execution, that gap will produce silent data loss and orphan rows, not panics, which is the worst kind of failure to debug.

## Test Coverage Assessment

**Unit tests** are concentrated in DAL, executor merge logic, dirty-flag math, and CG primitive types. They are real assertions, not smoke tests, and `failure_reason` cardinality is pinned with a positive test (`thread_task_executor.rs:1034`).

**Integration tests** under `crates/cloacina/tests/integration/` cover scheduler basic_scheduling, dependency_resolution, trigger_rules, stale_claims, executor task_execution, claim_loss_cancellation, multi_tenant, defer_until, signing trust_chain, packaging, fidius_validation, computation_graph, event_dedup, registry workflow_registry, dal sub_status/execution_events, and primitive_only_packaging. Coverage breadth is strong for happy paths.

**Notable gaps**:
- **No double-dispatch race test** — `get_ready_for_retry` (`claiming.rs:768`) doesn't filter `claimed_by IS NULL` and the dispatcher path's `claim_for_runner` doesn't transition Ready→Running, so a Ready task can be picked up by two SchedulerLoop ticks; only the atomic claim filter saves us. No test asserts this is safe.
- **`mark_failed`/`mark_completed` returning `false`** — the whole point of the post-T-0474 invariant, but no test forces the path where the runner_id filter matches zero rows (claim was stolen).
- **DST / TZ transitions** — cron evaluator has zero tests for spring-forward / fall-back behaviour, despite shipping with `chrono-tz`.
- **FFI panic propagation** — no test loads a cdylib whose plugin method panics; we don't know whether fidius converts that to `CallError` or aborts the host.
- **Reactor `WhenAll` after partial fire** — no test interleaves boundaries during graph execution to exercise the dirty-flag clear/cache-update race documented below (COR-09).
- **Sweeper grace period, real clock** — `stale_claims.rs:173` sleeps 1.5s + 1.1s wall-clock and asserts on heartbeat age. CI under load will flake here.
- **CronRecoveryService** — only `Default` constructor tested; no integration test creates a lost execution row, runs the recovery loop, and asserts re-execution.
- **fidius_validation tests skip silently** — `find_packaged_workflow_dylib()` returns `None` if the example is unbuilt, and the test `return`s with `eprintln!` (`fidius_validation.rs:95`). On CI, a missing fixture build = green test. They should `panic!` or be `#[ignore]`'d by name.
- **Python tests are smoke** — `test_scenario_11_retry_mechanisms.py` configures a retry policy on a task that doesn't fail and asserts only completion (`test_scenario_11_retry_mechanisms.py:30-39`). That's not a retry test.
- **`#[ignore]` cluster** — every signing test under `signing/trust_chain.rs` and `signing/security_failures.rs` is `#[ignore = "Requires database connection"]`. Those gates skip the meaningful security paths from the default test run; only `angreal test integration` exercises them.

## Key Risk Areas

- **Multi-tenant schema search-path silent fallback** (`database/connection/mod.rs:550`) — single transient SET failure → tenant queries hit `public`. No test, no metric, no alarm.
- **Reactor `Latest`-strategy clear/snapshot race** (`reactor.rs:591-592`) — a boundary arriving between the cache snapshot and the dirty-flag clear has its dirty bit cleared and its cache update lost from the snapshot. Next signal evaluation sees `any_set()=false`. Subsequent boundaries recover, so it's a fire-skip not data loss, but `WhenAll` semantics get weirder.
- **FFI shell macro panic-on-runtime-init** (`cloacina-workflow-plugin/src/lib.rs:223`) — `.expect("Failed to create cdylib tokio runtime")` aborts the whole host process if Tokio init fails inside a packaged cdylib (e.g., resource limits exhausted). No graceful degradation.
- **`get_ready_for_retry` doesn't filter on `claimed_by`** (`claiming.rs:768`) — relies entirely on atomic claim to dedupe; a slow claim path could double-execute Layer-2 setup before the claim guard fires. Has not been observed in practice but the design comment is missing.
- **`release_runner_claim` ignores ownership** (`claiming.rs:605-670`) — clears `claimed_by` and `heartbeat_at` for the task regardless of who owns the claim. If runner A's `release_runner_claim` lands after runner B has stolen the claim (post-stale-claim sweep), B's claim is released. Mitigated by `mark_completed`/`mark_failed` already being claim-guarded, but the bare release is a footgun.
- **`mem::forget(temp_dir)` per package load** (`reconciler/loading.rs:96`, `package_loader.rs:581`) — by design for daemon lifetime, but disk-fills if a daemon hot-reloads many package versions. No metric or warning.
- **SQLite migrations 006/007 use DROP+RECREATE** — explicitly contradicts the project memory `feedback_sqlite_migration_recreate.md` ("avoid DROP+CREATE; prefer ADD COLUMN + CREATE INDEX"). Existing DBs with these migrations applied are fine, but the pattern is in the codebase and will be copied.

## Findings

### COR-01: Multi-tenant schema search_path failure silently routes queries to public schema

**Severity**: Critical
**Location**: `crates/cloacina/src/database/connection/mod.rs:544-558`
**Confidence**: High

#### Description

`get_connection_with_schema` (used by `get_postgres_connection` for every Postgres query) attempts to set `search_path` on each acquired connection but uses `let _ = conn.interact(...).await` — silently ignoring any error from the SET command. If the `SET search_path` interaction fails (deadpool error, transient connection issue, malformed schema name despite validation), the connection is returned with whatever search_path the pooled connection had (likely `public` from the previous use, or the database default). Queries then hit the wrong tenant's tables — or, in the cross-tenant case, leak data across tenants.

This is the **load-bearing primitive of multi-tenant isolation**. The schema isolation model (per the system overview §6) hinges on every connection having `search_path TO {tenant}, public`. Silent failure here defeats the entire model.

#### Evidence

```rust
// crates/cloacina/src/database/connection/mod.rs:548-557
if let Ok(validated) = validate_schema_name(schema) {
    let schema_name = validated.to_string();
    let _ = conn
        .interact(move |conn| {
            let set_search_path_sql =
                format!("SET search_path TO {}, public", schema_name);
            diesel::sql_query(&set_search_path_sql).execute(conn)
        })
        .await;
}
```

The result of `interact` is dropped twice — outer `interact` Result and inner Diesel Result.

#### Suggested Resolution

Make this fail-closed: propagate any error from the SET as a connection-pool error so the caller gets a proper failure rather than a silently misrouted query. Alternatively, wrap the connection in a custom guard that asserts `current_schemas()` matches the expected tenant on first query, panicking on mismatch (defense in depth).

**Cross-cutting note**: Security review should look at this too — it's a tenant-isolation breach risk.

### COR-02: `release_runner_claim` is unguarded; can release another runner's claim

**Severity**: Major
**Location**: `crates/cloacina/src/dal/unified/task_execution/claiming.rs:605-670`
**Confidence**: High

#### Description

`release_runner_claim(task_id)` clears `claimed_by` and `heartbeat_at` for any task with the given id, with **no filter on ownership**. The companion `claim_for_runner` is properly guarded (`filter(claimed_by.is_null())`), and `mark_completed`/`mark_failed` accept an optional `runner_id` filter — so the path is asymmetric.

Concrete failure case:
1. Runner A claims task T, starts heartbeat.
2. Heartbeat stalls (network blip ≥ stale_threshold).
3. StaleClaimSweeper releases A's claim, marks T Ready, T is re-dispatched to runner B.
4. Runner A's heartbeat returns successfully (or A finishes the task).
5. Runner A's executor reaches the `release_runner_claim` line at `thread_task_executor.rs:990` and unconditionally clears B's claim.
6. B is now running with no claim guard; B's heartbeat will now report `ClaimLost` (its filter `claimed_by.eq(Some(B))` won't match because A cleared it). B's task is cancelled mid-flight.

The `mark_completed`/`mark_failed` path is claim-guarded, so step 5 happens AFTER A confirmed it had ownership — meaning A's release is "correct" if A actually did the work. But because the executor unconditionally calls `release_runner_claim` on its way out (`thread_task_executor.rs:986-998`), the asymmetry exists.

#### Evidence

```rust
// crates/cloacina/src/dal/unified/task_execution/claiming.rs:629-639
async fn release_runner_claim_postgres(...) -> Result<(), ValidationError> {
    ...
    diesel::update(task_executions::table.find(task_id))    // no claimed_by filter!
        .set((
            task_executions::claimed_by.eq(None::<UniversalUuid>),
            task_executions::heartbeat_at.eq(None::<UniversalTimestamp>),
            ...
        ))
        .execute(conn)
}
```

Compare to `mark_completed_postgres` at lines 83-95 which gates updates on `claimed_by.eq(Some(rid))`.

#### Suggested Resolution

Add an optional `runner_id: Option<UniversalUuid>` parameter to `release_runner_claim`, mirror the `mark_completed` shape, and require the executor to pass `self.instance_id`. Returning a bool indicating whether the release applied lets callers detect & log "claim was stolen mid-execution" cleanly.

### COR-03: Reactor `Latest` strategy: dirty-flag clear races boundary arrival; fire can be skipped

**Severity**: Major
**Location**: `crates/cloacina/src/computation_graph/reactor.rs:591-593`
**Confidence**: Medium

#### Description

In `InputStrategy::Latest` mode, the executor task does:

```rust
let snapshot = cache_exec.read().await.snapshot();          // read lock dropped
dirty_exec.write().await.clear_all();                       // separate write lock
let result = (graph)(snapshot).await;                       // graph runs
```

Between the snapshot read and the dirty-flag write, the receiver task (running concurrently) may have:
1. Acquired `cache_exec.write()` and updated cache with a new boundary (line 533).
2. Acquired `dirty_exec.write()` and set the source's dirty bit (line 534).

When the executor's `clear_all()` then runs, it clears the just-set dirty bit. The new cache value IS in the cache, but the next `BoundaryReceived` signal will check `any_set()` → false (until yet another boundary arrives), so the fire that should have happened for the in-flight boundary is **skipped**. For `WhenAll`, the consequence is worse: every source has to re-dirty before the next fire, and one source whose update arrived during the window is permanently masked until it sends again.

This isn't data loss in the cache, but it's a violation of the "every boundary triggers a re-evaluation" implicit contract, and it's especially bad for `WhenAll` semantics where a single missed dirty bit gates the entire graph from firing.

#### Evidence

The `test_reactor_cache_snapshot_isolation` test only checks that cache snapshots are decoupled from cache mutations; it doesn't interleave boundaries during graph execution.

#### Suggested Resolution

Take the dirty lock and capture-then-clear flags as a single atomic operation paired with the snapshot — for example:

```rust
let (snapshot, _cleared) = {
    let mut dirty = dirty_exec.write().await;
    let cache = cache_exec.read().await;
    let snap = cache.snapshot();
    let cleared = std::mem::replace(&mut *dirty, DirtyFlags::with_sources(&expected));
    (snap, cleared)
};
```

This ensures that any boundary arriving after the snapshot capture sees a fresh dirty set. Alternatively, snapshot-and-clear under a single `dirty.write()` lock paired with `cache.read()` (matching reads to writes via lock ordering).

### COR-04: `cloacina::package!()` shell macro: `.expect()` on tokio Runtime init aborts host

**Severity**: Major
**Location**: `crates/cloacina-workflow-plugin/src/lib.rs:223,341,486,581`
**Confidence**: High

#### Description

The unified package shell creates a per-cdylib tokio Runtime in `OnceLock` and panics with `.expect("Failed to create cdylib tokio runtime")` if init fails. Because this runs inside the loaded cdylib (called from the host's `tokio::task::spawn_blocking` path), a panic inside the cdylib unwinds across the FFI boundary back into fidius. fidius will translate this to a `CallError`, but only if the panic is caught — and the standard fidius pattern for `plugin_impl` doesn't always catch panics in nested `OnceLock::get_or_init` (the closure call site is opaque to fidius's catch_unwind layer).

Even if fidius does catch it, a packaged workflow that arrived as a zero-length cdylib, ran out of file descriptors at runtime init, or hit an OS thread limit will mark the whole package as broken on first execute and then keep failing forever — `OnceLock` poisons on panic so the next call hits the same path.

#### Evidence

```rust
// crates/cloacina-workflow-plugin/src/lib.rs:217-224
let rt = CDYLIB_RUNTIME.get_or_init(|| {
    cloacina_workflow::__private::tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .thread_name("package-shell-cdylib-worker")
        .build()
        .expect("Failed to create cdylib tokio runtime")
});
```

Same pattern at lines 333-340 (CG), 480-487 (trigger), 577-583 (trigger-less CG).

#### Suggested Resolution

Replace `OnceLock<Runtime>` with `OnceLock<Result<Runtime, String>>` (or a custom error enum) so the init path can fail without panicking; on subsequent calls, return the cached error as a `PluginError`. This keeps the host alive and gives operators a stable failure to act on.

### COR-05: `mem::forget(temp_dir)` leaks per package load; eats disk in long-running daemon

**Severity**: Major
**Location**: `crates/cloacina/src/registry/reconciler/loading.rs:96`, `crates/cloacina/src/registry/loader/package_loader.rs:581`
**Confidence**: High

#### Description

When a Rust cdylib package is loaded for trigger metadata extraction, the cdylib is written to a `tempfile::TempDir`, dlopen'd, and the tempdir is deliberately leaked via `std::mem::forget(temp_dir)` — comment at line 92-95 says "OS reclaims on process exit; for the long-running daemon/server use case this matches the intended load lifecycle." That works for one package per process lifetime, but `RegistryReconciler` reloads packages on every reconcile when content hashes change, and the daemon is a "long-running" process by design. Each reload writes another `~/tmp/.tmpXXX/trigger_plugin.so` (potentially MB) and never cleans up.

There's no metric for tempdir count, no upper bound, and no test for what happens at the 50th or 500th reload.

#### Evidence

```rust
// crates/cloacina/src/registry/reconciler/loading.rs:91-97
// Leak the temp_dir so the file path stays valid for the lifetime
// of the dlopen handle. The OS reclaims on process exit; for the
// long-running daemon/server use case this matches the intended
// load lifecycle (one tempdir per package, dropped on full process
// restart).
std::mem::forget(temp_dir);
```

`LoadedWorkflowPlugin` (`task_registrar/dynamic_task.rs:38`) holds a `_temp_dir: tempfile::TempDir` field that DOES drop with the plugin handle — the better pattern. The `load_plugin_handle_from_bytes` helper at `loading.rs:70` doesn't do this, instead leaking. Two paths, divergent behaviour.

#### Suggested Resolution

Mirror the `LoadedWorkflowPlugin` pattern: return a guard struct that owns both the `PluginHandle` and the `TempDir`, and drop them together when the package unloads. The reconciler already tracks per-package state (`PackageState`) — wire the trigger plugin handles into that lifecycle.

### COR-06: Cron recovery `chrono::Duration::from_std(...).unwrap()` panics on misconfigured `max_recovery_age`

**Severity**: Minor
**Location**: `crates/cloacina/src/cron_recovery.rs:212`
**Confidence**: High

#### Description

`if execution_age > chrono::Duration::from_std(self.config.max_recovery_age).unwrap()` will panic if `max_recovery_age` exceeds chrono::Duration's signed-i64-millisecond limit (~292 million years; not realistic for misconfiguration but a `Duration::from_secs(u64::MAX)` from a CLI arg parse is theoretically reachable). Other callers handle this defensively with `.unwrap_or(chrono::Duration::seconds(60))` (e.g., `claiming.rs:701,741`). One bare `.unwrap()` is inconsistent with the surrounding pattern.

#### Evidence

```rust
// crates/cloacina/src/cron_recovery.rs:212
if execution_age > chrono::Duration::from_std(self.config.max_recovery_age).unwrap() {
```

vs.

```rust
// crates/cloacina/src/dal/unified/task_execution/claiming.rs:701
let cutoff = UniversalTimestamp(
    chrono::Utc::now()
        - chrono::Duration::from_std(threshold).unwrap_or(chrono::Duration::seconds(60)),
);
```

#### Suggested Resolution

Replace with the same `unwrap_or` fallback pattern. Better: change `CronRecoveryConfig::max_recovery_age` to `chrono::Duration` directly so the conversion happens at config parse time with a typed error.

### COR-07: SQLite migrations 006 and 007 use DROP+CREATE; contradicts project policy

**Severity**: Minor
**Location**: `crates/cloacina/src/database/migrations/sqlite/006_add_storage_type_column/up.sql:30-34`, `crates/cloacina/src/database/migrations/sqlite/007_add_pause_support/up.sql:36-40`
**Confidence**: High

#### Description

Project memory `feedback_sqlite_migration_recreate.md` instructs: "Avoid DROP+CREATE SQLite migrations; prefer ADD COLUMN + CREATE INDEX." Migrations 006 and 007 do exactly the discouraged pattern: create new table, copy data, drop old, rename new. The reason given in the migration body ("SQLite doesn't support DROP CONSTRAINT") is true but `006` only adds a column with a default — `ALTER TABLE workflow_packages ADD COLUMN storage_type TEXT NOT NULL DEFAULT 'database'` works in SQLite back to 3.2.0. `007` similarly only adds two nullable columns and tightens a CHECK constraint.

The risk is data-corruption-on-migration if interrupted mid-rename — the original table is gone but the rename hasn't committed. SQLite's autocommit handling in diesel's `MigrationHarness` should wrap the migration in a transaction, but the rename trick is fragile under power loss in ways that ADD COLUMN isn't.

Existing installations are fine (the migrations have run); this is a "don't repeat this pattern" finding for future migrations.

#### Evidence

The migration script for 006:
```sql
CREATE TABLE workflow_packages_new (...);
INSERT INTO workflow_packages_new SELECT ...;
DROP TABLE workflow_packages;
ALTER TABLE workflow_packages_new RENAME TO workflow_packages;
```

#### Suggested Resolution

Document on these migrations that the pattern is deprecated and should not be copied. For the next column addition, use `ALTER TABLE ... ADD COLUMN`. If a CHECK constraint update is genuinely required, prefer adding a runtime validation in the DAL rather than rewriting the schema.

### COR-08: `release_runner_claim` happens AFTER state transition; heartbeat task may still be alive when it lands

**Severity**: Minor
**Location**: `crates/cloacina/src/executor/thread_task_executor.rs:907-998`
**Confidence**: Medium

#### Description

The executor's flow on task completion is:
1. `complete_task_transaction` (mark_completed, save context) — lines 911-944.
2. `heartbeat_handle.abort()` — line 908. Wait, this is BEFORE `complete_task_transaction` (lines 911+). Re-reading...

Actually order is: heartbeat_handle.abort (908), then result-handling (911-983), then release_runner_claim (986-998).

The abort fires at line 908. Tokio `JoinHandle::abort()` is a request, not a synchronous stop — the heartbeat task may run one more loop iteration before observing the abort. If that iteration runs `dal.task_execution().heartbeat(...).await` and the runner claim is now gone (because `mark_completed` set status to Completed, but the claim_by guard on heartbeat is `claimed_by.eq(Some(runner_id))` and that's still true until `release_runner_claim` runs), the heartbeat returns `Ok(HeartbeatResult::Ok)` for a task that's already Completed. That's just a wasted DB write and a benign log line — no correctness violation observed — but it indicates the lifecycle ordering hasn't been thought through holistically.

If the order were reversed (release first, then abort), heartbeats would observe `ClaimLost` and try to fire the cancel channel for an already-completed task, which is harmless but wasted work.

#### Evidence

```rust
// crates/cloacina/src/executor/thread_task_executor.rs:906-909
// Stop heartbeat and release claim after execution (success or failure)
if let Some(handle) = heartbeat_handle {
    handle.abort();
}
```

The abort is fire-and-forget; no `await` on the handle.

#### Suggested Resolution

Either `let _ = handle.await` after abort to wait for the task to finish (gracefully, with the abort interrupting it), or restructure so heartbeat takes a `watch::Receiver<bool>` shutdown signal that's flipped synchronously before the state transition, with the abort as fallback. The current pattern works but the "best-effort" timing is brittle.

### COR-09: Test suite has zero double-dispatch race coverage

**Severity**: Major
**Location**: `crates/cloacina/src/execution_planner/scheduler_loop.rs:248-281` (dispatch_ready_tasks), `crates/cloacina/src/dal/unified/task_execution/claiming.rs:768` (get_ready_for_retry)
**Confidence**: High

#### Description

`SchedulerLoop::dispatch_ready_tasks` iterates `get_ready_for_retry()` results and calls `dispatcher.dispatch(event).await`. Two SchedulerLoop ticks running in parallel (or two SchedulerLoop instances if there's ever a multi-runner deployment) would both load the same Ready task and both dispatch. The atomic claim in `claim_for_runner` is the only thing preventing duplicate execution. That's correct — `RunnerClaimResult::AlreadyClaimed` returns a "skipped" `ExecutionResult`. But:

- There's **no integration test** that simulates two concurrent SchedulerLoop ticks dispatching the same Ready task and asserts only one runs to completion.
- The `dispatcher.dispatch` returns `DispatchError::NoCapacity` or `ExecutorNotFound` and the loop just `warn!`s and moves on (`scheduler_loop.rs:271-277`). The task remains Ready and the next tick will try again, but nothing ensures forward progress under sustained back-pressure — `NoCapacity` could spin a tight loop pumping CPU in `dispatcher.dispatch` calls that immediately fail.
- `get_ready_for_retry` doesn't filter on `claimed_by IS NULL`, so a task that was claimed (status still Ready, claimed_by set) but not yet transitioned to Running by `claim_pending_tasks` (the polling-path-only transition) will be re-dispatched. Mitigated by `claim_for_runner`'s null filter, but the SQL surface is broader than necessary.

The push-based `claim_for_runner` path never sets `status="Running"` — that happens only in the polling path's `claim_pending_tasks` (`claiming.rs:373`). So tasks dispatched via the push path stay in `status="Ready"` for their entire execution lifetime, and `get_ready_for_retry` keeps including them in its results until they hit a terminal state.

#### Evidence

```rust
// crates/cloacina/src/dal/unified/task_execution/claiming.rs:786-795
task_executions::table
    .filter(task_executions::status.eq("Ready"))
    .filter(
        task_executions::retry_at
            .is_null()
            .or(task_executions::retry_at.le(now)),
    )
    .load(conn)
```

No `claimed_by.is_null()` filter. The dispatcher path's atomicity rests entirely on `claim_for_runner`'s `filter(claimed_by.is_null())`, which is correct, but the design is fragile to refactor.

#### Suggested Resolution

(a) Add `.filter(task_executions::claimed_by.is_null())` to `get_ready_for_retry` so the dispatch fan-in only sees genuinely-unclaimed work. (b) Add an integration test that spawns two SchedulerLoop instances pointing at the same DB, marks N tasks Ready, and asserts each runs exactly once.

### COR-10: `complete_task_transaction` "CRITICAL: mark_completed succeeded but context save failed" leaves partial state

**Severity**: Major
**Location**: `crates/cloacina/src/executor/thread_task_executor.rs:514-525`
**Confidence**: High

#### Description

`complete_task_transaction` marks the task Completed in DB, then calls `save_task_context` separately. If `save_task_context` fails AFTER `mark_completed` succeeded, the comment is loud:

```rust
"CRITICAL: mark_completed succeeded but context save failed"
```

— but the function returns `Err(e)`, which the caller (line 928-942) maps to a Failed execution result. The DB now has:
- `task_executions.status = "Completed"`
- `task_executions.completed_at` = now
- No `task_execution_metadata` row → no context saved
- No `contexts` row for this task

Downstream tasks that depend on this one will, in `ContextManager::load_context_for_task` (`context_manager.rs:101-138`), find no metadata for the dependency, return an empty `Context::new()` (line 127), and run with no upstream context. That's silent data loss — not a crash, not a failed workflow, just an empty context propagating downstream.

There's also a metrics asymmetry — `mark_completed` already incremented `cloacina_tasks_total{status=completed}`, but the executor calls `mark_failed` after this path on line 935, which is itself rejected (the row is already `Completed`, the `claimed_by` filter still matches, but the `set status=Failed` overwrite is permitted because `mark_failed` doesn't filter on current status either).

#### Evidence

```rust
// crates/cloacina/src/executor/thread_task_executor.rs:516-525
if let Err(e) = self.save_task_context(claimed_task, context).await {
    error!(
        task_id = %claimed_task.task_execution_id,
        ...
        "CRITICAL: mark_completed succeeded but context save failed"
    );
    return Err(e);
}
```

The downstream consumer's reaction to missing context:
```rust
// crates/cloacina/src/execution_planner/context_manager.rs:130-137
Err(_) => {
    // Dependency task hasn't completed yet or no metadata saved
    debug!(
        "Context loaded: empty (dependency '{}' not found)",
        dep_task_name
    );
    Ok(Context::new())
}
```

Returns empty context, not error.

#### Suggested Resolution

Reverse the ordering: save context first (with task-id reference), then `mark_completed` only if context save succeeded. If the context-save fails, the task remains in its prior state (Running, claimed) and the heartbeat keeps the claim alive — the next sweeper cycle marks it Failed with a meaningful error. Alternatively, wrap both writes in a single Diesel transaction so partial writes can't happen at all. The current ordering (status first, context after) was presumably chosen for the claim guard; that guard can apply equally if the order is reversed using a `WHERE claimed_by = $1 AND status = 'Running'` filter on the context save.

### COR-11: Silent JSON parse and context-merge errors swallow upstream data corruption

**Severity**: Major
**Location**: `crates/cloacina/src/executor/thread_task_executor.rs:271-296`, `crates/cloacina/src/execution_planner/context_manager.rs:155-189`
**Confidence**: High

#### Description

Two adjacent paths build dependency context for a task. Both swallow errors silently:

`thread_task_executor.rs:271-296` (the production path): `if let Ok(dep_context) = Context::<serde_json::Value>::from_json(json_str)` — if the dependency's saved context is malformed JSON (for example, written by a different version that used a different serialization format), the dependency is silently skipped, its keys never reach the downstream task, and the only signal is a `debug!("Failed to parse dependency context JSON")`.

`context_manager.rs:155-189`: `if let Ok(task_metadata) = ...` and `if let Ok(dep_context) = ...` — same pattern; failure at either layer skips the dependency without any visible error.

Both also use `let _ = context.insert(key, value)` (lines 229, 286, 289) which swallows even the in-memory insert failure (e.g., from a key validation rejection).

For an orchestrator whose primary value prop is "context flows correctly through DAGs," silent data loss in the context merge layer is a correctness foot-gun.

#### Evidence

```rust
// thread_task_executor.rs:271-294
for (_task_metadata, context_json) in dep_metadata_with_contexts {
    if let Some(json_str) = context_json {
        if let Ok(dep_context) = Context::<serde_json::Value>::from_json(json_str) {
            ...
            for (key, value) in dep_context.data() {
                if let Some(existing_value) = context.get(key) {
                    let merged_value = Self::merge_context_values(existing_value, value);
                    let _ = context.update(key, merged_value);
                } else {
                    let _ = context.insert(key, value.clone());
                }
            }
        } else {
            debug!("Failed to parse dependency context JSON");
        }
    }
}
```

#### Suggested Resolution

Promote these failures to errors: parse failures should become `ExecutorError::ContextLoadFailed` so the task fails-fast with a meaningful error rather than running with a stripped context. Same for `insert`/`update` returning errors. If the goal is best-effort merging (e.g., one bad dependency shouldn't kill all downstreams), at least surface a metric `cloacina_context_merge_failures_total{kind=...}` so this is observable.

### COR-12: API key cache TTL allows revoked keys to authenticate for up to 30 seconds

**Severity**: Minor
**Location**: `crates/cloacina-server/src/routes/auth.rs:60-78`, `crates/cloacina-server/src/routes/keys.rs:181`
**Confidence**: High

#### Description

`KeyCache` has a 30-second TTL and is cleared on revoke (`keys.rs:181 — state.key_cache.clear().await`). The full clear is a blunt instrument — every other tenant's cached keys are wiped — but it's correct. However:

- The clear happens AFTER `dal.api_keys().revoke_key(id).await` returns. There's a small window where the DB row is marked revoked but the cache still serves the old `ApiKeyInfo`.
- More importantly, **the fine-grained `evict()` method** at `auth.rs:106-110` is marked `#[allow(dead_code)]` — implying it was intended to evict by hash but is unused. The blunt `clear()` hides this dead code.
- For multi-server deployments (two `cloacina-server` instances behind a load balancer), each server has its own in-memory cache. Revoking a key on server A clears A's cache but server B continues to serve the cached key for up to 30s. There's no inter-server signaling.

The 30s window is small but defeats the implicit "revoke is immediate" expectation.

#### Evidence

```rust
// crates/cloacina-server/src/routes/auth.rs:106-110
/// Evict a specific key (used after revocation).
#[allow(dead_code)]
pub async fn evict(&self, hash: &str) {
    let mut cache = self.cache.lock().await;
    cache.pop(hash);
}
```

The doc says "used after revocation" but the only revocation path uses `clear()`.

#### Suggested Resolution

For single-server: switch revocation to `evict(&hash)` since that's what the dead method exists for. For multi-server: either drop caching (DB lookup per request is fine at cloacinactl scale), or implement a Postgres `LISTEN/NOTIFY` channel for revocation events that all servers subscribe to. Document the multi-server limitation in the meantime.

**Cross-cutting note**: Security review territory.

### COR-13: Sweeper grace-period and stale-claim tests rely on real wall-clock sleeps; flaky on CI

**Severity**: Minor
**Location**: `crates/cloacina/tests/integration/scheduler/stale_claims.rs:143,173,178,206,209`
**Confidence**: High

#### Description

`test_sweep_resets_stale_task_to_ready` sleeps 1500ms + creates a 1s-threshold sweeper + sleeps 1100ms more — total >2.6s wall-clock for a single test, with hard assertions on heartbeat-age comparison. On a busy CI runner, the second sleep can race the heartbeat-age comparison (the sweeper checks `chrono::Utc::now() - claim.heartbeat_at` against a 1s threshold; if scheduler scheduling delays push real time past the assumption, the test flakes either way).

The reactor tests in `crates/cloacina/tests/integration/computation_graph.rs` are far worse — 30+ `tokio::time::sleep(...)` calls of 50-300ms each, with no use of `tokio::time::pause()` or `tokio::time::advance()`.

#### Evidence

```rust
// crates/cloacina/tests/integration/scheduler/stale_claims.rs:172-181
// Wait for heartbeat to age past the stale threshold
tokio::time::sleep(Duration::from_millis(1500)).await;
let sweeper = test_sweeper(dal.clone(), Duration::from_secs(1));
tokio::time::sleep(Duration::from_millis(1100)).await;
sweeper.sweep().await;
let task = dal.task_execution().get_by_id(task_id).await.unwrap();
assert_eq!(task.status, "Ready", "Stale task should be reset to Ready");
```

#### Suggested Resolution

Use `tokio::time::pause()` + `advance()` for time-sensitive logic, or inject the clock as a trait so tests can supply a `MockClock`. The sweeper already takes the threshold as a parameter — the missing piece is letting the test drive the clock instead of waiting for real time.

### COR-14: Workflow `final_context` selection by `>` on completion timestamp is non-deterministic on equal times

**Severity**: Observation
**Location**: `crates/cloacina/src/execution_planner/scheduler_loop.rs:411-415`
**Confidence**: Medium

#### Description

`update_execution_final_context` walks all completed tasks and picks the one with the latest `completed_at`. The comparison is `completed_at.0 > latest_completion_time.unwrap()` — strictly greater. Two tasks completing at exactly the same chrono microsecond (which CAN happen — multiple Postgres `now()` calls in the same statement return identical values, and SQLite stores ISO-8601 strings with millisecond precision by default) would have non-deterministic selection because the iteration order of `all_tasks` depends on a query order that may not be defined.

Practically, two tasks rarely complete at the exact same microsecond, but in tests with mocked time or in burst-mode with very fast tasks, it's reachable. The output is one specific task's context being elevated to "the final context" — a workflow whose final context content varies between runs would be hard to debug.

#### Evidence

```rust
// crates/cloacina/src/execution_planner/scheduler_loop.rs:411-415
if latest_completion_time.is_none()
    || completed_at.0 > latest_completion_time.unwrap()
{
    final_context_id = Some(context_id);
    latest_completion_time = Some(completed_at.0);
}
```

#### Suggested Resolution

Add a deterministic tiebreaker — task name or task id — so equal timestamps resolve consistently. Alternative: walk tasks in a deterministic SQL order (`ORDER BY task_name ASC` for example) so the first-found-with-greatest-time is consistent.

### COR-15: Ignored signing tests are the only tests for security-critical code paths

**Severity**: Major
**Location**: `crates/cloacina/tests/integration/signing/trust_chain.rs:29,46,63,79`, `crates/cloacina/tests/integration/signing/security_failures.rs:216`, `crates/cloacina/tests/integration/signing/key_rotation.rs:125`
**Confidence**: High

#### Description

Six signing/security tests are decorated with `#[ignore = "Requires database connection"]`. They cover trust-chain validation, key rotation, and adversarial signature failures — exactly the security boundary you'd want to test. They run only via `angreal test integration` (which sets up Docker Postgres). A developer running `cargo test` or `angreal test unit` skips them silently.

The risk isn't that the tests are wrong (they look thorough) — it's that they're conditionally skipped and the green-CI signal in the most-run command (`unit`) doesn't reflect their state. A regression in trust-chain logic could slip past `ci fast`.

`fidius_validation.rs` has the same shape with a different mechanism: tests `return` silently if the example dylib isn't built (line 92-98), so a missing build artifact = green test.

#### Evidence

```rust
// crates/cloacina/tests/integration/signing/trust_chain.rs:29
#[ignore = "Requires database connection"]
async fn test_trust_chain_validation_one_org() { ... }
```

```rust
// crates/cloacina/tests/integration/fidius_validation.rs:91-98
fn test_metadata_fidelity() {
    let dylib_path = match find_packaged_workflow_dylib() {
        Some(p) => p,
        None => {
            eprintln!("Skipping: packaged-workflows example not built");
            return;
        }
    };
    ...
}
```

#### Suggested Resolution

For DB-dependent tests, use `serial_test::serial` and the `get_all_fixtures()` pattern that other integration tests use — they auto-skip cleanly when no DB is available but are run by default when the fixture is up. For dylib-dependent tests, change `return` to `panic!("test requires dylib at {:?} — build with `angreal test integration` or `cargo build -p packaged-workflow-example`", project_path)` so the missing fixture surfaces as a real failure.

### COR-16: `mark_build_success` and `mark_build_failed` ignore claim ownership; stale builder can overwrite

**Severity**: Minor
**Location**: `crates/cloacina/src/registry/workflow_registry/database.rs:1134-1255`
**Confidence**: Medium

#### Description

The compiler-service build-claim protocol is:
1. `claim_pending_build` — atomic CAS, sets status to `building` + `build_claimed_at` to now (lines 1090-1130).
2. `heartbeat_build` — refreshes `build_claimed_at`, filtered on `build_status = 'building'` (lines 1278-1281).
3. `mark_build_success` / `mark_build_failed` — writes the result. **No filter on claim ownership.**

Suppose builder A claims, runs cargo for 90 seconds, network blip, sweeper resets the row to `pending` after 60s threshold, builder B claims, completes the build successfully, marks success. Builder A's cargo finally finishes and calls `mark_build_success` with A's compiled bytes — overwriting B's compiled artifact.

Compared to the stale-claim sweeper for tasks (`stale_claim_sweeper.rs`) where `release_runner_claim` and `mark_ready` happen, the build claim has no equivalent of the `mark_completed`/`mark_failed` claim-guarded write. A's late completion can clobber B's correct one.

The `compiler` service ought to track its claim id and pass it back on the success/fail call.

#### Evidence

```rust
// crates/cloacina/src/registry/workflow_registry/database.rs:1157-1167
diesel::update(workflow_packages::table.filter(workflow_packages::id.eq(pid)))
    .set((
        workflow_packages::build_status.eq("success"),
        workflow_packages::compiled_data.eq(Some(bytes)),
        ...
    ))
    .execute(conn)
```

No filter beyond `id.eq(pid)`.

#### Suggested Resolution

Add an instance-id or build-claim-id column to `workflow_packages`, set it on claim, filter on it in mark_build_success/failed. Or, simpler: filter on `build_status.eq("building").and(build_claimed_at.eq(claimed_at_value))` where the claim returns a tuple including the timestamp.

### COR-17: Python loader stdlib deny-list checks file names but not nested package modules

**Severity**: Minor
**Location**: `crates/cloacina-python/src/loader.rs:43-69, 196-216`
**Confidence**: Medium

#### Description

`STDLIB_DENY_LIST` has 25 stdlib module names (`os`, `sys`, `subprocess`, etc.) and `validate_no_stdlib_shadowing` walks `workflow_dir` and `vendor_dir`, checking each `.py` file or directory against the list. But it only checks the **immediate name** — a malicious package can still ship `mypkg/os.py` (where `mypkg` itself is benign) and shadow `os` via `from mypkg import os`. The check at `loader.rs:202-212` only examines `entry.file_name()` of top-level entries.

Worse, the check is a "first-level shadowing" check that doesn't walk into subdirectories. A package structure like:
```
workflow_dir/
  workflow.py     # entry module
  utils/          # subpackage, not in deny list
    os.py         # shadows os when imported via "from utils import os"
```
... passes validation but allows the package to do `from utils import os; os.system("rm -rf /")` once imported.

The deny-list approach also assumes the threat model is "stop accidental shadowing" not "stop adversarial code." The README pitches this as a security measure. If it is, a determined attacker with package upload rights can trivially bypass it.

#### Evidence

```rust
// crates/cloacina-python/src/loader.rs:201-213
if let Ok(entries) = std::fs::read_dir(dir) {
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        // Check for module.py or module/ (package directory)
        let module_name = name_str.strip_suffix(".py").unwrap_or(&name_str);
        if STDLIB_DENY_LIST.contains(&module_name) {
            return Err(PythonLoaderError::ImportError(...));
        }
    }
}
```

`std::fs::read_dir` is non-recursive.

#### Suggested Resolution

Either (a) walk recursively and check every `.py` file at any depth, or (b) explicitly downgrade the security claim — this is a "sane defaults" check, not a sandbox. If the security model is "all uploaded packages are trusted by signature" (per the `--require-signatures` flag), the deny-list adds little; remove it or make the security model explicit in docs.

### COR-18: `get_execution_status` silently maps unknown statuses to `Failed`

**Severity**: Minor
**Location**: `crates/cloacina/src/runner/default_runner/workflow_executor_impl.rs:197-205`
**Confidence**: High

#### Description

```rust
let status = match execution.status.as_str() {
    "Pending" => WorkflowStatus::Pending,
    "Running" => WorkflowStatus::Running,
    "Completed" => WorkflowStatus::Completed,
    "Failed" => WorkflowStatus::Failed,
    "Cancelled" => WorkflowStatus::Cancelled,
    "Paused" => WorkflowStatus::Paused,
    _ => WorkflowStatus::Failed,    // ← anything else → Failed
};
```

If a future migration adds a new status (e.g., `Skipped`, `Recovering`) and an old runner reads a row written by a new runner, it reports `Failed` for what is actually a different state. Users polling for completion via `is_terminal()` would see `Failed` and their workflow's actual outcome (e.g., `Recovering`) is silently misreported.

The status enum is type-checked in Rust (`WorkflowStatus`), but the DB column is a string. The deserializer should error on unknown values rather than collapsing them.

#### Evidence

The match is exhaustive in name only — the `_` arm is open. No test asserts that all DB-written statuses are accounted for.

#### Suggested Resolution

Replace the match with a fallible parse that returns `Err(WorkflowExecutionError::ExecutionFailed { message: format!("Unknown status '{}'", s) })` on the wildcard. Add a unit test that round-trips every variant of `WorkflowStatus` through `mark_*` → `get_by_id` → status string parse.

## Positive Patterns

1. **Atomic claim-with-CAS** — `claim_for_runner` (`claiming.rs:424-510`) and `claim_pending_tasks` use `WHERE claimed_by IS NULL` predicates that map cleanly to single-row CAS in both Postgres and SQLite. The pattern is repeated consistently and gives true exactly-once semantics on the claim path.

2. **`event_dedup.rs` regression test** — the test pinning T-0474's "exactly one TaskCompleted per task" invariant (`event_dedup.rs:54-119`) is exemplary: it has a clear name, a documented motivation, and asserts a count not a presence. This is the shape correctness tests should take.

3. **Workflow-completion race guard** — `complete_execution` (`scheduler_loop.rs:288-377`) explicitly re-checks the workflow status after the completion check and returns early if another tick already finalised. The fix to T-0534's gauge leak (re-seeding the gauge from SQL each tick rather than trusting an in-memory counter) is a model recovery from a design mistake.

4. **Dual-layer cancellation** — the heartbeat-driven cancel channel (`thread_task_executor.rs:719-758`) plus `TaskHandle::cancelled()` cooperative observation (`claim_loss_cancellation.rs:73-84`) is well-designed: tasks that ignore cancellation get dropped, tasks that cooperate get clean shutdown. The integration test exercises both layers.

5. **Manifest deny-unknown-fields with friendly migration hints** — `CloacinaMetadata` rejects `package_type` and `[[triggers]]` with explicit migration messages (`reconciler/loading.rs:175-205`). Hard-fail at the boundary, soft-fail in the error message — the right shape for breaking changes that need user action.
