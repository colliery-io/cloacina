---
id: audit-t1-delete-dead-modules
level: task
title: "Audit T1: delete dead modules uncovered by post-I-0102 workspace sweep"
short_code: "CLOACI-T-0555"
created_at: 2026-05-04T16:10:18.790101+00:00
updated_at: 2026-05-04T19:48:39.931687+00:00
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

# Audit T1: delete dead modules uncovered by post-I-0102 workspace sweep

Whole modules / files surfaced by the nine-agent post-I-0102 workspace audit (2026-05-04) with zero non-self, non-test callers. Each item below is a single-PR delete candidate; total ~3000 LOC across the workspace.

## Objective

Delete modules where every public surface has been confirmed orphan by workspace-wide grep across `crates/`, `examples/`, and `tests/`. None of these are user-facing API; nothing outside the listed module/file references them.

## Backlog Item Details

### Type
- [x] Tech Debt — readability + grep-noise reduction; no behavior change.

### Priority
- [x] P2 — Medium. None blocks users; the LOC win + reduction in dual-implementation footguns is the value.

### Technical Debt Impact
- **Current Problems**: Audit agents kept tripping on dead surface (e.g. two competing `PyTriggerResult` APIs because the duplicate cdylib bindings module is still alive). Future audits + onboarding hit the same noise.
- **Benefits of Fixing**: ~3000 LOC delete; clearer "what's the real path" answer for FFI / packaging / Python; eliminates the documented divergence between wheel-mode and embedded-mode trigger result shapes.
- **Risk Assessment**: Low. Confirmed zero callers via `grep -rn` across the workspace. Tests must stay green.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] **`cloacina-python/src/trigger.rs::PyTriggerResult`** deleted (the `should_fire/context` API). Verified during T-0557 Bug 5 work that live Python tests at `tests/python/test_scenario_29_event_triggers.py:18,28,39,67` use the `skip/fire` API from `bindings/trigger.rs`, not `should_fire/context`. Correction to original audit direction: KEEP `bindings/trigger.rs` (the winner), delete the `PyTriggerResult` class from `src/trigger.rs`, reroute `cloacina-python/src/loader.rs:126` to register `bindings::trigger::PyTriggerResult`, and update Rust fixtures at `cloacina-python/tests/trigger_packaging.rs:270, 321` from `.should_fire/context` form to `.skip()` / `.fire()`. The rest of `src/trigger.rs` (TriggerDecorator, PythonTriggerWrapper) stays — only `PyTriggerResult` itself goes.
- [ ] **`cloacina-python/src/executor.rs`** deleted (`PythonTaskExecutor` trait + `PythonExecutionError` + `PythonTaskResult` — zero impls). `lib.rs:33-34,47` re-export lines removed.
- [ ] **`crates/cloacina/src/registry/loader/validator/`** subtree deleted (~600 LOC). `WorkflowRegistryImpl::validator` field (`registry/workflow_registry/mod.rs:54`, `#[allow(dead_code)]`) and `with_strict_validation` constructor (`:88`) removed.
- [ ] **`crates/cloacina/src/packaging/debug.rs`** deleted (`debug_package`, `extract_manifest_from_package`, `DebugResult`, `TaskDebugInfo`). `packaging/mod.rs:33` re-export removed.
- [ ] **`crates/cloacina/src/packaging/manifest.rs::generate_manifest`** + supporting types (`PackageMetadata`, `FfiTaskInfo`, `extract_task_info_and_graph_from_library`) removed. Reconciler reads `CloacinaMetadata` direct from fidius; `generate_manifest` is only exercised by `packaging/tests.rs`. Three corresponding tests deleted.
- [ ] **`crates/cloacinactl/src/shared/output.rs`** deleted entirely (`Renderable`, `emit`, `render_serialized`, `Redacted`). `shared/mod.rs:23` `pub mod output;` removed. Live render path is `shared/render.rs`.
- [ ] **`crates/cloacina-macros/src/packaged_workflow.rs`** trimmed: extract the 5–7 live helpers (`detect_package_cycles`, `dfs_package_cycle_detection`, `find_similar_package_task_names`, `calculate_levenshtein_distance`, `build_package_graph_data`, `calculate_max_depth`, `calculate_task_depth`) to a new `packaged_workflow_helpers.rs`. Delete `pub fn packaged_workflow` (L1260), `generate_packaged_workflow_impl` (L499), `PackagedWorkflowAttributes` (L80), the emitted `TaskMetadata`/`TaskMetadataCollection` codegen (L34/L55) — ~800 of 1292 LOC.
- [ ] **`crates/cloacina-macros/src/computation_graph/codegen.rs`** `let packaged_ffi = quote! { … }` block (L240-376, ~135 LOC) deleted. The marker `_packaged_ffi_was_stripped_in_t_c()` (L573-574) confirms it never lands in the final emit. Drop `ffi_accumulator_entries_expr` / `ffi_reaction_mode_expr` slots from the destructure (L189-190 + the assignments at L209-211, L226-228) since their only consumers were inside the dead block.
- [ ] **`crates/cloacina/src/trigger/registry.rs`** (16 LOC) deleted; `pub mod registry;` and the `pub use` at `trigger/mod.rs:51,168` removed. The exported `TriggerConstructor` alias has zero callers — superseded by `Runtime::TriggerConstructorFn`.
- [ ] **`crates/cloacina/src/dal/unified/workflow_registry.rs`** (35-LOC `WorkflowRegistryDAL` TODO stub with only a `_dal: &'a DAL` field) deleted; `dal/unified/mod.rs:69` registration removed.

