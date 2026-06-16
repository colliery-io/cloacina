---
id: engine-grounding-rust-python
level: task
title: "Engine: grounding + Rust/Python parity pass (accuracy review, caveats, build)"
short_code: "CLOACI-T-0696"
created_at: 2026-06-15T14:19:44.180452+00:00
updated_at: 2026-06-15T16:01:39.399115+00:00
parent: CLOACI-I-0122
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0122
---

# Engine: grounding + Rust/Python parity pass (accuracy review, caveats, build)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0122]]

## Objective

Accuracy-review the whole `/engine` section against `crates/`; confirm every
Rust + Python example traces to source; fix mismatches; confirm parity caveats.
See [[CLOACI-I-0122]].

## Findings (accuracy-reviewer, 2026-06-15) — all fixed

- **BLOCKER:** `workflow.md` Rust used `workflow! { tasks: [...] }` — that bang
  macro does **not exist**; the shipping API is the `#[workflow(name=, description=)]`
  module attribute containing `#[task]` fns (cloacina-macros/src/lib.rs:86, workflow_attr.rs).
  (The tutorial I grounded on is stale — verified against examples/.../01-basic-workflow.)
- **BLOCKER:** `computation-graph.md` Python — `reactor=` takes a
  `@cloaca.reactor` **class** (not a string), and `graph` is a **dict-of-dicts**
  (`{"n": {"inputs": [...], "next": "m"}}`), not dict-of-lists (computation_graph.rs:365,1278).
- **MAJOR:** `trigger.md` Python `poll_interval` is a duration **string** (`"30s"`),
  not an int (trigger.rs:97).
- **MINOR:** `accumulator.md` Python `interval`/`flush_interval` are strings; `_index.md`
  said "reactive dataflow" → corrected to "event-driven" per S-0011 nomenclature.
- **CONFIRMED parity caveats:** no Python `state_accumulator` (lib.rs:128-145); no
  Python packaged cron-trigger decorator (trigger.rs poll-only). Both → [[CLOACI-T-0688]].

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

- [x] Accuracy-reviewer run over all `/engine` pages (both dialects)
- [x] 2 blockers + 1 major + 2 minors fixed; everything else verified code-traceable
- [x] Parity caveats confirmed against code (state accumulator; packaged cron trigger)
- [x] `hugo` builds clean (522 pages) after fixes

## Status Updates

**2026-06-15** — Grounding pass complete; I-0122 (`/engine` primitives) done. Note
for I-3: the stale `workflow!` idiom also appears in the existing Rust tutorials
and `cloacina/src/lib.rs` doc-comment — fix when those move/are reviewed.

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
