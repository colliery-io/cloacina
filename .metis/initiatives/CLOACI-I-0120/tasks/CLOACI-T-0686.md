---
id: p4-close-naive-reader-gaps
level: task
title: "P4 — Close naive-reader gaps: WorkflowBuilder patterns, CLOACA_* doc, competitive framing, CORS, production checklist, embedded→server"
short_code: "CLOACI-T-0686"
created_at: 2026-06-15T12:40:39.249741+00:00
updated_at: 2026-06-15T13:09:34.732741+00:00
parent: CLOACI-I-0120
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0120
---

# P4 — Close naive-reader gaps: WorkflowBuilder patterns, CLOACA_* doc, competitive framing, CORS, production checklist, embedded→server

## Parent Initiative

[[CLOACI-I-0120]]

## Objective

Close the six gaps surfaced by the second-round "naive early-user" agent sweep
(docs-only reads). Two are documentation-correctness defects (the code is
correct; the docs are wrong/contradictory); four are missing-content gaps. All
work is documentation-only on branch `docs/i0120-p0-orientation` (PR #126).

## Findings to close (verified against code 2026-06-15)

**Defect #1 — `configuration.md` is heavily fabricated.** Verified real surface:
- `DefaultRunnerConfig` real kwargs (context.rs:34-49; defaults from
  config.rs:293-320): `max_concurrent_tasks=4`, `scheduler_poll_interval_ms=100`,
  `task_timeout_seconds=300`, `workflow_timeout_seconds=3600`, `db_pool_size=10`,
  `enable_recovery=true`, `enable_cron_scheduling=true`, `cron_*` family.
  FABRICATED: `max_concurrent_workflows`, `retry_attempts`,
  `connection_pool_size`, `enable_logging`, `log_level`.
- `DefaultRunner(config=...)` FABRICATED → real `DefaultRunner.with_config(url, config)`
  static method (runner.rs:850); also `.with_schema(url, schema)` (:877).
- `@cloaca.task(retry_policy={...})` and `timeout_seconds=` FABRICATED → real
  retry kwargs (task.rs:690-704): `retry_attempts`, `retry_backoff`,
  `retry_delay_ms`, `retry_max_delay_ms`, `retry_condition`, `retry_jitter`, plus
  `on_success`/`on_failure`/`invokes`/`post_invocation`.
- `cloaca.CronSchedule` FABRICATED → cron is runner methods returning dicts (runner.rs:963+).
- `DatabaseAdmin(connection_timeout/command_timeout/enable_ssl)` FABRICATED →
  `DatabaseAdmin(database_url)` only (admin.rs:121); `create_tenant(config)`,
  `remove_tenant(schema_name, username)`.
- `TenantConfig(schema_name, username, password=None)` REAL (admin.rs:36).
- ALL 8 `CLOACA_*` env vars FABRICATED → only `CLOACINA_VAR_*` and `RUST_LOG`.

**Defect #2 — WorkflowBuilder contradiction.** All three patterns are REAL
(clarify, don't delete): context-manager auto-registers on `__exit__`
(workflow.rs:160-211); manual `add_task`+`build()`+`register_workflow_constructor`
(workflow.rs:66,150,400); bare `@cloaca.task` decorators for packaged workflows.
WorkflowBuilder INSIDE a packaged module breaks (double-pushed context).

**Gap #3** — Competitive framing (Airflow/Temporal/Prefect).
**Gap #4** — CORS warning on Deploy-a-Server + web UI deploy how-tos.
**Gap #5** — Production checklist / deployment decision-tree.
**Gap #6** — Embedded→server transition runbook + fix overstated "no rewrite" claim.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `configuration.md` rewritten to the verified real surface
- [ ] `workflow-builder.md` gains a "which pattern, when" clarifying note
- [ ] Competitive-framing content added (honest positioning)
- [ ] CORS warning added to Deploy-a-Server tutorial
- [ ] Production checklist / deployment decision content added
- [ ] Embedded→server transition guidance added; "no rewrite" claim corrected
- [ ] 4-reviewer adversarial gate passes
- [ ] Committed to PR #126

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