### Test gates

- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` green.
- [ ] `angreal test integration --backend sqlite` + `--backend postgres` green.

## Implementation Notes

### Technical Approach

Mechanical deletion. One module per commit, each commit titled `Audit T1: delete <module>`. The Python `bindings/trigger.rs` removal is the riskiest because it forces an API choice between `skip/fire` (current wheel-mode) and `should_fire/context` (current embedded-mode). The audit recommends keeping `skip/fire` since live Python tests already use it; update the two Rust fixture sites in `cloacina-python/tests/trigger_packaging.rs` accordingly.

### Dependencies

None. Independent of the other Tier tickets.

### Risk Considerations

- **Python `TriggerResult` API choice** — must be made + announced; the change is a breaking shape difference for any external consumer relying on `should_fire/context`. Audit found zero such external consumers but signal in CHANGELOG.
- **`packaged_workflow.rs` helper extraction** — preserve the 5–7 live helpers verbatim; their callers in `workflow_attr.rs:35-37,208,229,341` must keep compiling.

## Status Updates

### 2026-05-04 — T-0555 done in one commit (6e2e6bb)

All 10 dead modules / files deleted in a single commit. Net: 24 files, 41 insertions, 3268 deletions.

**Module-level deletions:** `cloacina/src/dal/unified/workflow_registry.rs` (35 LOC stub), `cloacina/src/trigger/registry.rs` (16 LOC dead alias), `cloacinactl/src/shared/output.rs` (Renderable / emit / render_serialized / Redacted), `cloacina-python/src/executor.rs` (PythonTaskExecutor trait + types), `cloacina/src/registry/loader/validator/` subtree (~600 LOC), `cloacina/src/packaging/debug.rs`, `cloacina/src/packaging/manifest.rs`.

**Trimmed in-place:** `cloacina-macros/src/packaged_workflow.rs` 1292 → 282 LOC (kept the live helpers, dropped the orphan `#[packaged_workflow]` attribute machinery). `cloacina-macros/src/computation_graph/codegen.rs` lost the dead `let packaged_ffi = quote! { ... }` block + supporting tuple slots. The `_packaged_ffi_was_stripped_in_t_c()` marker fn is also gone.

**Coordinated with T-0557 Bug 5:** the `cloacina-python/src/bindings/trigger.rs` deletion was reversed during T-0557 — `bindings/trigger.rs` is the canonical winner shipping the skip/fire `TriggerResult` API. The `PyTriggerResult` class deletion in `src/trigger.rs` already landed in commit `b521316`.

**Field/method drops included in the same commit:** `WorkflowRegistryImpl::with_strict_validation` constructor + the `validator: PackageValidator` field; three `test_generate_manifest_*` tests + `test_manifest_error_display` + `test_extract_package_names_from_source` variants in `packaging/tests.rs`.

**Test gates (all green):**
- [x] `cargo check --workspace --all-features`
- [x] `angreal test unit` (659 cloacina + 45 cloacina-workflow; was 689 + 45 — the 30-test reduction reflects the dead-code tests we deleted alongside the dead code)
- [x] `angreal test integration --backend sqlite` (6 + 28 Python)
- [x] `angreal test integration --backend postgres` (295 Rust + 28 Python)

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
