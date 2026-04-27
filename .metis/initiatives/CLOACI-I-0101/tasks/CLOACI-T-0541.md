---
id: t-03-python-decorator-parity-for
level: task
title: "T-03: Python decorator parity for split CG + CG-invoking task"
short_code: "CLOACI-T-0541"
created_at: 2026-04-24T15:08:19.954995+00:00
updated_at: 2026-04-25T17:42:39.823528+00:00
parent: CLOACI-I-0101
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0101
---

# T-03: Python decorator parity for split CG + CG-invoking task

## Parent Initiative

[[CLOACI-I-0101]]

## Objective

Mirror the Rust declaration surface from T-01a and T-02 in the Python bindings. `@cloaca.computation_graph` accepts the `trigger = reactor("name")` kwarg; `@cloaca.task` accepts `invokes = computation_graph("name")`. Python is dynamic, so validation is runtime, not compile-time, but the authoring experience must match what Rust users see.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `@cloaca.computation_graph(trigger=cloaca.reactor("name"), ...)` compiles and registers a CG bound to the named reactor.
- [ ] `@cloaca.computation_graph(...)` without a `trigger` kwarg registers a trigger-less CG that can be invoked by a workflow task.
- [ ] `@cloaca.task(invokes=cloaca.computation_graph("name"), ...)` registers a workflow task whose body is a CG invocation. If the user supplies a function body, pre/post semantics match the Rust task.
- [ ] Runtime validation: at registration time, a CG with `trigger=reactor("name")` where the named reactor does not exist raises a `ValueError` with a clear message. Same for a task whose `invokes=computation_graph("name")` target is not registered.
- [ ] Runtime type-compatibility check (best-effort): when a CG's entry data class and a reactor's firing payload can be compared (both are `@dataclass` or both are `TypedDict`), a mismatch raises at registration. If the types cannot be statically introspected, skip with a debug log — Python dynamism.
- [ ] Python scenario tests match the Rust integration coverage:
  - CG with reactor trigger fires end-to-end.
  - Trigger-less CG invoked by a workflow task; outputs propagate to downstream tasks.
  - Fan-out: two CGs, same reactor trigger.
  - Adapter failure raises a clean error, not an opaque PyO3 crash.
- [ ] `angreal cloaca test` (or its successor under the cloacina namespace) green.
- [ ] `angreal demos python-tutorial-09` (or the CG-equivalent tutorial) green after its migration in T-04.

## Implementation Notes

### Technical Approach

1. Extend the Python decorators in `crates/cloacina-python/src/workflow.rs` (for `@task`) and `crates/cloacina-python/src/computation_graph.rs` (for `@computation_graph`). Add `trigger` and `invokes` kwargs to each.
2. On registration, resolve the referenced reactor / graph name against the scoped runtime. If missing, raise `PyValueError` with a migration-friendly message.
3. Context adaptation: Python context is the same `PyContext` wrapper as today. Marshal context → graph entry type at invocation time. For trigger-firings originating on the Rust side with a typed payload, the Python bridge already exists (from the existing reactor wiring); extend it to surface terminal outputs back to the calling task's context.
4. Python scenario tests go under `tests/python/test_scenario_*.py`. Expect to add two or three new scenarios; verify they run under both backends in CI.

### Key Files

- `crates/cloacina-python/src/workflow.rs` — task decorator extensions.
- `crates/cloacina-python/src/computation_graph.rs` — CG decorator extensions + reactor helper.
- `crates/cloacina-python/src/task.rs` — task registration hook.
- `tests/python/test_scenario_*.py` — new scenarios for split-CG-with-reactor, trigger-less CG invoked by task, fan-out.

### Dependencies

- **T-02** (the Rust workflow-task CG invocation must exist — Python is a thin wrapper over the Rust executor).
- Indirectly T-01a (the CG declaration surface this decorator targets).

### Risk Considerations

