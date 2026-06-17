---
id: python-migration-tutorial
level: task
title: "Python migration + tutorial reclassification (embedded tutorials -> /embed, Model A merge)"
short_code: "CLOACI-T-0700"
created_at: 2026-06-15T19:06:11.793922+00:00
updated_at: 2026-06-15T19:22:17.428868+00:00
parent: CLOACI-I-0123
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0123
---

# Python migration + tutorial reclassification (embedded tutorials -> /embed, Model A merge)

## Parent Initiative

[[CLOACI-I-0123]] — step 2 (python migration), expanded after a code-grounded finding.

## Objective

Migrate `/python/*` into the new IA AND correct a misfiling the earlier bulk move
introduced: several tutorials under `/service/tutorials` are actually **embedded**
(`DefaultRunner`, in-process — no server). Under the embedded-first principle they
belong in `/embed`. Per ratified decision (2026-06-15, AskUserQuestion):
**"Reclassify + merge (full Model A)"** — move all `DefaultRunner` tutorials into
`/embed/tutorials`, merge each topic's Rust + Python versions into ONE dual-language
`{{< tabs >}}` page (Rust default), renumber the embed track, and leave only the
genuinely server-side tutorials in `/service`.

## Code-grounded classification (the rule)

`DefaultRunner` in-process, no package/daemon/server → **EMBED**. Requires building
a `.cloacina` package + a daemon/server/reconciler → **SERVICE**.

**Embedded (misfiled in `/service/tutorials`, move to `/embed`):**
`05-cron-scheduling` (DefaultRunner×13), `06-multi-tenancy` (DefaultRunner.with_schema),
`09-event-triggers` (DefaultRunner×4), `10-task-deferral` (DefaultRunner), and
`08-workflow-registry` (DefaultRunner×4, embedded registry + cron).

**Genuinely server-side (stay in `/service/tutorials`):**
`01-deploy-a-server` (server×21), `02-the-web-ui`, `07-packaged-workflows` (server×5),
`07-packaging`, `08-websocket-events`, `09-kafka-stream`, `10-cross-package-binding`
(server×11). Plus `python/.../08-packaged-triggers` (packaging → daemon/reconciler).

**Python source versions:** `python/workflows/tutorials/05,06,07` map to cron /
multi-tenancy / event-triggers respectively (all embedded). `00-04` already
SUPERSEDED by `/embed/tutorials/01-04` (already dual-language). CG: embed `07-10`
are currently **Rust-only** (need Python tabs from `python/computation-graphs/09,10,11`).

## Target /embed tutorial track (renumbered, dual-language tabs)

01 first-workflow · 02 context · 03 dependencies · 04 error-handling (DONE, dual) ·
05 cron-scheduling (merge Rust svc/05 + Py py/05) · 06 multi-tenancy (svc/06 + py/06) ·
07 event-triggers (svc/09 + py/07) · 08 task-deferral (svc/10-task-deferral; Rust-only) ·
09 workflow-registry (svc/08-workflow-registry; Rust-mostly) ·
10 computation-graph (embed/07 + Py py-cg/09) · 11 accumulators (embed/08 + Py py-cg/10) ·
12 full-pipeline (embed/09; Rust-only) · 13 routing (embed/10 + Py py-cg/11).

## Other python/* moves (non-tutorial)

- `python/workflows/how-to-guides/`: `testing-workflows` → DROP (Rust dup exists at
  `/embed/how-to/testing-workflows`), repoint refs; `backend-selection`,
  `performance-optimization` → `/embed/how-to`; `packaging-python-workflows` →
  `/embed/how-to` (heavily referenced — keep basename stable).
- `python/workflows/explanation/python-runtime-architecture` → `/embed/explanation/`.
- `python/workflows/reference/environment-variables` → COLLISION with
  `/reference/environment-variables.md` → rename `/reference/python-environment-variables.md`.
- `python/computation-graphs/how-to-guides/{filter-reactor-subscriptions,package-a-python-computation-graph}`
  → `/engine/computation-graphs/how-to`.
- `python/computation-graphs/explanation/python-cg-decorator-surface` → `/engine/explanation`.
- `python/computation-graphs/reference/topology-dict-schema` → `/reference`.
- `python/quick-start.md` → SUPERSEDED by `/embed/quick-start`.
- Remove `python/_index`, `python/workflows/_index`, `python/computation-graphs/_index`
  and all sub `_index.md`; repoint bare `"/python` refs.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All embedded tutorials live in `/embed/tutorials`, renumbered, dual-language.
- [ ] Only server-side tutorials remain in `/service/tutorials` (renumbered clean).
- [ ] All `python/*` body content relocated or superseded; `python/` dir removed.
- [ ] No bare `"/python` or stale `"/service/tutorials/05|06|09|10-task-deferral|08-workflow-registry` refs.
- [ ] `hugo` build clean (no REF_NOT_FOUND), committed in green per-section commits.
- [ ] Backlog note for any tutorial left Python-only or Rust-only (parity) → ties to [[CLOACI-T-0688]].

## Status Updates

- 2026-06-15: Task created mid-execution of I-0123 step 2 after code audit revealed the misfiling. Decision ratified. Beginning the merges.
- 2026-06-15: Phase A done + committed — non-tutorial python/* relocated. B1 done — renumbered embed CG 07→10,08→11,09→12,10→13. B2-simple done — svc/10-task-deferral→embed/08, svc/08-workflow-registry→embed/09. GOTCHA: svc/05-cron + svc/09-event-triggers use stale `workflow!` macro → merges convert to `#[workflow]` module form. Dispatched 6 parallel merge agents (embed/05,06,07 new; Python tabs into embed/10,11,13). 12-full-pipeline Rust-only (parity note). Then remove consumed sources, build green, commit; B3 service renumber + packaged-triggers; remove python/.
- 2026-06-15: COMPLETE. Final /embed track: 01-04 (workflow basics) · 05 cron · 06 multi-tenancy · 07 event-triggers · 08 task-deferral · 09 workflow-registry · 10 computation-graph · 11 accumulators · 12 full-pipeline (Rust-only, parity hint) · 13 routing · 14 packaged-triggers (Python, moved from python/, embed because daemon how-tos live in /embed). Final /service track renumbered clean 01-07: deploy-a-server, the-web-ui, packaged-workflows, packaging, websocket-events, kafka-stream, cross-package-binding. `python/` dir fully removed; all external `/python` refs repointed; tutorial number labels reconciled (link + prose; verbatim program-output banners left intact). Build clean. Committed across 4 commits.
- 2026-06-15: PARITY NOTES for [[CLOACI-T-0688]] follow-up: (a) embed/12-full-pipeline has no Python walkthrough (hint added); (b) embed/14-packaged-triggers is Python-only (no Rust packaged-triggers tutorial); (c) accumulators Python lacks state_accumulator (already noted in /engine). REVIEW FLAGS for step-8 accuracy gate: embed/05-cron "cron-context" Rust tab is a judgment call (Rust has no context-on-schedule API — agent showed a second schedule instead); embed/07-event-triggers register/run tab shows closest-available Rust/Python parallel, not identical operations. Verify both against code at the gate.

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