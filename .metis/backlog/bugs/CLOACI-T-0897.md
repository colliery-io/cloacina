---
id: bug-trigger-less-computation
level: task
title: "BUG: trigger-less computation graphs and task-to-graph invocation never compiled in packaged crates — macro emits umbrella-crate paths"
short_code: "CLOACI-T-0897"
created_at: 2026-07-12T01:49:31.319227+00:00
updated_at: 2026-07-12T01:49:31.319227+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


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

*To be added during implementation*
