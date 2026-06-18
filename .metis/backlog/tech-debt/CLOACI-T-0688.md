---
id: close-rust-python-interface-parity
level: task
title: "Close Rust↔Python interface parity gaps (state accumulators, packaged cron-trigger authoring)"
short_code: "CLOACI-T-0688"
created_at: 2026-06-15T13:46:31.557405+00:00
updated_at: 2026-06-17T15:19:21.592385+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Close Rust↔Python interface parity gaps (state accumulators, packaged cron-trigger authoring)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Objective

Close the known authoring-interface parity gaps between Rust and Python so that
`cloaca` (Python) is a true peer interface to the Rust macros, not a subset.
Surfaced during the CLOACI-I-0120 docs work (interface-parity review,
2026-06-15). Per the "Python is a core capability" stance, these gaps are
documented in the primitive docs AND tracked here so we remember to close them.

This task is also **item 10 of the [[CLOACI-I-0125]]** authoring-cruft
decomposition (the [[CLOACI-T-0720]] sweep's T9 parity-failure theme); the two
hard parity *failures* (no Python `@state_accumulator`, no packaged cron
`@trigger`) live here rather than under I-0125 to avoid duplication.

## Known gaps (code-cited, 2026-06-15)

1. **State accumulators — Rust-only.** `#[state_accumulator(capacity=…)]` exists
   (`crates/cloacina-macros/src/lib.rs:238`; trait/impl in
   `crates/cloacina/src/computation_graph/accumulator.rs`) but there is **no
   Python decorator** — Python exposes only passthrough / stream / polling /
   batch (`crates/cloacina-python/src/computation_graph.rs:159-264`). Add
   `@cloaca.state_accumulator`.

2. **Packaged / decorator cron-trigger authoring — Rust-only.**
   `#[trigger(on=…, cron=…, timezone=…)]` generates a cron-backed trigger
   (`crates/cloacina-macros/src/trigger_attr.rs:292`); the Python
   `@cloaca.trigger(...)` is **poll-only** (no `cron`/`timezone` params,
   `crates/cloacina-python/src/trigger.rs:97`). **Nuance:** Python DOES have full
   cron *scheduling* at the runner API level (`register_cron_workflow`,
   `list_cron_schedules`, … in `crates/cloacina-python/src/bindings/runner.rs`),
   so the gap is specifically the *packaged/decorator* authoring form, not cron
   scheduling itself. Add cron params to `@cloaca.trigger` (or a packaged-cron
   authoring path).

3. **(Minor, reverse direction) `TaskHandle.defer_until()` — Python-only.**
   Exposed in Python (`crates/cloacina-python/src/task.rs:34`) with no Rust macro
   equivalent. Decide whether to surface it in Rust for symmetry, or document it
   as an intentional Python convenience.

> The parity sweep that found these was scoped to authoring interfaces; treat
> this list as seeded, not exhaustive — add gaps here as they're discovered.

## Technical Debt Impact

- **Current problems:** Python users cannot author state accumulators or
  packaged cron triggers; the docs must carry "Rust-only" caveats on otherwise
  dual-language primitive pages.
- **Benefits of fixing:** true Rust/Python parity; primitive docs can show both
  languages everywhere without exceptions.
- **Risk of not fixing:** "Python is core" erodes; the parity caveats accrete.

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

- [ ] `@cloaca.state_accumulator` implemented with parity to `#[state_accumulator]` (incl. DAL-backed history/capacity)
- [ ] Python `@cloaca.trigger` (or a packaged-cron path) supports cron expression + timezone authoring
- [ ] Decision recorded on `TaskHandle.defer_until()` (surface in Rust vs. document as Python-only)
- [ ] Parity caveats removed from the primitive docs once each gap is closed
- [ ] Tests cover the new Python authoring surfaces against a live runner

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

*To be added during implementation*

## Implementation Plan (2026-06-17) — plan-first, awaiting approval to build

Unblocked from the fidius-wasm deferral: these are **PyO3 binding** changes
(exposing existing Rust functionality to Python), independent of the cdylib/FFI
plugin-loading path the wasm work reshapes (see [[project_fidius_wasm_authoring_shift]]).
On branch `py-parity-t0688` (off main; #131's files are untouched here).

### Verified seams
- Python accumulator decorators: `crates/cloacina-python/src/computation_graph.rs`
  — `register_accumulator` + `PyAccumulatorRegistration { accumulator_type }`;
  four decorators today (passthrough/stream/polling/batch).
- Packaged accumulator factory: `packaging_bridge.rs:205` matches
  `accumulator_type` → only `"stream"` + a passthrough fallback (so polling/
  batch/state all currently fall through to passthrough). `AccumulatorFactory`
  trait `spawn(...)` at `:336`/`:469`.
- Rust state accumulator: `accumulator.rs:889` `StateAccumulator<T>`, `:907`
  `state_accumulator_runtime<T: Serialize+DeserializeOwned+Send+Clone>(capacity, …)`.
  Capacity: `>0` bounded (evict oldest), `<0` unbounded, `==0` write-only sink.
- Python trigger decorator: `trigger.rs` `trigger()` pyfunction (`poll_interval`,
  `allow_concurrent`) + `PythonTriggerDef { poll_interval }`.
- Python trigger → manifest emission: `cloacina-python/src/loader.rs:315`
  consumes `drain_python_triggers()` and emits trigger metadata.
- **Host cron plumbing already exists end-to-end**: `ffi_trigger.rs:52`
  `cron_expression: Option<String>`, `package_loader.rs:441`
  `TriggerPackageMetadata`, reconciler `register_cron_workflow` (mod.rs:183).
  Rust macro `trigger_attr.rs:40` carries `cron`/`timezone`.

### Part A — `@cloaca.state_accumulator(capacity=N)` (M, the harder half)
1. Decorator in `computation_graph.rs`: `state_accumulator_decorator(capacity)` →
   register with `accumulator_type="state"`; add a `capacity: i32` field to
   `PyAccumulatorRegistration`.
2. `StateAccumulatorFactory` in `packaging_bridge.rs` implementing
   `AccumulatorFactory::spawn`, launching
   `state_accumulator_runtime::<serde_json::Value>(capacity, …)`. Wire `"state"`
   into the `accumulator_type` match at `:205` (and the override matches at
   `:582`/`:644`).
3. Embedded (in-process cloaca) path: the graph builder that drains
   `drain_accumulators()` must also handle `"state"`.
4. **Open question (main risk):** reconcile `AccumulatorFactory::spawn`'s contract
   (boundary sender + DAL handle + shutdown) with `state_accumulator_runtime`'s
   loop shape (persists history to DAL, emits the full list as boundary). This
   wiring — dynamic `serde_json::Value` output + DAL persistence in the packaged
   factory — is the biggest unknown.

### Part B — cron `@cloaca.trigger(cron=…, timezone=…)` (M, more tractable)
1. Decorator in `trigger.rs`: add `cron: Option<String>`, `timezone:
   Option<String>` to `trigger()` + `PythonTriggerDef`. `cron` set ⇒ cron trigger
   (no poll body); mutually exclusive with `poll_interval`.
2. Manifest emission `loader.rs:315`: emit `cron_expression` (+ timezone) in the
   `TriggerPackageMetadata` when `cron` is set.
3. Runtime: **no new host work** — `ffi_trigger`/`package_loader`/reconciler
   already register cron schedules from `cron_expression`. Verify the
   poll-vs-cron branch keys on `cron_expression` presence.
4. Open question: scope to packaged/decorator authoring (the documented gap);
   Python already has runner-level cron scheduling via `register_cron_workflow`.

### Part C — `TaskHandle.defer_until()` reverse gap (XS)
Decision only: document as an intentional Python convenience (cite
`cloacina-python/src/task.rs:34`) vs. add a Rust equivalent for symmetry.
Recommend: document as Python-only unless symmetry is explicitly wanted.

### Test strategy (per AC + the SDK-live-server memory)
- New Python scenario tests against a **live runner**: a state-accumulator
  scenario (bounded capacity eviction + history emission) and a packaged
  cron-trigger scenario (authored via decorator, fires on schedule).
- Build packaged fixtures exercising both; verify register + fire end-to-end.
- Remove the "Rust-only" caveats from the primitive docs as each gap closes.

### Recommended sequencing
Part B first (host plumbing exists → lower risk, faster win) → Part A (factory
wiring, the real unknown) → Part C decision. Each its own commit; verify via
`angreal test integration` (Python scenarios) before moving on.

## Status Updates
- 2026-06-17: Unblocked (PyO3 bindings independent of the fidius-wasm plugin
  path, per user). Plan-first per request: implementation plan above, grounded
  in verified seams; **awaiting approval to build.** Key finding: Part B (cron
  trigger) is mostly Python-side wiring since the host cron plumbing already
  exists; Part A (state accumulator) needs a new packaged `StateAccumulatorFactory`.- 2026-06-17: **Part A (state accumulator) built + verified.** Added
  `@cloaca.state_accumulator(capacity=N)` (mirrors the stream decorator) and a
  `StateAccumulatorFactory` in `packaging_bridge.rs` that spawns
  `state_accumulator_runtime::<serde_json::Value>(capacity)` — wired into all
  three `accumulator_type` matches (FFI metadata + both override paths); capacity
  flows decorator → `config["capacity"]` → factory. No contract mismatch (the
  state runtime fits `AccumulatorFactory::spawn` exactly like the others).
  `cargo check` (cloacina + cloacina-python, default features) green; unit test
  `test_state_accumulator_decorator_registers_state_with_capacity` passes.
- 2026-06-17: **Part B (cron trigger) — design question found in-code, NOT a
  simple wiring change.** `@cloaca.trigger` produces a *poll* trigger
  (`PythonTriggerWrapper.poll()`); cron is a different mechanism
  (`TriggerPackageMetadata.cron_expression` → cron scheduler, not the poll
  registry). So `@cloaca.trigger(cron=…)` needs an API decision: what it
  decorates and how it ties to a workflow (Rust uses `#[trigger(on=wf, cron=…)]`).
  Deferred pending that decision; not guessed.
- 2026-06-17: **Part B (cron trigger) built — mirror-Rust API.** Resolved the
  design question: `@cloaca.trigger(on="wf", cron="…", timezone="…")` mirrors
  `#[trigger(on=…, cron=…, timezone=…)]`. Validation mirrors Rust (cron XOR
  poll_interval; `on` required for cron). Turned out **contained to `trigger.rs`**:
  `build_view_python` (loading.rs:1398) already emits `cron_expression()` +
  `workflow_name()` from the Trigger trait, but `PythonTriggerWrapper` returned
  the trait defaults (None / ""). Fix: carry `on`/`cron`/`timezone` on
  `PythonTriggerDef` + the wrapper and override `Trigger::workflow_name()` /
  `cron_expression()`. The reconciler's `step_load_cron_triggers` then registers
  the cron schedule — packaged-Python cron authoring now works end-to-end.
  Tests: cron decorator carries cron+workflow; wrapper exposes them; validation
  guards. 16+10 trigger tests pass; cloacina-python compiles (default features).
- 2026-06-17: **Two honest caveats.** (1) `timezone` is captured at authoring
  but **not honored downstream** — the reconciler hardcodes `"UTC"`
  (loading.rs:1543) and `TriggerPackageMetadata` has no `timezone` field. This is
  a **pre-existing cross-language gap** (Rust `#[trigger(timezone=)]` is dropped
  the same way), so Python is at parity; full plumbing is a separate follow-up.
  (2) Embedded (in-process, non-packaged) cron via the decorator isn't scheduled
  (cron needs a DB + the reconciler; import-time has neither) — embedded cron
  stays on the runner-level `register_cron_workflow` API. The packaged/decorator
  authoring form (the documented gap) is what's closed.
- 2026-06-17: **Runtime test gap closed (commit f35ac189, PR #132).** Added 4
  sqlite/no-docker Rust integration tests: `accumulator.rs` —
  `test_state_accumulator_runtime_bounded_evicts_and_emits_history` (capacity=2
  feeds 1,2,3 → boundaries [1],[1,2],[2,3], proving eviction+history) and
  `test_state_accumulator_runtime_write_only_emits_nothing` (capacity=0 emits no
  boundary); `reconciler/loading.rs` —
  `build_view_python_emits_cron_metadata_for_runtime_trigger` (cron_expression()
  + workflow_name() flow into the view) and
  `step_load_cron_triggers_selects_cron_bearing_triggers` (only cron-bearing
  triggers register, against their TARGET workflow, UTC; poll triggers ignored).
  All 4 pass; pre-commit fmt + both-backend cargo check green. Covers the Rust
  runtime tier; the Python-level / live-runner e2e tier (Docker-gated, AC line
  "against a live runner") remains the one open follow-up.
- 2026-06-17: **Demo-stack coverage added (commit 85162cab, PR #132).** Both new
  Python surfaces now ship in the UI demo (`docker/docker-compose.demo.yml`):
  `demo-py-state` (`@cloaca.state_accumulator(capacity=5)` → reactor → 2-node CG,
  fed over WS via the producer — `py_window` added to `HARNESS_WS_ACCUMULATORS`
  + a bid/ask generator in `produce.mjs`) and `demo-py-cron` (pure-Python task
  workflow + `@cloaca.trigger(on=, cron="*/15 * * * * *")`, the Python mirror of
  `demo-cron-rust`, self-driving via the cron scheduler). Both packed in
  `pack-demo-fixtures.sh`. This doubles as the live-runner exercise the AC wanted
  for the new Python surfaces — verifiable by bringing up the demo stack (Docker
  is down locally, so not yet run here). Python syntax checked; decorator
  signatures + graph-dict shape matched against the built bindings.
- 2026-06-17: **Live demo stack run — BOTH surfaces verified end-to-end; found +
  fixed a real shipping bug (commit 21fdc1d7, PR #132).** Brought up
  `docker-compose.demo.yml`. `demo-py-cron` loaded + its cron schedule registered
  immediately. `demo-py-state` FAILED to load: server reconciler reported
  `module 'cloaca' has no attribute 'state_accumulator'`. **Root cause:** Part A
  registered `state_accumulator` in the maturin `#[pymodule]` (lib.rs) but NOT in
  `ensure_cloaca_module` (loader.rs) — the SYNTHETIC `cloaca` the *server* injects
  into its embedded interpreter (the server image has no pip wheel). Two parallel
  registration lists drifted; the Part A unit test passed because it called the
  Rust decorator fn directly, never `import cloaca`. **Fix:** register
  `state_accumulator_decorator` in `ensure_cloaca_module` + extend
  `test_ensure_cloaca_module_registers_in_sys_modules` to assert the
  `state_accumulator` attr (guards the drift). After rebuild+restart:
  • `demo-py-state` → "Python computation graph imported", reactor running, CG
    `fires=1..N` every 2s off the `py_window` WS feed.
  • `demo-py-cron` → cron fires every 15s, `py_cron_step` completes, context
    carries `{demo_py_cron_ran, schedule_expression:"*/15 * * * * *",
    schedule_timezone:"UTC", scheduled_time}`, "Successfully executed and audited".
  This satisfies the AC "Tests cover the new Python authoring surfaces against a
  live runner". **Remaining AC:** Part C decision (`defer_until`) + remove the
  "Rust-only" caveats from the primitive docs.
- 2026-06-17: **Merged to main (#132, squash commit b4b3197b).** Core parity work
  (Parts A + B), the 4 runtime tests, the demo fixtures, the synthetic-module fix,
  AND the reactor-first CG view (CLOACI-T-0742) all landed. CI green after a
  re-run of the known T-0741 sqlite flake (macos `database is locked` on
  scenario_26, unrelated to this diff). **Two AC tails remain, both minor:**
  (1) Part C `defer_until` decision — recommend documenting as an intentional
  Python convenience; (2) remove the "Rust-only" caveats from the primitive docs
  now that both gaps are closed. Neither blocks; track as a small doc follow-up.
