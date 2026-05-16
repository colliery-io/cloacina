---
id: audit-t6-collapse-re-export-shims
level: task
title: "Audit T6: collapse re-export shims and overlapping registry-storage facades"
short_code: "CLOACI-T-0560"
created_at: 2026-05-04T16:10:26.426550+00:00
updated_at: 2026-05-04T20:16:52.174161+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Audit T6: collapse re-export shims and overlapping registry-storage facades

Re-export shims and overlapping facade modules that survived the I-0096 / I-0102 / T-0552 / T-0553 leaf-crate splits. Each one was originally added to soften a migration; the migrations are closed.

## Objective

Pick one canonical import path for each surface and delete the rest. Update callers to use the canonical path.

## Backlog Item Details

### Type
- [x] Tech Debt — import-path consolidation.

### Priority
- [x] P3 — Low. None of these are buggy; they just give reviewers / new contributors the wrong answer about "where does X live."

### Technical Debt Impact
- **Current Problems**: three places to import `FilesystemRegistryStorage` from. Two import paths for `CronEvaluator`. Test conventions diverge from production conventions. Future agents waste cycles reasoning about which path is canonical.
- **Benefits of Fixing**: one place per type; shorter `use` blocks.
- **Risk Assessment**: Low. All consumers are in-tree.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### `crates/cloacina/src/cron_evaluator.rs` (single-line re-export shim)

- [ ] Delete the file. The shim is a 1-line `pub use cloacina_workflow::cron_evaluator::{CronError, CronEvaluator}`.
- [ ] Update the four callers to import from `cloacina_workflow::cron_evaluator` directly:
  - `crates/cloacina/src/cron_trigger_scheduler.rs:48`
  - `crates/cloacina/tests/integration/error_paths.rs:215, 222, 229`
  - `crates/cloacina/tests/integration/scheduler/cron_basic.rs:19`
- [ ] Drop the `pub use` at `crates/cloacina/src/lib.rs:547`.

### `crates/cloacina/src/inventory_entries.rs` (re-export shim)

- [ ] The block at L43-45 re-exports `ComputationGraphEntry, ReactorEntry, TaskEntry, TriggerEntry, TriggerlessGraphEntry` from `cloacina_workflow_plugin`. Macros emit fully-qualified paths to the leaf crate. Only `crates/cloacina/tests/integration/workflow/macro_test.rs:113` uses `cloacina::TaskEntry`.
- [ ] Update the test to import from `cloacina_workflow_plugin::TaskEntry`.
- [ ] Delete the re-exports + the `pub use` at `crates/cloacina/src/lib.rs:570-573`.

### `cloacina-python::reactor::dispatch_runtime_reactors_into_scheduler` re-export

- [ ] `crates/cloacina-python/src/reactor.rs:48` is a test-only re-export. Production code (cloacina-server, cloacinactl, cloacina-daemon) doesn't reference it.
- [ ] Update `crates/cloacina-python/tests/python_reactor_library.rs:91, 116, 161` and `tests/cross_language_fan_out.rs` to import from `cloacina::computation_graph::packaging_bridge::dispatch_runtime_reactors_into_scheduler` (already done at `python_reactor_library.rs:272`).
- [ ] Delete the re-export at `reactor.rs:48`.

### `cloacina-computation-graph::types` "backward compat path" module

- [ ] `crates/cloacina-computation-graph/src/lib.rs:389-392 pub mod types { pub use ... }` — explicit "backward compat path" comment. Consumers: `crates/cloacina/tests/integration/computation_graph.rs:1402, 1810`.
- [ ] Update those tests to import from the canonical paths and delete the `mod types` block.

### Three overlapping registry-storage facades

