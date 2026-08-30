---
id: workflow-instance-surface-live
level: task
title: "Workflow-instance surface — live-stack proof, example docs, and UI (T-0894 follow-through)"
short_code: "CLOACI-T-0927"
created_at: 2026-08-08T13:22:25.328874+00:00
updated_at: 2026-08-09T01:40:33.994646+00:00
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

# Workflow-instance surface — live-stack proof, example docs, and UI (T-0894 follow-through)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Finish the parts of [[CLOACI-T-0894]] that its API surface work did not cover.
T-0894 built and merged the server routes, the `cloacinactl instance` noun, and
the SDK operations (PR #247), but was closed with three of its stated
acceptance criteria explicitly UNMET — recorded there rather than quietly
ticked. This ticket carries them.

**1. Live-stack end-to-end proof.** T-0894's own acceptance says: a packaged
workflow with `params(...)` gets a named scheduled instance created via
cloacinactl against the demo stack, and the instance FIRES on schedule with its
bound params visible in the execution context. Only unit-level proof exists
today (3 schedule-DAL tests, 3 CLI tests). Nothing has yet exercised
create → schedule fires → params land in context against a running server. That
is the difference between "the endpoint returns 200" and "the feature works",
and per [[feedback_sdk_live_server_drift]] the project has been bitten by
spec-vs-spec verification before.

**2. Example docs.** T-0889's example README still lacks the instance section
it could not have while the surface didn't exist.

**3. UI surface.** Scoped by T-0894 as a deliberate follow-up.

Also worth folding in, both recorded as deliberate omissions on T-0894:
PATCH/update of an existing instance (today: delete and recreate), and whether
instance pause/resume deserves its own route rather than relying on the
existing trigger pause/resume endpoints (which work because instances ARE
schedule rows, but are not discoverable under the instance noun).

## Reference — what already exists

- Routes: `POST/GET /v1/tenants/{t}/workflows/{name}/instances`,
  `GET/DELETE .../instances/{instance}` (`crates/cloacina-server/src/routes/instances.rs`)
- CLI: `cloacinactl instance create|list|inspect|delete`, `--param k=v`,
  `--params file.json`, `--cron`, `--timezone`, `--disabled`
  (`crates/cloacinactl/src/nouns/instance/mod.rs`)
- SDKs: `create_instance`/`list_instances`/`get_instance`/`delete_instance` in
  the rust, typescript and python clients
- An instance created WITHOUT `--cron` is unscheduled by design
  (`next_run_at NULL`, never selected by the due query) — the e2e proof should
  cover the scheduled path specifically

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [x] Feature - New functionality or enhancement
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [x] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

P1 because the surface is now merged and reachable by users, but its
end-to-end behavior has never been observed against a running server. An
untested-in-anger endpoint that users can already call is worse than one that
doesn't exist yet.

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

- [x] Against the demo stack: pack + upload a workflow declaring `params(...)`, create a named instance via `cloacinactl instance create --cron`, and observe it FIRE on schedule with its bound params present as top-level context keys on the resulting execution
- [x] That path is automated in the angreal harness so it cannot silently rot (added to the e2e compiler lane, which already owns the server+compiler+postgres lifecycle)
- [x] Params that violate the workflow's declared slots are rejected at create time against a live server, not just in unit tests
- [x] T-0889's example README documents creating, inspecting and deleting an instance
- [x] UI surface — read-only "Named instances" panel on WorkflowDetail; create/delete deliberately EXCLUDED with the reason recorded (needs a form driven by the declared param slots, which is its own design; the CLI owns create today)

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

- 2026-08-08 (IN PROGRESS — worktree `.claude/worktrees/t-0927`, branch
  `feat/t-0927-instance-e2e`, cut from main @ fb3ec2da).

  VERIFIED ASSUMPTIONS before writing anything — each could have sunk the lane:
  * The CRON fire path merges instance params (`cron_trigger_scheduler.rs:667`
    and `:970` call `workflow_instance::merge_instance_params`). The execute
    route doing it was NOT sufficient evidence; that is different code.
  * 6-field (seconds) cron IS supported —
    `cloacina-workflow/src/cron_evaluator.rs:132` `with_seconds_optional()`. So
    `*/2 * * * * *` fires every 2s and the lane sees a real fire fast.
  * The server never overrides `enable_cron_scheduling` (default `true`,
    `default_runner/config.rs:317`), so its runner does schedule.
  * Required params (no default) are valid syntax — `parameterized-workflow`
    and `packaged-workflows` both declare them.

  FINDING: there is NO existing e2e coverage of server-side cron FIRING
  anywhere in the harness (grep for demo-cron-rust / demo_cron_workflow under
  `.angreal/` returns nothing). This lane is the first thing to prove the
  server fires a schedule at all, not just that instances work.

  OBSERVATION CHANNEL: `ExecutionDetail` carries only `{tenant_id,
  execution_id, status}` — there is no HTTP surface for a run's context. The
  fired context is therefore read from the DB (`contexts.value` joined through
  `workflow_executions.context_id`), matching the schedule on `instance_name`
  so a concurrent anonymous schedule cannot be mistaken for the instance's own
  fire. The harness already uses `_psql` this way for the stale-heartbeat check.

  NEW FIXTURE `examples/fixtures/instance-params-rust`: declares `region:
  String` (REQUIRED, no default) plus `batch_size: u32 = 100`. The required one
  is what makes create-time validation observable. Its task echoes both params
  to `observed_*` keys — bound params already arrive as top-level context keys,
  so echoing proves the TASK saw them rather than the harness reading back what
  it wrote. Compile-checked standalone before running the lane.

  LANE ADDED to `angreal test e2e compiler` (already owns the
  server+compiler+postgres lifecycle): build+load fixture → assert a create
  missing `region` is REJECTED and never persisted → create with
  `--cron "*/2 * * * * *"` → assert it appears in `instance list` → wait for a
  real fire → assert `region`/`batch_size` AND `observed_*` present, plus
  `schedule_id` (reserved keys still win) → assert execution Completed →
  delete and assert the row is gone.

  DOCS DONE: the example README's "About named instances" paragraph claimed a
  server-side surface "is tracked separately" — now FALSE, rewritten with real
  CLI recipes. `docs/content/engine/scheduling/workflow-instances.md` had only
  embedded Rust/Python; gained a Server section with the CLI, the REST table,
  SDK method names, tenant scoping, and the note that pause/resume rides the
  existing trigger verbs.

  THE LANE FOUND A REAL BUG — this is the finding that justifies the ticket.

  **T-0894's four instance routes were merged UNREACHABLE.** They were never
  added to the server's authz route table (`routes/authz.rs`), and that
  middleware is FAIL-CLOSED: every call — create, list, get, delete — returned
  `Error: authentication — route not authorized` regardless of the key's role.
  All 80 CI checks passed on PR #247 because nothing exercised the routes
  against a running server. Unit tests, OpenAPI drift, SDK coverage and version
  lockstep all pass happily on routes no client can actually call.

  FIXED, both the bug and the hole that allowed it:
  1. Added the four routes to the table — GETs as `Access::tenant(Level::Read)`,
     POST/DELETE as `Access::tenant(Level::Write)`, matching the neighbouring
     workflow routes. Table size pin updated 64 → 68.
  2. NEW TEST `every_documented_v1_route_is_classified`. The pre-existing
     `authz_table_classifies_known_routes` only pins the table's SIZE, so it
     cannot see the router — a route added in `lib.rs` with no table entry
     leaves the count untouched and sails past. The new test walks the OpenAPI
     document (generated from the handlers' own `#[utoipa::path]`, and the only
     faithful stand-in available since axum exposes no route enumeration) and
     asserts every `/v1` path+method is classified. It immediately also flagged
     `POST /auth/local/login`, verified as a LEGITIMATE exemption — `lib.rs`
     deliberately merges the public auth entry points outside
     `require_auth`/`authz_mw` because they mint the key. The exemption list is
     an explicit `matches!` of five paths, kept narrow because a careless entry
     there is a genuine auth hole.

  TEST-DESIGN NOTE worth keeping: the lane asserts BOTH that the bad create
  fails AND that the rejection names the missing param. Asserting only failure
  would have passed here for entirely the wrong reason — the authz denial — and
  the bug would still be in main. When testing a rejection, assert the REASON.

  Two harness bugs of my own on the way, both caught by the lane:
  * `_poll_run_workflow` was used as a load signal, but it runs the workflow
    with an EMPTY context; a fixture with a required param is rejected forever
    by the execute route's validation, so it spun to timeout on a validation
    error while the workflow was loaded all along. Now takes an optional
    `context`. (Its rejection incidentally confirmed the execute route
    validates against a live server.)
  * `instance list -o json` renders a BARE JSON ARRAY, not an envelope; the
    parse assumed a dict. Now tolerates both.

- 2026-08-09: COMPLETED — PR #248 merged (squash). Full lane green, exit 0:

    ok: missing required param rejected at create time
    ok: instance created
    ok: instance listed (and the rejected one was never stored)
    instance fired: execution b98c2cf6-7f8d-44c7-b38e-e72c2260edd9
    ok: instance fire delivered bound params to the task (region=eu-west, batch_size=7)
    ok: instance execution Completed
    ok: instance deleted

  The existing upgrade / rollback / concurrent-upload lanes still pass, so the
  addition did not disturb the harness.

  CI NOTE, resolved: the sqlite-only Feature Build hung for 6h0m15s and was
  killed (orphaned angreal + cargo + integration-* processes; 786 lib tests
  passed, then the integration suite completed 2 of 7 and wedged with no
  failure output). Main and both prior branches had passed, so this was NOT
  dismissed as flake on assumption. Investigation: the same target runs 329
  tests in 35 SECONDS locally with no hang, and this branch's ONLY change to
  the `cloacina` crate is the 12-line additive `notify_cron_change()` method,
  which nothing in the integration suite calls — no mechanism. A re-run of the
  IDENTICAL commit went green, which is what settled it. Filed nothing new: the
  signature (CI-only wedge on a GitHub ubuntu runner, never reproducible
  locally) matches the class T-0910 documented. If it recurs, that is the
  thread to pull.

  CORRECTION worth recording: the three schedule-DAL tests were attributed to
  this change in earlier notes; they actually shipped with T-0894 in fb3ec2da.
  T-0927 touched the `cloacina` crate in exactly one place. This mattered — it
  is what ruled out a mechanism for the hang.
