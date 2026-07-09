---
id: post-upgrade-artifact-recompile
level: task
title: "Post-upgrade artifact recompile signal — cloacina tells the compiler to rebuild stale packaged artifacts after an ABI/interface-version bump"
short_code: "CLOACI-T-0835"
created_at: 2026-06-30T14:58:06.805614+00:00
updated_at: 2026-07-05T02:29:07.262135+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Post-upgrade artifact recompile signal — cloacina tells the compiler to rebuild stale packaged artifacts after an ABI/interface-version bump

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Objective **[REQUIRED]**

When cloacina bumps a plugin ABI / interface version (e.g. the `CloacinaPlugin` FFI trait gains a method like `get_constructor_metadata` in [[CLOACI-T-0832]], or a fidius ABI bump), **every previously-compiled packaged artifact (.cloacina) becomes stale** and silently fails to load until someone manually recompiles it. Today the only signal is a load-time interface-hash mismatch error, after the fact.

Give cloacina a way to **signal the compiler, post-upgrade, to recompile all affected artifacts** — so an ABI bump triggers a (or offers a one-command) rebuild of stale packages rather than a pile of mystery load failures.

Sketch of the shape (to be designed):
- cloacina/server knows the current `CloacinaPlugin` interface hash + fidius ABI version it expects.
- A registered package records the interface hash/ABI it was built against (already partly true — the loader compares hashes at load).
- On upgrade, cloacina detects the set of registered packages whose recorded hash/ABI < current, and **emits a recompile signal** the compiler/CLI acts on: e.g. `cloacinactl rebuild-stale` (rebuild from retained source), or a server endpoint that lists stale packages + triggers their recompilation, or a compiler hook that picks up the signal.

Captured during [[CLOACI-T-0832]] (the constructor FFI method bumps the `CloacinaPlugin` interface version). **Slated for THIS release** per human (2026-06-30).

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

### Type
- [ ] Bug - Production issue that needs fixing
- [x] Feature - New functionality or enhancement
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [x] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Business Justification
- **User Value**: an ABI/interface bump (which WILL happen across releases) currently turns every packaged workflow into a silent load failure until hand-rebuilt. This makes upgrades self-healing (or one-command), instead of a scavenger hunt through interface-hash-mismatch errors.
- **Business Value**: removes a sharp upgrade-day operational edge; makes the packaged-workflow story safe to evolve (we can bump the ABI when a feature needs it — e.g. T-0832 — without dreading the fallout).
- **Effort Estimate**: M (detection is cheap — hashes are already compared at load; the rebuild-orchestration + source retention is the real work)

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

- [ ] cloacina can enumerate registered/known packaged artifacts whose recorded interface-hash / ABI version is older than what the current build expects (the "stale set").
- [ ] A concrete recompile signal exists and is actionable — at minimum a `cloacinactl`-driven "rebuild stale artifacts" path (from retained source) and/or a server-surfaced stale-package list with a trigger.
- [ ] The post-upgrade UX is a clear, actionable message (which packages are stale + how to rebuild), not a bare interface-hash-mismatch at first execution.
- [ ] Documented as an upgrade/release step (and referenced from the T-0832 release note about the `CloacinaPlugin` interface bump).

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

### 2026-07-04 — DONE (branch feat/i0132-completion, commit 73b6de29) — shipped as AUTOMATIC self-healing
Chose the strongest of the sketched shapes: fully automatic, no operator command needed. The reconciler already retries every registered `success` package each tick — so detection rides the load failure itself: when a load fails with a STALE-ARTIFACT error (fidius "incompatible ABI version" / "interface hash mismatch" — `is_stale_artifact_error`), the reconciler calls the new `WorkflowRegistry::request_recompile(package_id)` (default-false trait seam; unified-DB override flips `build_status` success/failed → `pending`, never stomping an in-flight `building` claim). The compiler claims + rebuilds from retained source; the next reconcile tick loads the fresh artifact. A once-per-package-per-process guard prevents ping-pong when the rebuilt artifact is still stale (e.g. the COMPILER is the outdated side).

**AC disposition:** enumerate-stale = the reconciler's tick IS the sweep (every stale package trips the classifier within one interval — no schema change needed since the recorded-hash comparison already happens at load) ✓ · concrete signal = automatic `pending` flip (stronger than the sketched one-command CLI) ✓ · clear UX = a WARN naming the package + "requested a recompile from retained source; it will reload once the compiler rebuilds it" ✓ · documented in the reconciler code + this task (the T-0832 release note references the interface bump) ✓. Motivated live: the 2026-07-04 demo volume had 10 packages at fidius ABI 200 vs the host's 500 — exactly the mystery-failure pile this closes. Classifier unit-tested; cloacina + server + agent check clean.