---
id: scaffold-pin-spike-decision-does-a
level: task
title: "Scaffold-pin spike + decision — does a 0.7-scaffolded package build against current injected deps"
short_code: "CLOACI-T-0869"
created_at: 2026-07-08T11:36:26.546877+00:00
updated_at: 2026-07-08T11:47:49.208714+00:00
parent: CLOACI-I-0134
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0134
---

# Scaffold-pin spike + decision — does a 0.7-scaffolded package build against current injected deps

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0134]]

## Objective **[REQUIRED]**

Spike (D-5): decide whether `CLOACINA_CRATE_VERSION` (scaffold pin for generated packages' `cloacina-*` deps) should TRACK the release or stay deliberately pinned, by finding out if it actually binds resolution.

## Status Updates **[REQUIRED]**

### 2026-07-08 — DONE — verdict: TRACK the release (it's load-bearing + a real bug)
`crates/cloacinactl/src/nouns/package/new.rs:39` `CLOACINA_CRATE_VERSION = "0.7"` (comment: "Version pin for the generated Rust package's cloacina-* dependencies"), set at I-0119 (2026-06-14, commit da3f4e3f) and NEVER bumped for 0.8/0.9. The compiler does NOT patch/override the dep version — `build.rs` just runs cargo (optionally with a `--vendor-dir` CARGO_HOME + `--frozen`) against the generated `Cargo.toml`. So `cloacina-workflow = { version = "0.7" }` is a real `^0.7` requirement (`>=0.7.0,<0.8.0`) that WON'T resolve against a 0.9/0.10 crate (0.x minors are semver-incompatible) — offline it fails "not available", online it pulls a stale incompatible 0.7.x. Internal tests miss it because fixtures use `path` deps, not `cloacinactl package new` output; it bites real users only.
**Decision:** the pin must TRACK the release. The bump command (T-0867) sets it to the release's `major.minor` (style stays minor-precision, e.g. `"0.10"`). The drift guard (T-0868) asserts `CLOACINA_CRATE_VERSION`'s minor == the workspace version's minor. (NOT deliberately pinned; no whitelist.)

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

*To be added during implementation*