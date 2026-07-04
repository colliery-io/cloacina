---
id: fleet-agent-constructor-execution
level: task
title: "Fleet/agent constructor execution — ship provider bundles to agents + resolve constructor nodes in the agent load path"
short_code: "CLOACI-T-0838"
created_at: 2026-07-04T03:30:18.377372+00:00
updated_at: 2026-07-04T03:30:18.377372+00:00
parent: CLOACI-I-0132
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0132
---

# Fleet/agent constructor execution — ship provider bundles to agents + resolve constructor nodes in the agent load path

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0132]]

## Objective **[REQUIRED]**

Make packaged constructor workflows executable on the **agent fleet** (`CLOACINA_DEFAULT_EXECUTOR=fleet`). Today only the SERVER can execute them: its reconciler Step 5b ([[CLOACI-T-0836]]) fetches the bundled providers from `package_providers`, unpacks them, and resolves each `constructor!` node into its runtime registry — but a task dispatched to an **agent** fails, because the agent's package-load path neither receives the provider bundles nor resolves constructor nodes.

**Live evidence (2026-07-04, demo stack, finding #4 of the T-0836 verification):** with the fleet executor, the `reader` node of `demo-constructor-rust` dispatched to an agent which loaded the package via the task-registrar "host-managed approach" and reported:
`"task public::demo-constructor-rust::constructor_demo::reader not registered after loading package (registered: [summarize])"`.
The demo works only with the in-process executor (`default`) via a compose override.

**Scope:**
1. **Provider-bundle delivery to agents** — agents sync packages over the fleet protocol / server API, not the DB, so their registry's `get_package_providers` is the default-empty impl. Either extend the fleet package-sync payload to carry the `package_providers` rows (name, version, hash, archive bytes), or add a server endpoint agents fetch bundles from by content hash (mirrors the artifact-by-digest fetch).
2. **Constructor-node resolution in the agent load path** — the agent-side package load (task-registrar driven) must run the Step-5b equivalent: extract `ConstructorPackageMetadata` (FFI idx 10), stage the bundles (`stage_bundled_providers` seam), resolve via `load_constructor_node`, and register the node before task lookup. The agent already links `constructors-wasm` (explicit since T-0836) and runs the same loader code — the gap is purely wiring + bundle delivery.
3. **Fail-closed parity** — an agent handed a constructor-bearing package without its bundles must refuse the load with the same clear error the server gives, not a mystery "task not registered".
4. **Verification** — the demo stack with `CLOACINA_DEFAULT_EXECUTOR=fleet` (the compose default, no override) runs `constructor_demo` to completion on an agent; e2e fleet lane covers it.

## Acceptance Criteria **[REQUIRED]**

- [ ] Agents receive (or fetch) the provider bundles for constructor-bearing packages
- [ ] The agent load path resolves + registers `constructor!` nodes (Step-5b parity), grants enforced identically
- [ ] Missing bundles fail the agent load closed with a named error
- [ ] `constructor_demo` completes on the demo stack with the stock fleet executor (no compose override)

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
