---
id: workflow-instance-registration-has
level: task
title: "Workflow-instance registration has no server surface — I-0116 instances are embedded-runner-only"
short_code: "CLOACI-T-0894"
created_at: 2026-07-11T22:28:03.026241+00:00
updated_at: 2026-07-11T22:28:03.026241+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Workflow-instance registration has no server surface — I-0116 instances are embedded-runner-only

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

**Finding from T-0889 (2026-07-11, the I-0138 feature-coverage push):** I-0116 shipped "named, scheduled, param-bound workflow instances" — but instance REGISTRATION exists only on the embedded runner: `DefaultRunner::register_cron_workflow_instance` (runner/default_runner/cron_api.rs) and its python binding (`register_workflow_instance`, bindings/runner.rs:1015). There is **no server route and no cloacinactl noun** to create/list/delete a named instance. On the primary interface (the gold path, per I-0138 D-3), users can bind params PER RUN (`workflow run --context`, typed-validated per T-0757) but cannot create a persistent named/scheduled instance at all — the feature's headline capability is unreachable in the deployment mode we lead with.

The engine side is ready: schedules rows carry `params` JSON + `instance_name` (migration 040), the fire-time merge delivers bound params as top-level context keys, and `WorkflowInstance` (cloacina::workflow_instance) validates against declared InputSlots.

**Build the server surface:**
- Routes (per-tenant): create/list/get/delete workflow instances — `POST/GET/DELETE /v1/tenants/{t}/workflows/{name}/instances[/{instance}]` — body = instance name + params + optional cron schedule/timezone; validate params against the workflow's declared InputSlots (same `validate_declared_params` the execute route uses); persist via the existing schedule DAL (`find_by_instance_name`, UnifiedSchedule params fields).
- `cloacinactl instance` noun (or `workflow instance` subverb): create/list/inspect/delete, `--param k=v`/`--params file.json`, `--cron`.
- UI surface can follow separately.

**Acceptance:** a packaged workflow with `params(...)` gets a named scheduled instance created via cloacinactl against the demo stack; the instance fires on schedule with its bound params (visible in the execution context); T-0889's example README gains the instance section it currently can't have. Related: [[CLOACI-T-0889]], I-0116 (#181).

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
