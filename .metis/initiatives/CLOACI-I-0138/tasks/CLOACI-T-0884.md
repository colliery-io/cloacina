---
id: canonical-rust-packaged-example
level: task
title: "Canonical Rust packaged example — promote simple-packaged + gold-path demo-stack README"
short_code: "CLOACI-T-0884"
created_at: 2026-07-10T01:16:01.284448+00:00
updated_at: 2026-07-10T01:16:01.284448+00:00
parent: CLOACI-I-0138
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0138
---

# Canonical Rust packaged example — promote simple-packaged + gold-path demo-stack README

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0138]]

## Objective **[REQUIRED]**

Make `examples/features/workflows/simple-packaged` THE canonical Rust packaged example by adding the gold-path (server/daemon) run recipe it's currently missing, and verify it end-to-end against the docker compose demo stack.

**Key finding (2026-07-09):** the server GOLD-PATH recipe does not exist yet anywhere. Even the "service" tutorial (`docs/content/service/tutorials/03-packaged-workflows.md`) "Build and run" is `cargo run --example end_to_end_demo` — an IN-PROCESS demo that packages+loads+executes embedded. `registry-execution` is likewise embedded (`DefaultRunner`). So this task authors the genuinely-new flow, not a README polish.

**The recipe (grounded — cloacinactl mechanism):**
1. `cloacinactl package pack . --out simple-packaged.cloacina` (build the `.cloacina`; `package validate` to check the archive).
2. Bring up the **docker compose demo stack** (server + UI + services) — the canonical "server" for examples ([[feedback_use_container_stack]]).
3. `cloacinactl config profile set demo <server-url> --api-key <bootstrap-key> --default` — point the CLI at the running server.
4. `cloacinactl package upload simple-packaged.cloacina` (or `publish`) — register with the running server.
5. Trigger the workflow via the server (cloacinactl trigger/execution noun or API) and observe via the API / web UI — NOT an in-process `DefaultRunner`.

**Deliverable:** the README recipe above (verified against a live stack), plus keeping the existing embedded `end_to_end_demo` only if it's clearly labeled as the alternative embedded path. Nail the exact `package upload` args + trigger command + demo-stack bring-up + bootstrap-key retrieval during implementation.

**REMAINING (needs a live stack):** end-to-end verification requires docker + the demo stack up. Not yet run.

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

### 2026-07-10 — recipe VERIFIED through upload; trigger/observe blocked by a stale demo stack
Ran the gold path live against `angreal ui up`:
1. `cloacinactl package pack examples/features/workflows/simple-packaged --out X.cloacina` → 20KB source archive ✅
2. `cloacinactl config profile set demo http://localhost:8080 --api-key clk_demo_public_key_0003 --tenant public --default` ✅
3. `cloacinactl package upload X.cloacina` → **server accepted, returned package id 9dc1e30b-… ✅** (server-side registration + compiler build — the genuinely-new part works).
Trigger/observe commands CONFIRMED but not completed: `cloacinactl workflow run data_processing` (POST `/v1/tenants/{t}/workflows/{name}/execute`) and `cloacinactl execution list`.

**Blocker = environment, not recipe.** The running demo stack was a STALE ~3-day-old instance (the earlier `angreal purge` docker-down used a different project name); its DB pool repeatedly exhausts (`Connection pool error: Timeout waiting for a slot`) under 5 agents + other postgres projects on the shared docker host — `/health` returns 200 but API calls flap with network errors. One `docker restart cloacina-demo-server-1` cleared it long enough for the upload; it re-saturates.

**To finish:** on a CLEAN stack (`angreal ui down` → `angreal ui up`, isolated), complete `workflow run` + `execution list`, then write the verified recipe into the simple-packaged README as the canonical gold-path example. Recipe is otherwise ready.
