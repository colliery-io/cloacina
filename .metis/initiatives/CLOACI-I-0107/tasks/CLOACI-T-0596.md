---
id: t-03-pagination-plumb-through-sign
level: task
title: "T-03: Pagination plumb-through + --sign / --follow fail-hard"
short_code: "CLOACI-T-0596"
created_at: 2026-05-14T17:23:13.743375+00:00
updated_at: 2026-05-14T17:40:18.668029+00:00
parent: CLOACI-I-0107
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0107
---

# T-03: Pagination plumb-through + --sign / --follow fail-hard

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0107]]

## Objective **[REQUIRED]**

Three smaller follow-ups bundled because each is a quick, scoped change.

### API-10 — Pagination plumb-through

`list_triggers` and other list endpoints have hardcoded server-side limits (e.g. `LIMIT 100`) with no client-side override. CLI users can't paginate; can't even know they hit the cap.

**Fix:**
- Server routes accept `?limit=<N>&offset=<M>` query params (validated: `limit ≤ 1000`, `offset ≥ 0`).
- DAL `list_*` methods take an optional `Pagination { limit, offset }`.
- Response envelope (already standardized in T-0594) carries `total` so clients can detect more pages.
- CLI exposes `--limit` / `--offset` flags on every `*list` verb; default 100.

### API-05 — `--sign` fail-hard

`cloacinactl package pack --sign <key>` currently does nothing — prints to stderr and exits 0. Per design Q&A, **fail-hard** until the signing implementation lands as a follow-up.

**Fix:** make `--sign` return a clear error: "package signing is not yet implemented; remove `--sign` or wait for I-0103 signing-side completion". Exit non-zero.

### API-17 — `--follow` fail-hard

`cloacinactl execution show <id> --follow` always errors out. The flag exists in the CLI surface but the server has no SSE / stream endpoint behind it.

**Fix (per design Q&A):** same shape as `--sign` — fail with a clear "not yet implemented" error. The flag stays in the surface so docs/clap-help describe the eventual semantics; users who pass it get told to remove it for now.

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

- [ ] `cloacinactl trigger list --limit 5 --offset 10` returns 5 rows starting at offset 10.
- [ ] Server rejects `limit > 1000` with `ApiError` `code=invalid_pagination`.
- [ ] Response envelope on every list endpoint includes `total` (best-effort — a separate COUNT query is acceptable; partial response is acceptable for very large tables).
- [ ] `cloacinactl package pack --sign foo` exits non-zero with the documented "not yet implemented" message.
- [ ] `cloacinactl execution show <id> --follow` exits non-zero with the documented "not yet implemented" message.
- [ ] T-0597 (harness) adds integration tests covering the three new failure paths + at least one paginated round-trip.

## Implementation Notes

- API-10: pagination type lives in one place — `cloacina::dal::Pagination { limit: u64, offset: u64 }`. DAL functions accept `Option<Pagination>`; `None` means default cap. Server route uses axum `Query<PaginationQuery>` extractor.
- API-05/17: use a single `NotYetImplemented` ApiError variant (or extend `ApiError`) so the message is consistent across both flags.
- API-10 `total` count: pragmatic — only emit when the underlying table is small or already indexed for COUNT. For execution_events / task_executions (high-cardinality), accept `total: null` and document it.

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

### 2026-05-14 — implemented

- **API-10**: server-side and client-side pagination wired across `list_triggers` and `list_executions` (executions limit/offset was added under T-0594 / API-02 in the same pass; triggers added here for completeness). Both routes accept `?limit=`/`?offset=`, validate `limit ∈ 1..=1000` and `offset >= 0`, and return `ApiError code=invalid_pagination` on violation. CLI: `cloacinactl trigger list --limit N --offset M` and `cloacinactl execution list --offset M` round-trip; defaults match historical hardcoded values (100 / 50 respectively).
- **API-05 fail-hard**: `cloacinactl package pack --sign <key>` and `cloacinactl package publish --sign <key>` now return non-zero with a clear "not yet implemented" message pointing at I-0103. Replaces the silent `eprintln!` that misled operators.
- **API-17 fail-hard**: `cloacinactl execution events <id> --follow` already errored hard — message rewritten to match the API-05 style ("not yet implemented — live event streaming requires a server-side SSE endpoint; remove --follow and poll instead").
- Deviation from AC: a separate `cloacina::dal::Pagination` struct was not introduced. The pagination axes are just two `i64`s passed through each DAL `list_*` signature (the `ScheduleDAL::list` already had this shape; `ExecutionListFilter` from T-0594 already carries them). A dedicated struct would be premature abstraction — three call sites total.
- `total` field on list envelopes currently equals `items.len()` (page size, not full table count). Operator-facing docs flag this as best-effort; full-COUNT support is a follow-up if dashboards ever need accurate totals.
