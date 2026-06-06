---
id: containerized-in-flight-reclaim
level: task
title: "Containerized in-flight reclaim e2e — slow fixture, kill agent mid-execution"
short_code: "CLOACI-T-0638"
created_at: 2026-06-04T15:19:10.079581+00:00
updated_at: 2026-06-06T03:15:16.075128+00:00
parent: CLOACI-I-0114
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0114
---

# Containerized in-flight reclaim e2e — slow fixture, kill agent mid-execution

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0114]]

## Objective **[REQUIRED]**

Prove **true in-flight dead-agent reclaim** end-to-end in the containerized fleet: kill
the agent that is *actively executing* a task, and assert the server reclaims the orphaned
in-flight work and reschedules it to a surviving agent, where it completes — exercising the
T-0634 reclaim path (`reassign_open_rows` → `"fleet: reclaimed dead agent's in-flight work"`).

T-0637's churn step only proved *survivor handling* (delete an idle/finished agent, re-run
a fresh workflow). This proves the harder property: work in flight on a dying agent is not
lost. Builds on the `angreal helm fleet` harness.

## Reclaim mechanism (verified, T-0634)

- Agent heartbeat every 15s (`DEFAULT_HEARTBEAT_INTERVAL_SECONDS`); server marks an agent
  dead after 3 missed beats = **45s**, swept every 15s → up to ~60s detection
  (`cloacina-server/src/lib.rs:777-856`, hardcoded).
- In-flight assignment = a non-acked `delivery_outbox` row addressed `agent:<id>` for a
  `task_execution_id` (`fleet_executor.rs:313-325`).
- On death: `reassign_open_rows("agent:dead","agent:live")` re-targets non-acked rows to the
  most-free live agent **in the same tenant**, resets state→pending, clears delivered_at;
  the relay re-pushes; the rendezvous (same `task_execution_id`) wakes unchanged
  (`lib.rs:797-856`, `dal/unified/delivery_outbox.rs:426-502`). Logs at info; metric
  `cloacina_fleet_work_reassigned_total`.

## Design (deterministic, no agent_id↔pod guessing)

Dispatch is only logged at debug, so don't *guess* which agent got the work — *force* it:

1. New fixture `examples/fixtures/fleet-slow-rust/` — one task `slow` that sleeps
   `context["sleep_seconds"]` (default small for cheap standalone use) via `tokio::time::sleep`
   (`tokio` w/ `time` already a fixture dep). Workflow `fleet_slow_workflow`. Models on
   `compiler-happy-rust` so it builds green through the in-cluster compiler.
2. In `angreal helm fleet`, after the happy path (step 8, 2 agents = multi-agent proof),
   add a reclaim step:
   - `kubectl scale deploy/cloacina-agent --replicas=1`; capture the sole pod **P1**.
   - upload+compile `fleet-slow-rust`; `workflow run fleet_slow_workflow --context
     {sleep_seconds: 90}` → must land on P1 (only agent). exec_id captured.
   - brief settle so P1 is executing (still well inside the 90s sleep).
   - `kubectl scale --replicas=2`; wait P2 ready+registered (the reclaim survivor).
   - `kubectl delete pod P1` (the known executor).
   - poll execution status → **Completed** (generous deadline ~240s: ~60s detect + 90s
     re-run + margin).
   - assert server log contains `reclaimed dead agent's in-flight work`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] `fleet-slow-rust` fixture builds green through the in-cluster compiler.
- [ ] `angreal helm fleet` reclaim step: kill the executing agent → execution reaches
      Completed on a survivor.
- [ ] Server log carries `reclaimed dead agent's in-flight work` (the reclaim proof), not
      just a generic completion.
- [ ] Failure path stays loud (build_error + pod describe/logs dumped before teardown).

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

### 2026-06-04 — implemented + statically verified; ready for a real kind run

- **Fixture `examples/fixtures/fleet-slow-rust/`** authored (package.toml, Cargo.toml,
  build.rs, src/lib.rs). Task `slow` sleeps `context["sleep_seconds"]` (default 2) via
  `tokio::time::sleep`; workflow `fleet_slow_workflow`. **Builds green locally** with
  `cargo build --release --lib` (~16s).