- Python's dynamic nature means some type validations the Rust macro catches at compile time become runtime checks here. Err on the side of explicit, readable error messages at registration (not at first firing).
- Keep the decorator kwargs symmetric with Rust's: `trigger=cloaca.reactor("name")` mirrors `trigger = reactor("name")`. Don't let ergonomic drift between the languages creep in.

## Status Updates

### 2026-04-27 — Scope reset: minimum Python parity for Rust split-CG work

**Initial drift caught.** First-pass design was creeping toward redesigning Python CG authoring (fluent topology builder, removing `@cloaca.node`, etc.). That's new scope, not parity. T-0541's actual job: **mirror in Python the Rust surface that landed in T-0538 (`#[reactor]`), T-0539 (bundled-form removal), and T-0540 (`#[task(invokes = ...)]`)** — nothing more.

**Conceptual frame:** a computation graph is a unit-of-work primitive that runs in one of two modes — embedded in a workflow as a task-invoked function (trigger-less), or driven by a reactor as a fast event-driven path (reactor-triggered). T-0541 makes both modes available from Python with the same authoring shape Rust has.

**Locked scope (the minimum, do not extend):**

1. **`@cloaca.reactor` class decorator** — new, mirrors Rust's `#[reactor]`. Class gets `NAME` / `ACCUMULATORS` / `REACTION_MODE` attributes for handle use; registers a `ReactorRegistration` in the Rust runtime via FFI.
2. **`ComputationGraphBuilder` kwarg change** — `react={"mode": ..., "accumulators": [...]}` → `reactor=ReactorClass` (split form) or omit entirely (trigger-less). Everything else about the existing builder stays: `name=`, `graph={...}` topology dict, `@cloaca.node` decorator inside the `with` block, the `__enter__`/`__exit__` thread-local plumbing, the validation pass at `__exit__`. Do NOT rename the class. Do NOT add a fluent topology API. Do NOT remove `@cloaca.node`.
3. **Two-flavor entry contract** — trigger-less CG entry takes `ctx: cloaca.Context`; reactor-triggered CG entry keeps today's typed accumulator payloads. Validation at `__exit__`: in trigger-less form, every node's `(...)` cache-input list must be empty.
4. **`@cloaca.task` extensions** — add `invokes=GraphHandle` (class reference, where the class is what the builder emits as its public handle) and `post_invocation=fn`. Mirrors Rust T-0540 M3+M5: invocation runs after the user body, calls compiled fn with task context, routes terminals back into the context. Registration-time error if `invokes` target is reactor-triggered.
5. **Hard-break migration** — delete the `react={...}` kwarg parser path. Live in-tree Python users (`.angreal/test/soak/server.py`, `examples/features/computation-graphs/python-packaged-graph/`) migrate in this task. Tutorials 09/10/11 are explicitly T-0542's territory.

**Deferred / out of scope:**
- Fan-out (one reactor → many graphs): T-0544.
- Tutorial rewrites + how-to + release notes: T-0542.
- Type-compatibility runtime check between reactor payload shape and graph entry signature: skip — Python lives with runtime errors and the check would be fragile.
- Fluent topology API inside `with` block: not parity work.
- Removing `@cloaca.node`: not parity work.

**Implementation milestones** (each commits separately on `i-0101-cg-reactor-decouple`):
- **M1** — `@cloaca.reactor` class decorator + Rust-runtime registration via FFI. Class gains `NAME`/`ACCUMULATORS`/`REACTION_MODE` attributes.
- **M2** — `ComputationGraphBuilder`: drop `react={...}` kwarg, add `reactor=ReactorClass` (split) and trigger-less form. Trigger-less entry contract enforced at `__exit__`. The class also becomes the handle (gets a `NAME` attribute) so `@cloaca.task(invokes=...)` can reference it.
- **M3** — `@cloaca.task(invokes=GraphHandle, post_invocation=fn)`. Body resolves graph by name from runtime registry, calls compiled fn with task context, routes terminals back. Reject reactor-triggered targets at registration.
- **M4** — Migrate `.angreal/test/soak/server.py` and `examples/features/computation-graphs/python-packaged-graph/` to the new surface. Delete the bundled-form parser path; emit a clear migration error pointing at I-0101 if the old kwarg is used.
- **M5** — Python scenario tests covering trigger-less CG invoked by task + `@cloaca.reactor` registration. `angreal test integration --backend sqlite` and `--backend postgres` green before phase transition.

