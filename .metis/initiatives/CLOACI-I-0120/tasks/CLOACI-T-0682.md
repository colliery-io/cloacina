---
id: p0-orientation-onboarding-what-why
level: task
title: "P0 — Orientation & onboarding: what/why/when-not/features/getting-started/concepts"
short_code: "CLOACI-T-0682"
created_at: 2026-06-15T03:17:05.672111+00:00
updated_at: 2026-06-15T03:17:52.251464+00:00
parent: CLOACI-I-0120
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0120
---

# P0 — Orientation & onboarding: what/why/when-not/features/getting-started/concepts

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0120]]

## Objective **[REQUIRED]**

P0 slice of CLOACI-I-0120. Build the orientation/onboarding layer so a newcomer
can answer, up front: what Cloacina is, why use it, **when not to**, what its
features are, and how to start. Improve within the existing feature-area-first
IA (preserve URLs). Branch: `docs/i0120-p0-orientation`.

### P0 doc set
1. **`docs/content/_index.md`** (landing) — DONE. Added What-it's-for / Why /
   When-*not*-to-use / Ways-to-run matrix / Get-started paths. Grounded in the
   Phase-1 inventory + ADR-0005 trust model.
2. **`docs/content/quick-start/_index.md`** — tighten the role-based router; make
   the demo-stack "see it all running" path prominent (currently buried).
3. **New explanation doc: "When to use Cloacina (and when not)"** — the full
   version of the landing summary (workload fits, non-goals, SQLite-vs-Postgres,
   Linux-only server, at-least-once, async-only). Cite ADR-0005 / specs / code.
4. **New "Features overview"** — a single catalog of capabilities (engine,
   embedded vs server, workflows, computation graphs, packaging, multi-tenancy,
   fleet, SDKs, UI, observability) with links into the deep docs. Must reflect
   SHIPPED reality (fleet/UI/SDKs/substrate = done).
5. **Concepts/primitives orientation** — short page introducing the five S-0011
   primitives and how they relate (link glossary for full definitions).
6. **README alignment** — keep the repo README's pitch consistent with the new
   landing (don't let them drift).

Then run the Phase-4 review gate (accuracy/completeness/clarity/diataxis-
compliance) on the P0 set, address blockers+majors, commit, open PR.

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

## Acceptance Criteria **[REQUIRED]**

- [ ] Landing `_index.md` answers what/why/**when-not**/ways-to-run/get-started. (DONE)
- [ ] `quick-start/_index.md` surfaces the demo-stack first-success path prominently.
- [ ] A dedicated "When to use Cloacina (and when not)" explanation doc exists, linked from the landing.
- [ ] A "Features overview" page catalogs capabilities and reflects SHIPPED reality (fleet/UI/SDKs/substrate done), linked into deep docs.
- [ ] A concepts/primitives orientation page introduces the five S-0011 primitives.
- [ ] README pitch is consistent with the new landing.
- [ ] The four reviewers return zero blockers/majors (clarity zero blockers) on the P0 set; PR opened.

## Status Updates

**2026-06-15 — P0 started.** Initiative I-0120 active; this slice on branch
`docs/i0120-p0-orientation`. Landing `_index.md` overhauled (what-for / why /
when-*not* / ways-to-run matrix / get-started), grounded in the Phase-1 inventory
+ ADR-0005. Remaining: quick-start router, the dedicated when-not explanation
doc, features overview, concepts page, README alignment, then the 4-agent review
loop + PR.

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