- **`angreal helm fleet` reworked** (`.angreal/task_helm.py`):
  - `_upload_and_compile(fixture)` helper (DRY; dumps build_error + compiler logs on
    failure). Step 7 now compiles BOTH compiler-happy-rust and fleet-slow-rust.
  - Step 8 = happy path (2 agents, multi-agent proof) — unchanged assertions.
  - Step 9 = **true in-flight reclaim** (replaces the old weak churn):
    `_scale_agents(1)` → capture sole pod P1 → run `fleet_slow_workflow` with
    `--context {sleep_seconds:90}` → **poll until status==Running** (work committed to P1)
    → `_scale_agents(2)` (add survivor P2) → `kubectl delete pod P1` → `_wait_exec`
    (timeout 300s) must reach Completed AND server log must contain
    `reclaimed dead agent's in-flight work`.
  - `_run_workflow` gained a `context=` arg (`--context <file>`; inline JSON isn't
    supported — load_context reads a path or `-`).
  - `_agent_pods()` filters to LIVE pods (Running, not Terminating) so a scaled-down pod
    in its grace period isn't miscounted; `_wait_agent_pods(n)` + `_scale_agents` wait for
    the exact live count.
  - `_exec_status` one-shot helper extracted from `_wait_exec`.
- Static checks: `py_compile` + `ruff` clean; task registers in `angreal tree`.

**Timing budget (why it holds):** kill happens ~30s after dispatch (settle to Running +
scale-up), well inside the 90s sleep → P1 still executing. After kill: ~45-60s death
detection + 90s P2 re-run ≈ 180s from dispatch < FleetExecutor RESULT_WAIT_TIMEOUT (300s)
and < the step's 300s `_wait_exec`. Single server replica → in-memory rendezvous holds.

**Not yet committed.** Next: run `angreal helm fleet`; on green, commit fixture +
harness into PR #115 and mark complete.

### 2026-06-04 (run 1) — slow task Failed before kill → tokio-in-cdylib; fixed to blocking sleep

