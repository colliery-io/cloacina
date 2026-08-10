---
id: bug-trigger-less-computation
level: task
title: "BUG: trigger-less computation graphs and task-to-graph invocation never compiled in packaged crates — macro emits umbrella-crate paths"
short_code: "CLOACI-T-0897"
created_at: 2026-07-12T01:49:31.319227+00:00
updated_at: 2026-08-09T22:44:28.739648+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# BUG: trigger-less computation graphs and task-to-graph invocation never compiled in packaged crates — macro emits umbrella-crate paths

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

**Finding from T-0891 (2026-07-12):** a packaged crate declaring a trigger-less `#[computation_graph]` or a `#[task(invokes = computation_graph(...))]` failed to compile with `cannot find crate cloacina` — the macros emitted umbrella-crate paths that only resolve for embedded consumers:
1. The trigger-less compiled-fn SIGNATURE hardcoded `&cloacina::Context<Value>` / `cg_runtime_root::GraphResult` in a single emission (`codegen.rs` ~:252), while the ctor + trait impl were already dual-emitted under `cfg(feature = "packaged")` (the T-0552 pattern) — the fn between them wasn't.
2. The `invokes` tail in `tasks.rs` (~:876/:891) matched on `::cloacina::computation_graph::GraphResult` ungated.

Since I-0138 makes packaged the primary shape, task→CG invocation was effectively unshippable. Same macro-portability class as [[feedback_macro_generated_deps_invisible]].

**FIXED (this task, same day):**
- `codegen.rs`: the trigger-less compiled fn is now dual-emitted — `cfg(not(packaged))` via `::cloacina::cloacina_workflow::Context`/`::cloacina::computation_graph::GraphResult`, `cfg(packaged)` via `::cloacina_workflow::Context`/`::cloacina_computation_graph::GraphResult` (host-crate emission unchanged).
- `tasks.rs`: the invoke tail imports a cfg-gated `GraphResult as __CgGraphResult` alias and matches on it.
- Verified: the `cg-feature-tour` packaged example (triggerless CG + invoking task + post_invocation) compiles clean offline against the local crates; host-crate integration tests (T-0538/T-0540) unaffected (is_cloacina branch untouched).

**REMAINING (this task's open tail):** `tasks.rs:741-754` still emits ungated `::cloacina::take_task_handle()` / `return_task_handle` for tasks with a HANDLE parameter — a packaged task using the handle param will hit the same compile error. Route those through the packaged-safe re-export (or dual-emit) and add a packaged fixture using a handle param as the regression net.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

- 2026-08-09 (TAIL RE-SCOPED — the ticket's own suggested fix is not possible;
  maintainer chose full support over a clear error).

  The tail above says to "route those through the packaged-safe re-export (or
  dual-emit)". **There is nothing to re-export.** `TaskHandle` is not a leaf
  type: `crates/cloacina/src/executor/task_handle.rs` depends on `crate::dal::DAL`,
  `UniversalUuid`, `ExecutorError` and `super::slot_token::SlotToken`. Putting it
  in `cloacina-workflow` — the only cloacina crate a packaged cdylib links —
  would drag the whole engine into the small crate and defeat packaging. A
  dual-emit has the same problem: there is no packaged-side path that resolves.

  WHY IT IS ACTUALLY HARD. `defer_until(cond, poll_interval)` interleaves plugin
  code with FOUR host-state operations (`task_handle.rs:170-215`): set
  `sub_status = Deferred` in the DB → `slot_token.release()` → poll the user's
  predicate (this part is plugin code) → `slot_token.reclaim().await` → set
  `sub_status = Active`. The predicate lives in the plugin; the slot and DB live
  in the host.

  **fidius has no plugin→host callback.** `#[fidius::plugin_interface]` defines a
  one-way vtable the HOST calls on the PLUGIN; fidius-core exposes no host-function
  facility. So the plugin cannot ask the host to release its slot mid-execution.
  This is why T-0895's secrets solution does not generalise: that is
  push-at-invocation (host resolves values UP FRONT and ships them in
  `TaskExecutionRequest`), a one-way data push, not a callback.

  fidius is `colliery-io/fidius`, i.e. ours — adding host callbacks IS possible,
  but it is a separate repo needing its own release and a dependency bump here.

  CHOSEN DESIGN — INVERSION, no fidius change:
  1. packaged `defer_until` evaluates `cond()` ONCE.
  2. true → return immediately; the task proceeds. Fast path, no deferral, no
     re-run.
  3. false → unwind and return `Deferred { poll_interval }` to the host.
  4. Host sets `sub_status=Deferred`, RELEASES the slot, sleeps `poll_interval`,
     reclaims, re-invokes `execute_task` with the same context.
  5. Task re-runs, re-evaluates `cond()`, eventually proceeds.

  This keeps the ACTUAL benefit — the concurrency slot is genuinely released
  while waiting — which is the entire point of the feature.

  ACCEPTED SEMANTIC CAVEAT, to be documented loudly: **code before
  `defer_until` re-runs on every poll.** For the canonical use (wait for a file,
  then process it) the prefix is trivial, and cloacina tasks already tolerate
  re-execution because of `retry_attempts`. Any side effect before the defer
  WILL repeat. Embedded semantics are unchanged (it still blocks in place).

  REJECTED: blocking in-process without releasing the slot. It would compile and
  look correct while silently discarding the whole benefit — worse than the
  honest compile error, because the failure is invisible.