- [ ] `crates/cloacina/src/dal/mod.rs:42-46` re-exports `FilesystemRegistryStorage` + `UnifiedRegistryStorage`.
- [ ] `crates/cloacina/src/registry/storage/mod.rs:46-47` re-exports the *same two* via `pub use crate::dal::...`.
- [ ] `crates/cloacina/src/dal/unified/workflow_registry.rs` is a third wrapper (covered by T-0555).
- [ ] Pick one canonical path (recommend `cloacina::dal::FilesystemRegistryStorage`). Delete the other facade. Audit + update tests + production code (services.rs uses `cloacina::dal::UnifiedRegistryStorage`).

### `WorkflowRegistryImpl` convenience aliases narrowed

- [ ] Confirm two convenience aliases are kept (used by cloacina-server): `register_workflow_package`, `unregister_workflow_package_by_name`. The other six (`unregister_workflow_package_by_id`, `exists_by_id`, `exists_by_name`, `get_workflow_package_by_id`, `get_workflow_package_by_name`, `list_packages`) are removed via T-0558 (T4) — coordinate.

### Trigger re-exports

- [ ] `crates/cloacina/src/trigger/mod.rs:114, 166` — `crate::Trigger` / `crate::TriggerResult` re-exports + `cloacina::Trigger` re-export form a 3-hop chain `cloacina::Trigger` → `cloacina::trigger::Trigger` → `cloacina_workflow::Trigger`. Flatten to a single re-export at `lib.rs`.

### Test gates

- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` green.
- [ ] `angreal test integration --backend sqlite` + `--backend postgres` green.

## Implementation Notes

### Technical Approach

One commit per shim. Each commit: delete the shim → update each caller's `use` line → run tests. The caller updates are mechanical sed; the test gates catch any miss.

### Dependencies

- The `WorkflowRegistryImpl` convenience-alias narrowing couples to T-0558 (T4).
- The `dal/unified/workflow_registry.rs` deletion couples to T-0555 (T1).

### Risk Considerations

- None substantive. The shims have well-bounded caller sets, all in-tree.

## Resolution: Won't Fix (2026-05-04)

After review with the user, this ticket is **closed as won't-fix**. The audit framing ("re-export shims = dead migration leftovers, delete them") is wrong: every item in this ticket is a legitimate public-API ergonomics choice with zero runtime/compile cost. Re-exports in Rust are symbol aliases — they don't ship code, slow builds, or create circulars. They exist precisely to give consumers short, stable import paths while internals are free to reorganize.

Per-AC rationale:

- **`cron_evaluator.rs` shim** — `pub use cloacina::CronEvaluator` is the documented public surface. Forcing every consumer (and external users) to type `cloacina_workflow::cron_evaluator::CronEvaluator` exposes the leaf-crate split as a public concern. Keep the shim.
- **`inventory_entries.rs` re-exports** — same shape: `cloacina::TaskEntry` is the macro-emit target and the documented public name. Hiding it behind `cloacina_workflow_plugin::TaskEntry` leaks the FFI-types crate boundary into user code.
- **`cloacina-python::reactor::dispatch_runtime_reactors_into_scheduler`** — possibly used by external Python embedders; even if only tests use it today, the cost of leaving it is zero.
- **`cloacina-computation-graph::types` "backward compat path"** — the comment is honest about its purpose; the alternative is breaking external CG-crate consumers for no win.
- **Three registry-storage facades (`dal::` vs `registry::storage::`)** — these aren't redundant, they're two legitimate mental models: DAL-layer access vs. registry-subsystem access. Both are valid import sites depending on the caller's perspective. Picking one and forcing all callers to the other is busywork that buys nothing.
- **Trigger 3-hop chain (`cloacina::Trigger` → `cloacina::trigger::Trigger` → `cloacina_workflow::Trigger`)** — zero-cost. Each hop serves a different consumer (top-level convenience, namespaced, leaf-crate). Flattening saves no bytes and breaks any consumer using the middle path.

The audit findings themselves are accurate observations. The conclusion ("therefore delete them") fights against how Rust public APIs work. If a future change requires renaming or relocating one of these symbols, the shim is the seam that lets that happen without a breaking change.

No code changes. Closing as won't-fix.

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

{Clear statement of what this task accomplishes}

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
