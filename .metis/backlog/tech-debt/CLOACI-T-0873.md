---
id: generic-invoke-ffi-marshaling-seam
level: task
title: "Generic invoke_ffi marshaling seam — investigate collapsing the per-method-index FFI bridges"
short_code: "CLOACI-T-0873"
created_at: 2026-07-08T14:20:14.965414+00:00
updated_at: 2026-08-16T14:33:24.380501+00:00
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

# Generic invoke_ffi marshaling seam — investigate collapsing the per-method-index FFI bridges

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Tech-debt investigate-and-decide (the ONE "Worth exploring" candidate from the 2026-07-08 architecture deepening review). NOT committed to implement — investigate the seam, decide.

**The shallowness.** (Filed as "~62 call sites"; the 2026-08-10 audit below
measured **16** `call_method` sites over **12** method indices sharing **8**
shared contract types. Corrected here — the original figure was never accurate,
and it inflated this ticket's apparent priority.) Call sites across `crates/cloacina/src/registry/loader/{package_loader,constructor_loader,ffi_trigger,ffi_triggerless_graph,task_registrar/*}.rs` hand-roll the same shallow FFI bridge per plugin-ABI method index: `serde_json::to_string(&XInvocation) → spawn_blocking { handle.call_method::<_,String>(METHOD_X, &(json,)) } → serde_json::from_str::<XOutcome>`, each with its own `XInvocation`/`XOutcome` struct pair. `ffi_triggerless_graph.rs` literally says *"Same pattern as ffi_trigger.rs but for graphs"*; `constructor_loader.rs` names the pattern outright. The adapter's interface is nearly as simple as its implementation.

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

- 2026-08-10 — BACKLOG AUDIT. Verdict: **premise still true, but the SCALE is
  overstated — and that changes the decision this ticket exists to make.**

  Still true: there is no generic seam. `grep "fn invoke_ffi"` finds nothing,
  and the per-index bridges are still hand-rolled exactly as described.

  **The "~62 call sites" figure is not supported by the code.** Counted in the
  files this ticket names (`crates/cloacina/src/registry/loader/**`):
    * `call_method` call sites: **16**
    * distinct `METHOD_*` indices referenced: **12**
    * `spawn_blocking` sites: 24 (not all are FFI bridges)
    * `*Invocation` / `*Outcome` structs: **8 total**, all living in
      `cloacina-constructor-contract/src/lib.rs` (`TaskInvocation`,
      `TriggerInvocation`, `AccumulatorInvocation`, `ReactorInvocation`, …) —
      i.e. SHARED contract types, not "its own struct pair" per call site as
      the objective states.

  So the real shape is ~16 bridge sites over 12 indices sharing 8 contract
  types — roughly a quarter of the stated duplication.

  WHY THIS IS MORE THAN PEDANTRY: this ticket is explicitly "investigate the
  seam, DECIDE — NOT committed to implement", and it ranks itself below the
  DAL/GIL/registrar collapses. Those siblings earned their rank on volume
  (I-0135 collapsed ~168 DAL method twins). At 16 sites — with this ticket's
  own caveat that "the per-index metadata differences are slightly more real"
  — the payoff case is materially weaker than the headline implies. Anyone
  triaging off the title would rate this far higher than the code warrants.

  Not merely stale, either: T-0925 ADDED `_in` variants in
  `constructor_loader.rs` since filing, so the count has grown since 2026-07-08
  rather than shrunk. The 62 was never accurate.

  RECOMMEND: keep open as investigate-and-decide, but retitle to drop the "62"
  (see [[feedback_metis_title_embedded_quotes]] for the retitle mechanics), and
  expect the honest answer to be "not yet". Revisit if a future plugin-ABI
  version adds several more method indices — the seam gets more attractive as
  the index count grows, and that is the trigger to watch for.

- 2026-08-16 — INVESTIGATED AND DECIDED: **do not build the proposed seam.**
  Closing. The reason is not "too small" — it is that the premise does not
  survive reading the call sites instead of counting them.

  **The bridges are TWO families, not one.**

  *Family A — typed `call_method`; fidius owns serialization. No JSON at all:*
    * `ffi_trigger.rs` — `call_method(METHOD_INVOKE_TRIGGER_POLL, &request)`,
      `TriggerInvokeRequest` → `TriggerInvokeResult`
    * `ffi_triggerless_graph.rs` — same, `TriggerlessGraphInvokeRequest`
    * `task_registrar/dynamic_task.rs` — `call_method(METHOD_EXECUTE_TASK, &(request,))`
    * `package_loader.rs` (×3) and `task_registrar/extraction.rs` — typed,
      e.g. `call_method::<(), GraphPackageMetadata>(..)`
    ≈ 8 sites.

  *Family B — manual `serde_json` string round-trip.* `constructor_loader.rs`
  ONLY: `METHOD_EXECUTE`, `METHOD_POLL`, `METHOD_INGEST`, `METHOD_EVALUATE`.
  4 sites.

  The objective presents
  `to_string(&XInvocation) → call_method::<_,String> → from_str::<XOutcome>`
  as the universal pattern. It is family B only. **Family A already has the
  generic marshaling seam this ticket proposes building — it is fidius's typed
  `call_method`.** Wrapping that would add a layer, not remove one.

  **So the seam serves 4 sites, and those 4 disagree with each other:**
    * `METHOD_EXECUTE` — async, fails as `TaskError` (via `exec_err`)
    * `METHOD_POLL` — async, fails as `TriggerError::PollError`
    * `METHOD_EVALUATE` — async, fails as `LoaderError::Validation`
    * `METHOD_INGEST` — **synchronous**, no `spawn_blocking` at all (`process`
      is sync), and returns `Option`: `tracing::error!` + `return None` rather
      than propagating an error

  Three error types across three sites, and the fourth has a different control
  shape entirely. A generic `invoke_ffi::<Req, Res>` would cover 3 sites and
  each would still need its own `map_err`. This is precisely the ticket's own
  stated fear — "the per-index metadata differences are slightly more real" —
  confirmed by reading rather than assumed.

  **THE REAL DEEPENING IS A DIFFERENT ONE.** The interesting question is not
  "how do we wrap constructor_loader's JSON round-trip?" but "why does
  constructor_loader hand-roll JSON strings when every other bridge passes
  typed values and lets fidius serialize?" Converting family B to typed calls
  DELETES the manual serialization layer rather than wrapping it — strictly
  better, and it makes the loader internally consistent. It is also a bigger
  job: the guest exports these methods taking `(String,)`, so it changes the
  plugin ABI and needs an interface-version bump. Filed as [[CLOACI-T-0931]]
  rather than smuggled in here.

  DECISION: close. Not "deferred with a trigger" — the proposed design is
  superseded, because T-0931 reaches the same goal by removing the duplication
  instead of centralizing it. No code changed under this ticket; the
  deliverable is the decision and its evidence.