- 2026-08-09 (fidius#8 MERGED — API confirmed against source; groundwork landed).

  fidius main is now 55287b1: #8 (`a015a9e`, the callback channel + 0.5.7) plus
  `f4b17ac` adding WASM host functions via a `fidius:host-call` import, so the
  dylib-only v1 caveat in the PR body is already superseded.

  DEPENDENCY GROUNDWORK DONE. fidius was pinned as **11 independent "0.5.6"
  literals across 7 crates** — the exact drift class I-0134 exists to kill, and
  dangerous here specifically: the wire is positional bincode, so two fidius
  versions in one build would mean two incompatible copies of the FFI types.
  Collapsed to ONE `[workspace.dependencies]` entry per crate (maintainer's
  call, and the right one). All six fidius crates now resolve at 0.5.7 from a
  single source; `cloacina-workflow-plugin` compiles unchanged against it,
  confirming the change really is ABI-additive for our existing interface v5.

  API CONFIRMED FROM SOURCE (`fidius/tests/test-plugin-hostcall/src/lib.rs`,
  `crates/fidius-host/tests/host_functions_e2e.rs`) — note the fidius fixture is
  modelled on THIS use case: `TestHost { release_slot, reclaim_slot }` driving a
  `Deferrable` plugin.

    * declare: `#[host_interface(version = 1)] pub trait H: Send + Sync { fn f(&self, a: String) -> Result<T, PluginError>; }`
    * plugin:  `let host = HClient::bound()?; host.release_slot(&id)?;`
    * host:    `load_library(path)` -> `lib.host_imports()` (discovery, carries
      version + FNV-1a hash) -> `HBinding::bind(&lib, Arc<dyn H>) -> bool` ->
      THEN `PluginHandle::from_loaded(plugin)`. Binding happens on the
      LoadedLibrary BEFORE the handle exists — that is the hook point.
    * `HBinding::INTERFACE_VERSION` / `INTERFACE_HASH` for gate assertions.
    * `host_ffi::host_callback_depth()` — per-thread reentrancy probe.

  **Host-function calls are SYNCHRONOUS, not awaited.** A summary I read first
  claimed the generated client methods were `async`; the source says otherwise.
  Checked, because it decides how `defer_until` is written.

  THREADING — a deadlock was raised and then RETRACTED. First reading: the task
  body runs under `rt.block_on(..)` on the thread that called `execute_task`, so
  a synchronous `reclaim_slot` would block a host executor thread while waiting
  for capacity only other tasks can free — a self-deadlock.

  **That is wrong.** `dynamic_task.rs:216` already invokes the plugin via
  `tokio::task::spawn_blocking(move || plugin.execute_task(request))`. The task
  body, and every synchronous host callback made from inside it, therefore runs
  on tokio's BLOCKING POOL (default 512, grows on demand), never on a runtime
  worker. Async workers keep progressing, other tasks keep completing and
  freeing slots, so the deadlock shape does not arise. Raised before checking
  where the call actually happens — the assumption was the error.

  So sync host functions are FINE for this design; fidius does not need an
  async boundary.

  REAL residual cost, benign and worth documenting: a deferred task **holds a
  blocking-pool thread for the whole wait** (it released its concurrency slot,
  but the plugin sits in the poll loop). Many long deferrals means many held
  threads. Bound the reclaim wait as ordinary hygiene, not as deadlock defense.

  And async host functions would NOT fix even that: `execute_task` is itself a
  synchronous FFI method, so the thread is occupied for the task's whole
  lifetime regardless of callback shape. Making a deferred task cost a future
  instead of a thread would require the PLUGIN interface to be async across FFI
  — pollable futures and wakers over a C ABI — a far larger change than host
  functions, and out of scope here.

  HOST-SIDE ARCHITECTURE (decided; the non-obvious part).

  `CloacinaHost` lives in `cloacina-workflow-plugin` — the one crate BOTH sides
  already depend on (host via `cloacina`, packaged cdylibs directly), symmetric
  with `CloacinaPlugin`. `cloacina-workflow` cannot host it: the plugin crate
  depends on it, so referencing the generated client there would be a cycle.

  The hard part is slot ownership. `SlotToken::release/reclaim` take `&mut self`
  and the token is owned by the `TaskHandle` the executor installs in a
  TASK-LOCAL (`thread_task_executor.rs:633 with_task_handle`). The host callback
  arrives on a blocking-pool thread (`dynamic_task.rs:216 spawn_blocking`), where
  that task-local is invisible — and the token cannot simply be copied into a
  registry, because then two places would own it.

  So: **register a command channel, not the token.**
    * `DynamicLibraryTask::execute` creates an mpsc command channel, registers
      `task_execution_id -> Sender` in a process-wide registry, and then runs the
      `spawn_blocking(plugin call)` CONCURRENTLY with a loop that services
      commands using the task-local `TaskHandle` it still owns.
    * `CloacinaHost::release_slot(id)` (running on the blocking thread) looks up
      the sender, sends `Release` plus a oneshot reply, and blocks on the reply.
    * The async side performs `slot_token.release()` / `.reclaim().await` /
      `dal.set_sub_status(..)` and acks.
    * Deregister when the task finishes.

  This keeps `SlotToken` ownership in exactly one place, makes `reclaim` a
  genuine async await on the runtime (never a blocked worker), and leaves the
  blocking-pool thread merely parked on a oneshot — which is what that pool is
  for. It also degrades cleanly: a plugin that never defers registers a channel
  nobody uses.

- 2026-08-09 (IMPLEMENTATION — all layers built; packaged fixture + e2e remain).

  fidius 0.5.7 is PUBLISHED (tagged from cloacina's verification) and cloacina
  now depends on it from crates.io — the temporary local-path override is gone.

  BUILT AND GREEN:
  1. `CloacinaHost` (3 methods) in `cloacina-workflow-plugin`, behind an
     OFF-by-default `host` feature so packaged cdylibs get only the client and
     never drag in fidius-host. Round-trip test passes against the PUBLISHED
     crate: ordered callbacks with arguments intact across bincode, a
     host-raised typed error that surfaces AND is not recorded, double-bind
     refused.
  2. Deferral registry + `EngineHost`. `TaskHandle.slot_token` became
     `Arc<Mutex<SlotToken>>` and the executor registers THAT SAME Arc keyed by
     task-execution UUID; `Arc::ptr_eq` is asserted, because a registry that
     cloned the token would be a double-release bug. Registration brackets the
     task, so a late callback gets `TASK_NOT_RUNNING` instead of touching a dead
     slot. All 10 embedded TaskHandle tests still pass — including the three
     defer_until ones, which was the entire risk of that change. 810 lib tests
     green.
  3. Packaged `TaskHandle` with the same surface (`defer_until`,
     `task_execution_id`), driving the callbacks. Sets sub_status Deferred
     BEFORE releasing so an operator never sees a slotless task reported Active;
     restores Active best-effort, since failing a task over a stale status
     string would be worse than the stale string.
  4. Macro dual-emits the handle path — the original bug. `::cloacina::` under
     `not(packaged)`, `::cloacina_workflow_plugin::TaskHandle` under `packaged`.

  WIRE CHANGE, interface version 5 -> 6: `TaskExecutionRequest` gained
  `task_execution_id`. A packaged task MUST name itself when calling back — the
  host has no other way to know which of many concurrent tasks wants its slot
  released. Host-side it is read from the executor's task-local via a new
  `current_task_execution_id()` that PEEKS rather than takes, since the embedded
  path still owns that handle. Plugin-side a `TaskExecutionIdGuard` installs it
  for the invocation and clears on drop so it cannot leak into the next task on
  a reused shell thread. Same precedent as 4 -> 5 for secrets; existing packages
  must be rebuilt, which they would need anyway to use the feature.

  `cargo check --workspace` clean throughout.

  FIXTURE DONE + E2E LANE WRITTEN. `examples/fixtures/defer-handle-rust` (a
  packaged task WITH a handle parameter) compiles, and `cargo tree -i cloacina`
  reports "did not match any packages" — the engine crate is not in its graph at
  all, so the old ungated emission could not have succeeded. The e2e lane in
  `angreal test e2e compiler` also gets it BUILT through the real compiler
  service: "ok: packaged task with a handle parameter BUILT". The original bug
  is fixed and proven.

  THE LANE THEN FOUND TWO REAL BUGS, in the pattern this session keeps hitting —
  things no unit test surfaces:

  BUG A (FIXED): binding is PER DYLIB IMAGE and there are TWO loads.
  `package_loader.rs` loads the library for metadata extraction; but
  `task_registrar/dynamic_task.rs:65` performs its OWN `dlopen` of a freshly
  written temp file for EXECUTION — a different image with its own bind cell.
  Binding only the first meant `defer_until` failed with "not bound" while the
  log simultaneously showed a successful bind. Now bound at both sites, with the
  reason commented so the apparent duplication is not "cleaned up" later.

  BUG B (OPEN — this is where the work stopped): the task-execution id arrives
  EMPTY, so every callback fails `BAD_TASK_ID: malformed task id ""`.

  Root cause identified: `current_task_execution_id()` peeks the task-local that
  `with_task_handle` installs — and **only `ThreadTaskExecutor` installs it**
  (`thread_task_executor.rs:633`). The SERVER registers a `FleetExecutor`
  (`cloacina-server/src/fleet_executor.rs`), which never calls
  `with_task_handle`. So the peek is correct on the embedded path and returns
  `None` on the server path. A new unit test,
  `peek_sees_the_id_inside_the_scope_and_nothing_outside`, pins that the peek
  itself works — which is what isolated the fault to the executor, not the peek.

  BUG B (FIXED): the id is now installed by the DISPATCHER
  (`dispatcher/default.rs`), the one choke point every executor passes through,
  rather than by `ThreadTaskExecutor`. A future executor therefore cannot
  reintroduce the gap. The handle peek remains as a fallback for direct
  `ThreadTaskExecutor` calls outside the dispatcher. New test
  `dispatcher_installed_id_needs_no_task_handle` asserts resolution with NO
  handle in scope — the earlier test passed while production was broken
  precisely because a handle was always present. Verified live: the id arrives
  as a real UUID and the callback reaches the host.

  BUG C — ROOT CAUSE FOUND, AND MY FIRST DIAGNOSIS OF IT WAS WRONG.

  **`DynamicLibraryTask` never implements `requires_handle()`**, so it inherits
  the trait default of `false` (`cloacina-workflow/src/task.rs:219`). The
  executor gates the whole handle path on `if task.requires_handle()`
  (`thread_task_executor.rs:610`) — so for EVERY packaged task that branch is
  skipped: no `TaskHandle` is built, nothing is registered in the deferral
  registry, and the callback fails `TASK_NOT_RUNNING`.

  Worse, `requires_handle` does not cross the FFI boundary at all —
  `TaskMetadataEntry` has no such field — so the host currently has no way to
  learn that a packaged task wants a handle.

  **MERGED as PR #250 (squash), 2026-08-09.** Packaged `defer_until` works end
  to end on the primary interface.

  **RESOLVED — full lane EXIT=0 against a live server:**
    ok: packaged task with a handle parameter BUILT
    ok: task observed in Deferred state (host callback ran)
    ok: packaged defer_until round-tripped (slot released and reclaimed)

  `requires_handle` is now carried on `TaskMetadataEntry` and plumbed host-side
  through `OwnedTaskMetadata` into `DynamicLibraryTask`, following exactly the
  route `trigger_rules` already takes — which had the IDENTICAL bug in T-0721
  (trait default silently applied to every packaged task). That makes this a
  RECURRING SHAPE in this codebase, not a one-off: any `Task` trait method with
  a default that packaged tasks must override needs an explicit FFI field, or
  it silently no-ops for every packaged workflow. Worth checking the remaining
  defaulted trait methods against that.

  Interface version 6 -> 7 (positional bincode again).

  PROCESS NOTE: `cargo check --workspace` could not catch the miss that broke
  the first attempt. Adding the field left a SECOND `TaskMetadataEntry`
  construction inside the `package!()` macro un-updated, and that only fails
  after macro expansion in a CONSUMER crate — the workspace checked clean while
  every fixture build failed. For macro/wire changes, building an actual fixture
  is the only real check.

  THE FIX (as implemented):
  1. Add `requires_handle: bool` to `TaskMetadataEntry` (another positional
     bincode change — interface version 6 -> 7).
  2. The `#[task]` macro already detects the handle parameter; emit it into the
     task metadata.
  3. `DynamicLibraryTask` overrides `requires_handle()` from that metadata.
  4. Regression test: assert a packaged task with a handle parameter reports
     `requires_handle() == true` host-side, since that single boolean is what
     gates the entire feature.

  RETRACTION — the "FleetExecutor has no slots" conclusion below was WRONG as a
  diagnosis of this failure. The dispatcher log says `executor="default"`, which
  is the `ThreadTaskExecutor` registered by `DefaultRunner`
  (`default_runner/mod.rs:201`); the fleet executor was never involved in this
  run. I observed a true fact (FleetExecutor manages no slots) and leapt to an
  architectural conclusion without checking which executor actually ran the
  task. The FleetExecutor observation may still matter for a real fleet
  deployment — where the task runs on an AGENT, and the slot that matters is the
  agent's — but it is NOT why this test failed, and that question should be
  settled separately with evidence rather than inherited from this entry.

  (Superseded analysis kept below for the record.)

  The next live run failed `TASK_NOT_RUNNING: no running task registered for
  <uuid>`, and the reason is not a wiring mistake:
  **`cloacina-server`'s `FleetExecutor` has no concurrency slots at all** —
  grep finds no `SlotToken`, no `Semaphore`, no permit acquisition in
  `cloacina-server/src/fleet_executor.rs`.

  `defer_until` exists to RELEASE A CONCURRENCY SLOT while waiting. On the
  server path there is no slot to release, so the deferral registry has nothing
  to register and the feature's premise does not hold. This is not fixable by
  registering in a different place.

  So the honest scope is: **packaged `defer_until` works where the executor
  manages concurrency slots** — the embedded runner / `ThreadTaskExecutor` — and
  is meaningless under `FleetExecutor` as it exists today. Everything built here
  (host interface, registry, packaged handle, macro dual-emission, the
  dispatcher-installed id) is correct and needed for that path; the packaged
  fixture BUILDING through the real compiler service already closes this
  ticket's original bug.

  THREE OPTIONS for the server path, a maintainer decision:
  (a) Fail loudly — `defer_until` under an executor with no slots returns a
      clear "deferral unsupported by this executor" rather than
      `TASK_NOT_RUNNING`, which is an implementation detail leaking out.
  (b) Give `FleetExecutor` slot management — the faithful fix, and a much
      larger change touching server concurrency.
  (c) Degrade to a plain in-plugin wait with a warning — the task still works,
      it just holds its (nonexistent) slot. Cheapest, least honest.

  (a) is the smallest honest step and does not foreclose (b). Whichever is
  chosen, the e2e lane as written asserts the SERVER path and will keep failing
  until one is implemented — it should be adjusted to assert the chosen
  behavior, not deleted.

  SCOPE: `cloacina-workflow` gains a packaged `TaskHandle`; the plugin wire
  `TaskExecutionResult` gains a deferral field (interface version 5 → 6 — a
  bincode layout change, so stale artifacts must fail the version gate rather
  than mis-decode); `cloacina-macros` emits the packaged handle path; the host
  loader grows the release/sleep/re-invoke loop; plus a packaged fixture using a
  handle param as the regression net.