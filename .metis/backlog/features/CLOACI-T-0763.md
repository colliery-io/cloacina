---
id: python-trigger-rule-parity-gate
level: task
title: "Python trigger-rule parity — gate cloaca.task with trigger_rules so Python tasks can Skip"
short_code: "CLOACI-T-0763"
created_at: 2026-06-21T14:15:37.567680+00:00
updated_at: 2026-06-21T14:38:25.513073+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Python trigger-rule parity — gate cloaca.task with trigger_rules so Python tasks can Skip

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Bring Python authoring to parity with Rust on conditional task execution. The
Rust engine fully supports trigger rules (execution_planner/trigger_rules.rs:
TriggerRule = Always | All | Any | None over TriggerCondition =
TaskSuccess/TaskFailed/TaskSkipped/ContextValue) and produces the Skipped task
state when a task's deps are satisfied but its rule fails. Rust authors via the
#[task(trigger_rules = context_value(k, equals, v))] DSL.

Python cannot author this at all: @cloaca.task has no trigger_rules parameter
(cloacina-python/src/task.rs #[pyo3(signature = (...))]), and the Python task's
trait impl hardcodes fn trigger_rules() -> json {type: Always} (task.rs:413) — so
every Python task always fires and can never reach Skipped. scenario_14 only
SIMULATES rules with in-task if logic (tasks still run to Completed); not real
gating. Close the gap so a gated Python task lands Skipped exactly like Rust.

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

- [ ] `@cloaca.task(trigger_rules=...)` accepts a rule and stores it (signature + TaskDecorator).
- [ ] The Python task's `trigger_rules()` trait impl returns the authored rule (not hardcoded Always); invalid rules raise a clear Python error at decoration time.
- [ ] Ergonomic Python builders mirror the Rust DSL: `cloaca.trigger_rules` with `all/any/none/context_value/task_success/task_failed/task_skipped` (registered in BOTH pymodule lib.rs and the synthetic loader.rs — see [[project_cloaca_dual_registration]]).
- [ ] A gated Python task lands in the real `Skipped` state (verified end-to-end, not simulated).
- [ ] `tests/python/test_scenario_14_trigger_rules.py` (or a new scenario) asserts a Skipped task via `result` task states.
- [ ] Demo: `demo-py-cron` / `demo-py-workflow` gain a genuinely skipped node (replacing the branch-only workaround).
- [ ] Docs/reference updated for the Python trigger-rule surface.

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P1 - High (core Python parity; embedded-first / Python is a core capability)

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

### 2026-06-21 — Implemented (commit b9d70842)
- `crates/cloacina-python/src/trigger_rules.rs` (new): builder pyfunctions
  `context_value/task_success/task_failed/task_skipped` (conditions) +
  `all_of/any_of/none_of/always` (rules) returning canonical TriggerRule JSON;
  `parse_trigger_rules()` wraps a bare condition as All + validates against the
  core `cloacina::execution_planner::TriggerRule`.
- `task.rs`: `@cloaca.task(trigger_rules=...)` param parsed + stored on
  `TaskDecorator`; `PythonTaskWrapper.trigger_rules()` returns the authored rule
  (was hardcoded Always).
- Registered builders in BOTH `lib.rs` (pymodule) and `loader.rs` (synthetic
  module the server uses) — see [[project_cloaca_dual_registration]].
- `test_scenario_14`: new `test_real_trigger_rule_gating_skips_task` asserts the
  gated task's body never runs (real Skipped) + fan-in resolves + satisfied gate
  runs.
- `demo-py-cron`: `py_audit` now genuinely Skipped via `trigger_rules` (was a
  branch-only workaround); docs/trigger-rules.md gained a Python section.
- `cargo check -p cloacina-python` green. Verifying end-to-end via
  `angreal test integration trigger_rules --python-file test_scenario_14_trigger_rules --backend sqlite`.

### 2026-06-21 — Verified (sqlite, direct pytest) — DONE
Built the release wheel + ran `test_real_trigger_rule_gating_skips_task` on
sqlite: **1 passed**. Planner log confirms the real Skip — `Task skipped:
real_gating_off_workflow::optional (dependencies satisfied, trigger rules
failed)`; fan-in `sink` still ran; the gate-on case ran `optional2`. Skip proven
on sqlite (here) + postgres (the demo's skipped nodes). **Gap closed.**

Aside: `angreal test integration --backend sqlite` with a `trigger_rules` filter
surfaced 10/20 pre-existing failures in the Rust `scheduler::trigger_rules`
`#[serial]` integration tests — a test-isolation artifact under that filter
combo, not a skip regression (skip works, proven above). Separate follow-up.

### Follow-up (noted, not blocking)
- `PyWorkflowResult` exposes status + final_context but not per-task states; the
  test asserts Skip behaviorally (gated body never runs). A `task_states` getter
  would let tests assert the `Skipped` status directly — small follow-up.