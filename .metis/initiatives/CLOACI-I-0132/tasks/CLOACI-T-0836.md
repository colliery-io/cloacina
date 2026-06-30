---
id: constructor-provider-build-side
level: task
title: "Constructor provider build-side: resolve provider Cargo dep → build to wasm → bundle into the .cloacina (S-0015)"
short_code: "CLOACI-T-0836"
created_at: 2026-06-30T15:57:36.954974+00:00
updated_at: 2026-06-30T15:57:36.954974+00:00
parent: CLOACI-I-0132
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0132
---

# Constructor provider build-side: resolve provider Cargo dep → build to wasm → bundle into the .cloacina (S-0015)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0132]]

## Objective **[REQUIRED]**

Implement the build/distribution half of [[CLOACI-S-0015]] (decision [[CLOACI-A-0010]]): make a constructor **provider a normal Cargo dependency** that the consumer's build resolves → builds to a wasm component → **bundles into the `.cloacina`**, so packaged workflows are hermetic and the server resolves constructors against the bundled provider (no provider dir, no network).

This is the **unblock** for packaged constructors end-to-end: it lets a server load + run a `constructor!`-using workflow (the gate for an examples-based server test), and it lets [[CLOACI-T-0832]]'s held `step_load_constructor_nodes` resolve against the bundled provider.

**Scope (per the S-0015 decisions):**
- **Discovery**: collect every `from = "<crate>"` across the package's `constructor!`/`#[reactor]` declarations; map each to the matching `Cargo.toml` dependency; build+bundle ONLY those. Validate each is a real provider (`__constructor_manifest()` export).
- **`from` = the exact Cargo package name** as declared in `Cargo.toml` (no alias); `@version` optional, must be satisfiable by the resolved dep.
- **Locate** each provider crate via `cargo metadata` in the consumer's resolved dep graph (crates.io / path / git uniformly).
- **Build + pack** each via the existing `package_constructor_provider` flow (cargo build → wasm32-wasip2 → fidius pack).
- **Bundle** each as a nested fidius provider package under `providers/<crate>-<version>/` inside the `.cloacina`; record the `from`→bundled-dir map in the workflow manifest.
- (Fast-follow) cache built providers keyed on (crate, version, source, fidius interface hash).

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P1 - High (important for user experience)

### Business Justification
- **User Value**: lets a packaged workflow USE constructors and deploy hermetically — the primary consumer window the whole feature targets.
- **Effort Estimate**: L (new build orchestration in cloacinactl/compiler + the .cloacina bundle format wiring).

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