**Context handoff (for the next session):**
- Branch is `i-0101-cg-reactor-decouple`, pushed through commit `9b5bd31` (T-0540 M5).
- Live Python users of bundled-form CG: `.angreal/test/soak/server.py`, `examples/features/computation-graphs/python-packaged-graph/market_maker/graph.py`. Tutorials in `examples/tutorials/python/computation-graphs/{09,10,11}*.py` are explicitly out of scope here — T-0542 owns them.
- Key files: `crates/cloacina-python/src/computation_graph.rs` (existing builder, accumulator decorators, `@cloaca.node`), `crates/cloacina-python/src/task.rs` and `workflow.rs` (task decorator), `crates/cloacina-python/src/lib.rs` (pymodule registration).
- Rust-side reference for the surface to mirror: `crates/cloacina-macros/src/reactor_attr.rs`, the trigger-less branch of `crates/cloacina-macros/src/computation_graph/codegen.rs`, the `invokes` + `post_invocation` paths in `crates/cloacina-macros/src/tasks.rs`.
- No code edits landed for T-0541 yet. Start with M1.

### 2026-04-27 — M1 in progress

Starting `@cloaca.reactor` class decorator. Putting it in a new `crates/cloacina-python/src/reactor.rs` to keep it separate from `computation_graph.rs` (which already pushes 1k LOC). Decorator surface:

```python
@cloaca.reactor(name="risk_signals", accumulators=["alpha","beta"], mode="when_any")
class RiskSignals: pass
```

`mode` kwarg only (criteria_accumulators == accumulators) — that's what the existing bundled-form Python users encoded in `react={"mode": ..., "accumulators": [...]}`, so it's the parity surface. We can extend to a split criteria list later if a scenario needs it; today's two in-tree users (`server.py`, `market_maker/graph.py`) don't.

Decorator: validates name non-empty + accumulators non-empty + unique + mode in {"when_any","when_all"}; sets `NAME`, `ACCUMULATORS`, `REACTION_MODE` class attrs; calls `current_runtime().register_reactor(name, || ReactorRegistration { ... })`. Returns class unchanged.

### 2026-04-27 — M1 done, M2 done

**M1 landed** (commit `8682538`): `crates/cloacina-python/src/reactor.rs` exposes `@cloaca.reactor`. Wired into both the maturin pymodule (`lib.rs`) and the synthetic `cloaca` module created by the loader (`loader.rs`). 6 unit tests, all green.

**M2 landed** (this turn): `ComputationGraphBuilder` rebuilt around two flavors:

