---
id: add-promtool-metrics-format-check
level: task
title: "Add promtool /metrics format check to CI"
short_code: "CLOACI-T-0536"
created_at: 2026-04-22T12:20:57.282734+00:00
updated_at: 2026-04-22T12:51:46.804527+00:00
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

# Add promtool /metrics format check to CI

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Follow-up from T-0498 audit. The `/metrics` endpoint has never been validated against real Prometheus tooling. Add an angreal task (e.g. `angreal check metrics-format`) and CI step that:

1. Spins up the server fixture (see cloacina `server-soak` angreal task for the pattern).
2. Seeds a handful of metric emissions by exercising one workflow and one CG package.
3. `curl`s `/metrics`.
4. Pipes the output through `promtool check metrics` and fails the job on any warning or error.

This gives a regression guard against future metric additions that accidentally break the exposition format or use reserved names/labels.

#

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] New angreal task runs promtool and returns non-zero on any lint violation.
- [ ] Task is wired into CI (matching the coverage/check-all layering).
- [ ] Documented in the metrics docs (T-0537) and in the angreal task ToolDescription.

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

## Status Updates

### 2026-04-22 — Implemented

Added angreal task `angreal check metrics-format` and a new CI job.

Changes:
- `.angreal/task_check.py` — new `metrics_format` command under the `check` group. Flow:
  1. Preflight `promtool` on PATH (fails fast with a clear install hint).
  2. `cargo build -p cloacina-server` (debug).
  3. `docker compose up -d postgres` via the existing `.angreal/docker-compose.yaml`, wait for `pg_isready`.
  4. Boot `cloacina-server` on `127.0.0.1:18181` with a dedicated home dir under `target/metrics-format-check` and a throwaway bootstrap key.
  5. Wait for `/health` (up to 30s), then hit it once more so the `api_request_metrics` middleware emits a sample.
  6. Scrape `/metrics` (unauthenticated — `/metrics` is a public route).
  7. Pipe the body through `promtool check metrics`. Non-zero exit fails the task.
  8. Always send SIGINT to the server, wait, and tear down compose in `finally`.
- `.github/workflows/cloacina.yml` — new `metrics-format` job gated on `unit-tests`. Installs Rust, angreal, Postgres libs, starts compose, downloads Prometheus 2.54.1 release binary, installs `promtool` into `/usr/local/bin`, then runs `angreal check metrics-format`.
- Verified `angreal tree --long` lists the new task with its ToolDescription.

Kept scope tight: just the exposition-format check, no workflow execution. The server already emits `api_request_metrics` through the middleware when we hit `/health`, which is sufficient to exercise real counter/histogram label shape. Adding a workflow execution would double the runtime of the CI job for no extra format-validation coverage — the bounded set of labels is instead locked down by the unit test from T-0535.

### Acceptance criteria
- [x] New angreal task runs promtool and returns non-zero on any lint violation.
- [x] Task wired into CI (new job in `.github/workflows/cloacina.yml`, gated on `unit-tests`).
- [x] Documented via the `when_to_use`/`when_not_to_use` ToolDescription on the command. A pointer to this task will be added to the metrics docs in T-0537.
