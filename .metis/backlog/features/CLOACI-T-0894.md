---
id: workflow-instance-registration-has
level: task
title: "Workflow-instance registration has no server surface — I-0116 instances are embedded-runner-only"
short_code: "CLOACI-T-0894"
created_at: 2026-07-11T22:28:03.026241+00:00
updated_at: 2026-08-08T13:23:07.940563+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


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

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] Per-tenant routes to create/list/get/delete named workflow instances
- [x] Instance params validated against the workflow's declared InputSlots with the same `validate_declared_params` the execute route uses
- [x] Instances persisted via the existing schedule DAL (`find_by_instance_name`, params/instance_name fields) — no new engine machinery
- [x] A `cloacinactl instance` noun with create/list/inspect/delete, `--param k=v` / `--params file.json`, `--cron`
- [ ] NOT DONE — end-to-end proof against the demo stack: a packaged workflow with `params(...)` gets a named scheduled instance via cloacinactl and fires on schedule with its bound params visible in the execution context
- [ ] NOT DONE — T-0889's example README gains the instance section it currently can't have
- [ ] NOT DONE — UI surface (scoped by the ticket itself as a separate follow-up)

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

- 2026-08-08: SURFACE COMPLETE — PR #247 merged (squash). Closing with the
  live-stack proof and docs EXPLICITLY NOT DONE; carried to a follow-up rather
  than left implied. See the unchecked criteria above.

  WHAT SHIPPED. Four tenant-scoped routes (POST/GET list, GET one, DELETE)
  under `/v1/tenants/{t}/workflows/{name}/instances`, a `cloacinactl instance`
  noun, and the operations added to all three SDKs. Nothing in the execution
  path changed: the engine was already complete (schedules rows carry params +
  instance_name from migration 040, the fire-time merge delivers bound params
  as top-level context keys, the DAL already had create /
  find_by_instance_name / find_by_workflow / delete). This was the missing
  surface, not new machinery.

  Tenant scoping follows the house rule — everything through the tenant-scoped
  Database from TenantDatabaseCache, so a cross-tenant request 404s naturally
  instead of leaking existence through a distinct error code. Routes are
  declared BEFORE the `{version}` workflow-delete so the static `instances`
  segment is never shadowed by the version wildcard.

  DESIGN POINT worth keeping: `cron` is optional per this ticket's own body,
  which raises the question of what an unscheduled instance does. It is stored
  with `next_run_at = NULL`, and the scheduler's due query filters
  `next_run_at <= now` — NULL never satisfies that, so it can never fire. The
  entire "optional cron" affordance rests on that one SQL property, so it is
  asserted by test rather than reasoned about, PLUS a scheduled counterpart
  test so the first cannot pass merely because nothing is ever due.

  Duplicate names are caught twice: a find_by_instance_name pre-check for the
  common case and the unique index for the genuine race, both mapped to 409
  (new `ApiError::conflict` — there was no constructor for it).

  PRE-EXISTING DRIFT SURFACED: regenerating the python SDK with ITS OWN
  documented pin (openapi-python-client 0.29.0, per clients/python/README.md)
  rewrote ~100 already-committed model files, i.e. the committed output came
  from a different generator build than the README claims. The emitted code
  imports typing_extensions, which was NOT a declared dependency and IS
  required on the supported 3.10 floor — now declared in
  clients/python/pyproject.toml. Included rather than hand-reverted, because
  selectively keeping stale generated files is how the drift arrived. CI passed
  with it, so nothing downstream depended on the old shape.

  Tests: 3 schedule-DAL (unscheduled never due; scheduled round-trips params
  and IS due; named vs anonymous) — 30 schedule tests pass; 3 CLI tests for
  --param typing, values containing '=', malformed pairs. cargo check
  --workspace, cargo fmt --check, docs spec-check, SDK coverage (59 ops across
  3 SDKs) and version lockstep all clean.

  ALSO NOT INCLUDED, deliberately: PATCH/update of an existing instance (delete
  and recreate for now); pause/resume, already reachable through the existing
  trigger pause/resume endpoints since instances ARE schedule rows.
