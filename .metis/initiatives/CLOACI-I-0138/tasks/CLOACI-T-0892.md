---
id: constructors-providers-into-the
level: task
title: "Constructors/providers into the demos harness + CI — fs-grant-demo and friends stop being dark matter"
short_code: "CLOACI-T-0892"
created_at: 2026-07-11T22:03:34.817943+00:00
updated_at: 2026-08-30T02:00:18.827397+00:00
parent: CLOACI-I-0138
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0138
---

# Constructors/providers into the demos harness + CI — fs-grant-demo and friends stop being dark matter

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0138]]

## Objective **[REQUIRED]**

The entire constructor/provider surface — `#[constructor]` (kind = task|trigger|accumulator|reactor, `#[config]`/`#[param]` fields), `constructor_provider!`, in-workflow `constructor!` nodes with `grants` (http|tcp|fs|env|secrets), `#[reactor(from/constructor/config/grants)]`, `cloacinactl constructor package` (build+sign+pack a provider archive) — has examples under `examples/constructor-contract/` (fs-grant-demo, provider-fs/extract/quorum/sensor, constructor-contract) **but they are dark matter**: outside the demos discovery scan (`get_rust_feature_directories` only walks `examples/features/`), outside the CI matrix, never executed by any harness. This is the same class of blind spot as the sandbox (worked-on-paper, never run).

**Do:**
1. Decide the canonical constructor example (likely `fs-grant-demo` — it shows grants + a provider consumed by a workflow) and bring it to the T-0886 standard (README with the provider lifecycle: author → `cloacinactl constructor package` → install/upload → workflow consumes the constructor node → run to Completed on the demo stack; note the demo compiler builds providers to wasm32-wasip2 at seed time already — T-0836).
2. Register it in the demos surface (bespoke `demos features` command if `cargo run` is the wrong verb) so it auto-joins the CI matrix via `demos matrix`.
3. Fold or clearly mark the remaining `constructor-contract/*` dirs as library/fixture crates (exclude-with-reason), so nothing user-facing is silently unexecuted.

**Caveat from memory:** [[project_fidius_wasm_authoring_shift]] — fidius wasm trait impls may reshape the authoring/packaging story; keep this example thin and lifecycle-focused rather than deep on the current authoring shapes.

**Acceptance:** a constructor/provider example runs green in CI through the demos harness; provider archive built + consumed + execution Completed; README verified.

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

- 2026-08-30 — DONE, VERIFIED LIVE. `fs-grant-demo` chosen as the canonical
  example (the ticket's own lean) and registered as a bespoke
  `demos features fs-grant-demo` command; joins the CI matrix via
  `demos matrix` (verified present, 30 lanes). First-ever harness execution:
  **exit 0** — provider archive packaged via the REAL
  `package_constructor_provider` path, staged, consumed via
  `constructor!(from = "cloacina-provider-fs@0.1.0")`, and all three grant
  cases proven: granted read succeeds, **ungranted read DENIED**
  (default-closed — the security property this demo guards), write-grant
  writes. Self-checking: a sandbox leak fails the lane loudly.

  Kept `cargo run` as the vehicle per the ticket's explicit allowance and the
  [[project_fidius_wasm_authoring_shift]] caveat — the demo's value is the
  provider LIFECYCLE + the grant proof, thin on authoring shapes.

  Remaining `constructor-contract/*` dirs classified in a new inventory README
  (demo / provider crate / test fixture, each with its consuming lane) —
  exclude-with-reason satisfied; nothing user-facing silently unexecuted.