- Drop the bundled `react={"mode":..., "accumulators":[...]}` kwarg. If passed, raises a clear migration error pointing at I-0101.
- Add `reactor=ReactorClass` (split form). Class must be `@cloaca.reactor`-decorated — pulls `NAME`/`ACCUMULATORS`/`REACTION_MODE` off the class. Non-decorated classes get a precise `TypeError`.
- Add trigger-less form (omit `reactor=` entirely). Validation in `__exit__`: every node's `inputs=[...]` must be empty. The builder's `NAME` getter exposes the graph name so the instance serves as the `@cloaca.task(invokes=...)` handle.
- Added split-form sanity check: any cache input declared by a node must be in the reactor's accumulator set (catches typos at registration).
- `PythonGraphExecutor` gained `has_reactor: bool`. `build_python_graph_declaration` returns `None` for trigger-less graphs (they aren't reactor-driven).
- Migrated all six existing CG tests to the new API (split form via `@reactor` decorator).
- Added 5 new tests: trigger-less builds + `NAME` exposed; trigger-less rejects cache inputs; bundled-form rejected with migration message; split-form unknown-accumulator rejected; non-decorated class rejected.

All 118 cloacina-python lib tests green. Ready for M3 (`@cloaca.task(invokes=GraphHandle, post_invocation=fn)` body resolution + terminal routing).

### 2026-04-27 — M3 done

`@cloaca.task` now mirrors Rust T-0540 M3+M5:

- Two new kwargs: `invokes=GraphHandle` (a `ComputationGraphBuilder` instance, identified by its `NAME`) and `post_invocation=fn`. Validation at decoration time rejects (a) `post_invocation` without `invokes`, (b) `invokes=` targets that aren't registered yet (the `with` block must precede the `@task`), and (c) reactor-triggered targets (`has_reactor == true`).
- `PythonGraphExecutor` gained `terminal_names()` (terminal nodes in execution order) and an async `execute_trigger_less(ctx)` that runs the graph with a Python `Context` as the entry-node arg. Implementation: refactored `execute_graph_sync` into an `_inner` that threads `Option<&PyObject>` through `build_node_args`; for "root" nodes (no incoming edges, no cache inputs) the trigger-less ctx is prepended as the sole arg.
- `PythonTaskWrapper` carries an `Option<CGInvocation>` (graph_name + terminal_names + post_invocation). Restructured `execute()` so the user body runs first under spawn_blocking+with_gil, then the CG invocation `.await`s outside the GIL, then post_invocation, then on_success — preserving the Rust ordering.
- Terminal routing: each terminal's `serde_json::Value` lands in the task context under the terminal node's name (insert if absent, update if present).
- New `fire_on_failure` helper rebuilds the on_failure callback context across both graph-error and post-invocation-error paths.

Four new M3 tests (in `task.rs::m3_tests`):
1. trigger-less graph invoked by task; verifies terminal lands in context.
2. `post_invocation` runs after graph fires; mutates context which propagates.
3. `post_invocation` without `invokes` rejected at decoration.
4. `invokes=` target that's reactor-triggered rejected at decoration.

All 122 cloacina-python lib tests green (was 118). Ready for M4 (in-tree migration of `.angreal/test/soak/server.py` and `examples/.../python-packaged-graph/market_maker/graph.py`).

### 2026-04-27 — M4 done

Migrated the two in-tree bundled-form Python users:

- `examples/features/computation-graphs/python-packaged-graph/market_maker/graph.py`: added `@cloaca.reactor(name="market_maker", accumulators=["orderbook","pricing"], mode="when_any") class MarketMakerReactor`, switched the builder to `reactor=MarketMakerReactor`.
- `.angreal/test/soak/server.py`: both Python CG fixtures (passthrough and Kafka source) now declare `@cloaca.reactor` classes (`_SoakReactor` / `_SoakKafkaReactor`) and use `reactor=...`. Reactor names are `<graph>_rx` so each fixture has a unique reactor in the runtime registry.
- Bug-fix in passing: the Kafka soak fixture's `output: {"inputs": ["process"]}` was nominally referencing a node as if it were an accumulator (silently produced None under the bundled form); the new split-form validator would reject this. Topology corrected to `process: {"inputs": ["..."], "next": "output"}, output: {}` — same intent, semantically faithful, and passes the new validator.

Both files parse cleanly; the soak f-string templates expand to valid Python under both substitution paths. All 122 cloacina-python lib tests green. Ready for M5 (Python scenario tests + `angreal test integration` runs under both backends).

In-tree bundled-form Python users still on the old surface (M4 migrates these):
- `.angreal/test/soak/server.py:407,498`
- `examples/features/computation-graphs/python-packaged-graph/market_maker/graph.py:28`
