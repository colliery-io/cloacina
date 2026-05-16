---
id: audit-t2-excise-dead-branches-in
level: task
title: "Audit T2: excise dead branches in reconciler load_package + scheduler"
short_code: "CLOACI-T-0556"
created_at: 2026-05-04T16:10:20.238766+00:00
updated_at: 2026-05-04T20:07:46.953244+00:00
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

# Audit T2: excise dead branches in reconciler load_package + scheduler

Dead branches inside live files surfaced by the post-I-0102 workspace audit. The unified pipeline (T-0554) and the FFI bridges (T-0553 follow-ups) made several long-running branches unreachable; the audit confirmed each via grep + control-flow analysis.

## Objective

Excise the unreachable branches in `crates/cloacina/src/registry/reconciler/loading.rs` and the bundled-form fallbacks in `crates/cloacina/src/computation_graph/scheduler.rs::load_graph` that the unified pipeline + I-0101 split form made obsolete.

## Backlog Item Details

### Type
- [x] Tech Debt — pure dead-code removal inside live files.

### Priority
- [x] P2 — Medium. ~150 LOC delete in the hottest file in the codebase (`loading.rs`); easier audits going forward.

### Technical Debt Impact
- **Current Problems**: `loading.rs` has a `language == "rust"` arm in step 7 that the unified pipeline above guarantees is unreachable; the file's own comment at L434 admits "T-0554: Rust CG handling moved into the unified pipeline above" yet the dead arm remains. Reviewers + future agents waste time tracing it.
- **Benefits of Fixing**: ~150 LOC trimmed; one fewer language-shaped fork to reason about.
- **Risk Assessment**: Medium. The branches are inside the package-load critical path; a careless change could regress the reconcile() pipeline. Integration tests on both backends must pass after each commit.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] **`loading.rs:441-549`** Rust-CG fallback branch (~110 LOC) deleted. The unified pipeline at L282 always populates `rust_graph_name` for Rust packages, so `if rust_graph_name.is_some() { rust_graph_name } else if has_computation_graph() { … rust branch … }` always takes the first arm. Drop the `else if … rust` block; the Python branch becomes the only fallback.
- [ ] **`loading.rs` Python workflow + Python CG branches** consolidate the duplicated pre/post-snapshot diff scaffolding (L321-339 vs L562-576) into a single `snapshot_runtime_registries(&self) -> (HashSet, HashSet, HashSet)` helper invoked from both branches.
- [ ] **`scheduler.rs::load_graph_split`** (`scheduler.rs:635-673`) made `#[cfg(test)]` or moved to the test module. Production reconciler calls `load_reactor` + `bind_graph_to_reactor` directly; only integration tests at `tests/integration/computation_graph.rs` use this helper.
- [ ] **`scheduler.rs::load_graph` empty-accumulators short-circuit** (L583-601) audit: confirm it covers a path that nothing in production exercises post-I-0101. If true, delete + add a unit test asserting the bundled-form path still works for the bundled-form `load_graph(decl)` call shape.
- [ ] Bundled-form back-compat in `scheduler.rs` audit: the alias-name registration (L243-244, L314-315, L754-755, L953-954), the `__Reactor_<name>` synthetic naming (L570), and `unload_graph`'s last-subscriber teardown (L764-787) are all "kept for bundled-form callers." Audit whether any in-tree code still produces bundled-form CGs (search for `#[computation_graph]` invocations without `trigger = reactor(...)`); if not, delete.
- [ ] `loading.rs` second call site of `build_declaration_from_ffi` at L486 (inside the dead Rust-CG branch) is removed; the remaining call at L1881 stays.
- [ ] `loading.rs::seed_from_inventory` call at L1942 (post-dlopen) gets a corrected comment: cross-cdylib triggers/reactors do NOT flow through this path (FFI bridges do); the call only reseeds late host-side inventory submissions.

### Test gates

- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` green.
- [ ] `angreal test integration --backend sqlite` + `--backend postgres` green.

## Implementation Notes

### Technical Approach

Best done as 4 sequential commits:

1. Delete the Rust-CG fallback branch + its `build_declaration_from_ffi` call.
2. Extract the snapshot helper + dedupe Python branches.
3. Move `load_graph_split` to `#[cfg(test)]`.
4. Audit + (if confirmed) drop bundled-form scheduler back-compat.

### Dependencies

Independent of T1, T3-T8. Some bundled-form-removal work overlaps with T-0539 (closed) but goes further.

### Risk Considerations

- **Bundled-form removal** is the largest behavior question. Confirm no in-tree fixtures use bundled form before deleting; check `examples/features/workflows/` and `examples/tutorials/` for any `#[computation_graph]` without an explicit `trigger = reactor(...)`.
- **`load_graph_split`** has integration-test users; moving rather than deleting preserves them.

## Status Updates

### 2026-05-04 — T-0556 done in commit e79ac63

Net: 2 files, 54 insertions, 153 deletions.

- Removed the ~110-LOC Rust-CG fallback branch in `loading.rs:441-549`. Unified pipeline always populates `rust_graph_name` for Rust packages, so the `else if has_cg() { if "rust" { ... } }` arm was unreachable. Simplified to `else if has_cg() { if "python" { ... } else { ... unsupported } }`.
- Added `snapshot_runtime_registries()` helper on `RegistryReconciler`. Both Python load paths (workflow + CG) now one-line that instead of duplicated 12-line snapshot blocks.
- Corrected the misleading post-dlopen `seed_from_inventory()` comment to name the FFI bridges as the cross-cdylib path and flag the re-seed as a no-op for typical packaged-cdylib loads.
- Documented `scheduler.rs::load_graph_split` as a test-only convenience API. Audit confirmed zero non-test callers; gating with `#[cfg(test)]` doesn't help since integration tests are external crates.
- Audited bundled-form CG back-compat: all 9 in-tree example/tutorial sites use split-form (`trigger = reactor(...)`). The scheduler's bundled-form fallbacks only serve hand-constructed `ComputationGraphDeclaration` calls (integration tests + `load_graph_split`). Left in place — deletion would risk external consumers constructing `ComputationGraphDeclaration` directly. Audit conclusion captured in the commit message.
- The empty-accumulators short-circuit in `load_graph` is the live production cross-package fan-out path (verified by T-0554 e2e test). Kept.

**Test gates (all green):**
- [x] `cargo check --workspace --all-features`
- [x] `angreal test unit` (659 cloacina + 45 cloacina-workflow)
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
