---
id: ws-7-naive-user-polish-status
level: task
title: "WS-7 — Naive-user polish (status vocab, placeholder leak, Tasks:0)"
short_code: "CLOACI-T-0709"
created_at: 2026-06-16T01:50:20.066720+00:00
updated_at: 2026-06-16T04:19:09.684359+00:00
parent: CLOACI-I-0124
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0124
---

# WS-7 — Naive-user polish (status vocab, placeholder leak, Tasks:0)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0124]]

## Objective

(P2) Fix the naive-user confusion bugs the audit found — small, high-clarity wins.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria (real)

- [ ] **Status vocabulary explained inline** (tooltips/legend) for WARMING, `socket_only`, `connecting`, WHEN_ANY/WHEN_ALL, LATEST/SEQUENTIAL; no raw quoted enum strings.
- [ ] **Settings placeholder** no longer prints internal task codes ("Built in T-0651." → real content or a neutral "coming soon").
- [ ] Computation-graph packages show **type** instead of "Tasks: 0" (which reads as broken).
- [ ] Re-passes the Playwright walk.

## Dependencies

Coordinate the "type vs Tasks: 0" column with [[CLOACI-T-0705]].

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

### 2026-06-16 — DONE (all three confusion bugs fixed + verified)

1. **Status vocabulary** — new `ui/src/util/vocab.ts` (`explainToken`) maps the
   internal enum tokens to a friendly label + one-line tooltip. Key bug:
   `GraphHealth` only understood `{state: …}` objects, so a bare-string
   accumulator `status` (`"socket_only"`) fell through to `JSON.stringify` and
   rendered as a **raw quoted string**. Now it handles bare strings and badges
   `live` / `warming` / `socket_only` / `connecting` / `running` / `stopped`
   with explanations. `GraphDetail`'s `reaction_mode` (`when_any` → "when any")
   and `input_strategy` (`latest`) badges + the node drawer's reactor rows use
   the same map. No raw quoted enum strings remain in the CG views.
2. **Settings placeholder** — `Placeholder` no longer prints an internal task
   code (`"Built in T-0651."` → `"This area isn't available yet — coming
   soon."`); dropped the unused `task` prop and its call site.
3. **CG "Tasks: 0"** — a package with zero workflow tasks is a computation-
   graph package; Workflows + Overview now show a **`graph`** badge (with a
   tooltip pointing at the Graphs view) instead of a `0` that reads as broken.

Verified live (`ui/e2e/ws7.spec.ts`, screenshots in `/tmp/cloacina-ui-uat/ws7/`):
`demo-kafka-stream-rust` shows the GRAPH badge; Graphs shows **LIVE** + **SOCKET
ONLY**; graph detail shows **WHEN ANY** / **LATEST**; Settings shows "coming
soon". `tsc --noEmit` clean. UI-only change, committed `ce486f08`.