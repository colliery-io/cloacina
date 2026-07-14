---
id: bug-packaged-tasks-cannot-receive
level: task
title: "BUG: packaged tasks cannot receive secrets — the fidius task protocol has no secrets channel"
short_code: "CLOACI-T-0895"
created_at: 2026-07-12T01:10:07.780717+00:00
updated_at: 2026-07-12T01:34:05.671813+00:00
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

# BUG: packaged tasks cannot receive secrets — the fidius task protocol has no secrets channel

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

**Finding from T-0890 (2026-07-12, I-0138 feature-coverage push) — the deepest yet:** a PACKAGED task calling `context.secret(...)` always fails with "secrets backend not configured for this execution scope", in EVERY deployment mode, because the fidius task-execution protocol has no secrets channel:

`DynamicLibraryTask::execute` (registry/loader/task_registrar/dynamic_task.rs:156) serializes **only `context.data()`** into `TaskExecutionRequest { task_name, context_json }` and calls the plugin. The host context's secret resolver — a runtime object — never crosses the boundary, and `cloacina-workflow-plugin` has zero secret handling on the receiving side (the plugin-side Context is rebuilt with no resolver).

**Consequences:**
- The agent path's whole D-5 machinery (HPKE wrap to one-time pooled keys → agent unwraps → `context.set_secret_resolver(...)` in main.rs:1616) attaches the resolver to a host context that dynamic_task then drops — agents execute packaged tasks via `runtime.get_task(...).execute(context)` → same bridge. **D-5 delivery has never reached a packaged task.**
- The in-process path (T-0890 wired `with_database_secrets` → ThreadTaskExecutor → context_builder → `context.set_secret_resolver`) is equally dropped at the same bridge.
- Only HOST-COMPILED (embedded) tasks can resolve secrets — e.g. the `secret_no_leak` integration test's local `#[task]`. Since all examples/e2e lead packaged (I-0138), secrets are effectively undeliverable in the product's primary shape.

**Everything upstream now works** (fixed under T-0890): `workflow run --context` accepts `{"$secret": "name"}` (execute route merges via `merge_instance_params`; literal values rejected for encrypted slots), and both server runner paths thread tenant-scoped resolvers. The alias map demonstrably reaches the task input context: `{"channel":"#oncall","__cloacina_secret_refs__":{"api_token":"oncall_api"}}`.

**Fix (needs a short design pass — it touches the plugin wire format):** extend the task-execution interface so resolved secret VALUES (or a host callback) cross the boundary. Sketch: dynamic_task reads `SECRET_REFS_KEY` from the context, resolves each name via the host-side resolver BEFORE the FFI call, and passes a `resolved_secrets` map alongside `context_json`; the plugin-side `package!` execute path attaches an in-memory resolver built from that map to the rebuilt Context. In-process the plaintext never leaves the process; agent-side it's the already-unwrapped values. NOTE: adding a field to the bincode-serialized request struct is a WIRE CHANGE → likely `interface_version` bump → all packaged fixtures/examples rebuild. Decide versioning strategy with the maintainer before implementing.

**Verification vehicle exists:** `examples/features/workflows/workflow-secrets` + `angreal demos features workflow-secrets` (T-0890) — currently blocked on this bug; it asserts create → `$secret`-bound run → **Completed** → rotate → rerun → metadata-only reads → literal rejected. Re-enable it in the CI matrix (`_BESPOKE_FEATURES`) when this lands. Related: [[CLOACI-T-0890]], [[CLOACI-T-0894]], I-0133 D-5.

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

### 2026-07-12 — FIXED + VERIFIED (maintainer approved the interface bump)
Implemented the sketch exactly:
- `cloacina-workflow::secret::MapSecretResolver` — in-memory resolver over `{secret_name: {field: value}}`; Debug renders names only.
- `TaskExecutionRequest.resolved_secrets: BTreeMap<String, BTreeMap<String,String>>` (workflow-plugin types.rs); manual Debug keeps values out of logs; round-trip + no-leak-in-Debug unit test.
- Host bridge (`dynamic_task.rs::execute`): reads the `SECRET_REFS_KEY` alias map, resolves each CONCRETE secret name via the context's resolver (executor- or agent-attached) BEFORE the plugin call — fail-closed with a clear per-secret error — and ships the values in the request.
- Plugin shell (`package!` execute_task): attaches `MapSecretResolver` to the rebuilt scope when `resolved_secrets` is non-empty, so `context.secret(...)` works identically inside packages. One bridge serves in-process AND agent execution.
- **Interface version 4 → 5** (`#[fidius::plugin_interface]`): the request struct is a bincode wire change; stale artifacts fail the version gate at load instead of mis-decoding.

**Verified:** `angreal demos features workflow-secrets` FULL PASS — secret create → `$secret`-bound run → execution **Completed** (a packaged task resolved a secret across the boundary for the first time) → rotate → rerun Completed → `secret get` metadata-only → literal value rejected pre-execution. The lane also proves the v5 gate end-to-end (package rebuilt + loaded under the new interface). Lane re-enabled in the CI matrix. cargo check clean across cloacina-workflow/-plugin/cloacina/server/agent.
