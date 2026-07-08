---
id: version-pinning-for-constructor
level: task
title: "Version pinning for constructor provider resolution (from = name@version)"
short_code: "CLOACI-T-0833"
created_at: 2026-06-29T14:00:01.127657+00:00
updated_at: 2026-07-05T01:25:29.541513+00:00
parent: CLOACI-I-0132
blocked_by: [CLOACI-T-0829]
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Version pinning for constructor provider resolution (from = name@version)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

`from = "name@version"` currently strips `@version` as advisory — the provider is resolved by name only against the provider search path. Add real version resolution/pinning so a constructor reference binds to a specific provider version.

The Rust consumer surface ([[CLOACI-T-0829]]) resolves `from` against one provider dir (`CLOACINA_PROVIDER_PATH` / `./providers`) by package name. Lift: honor `@version` (semver match/pin) when resolving, error on missing/ambiguous version.

**AC:** `from = "prefix@0.1.0"` resolves to that version; a missing/mismatched version errors clearly. Relates to [[CLOACI-T-0827]] (packaging carries the version). Blocked by CLOACI-T-0829.

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

### 2026-07-04 — DONE (branch feat/i0132-completion, commit 9170c1a8)
`@version` pins are now ENFORCED at load at both consumer entry points (`load_constructor_node` + `load_reactor_constructor_node`): after `find_wasm_package` resolves the provider dir, `enforce_version_pin` compares the pin against the resolved `provider.json` version — exact or SEGMENT-prefix match ("0.1" matches 0.1.x, NOT 0.10.x; same semantics as the build-side bundle filter in `provider_bundle::resolve_provider_crate`, so a ref that bundled also loads). Mismatch fails closed naming BOTH versions. Unit tests (parse + boundary) + wasm-lane behavioral test (`version_pin_is_enforced_at_load` in constructor_workflow_node_wasm: exact/prefix/unpinned load, mismatch + 0.10-vs-0.1.x fail) — 4/4 green through wasmtime. Full semver-req operators (`^`, `~`, ranges) remain out of scope (segment-prefix covers the cargo-habit pins).