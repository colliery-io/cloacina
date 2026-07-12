---
id: gold-path-example-workflow-secrets
level: task
title: "Gold-path example: workflow secrets — authoring secrets() plus the cloacinactl secret lifecycle"
short_code: "CLOACI-T-0890"
created_at: 2026-07-11T22:03:17.286920+00:00
updated_at: 2026-07-12T01:34:13.553683+00:00
parent: CLOACI-I-0138
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0138
---

# Gold-path example: workflow secrets — authoring secrets() plus the cloacinactl secret lifecycle

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0138]]

## Objective **[REQUIRED]**

Tenant secrets have full plumbing — authoring (`#[workflow] secrets( n1, n2, … )` workflow_attr.rs:250; `@cloaca.workflow_secrets(*args)` lib.rs:138/workflow.rs:483), server routes (create/list/get/rotate/delete, write-only values — routes/secrets.rs), and a `cloacinactl secret` noun (Create/Rotate/List/Get/Delete) — but ZERO example or tutorial teaches any of it.

**Build:** `examples/features/workflows/secret-consumer/` (or fold into an existing gold-path example if that reads better) — a packaged workflow declaring `secrets(api_token)`, a task that reads the secret at execution, and a README walking the lifecycle: `cloacinactl secret create api_token …` → pack/upload/build → run → Completed → `secret rotate` → run again (new value visible) → show `secret get` returns metadata only (write-only values).

**Shape:** T-0886 standard (package.toml, version deps, gold-path README, demos-features runner, auto-joins CI).

**Acceptance:** execution Completed consuming a real tenant secret on the demo stack; rotation demonstrated; README verified command-by-command; CI runs it. Trace first HOW a task actually receives the secret value at runtime (context injection? env? grants?) and document that accurately — if the delivery mechanism turns out to be missing/broken for packaged workflows, that's a loud finding (I-0137 clause).

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

### 2026-07-12 (update 2) — BLOCKED on T-0895: the fidius task protocol has no secrets channel
Lane failed at the last hop, revealing the deepest gap of the audit: `DynamicLibraryTask::execute` serializes only `context.data()` across the plugin boundary — the secret resolver never crosses, and the plugin side has no secret handling. So `context.secret(...)` in a PACKAGED task fails everywhere (agents included — their D-5 unwrap attaches a resolver the bridge then drops). Filed as [[CLOACI-T-0895]] (needs a design pass: wire-format change → likely interface_version bump → maintainer call).

Everything upstream is fixed and verified: alias map reaches the task input context (`__cloacina_secret_refs__` visible in the server log), both runner paths thread tenant resolvers (`with_database_secrets`, `runner_secret_resolver`). Third fix this task: `DefaultRunner::with_database_secrets` + server wiring (global + tenant cache). The `workflow-secrets` lane is EXCLUDED from the CI matrix (commented in `_BESPOKE_FEATURES`) until T-0895 lands; README carries an honest status banner. This task is blocked_by T-0895; the example + lane are its verification vehicle.

### 2026-07-11 — LOUD FINDING (fixed in-place) + example built; lane running
**Finding:** workflow secrets were END-TO-END UNREACHABLE via the primary interface. Delivery mechanism (grounded): tasks read via `context.secret_field(name, field)` (side channel, T-0858; plaintext never persisted); a run binds a declared secret with `{"$secret": "name"}`; the fleet executor wraps values to one-time agent keys (D-5) and fails closed without `CLOACINA_SECRET_KEK`. BUT the `$secret`→alias-map conversion (`merge_instance_params`, T-0859) only ran on the INSTANCE fire path — and instances can't be registered server-side (T-0894). The execute route merged context naively, so a direct `workflow run` could never bind a secret.

**Fixed (small, surgical):** `routes/executions.rs` now merges the provided context through the SAME `merge_instance_params` the fire path uses ($secret refs routed to the alias map, reserved keys protected, malformed refs → 400); `validate_declared_params` now handles `encrypted` slots — requires a `{"$secret": "name"}` ref and REJECTS literal values with a guiding message. cargo check clean.

**Built:** `examples/features/workflows/workflow-secrets/` — `notify_oncall` declares `params(channel = "#ops") + secrets(api_token)`; `resolve_token` reads `secret_field("api_token","token")` and persists only derived facts (len + bool), `send_notification` consumes. T-0886 shape + README covering the full lifecycle (create from file → bind → run → rotate → rerun → metadata-only reads → literal rejected). Demo compose: server gets a fixed demo `CLOACINA_SECRET_KEK` (documented demo-only). Harness: `_run_gold_path` gained `server_env`; `demos features workflow-secrets` asserts create→Completed→rotate→Completed→no-value-in-get→literal-rejected. Matrix now 14.

Lane run in progress.
