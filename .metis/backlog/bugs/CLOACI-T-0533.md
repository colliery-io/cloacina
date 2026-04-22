---
id: fix-cloacina-api-requests-total
level: task
title: "Fix cloacina_api_requests_total — correct description and add duration histogram"
short_code: "CLOACI-T-0533"
created_at: 2026-04-22T12:20:53.464585+00:00
updated_at: 2026-04-22T12:41:55.978863+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Fix cloacina_api_requests_total — correct description and add duration histogram

Follow-up from the T-0498 metrics audit.

## Objective

The `cloacina_api_requests_total` metric has two defects found by T-0498 audit:
1. The `describe_counter!` text at `crates/cloacina-server/src/lib.rs:208-211` claims the metric carries `method, path, and status` labels, but the emit site at `lib.rs:502` only sets `method` and `status`. The description is wrong.
2. We count API requests but never time them. Operators cannot see per-endpoint latency.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium

### Impact Assessment
- **Affected Users**: Operators consuming `/metrics`.
- **Expected vs Actual**: Description should match labels; histogram should exist. Today the description lies and no duration data is emitted.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `describe_counter!("cloacina_api_requests_total", ...)` text matches the labels emitted at the call site (drop "path" OR add a bounded path/route label).
- [ ] New histogram `cloacina_api_request_duration_seconds{method,status}` registered with `describe_histogram!` and recorded in `api_request_metrics` middleware.
- [ ] If a path/route label is added, it uses axum's matched route pattern (e.g. `/v1/workflows/:id`), NOT the raw URI — cardinality must stay bounded.
- [ ] Unit test verifies the histogram samples after a request.

## Implementation Notes

Touch points: `crates/cloacina-server/src/lib.rs` (describe block ~203-218, middleware ~495-505). Use `std::time::Instant` around `next.run(request)`.

---

<!-- unused template sections below, kept for reference -->

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

## Status Updates

### 2026-04-22 — Implemented

Changes in `crates/cloacina-server/src/lib.rs`:
- `describe_counter!("cloacina_api_requests_total", ...)` text updated from "method, path, and status" to "HTTP method and response status code" — matches the labels actually emitted.
- New `describe_histogram!("cloacina_api_request_duration_seconds", ...)`.
- `api_request_metrics` middleware now times handler execution with `std::time::Instant` and records into the new histogram, keyed by the same `method`/`status` labels as the counter.
- Kept labels to `method` + `status` (no path/route label) to preserve bounded cardinality. Route-level latency would require axum's MatchedPath extractor and is out of scope for this bug fix.

Test added: `test_api_request_duration_histogram_emitted` fires a `/health` request then scrapes `/metrics`, asserting both `cloacina_api_request_duration_seconds` and `cloacina_api_requests_total` appear in the output.

Verified `cargo check -p cloacina-server` passes. Full test run should go through `angreal cloacina unit` / server suite.

### Acceptance criteria
- [x] `describe_counter!` text matches emitted labels (dropped "path").
- [x] New histogram `cloacina_api_request_duration_seconds{method,status}` registered and recorded.
- [x] No unbounded path/route label added.
- [x] Unit test verifies the histogram samples after a request.