First kind run failed at step 9: `slow task reached Failed before it could be killed`. NOT
a timing issue (work packet `timeout_seconds=300` ≫ 90s; `fleet_executor.rs:300`). Root
cause: **`tokio::time::sleep` does not work inside a dlopened cdylib.** The cdylib
statically links its own tokio instance, whose runtime-context thread-local is distinct
from the agent's runtime, so the timer call finds no reactor and panics → task Failure.
(The noop happy-path task survives because it makes no async-runtime calls. The
`event-triggers` example uses `tokio::time::sleep` but runs in-process, not as a fleet
cdylib, so it's not a counter-example.)

Fix: `fleet-slow-rust` now uses **`std::thread::sleep`** (no runtime needed — parks the
worker thread). The agent runs each packet as a spawned task on its multi-threaded
`#[tokio::main]` runtime, so blocking one worker leaves others for heartbeats (assumes a
multi-core kind node — true on the dev Mac). Rebuilt green (~17s).

Also hardened the step-9 early-termination branch: now dumps the full `execution status`
JSON + the executing agent's pod logs (the task panic/failure message) + server logs before
teardown, so any future early failure here is conclusive in one run.

Implication worth noting for fleet users: **packaged/fleet task bodies can't rely on the
host tokio runtime** (timers, async I/O via tokio) — they get their own. Worth a docs note
under T-0635.

### 2026-06-04 (run 2) — BLOCKED: reclaim test uncovered a real fleet bug (double-dispatch of long tasks)

Run 2 (std::thread::sleep fixture) reached step 9, the agent **ran the slow task to success
(twice)** — but the workflow still **Failed** with `fleet result rendezvous canceled`. The
test is correct; it exposed two genuine product bugs:

**BUG 1 (PRIMARY, real fleet correctness bug) — fleet dispatch never atomically claims the
task, so any fleet task longer than the scheduler poll interval (100ms) is re-dispatched
every tick.** Compounding factor: tenant `public` reuses the admin DB / `public` schema
(`lib.rs:64-71`), so the global runner AND the per-tenant `public` runner both poll
`public.task_executions` and both dispatch the same Ready row. Evidence:
- `scheduler_loop.rs:265-299` dispatches Ready tasks with NO status flip; `get_ready_for_retry`
  (`dal/unified/task_execution/claiming.rs:843-857`) is a pure SELECT (no claim / SKIP LOCKED).
- The ThreadTaskExecutor self-claims (`enable_claiming`); **`FleetExecutor::execute`
  (`fleet_executor.rs:142-147,315`) does NOT** — it synthesizes a ClaimedTask from the event
  and calls `register_pending` without flipping status off Ready.
- So the row stays Ready for the whole 2s sleep → every 100ms poll re-dispatches → each
  `register_pending` clobbers the prior oneshot (`fleet_coordinator.rs:63-69`) → "rendezvous
  canceled" → retry → agent saturates (`in_flight=2`) → final invocation also loses its
  rendezvous → Task Failed, despite the agent succeeding. Noop survives because it reconciles
  to terminal inside one 100ms tick.
- Per-tenant runner lifecycle is NOT the cause — it's correctly retained in the LRU; no leak.

**Impact:** the fleet can currently only run sub-~100ms tasks reliably; any real (longer)
task double-dispatches. This blocks the reclaim e2e (which needs a long task) AND is a
correctness problem for the fleet feature generally. Fix = give the fleet dispatch path an
atomic DB claim (Ready→Running/claimed, SKIP LOCKED) like the thread executor, and resolve
the global-vs-per-tenant `public` double-scheduling (e.g. don't run a per-tenant runner for
`public`, or don't let the global runner schedule tenant-owned tasks).

**BUG 2 (SECONDARY, CLI contract) — `cloacinactl workflow run --context` is silently
dropped.** The execute endpoint deserializes `ExecuteRequest { context: Option<Value> }`
and reads only `body.context` (`routes/executions.rs:36-71`), but the CLI POSTs the file's
JSON as the RAW body (`nouns/workflow/mod.rs:90-123`). So `{"sleep_seconds":90}` has no
`context` key → empty context → "Skipping insertion of empty context" → task ran the 2s
default. Fix = CLI should wrap as `{"context": <file-json>}` (or the endpoint should accept
a bare object). Test workaround: write the context file as `{"context": {"sleep_seconds":N}}`.

**Status: BLOCKED on a product decision.** The fixture + harness are done and correct; the
test can't pass until BUG 1 is fixed. Awaiting direction on scope (fix fleet claim now vs
file + park).

### 2026-06-04 — investigation: concrete fix plan for BUG 1 (+ BUG 2 fixed)

**BUG 2 fixed:** `cloacinactl` now wraps the context as `{"context": <file-json>}`
(`nouns/workflow/mod.rs:89`). Rebuilds with the next `angreal helm fleet`.

**BUG 1 fix plan (the design is "make FleetExecutor claim like ThreadTaskExecutor"):**
The scheduler intentionally over-selects (`get_ready_for_retry` SELECT-only,
`scheduler_loop.rs:265-302`); dedup is the executor's job via an atomic claim. The thread
executor does claim→heartbeat→release; the fleet executor does none. Mirror it:
- ThreadTaskExecutor pattern: `claim_for_runner(task_id, instance_id)` →
  `AlreadyClaimed` ⇒ `return Ok(ExecutionResult::skipped(id))` (thread:342-383); spawn a
  `heartbeat(task_id, instance_id)` loop while executing (thread:393-426); `release_runner_claim(id)`
  when done (thread:617). `claim_for_runner` sets `claimed_by` if NULL (claiming.rs:494-530);
  heartbeat keeps it alive vs the stale-claim sweeper (threshold 60s, so a >60s task MUST
  heartbeat); `release_runner_claim` clears it (claiming.rs:672-699); `schedule_retry`
  re-opens it for re-claim.
- **FleetExecutor changes (`crates/cloacina-server/src/fleet_executor.rs`):**
  1. Add an `instance_id: UniversalUuid` (generate at construction).
  2. At top of `execute`, before `register_pending`: `claim_for_runner(event.task_execution_id,
     self.instance_id)`. `AlreadyClaimed` ⇒ return `ExecutionResult::skipped` (no
     register_pending, no enqueue). `Err` ⇒ log + proceed (best-effort, like thread).
  3. On `Claimed`: register_pending + enqueue as today, then spawn a heartbeat loop
     (interval < 60s) for the duration of the rendezvous wait; abort it when the result
     arrives or on timeout.
  4. `release_runner_claim(event.task_execution_id)` before returning (success/error/timeout
     paths), so retries + reclaim can re-claim.
- This dedupes BOTH the same-runner 100ms re-dispatch AND the global-vs-per-tenant `public`
  double-scheduling (whichever claims first wins; the other skips). Reclaim (T-0634) is
  unaffected: the claim is on `task_executions`, the reclaim re-targets `delivery_outbox`;
  the heartbeated claim keeps the scheduler off the row while the original `execute`
  invocation awaits the (reassigned) agent's result on the same `task_execution_id`.

**Blast radius:** moderate, localized.
- Only `fleet_executor.rs` (+ its 2 construction sites: global runner in `lib.rs`, per-tenant
  via the `FleetRegistrar`) — both just pass/generate an `instance_id`.
- No DAL changes (claim/heartbeat/release + schedule_retry all already exist + are exercised
  by the thread executor).
- The `public` schema double-scheduling becomes correct-but-slightly-wasteful (extra
  claim-skips). A cleaner architectural fix (don't run a per-tenant runner for `public`, or
  stop the global runner scheduling tenant-owned rows) is a **separate, optional** follow-up,
  not required for correctness.
- Risk to watch: claim must be released on EVERY exit path of `execute` (incl. the early
  `reconcile_error` returns) or a failed fleet task gets stuck claimed. Verify reclaim +
  retry still pass (the host e2e `angreal test e2e fleet` + this containerized test).

### 2026-06-04 — BUG 1 + BUG 2 implemented (compiles clean); ready to re-run

**BUG 1 fix landed** in `crates/cloacina-server/src/fleet_executor.rs`:
- Added `instance_id: UniversalUuid` (gen'd in `new()`) + `FLEET_CLAIM_HEARTBEAT_INTERVAL`
  (20s, < the 60s sweeper threshold).
- `execute` now: `claim_for_runner(task, instance_id)` → `AlreadyClaimed` ⇒ return
  `ExecutionResult::skipped` (no dispatch); `Claimed` ⇒ spawn a heartbeat task + run the
  original dispatch body wrapped in an inline `async {…}.await` so the claim is
  `release_runner_claim`'d on EVERY exit path (the body has ~10 early returns). Body itself
  is unchanged (wrapped, not re-indented → reviewable diff). Dedupes both the 100ms
  re-dispatch and the global-vs-per-tenant `public` double-scheduling; reclaim/retry
  unaffected (claim is on `task_executions`, reclaim is on `delivery_outbox`).

**BUG 2 fix landed** in `crates/cloacinactl/src/nouns/workflow/mod.rs`: `workflow run` now
POSTs `{"context": <file-json>}` so `--context` actually reaches the task.

Both verified to compile: `angreal check crate crates/cloacina-server` ✅ and
`crates/cloacinactl` ✅ (only pre-existing unrelated warnings).

**Uncommitted.** Verification before commit into PR #115:
1. `angreal test e2e fleet` (host) — confirms the claim fix didn't break the basic fleet path.
2. `angreal helm fleet` (containerized) — happy path + in-flight reclaim should now reach
   Completed with `reclaimed dead agent's in-flight work` in the server log (and no
   `result rendezvous canceled` / duplicate dispatch).

### 2026-06-04 (run 3) — fixes WORK; test gate corrected (fleet execs show Pending while running)

Run 3: **happy path Completed cleanly (no rendezvous-cancel, no duplicate dispatch) and the
host e2e `angreal test e2e fleet` passed** — BUG 1 + BUG 2 fixes are confirmed good. Step 9
then failed on a TEST-harness gate, not a product issue: `slow task never reached Running
(last=Pending)`.

Finding: a **fleet** workflow execution stays `Pending` for the whole agent run and jumps
straight to `Completed` on report — it never surfaces `Running` (the claim sets `claimed_by`,
not `status`; nothing marks the workflow execution Running on the fleet path). So gating on
status==Running was wrong. Fixed the gate to poll the **agent pod log for `fleet-slow-rust`**
(the agent logs the cdylib registration when it dlopens the package to run it) — an
unambiguous "P1 is executing" signal that also proves placement on P1 (the sole agent).
Added `_pod_logs(pod)` helper. Python-only; py_compile + ruff clean.

Minor product observability note (→ T-0635): fleet executions reading `Pending` while
actually running on an agent is misleading; consider marking the task/execution `Running` on
fleet dispatch.

**Next:** re-run `angreal helm fleet` only (host e2e already green; Rust unchanged).

### 2026-06-04 — observability fix: fleet dispatch marks the workflow execution Running

Investigated the "Pending while running" gap: workflow-execution completion guards are
`status.ne_all(["Completed","Failed"])` (workflow_execution.rs:417/474/619/683), so the
normal dispatch path goes `Pending → Completed` and **never surfaces `Running`** (the only
`Running` writers were pause/unpause at :895/:946). Not fleet-specific, but most visible for
long fleet tasks.

Fix (fleet-scoped, as requested): `FleetExecutor::execute` now calls
`workflow_execution().update_status(workflow_execution_id, "Running")` right after it claims
the task — so `execution status` reads `Running` while the agent works it. Safe because the
scheduler only dispatches tasks for active (Pending/Running) executions and re-dispatches
short-circuit at the claim, so it's only ever Pending→Running (idempotent if already
Running); completion/fail/pause transitions all accept Running. Compiles clean
(`angreal check crate crates/cloacina-server` ✅).

Test: kept the robust pod-log pickup gate AND added a regression guard asserting the
execution reads `Running` while in flight (validates this fix). Python clean.

Observability follow-up (→ T-0635): the **thread/in-process** path has the same
Pending-while-running characteristic; consider surfacing Running there too for parity.

**Next:** re-run `angreal helm fleet` (host e2e green; both Rust crates compile).

### 2026-06-04 (run 4) — fixes hold; step-9 scale-down race fixed (identify executor instead)

Run 4 confirmed the product fixes again: happy path Completed cleanly (no rendezvous-cancel,
no double-dispatch). Step 9 failed because the slow task was dispatched to the agent whose
pod was being **scaled down** — the server's agent registry lags k8s pod termination (the
killed agent's WS only reset ~30s later; it stays selectable until its heartbeat goes stale
~45-60s), so the fleet executor picked the dying pod and the real survivor never got the
work. The pod-log gate correctly caught it ("never picked up").

