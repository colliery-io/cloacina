---
id: macro-hygiene-generated-cg-reactor
level: task
title: "Macro hygiene — generated CG/reactor/accumulator code must not require a user serde_json dep"
short_code: "CLOACI-T-0805"
created_at: 2026-06-24T22:10:49.123917+00:00
updated_at: 2026-06-24T22:33:55.297307+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Macro hygiene — generated CG/reactor/accumulator code must not require a user serde_json dep

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

The `#[reactor]` / `#[computation_graph]` / accumulator macros emit **bare `serde_json::`** in their generated code, so every user crate using them must add a `serde_json` dependency (surfaced as `E0433: cannot find serde_json in the crate root` — it broke tutorials 07–10 in CI when the `GraphResult.outputs_json` change landed; worked around in T-0804 by adding `serde_json` to those 4 tutorials). Make the generated code reference `serde_json` via a cloacina re-export so users don't need the dep — and remove the workaround dep from the tutorials to prove it.

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

**2026-06-24 — fully diagnosed; exact recipe below (deferred from the T-0804 CI fire-fight — delicate core-macro change, do with a clean build).**

**Mechanism:** the CG macros already resolve a crate root via `CARGO_CRATE_NAME` — `cloacina-macros/src/computation_graph/codegen.rs:78` (`is_cloacina_crate_early`) → `cloacina_root` = `crate` (internal) / `cloacina` (external) at codegen.rs:197. But `cloacina_root` is **local to `fn generate` (33–457)**; the other emit sites are in different fns, and `accumulator_macros.rs` computes no root at all.

**Step 1 — re-export.** Add `pub use serde_json;` at the root of `crates/cloacina/src/lib.rs` (cloacina already depends on serde_json). Then `crate::serde_json` works internally and `cloacina::serde_json` externally. (Check there's no existing name clash.)

**Step 2 — emit the rooted path** at the EMITTED (`quote!`) sites. A small helper that returns `crate::serde_json` vs `cloacina::serde_json` from the same `CARGO_CRATE_NAME` check, called in each fn that emits, is cleaner than threading a param. Sites:
- `codegen.rs` `fn generate` (root in scope): lines 240, 260, 275, 396, 407 — `::serde_json::Value` in graph_fn `Context<…>`.
- `codegen.rs` `fn generate_compiled_function` (591) + `fn generate_node_execution` (674): 646 (`Vec<::serde_json::Value>`), 727/728 + 747/748 (`::serde_json::Value`, `::serde_json::to_value`, `::serde_json::Value::Null`). **These fns don't have the root — add it.**
- `accumulator_macros.rs` (`passthrough_accumulator_impl` 90, `stream_accumulator_impl` 133): 120, 182, 218 — `serde_json::from_slice(&event)`. **Add the root computation here.**
- **Do NOT touch** the macro-internal serde_json (codegen.rs 914/947 helper `graph_topology_json`, 990/1038 `#[test]`s) — those use cloacina-macros' own dep, not emitted.

**Step 3 — prove it.** Revert the T-0804 workaround: remove `serde_json` from `examples/tutorials/computation-graphs/library/{07,08,09,10}/Cargo.toml`.

**Verification (both contexts matter):** (a) `angreal check crate crates/cloacina` (internal macro use → `crate::serde_json`); (b) build tutorials 07 + 08 **without** the serde_json dep (external → `cloacina::serde_json`); (c) `angreal test macros`. Watch disk — the per-tutorial target dirs are ~3GB each; `rm -rf examples/tutorials/**/target` between builds.

**Note:** `#[task]`/`#[workflow]` macros also reference `serde_json::Value` in emitted code but did NOT break (tutorials 01–06 passed without a serde_json dep), so either they already resolve it via a path or those tutorials transitively get it — worth confirming whether they need the same treatment for consistency.