---
id: generic-invoke-ffi-marshaling-seam
level: task
title: "Generic invoke_ffi marshaling seam — investigate collapsing the 62 per-method-index FFI bridges"
short_code: "CLOACI-T-0873"
created_at: 2026-07-08T14:20:14.965414+00:00
updated_at: 2026-07-08T14:20:14.965414+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Generic invoke_ffi marshaling seam — investigate collapsing the 62 per-method-index FFI bridges

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Tech-debt investigate-and-decide (the ONE "Worth exploring" candidate from the 2026-07-08 architecture deepening review). NOT committed to implement — investigate the seam, decide.

**The shallowness.** ~**62** call sites across `crates/cloacina/src/registry/loader/{package_loader,constructor_loader,ffi_trigger,ffi_triggerless_graph,task_registrar/*}.rs` hand-roll the same shallow FFI bridge per plugin-ABI method index: `serde_json::to_string(&XInvocation) → spawn_blocking { handle.call_method::<_,String>(METHOD_X, &(json,)) } → serde_json::from_str::<XOutcome>`, each with its own `XInvocation`/`XOutcome` struct pair. `ffi_triggerless_graph.rs` literally says *"Same pattern as ffi_trigger.rs but for graphs"*; `constructor_loader.rs` names the pattern outright. The adapter's interface is nearly as simple as its implementation.

**Candidate deepening:** one generic `invoke_ffi::<Req, Res>(handle, METHOD_INDEX, req)` seam owning the JSON round-trip + `spawn_blocking` + FFI-error mapping; the per-index files shrink to their genuinely-unique metadata (poll interval, terminal-output reconstruction, etc.). **Deletion test: PASS** (concentrates the sync/async + serialization boundary where the FFI seam belongs).

**Why tech-debt not initiative:** rated lower than the DAL/GIL/registrar candidates because the per-index metadata differences are slightly more real (each `XInvocation`/`XOutcome` genuinely differs), so the collapse needs a design pass to confirm a generic `<Req,Res>` doesn't fight the per-index specifics. Investigate feasibility + payoff, then decide (fold into an initiative or close). Relates to [[CLOACI-I-0135]]/[[CLOACI-I-0136]]/[[CLOACI-I-0137]] (same "collapse the repeated shallow adapter" theme).

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