Root issue was the **scale-to-1 → run → scale-to-2** choreography racing the registry. Fixed
the test to keep BOTH agents live (no scaling) and instead **identify the actual executor by
which agent pod logs the `fleet-slow-rust` cdylib load**, then kill that pod; the other
(genuinely-live) agent is the reclaim survivor. Removed `_scale_agents` (now unused). Pure
test change; py_compile + ruff clean. No Rust change (claim + Running fixes already compiled
and confirmed working in run 3/4).

**Next:** re-run `angreal helm fleet` (harness-only change since run 4).

### 2026-06-05 (run 5) — GREEN. In-flight reclaim e2e passes.

`angreal helm fleet` passed end to end: happy path Completed cleanly (no rendezvous-cancel /
double-dispatch) AND step 9 — identified the executing agent by its `fleet-slow-rust` cdylib
log, asserted the execution reads `Running` in flight (observability fix), killed that pod,
and the live survivor reclaimed the work → `Completed` with `reclaimed dead agent's
in-flight work` in the server log.

Re the "that's slow" smell: it is NOT the buggy path. The only slow fallback is the fleet
rendezvous timing out at 300s then retrying; since `_wait_exec(300s)` returned Completed, it
necessarily completed via the FAST reclaim path (the fallback's first event is at 300s →
`_wait_exec` would have failed). The duration is inherent: hardcoded ~45-90s dead-agent
detection (3×15s heartbeat + sweep tick + the killed pod's SIGTERM grace, agent in a blocking
sleep) + a full `SLEEP_SECONDS` re-run on the survivor (the task restarts from scratch, not
resume). Added instrumentation: prints reclaim wall-clock after the kill and warns if the
300s timeout-fallback marker ever appears (regression guard). SLEEP_SECONDS kept at 90 for
kill-timing robustness; could trim to ~45-60 to shave the re-run if desired.

**All exit criteria met.** Uncommitted set for PR #115:
- `crates/cloacina-server/src/fleet_executor.rs` — BUG 1 fix (claim + heartbeat + release +
  mark execution Running on dispatch).
- `crates/cloacinactl/src/nouns/workflow/mod.rs` — BUG 2 fix (`--context` wrap).
- `examples/fixtures/fleet-slow-rust/` — slow reclaim fixture.
- `.angreal/task_helm.py` — reclaim step + helpers (`_upload_and_compile`, `_agent_pods`
  live-filter, `_wait_agent_pods`, `_pod_logs`, `_run_workflow(context=)`, diagnostics).
- this task doc.

Follow-ups (T-0635): thread-path "Pending while running" parity; optional architectural fix
for the global-vs-per-tenant `public` double-scheduling (harmless now the claim dedupes it).
