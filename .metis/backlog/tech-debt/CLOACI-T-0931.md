---
id: constructor-loader-hand-rolls-json
level: task
title: "constructor_loader hand-rolls JSON marshaling while every other FFI bridge passes typed values"
short_code: "CLOACI-T-0931"
created_at: 2026-08-16T14:32:50.577879+00:00
updated_at: 2026-08-16T14:32:50.577879+00:00
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

# constructor_loader hand-rolls JSON marshaling while every other FFI bridge passes typed values

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Successor to [[CLOACI-T-0873]], which asked whether to build a generic
`invoke_ffi::<Req,Res>` seam and concluded **no** — because investigating it
surfaced a better question. Read T-0873's 2026-08-16 status update for the full
evidence; the short version:

The loader's FFI bridges are **two families**, not one.

* **Family A — typed `call_method`; fidius owns serialization.** No JSON.
  `ffi_trigger.rs`, `ffi_triggerless_graph.rs`,
  `task_registrar/dynamic_task.rs`, `package_loader.rs` (×3),
  `task_registrar/extraction.rs`. ≈ 8 sites.
* **Family B — manual `serde_json` string round-trip.** `constructor_loader.rs`
  only: `METHOD_EXECUTE`, `METHOD_POLL`, `METHOD_INGEST`, `METHOD_EVALUATE`.
  4 sites. Each does
  `to_string(&XInvocation)` → `call_method::<_, String>(M, &(json,))` →
  `from_str::<XOutcome>`.

Family A already has the generic marshaling seam — it is fidius's typed
`call_method`. Family B re-implements by hand, per method index, what the rest
of the loader gets for free.

**The work:** convert family B to typed calls, DELETING the manual
serialization layer rather than wrapping it. This is the deepening T-0873 was
reaching for; wrapping would have added a layer, removing subtracts one, and it
makes the loader internally consistent so there is one way to cross the FFI
boundary instead of two.

**Why this is bigger than it looks (the reason it is its own ticket):** the
guest side exports these four methods taking `(String,)`. Changing the host to
pass typed values requires changing the guest signatures too — a plugin **ABI
change** needing an interface-version bump in
`crates/cloacina-workflow-plugin`, plus the usual rebuild-every-fixture
consequences. Not a refactor that stays inside one crate.

**Watch out for `METHOD_INGEST`:** unlike its three siblings it is
*synchronous* (`process` is sync — no `spawn_blocking`) and returns `Option`,
logging via `tracing::error!` + `return None` instead of propagating an error.
Any uniform treatment has to accommodate that or deliberately leave it alone.

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P3 - Low (no user-visible defect; consistency + one-way-to-do-it. The ABI
      bump means it is best bundled with another interface-version change
      rather than spent on its own.)

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

- [ ] `constructor_loader.rs` contains no `serde_json::to_string` /
      `from_str` around a `call_method` — the four bridges pass typed values.
- [ ] Guest-side signatures updated to match, with the plugin interface version
      bumped and the version check rejecting stale packages.
- [ ] `METHOD_INGEST`'s sync/`Option` shape either accommodated or explicitly
      left as-is with the reason recorded in-code.
- [ ] All packaged fixtures rebuilt and the packaged e2e lane green
      (`angreal test e2e compiler`) — this is an ABI change, so a passing
      `cargo check` proves nothing about whether real packages still load.
- [ ] Verified against a live server, not just unit tests: a constructor-backed
      workflow executes end to end (constructors are the surface T-0925 touched).

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