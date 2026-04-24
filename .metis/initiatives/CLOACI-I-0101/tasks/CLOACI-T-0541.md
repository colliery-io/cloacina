---
id: t-03-python-decorator-parity-for
level: task
title: "T-03: Python decorator parity for split CG + CG-invoking task"
short_code: "CLOACI-T-0541"
created_at: 2026-04-24T15:08:19.954995+00:00
updated_at: 2026-04-24T15:08:19.954995+00:00
parent: CLOACI-I-0101
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0101
---

# T-03: Python decorator parity for split CG + CG-invoking task

## Parent Initiative

[[CLOACI-I-0101]]

## Objective

Mirror the Rust declaration surface from T-01a and T-02 in the Python bindings. `@cloaca.computation_graph` accepts the `trigger = reactor("name")` kwarg; `@cloaca.task` accepts `invokes = computation_graph("name")`. Python is dynamic, so validation is runtime, not compile-time, but the authoring experience must match what Rust users see.

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

*To be added during implementation.*
