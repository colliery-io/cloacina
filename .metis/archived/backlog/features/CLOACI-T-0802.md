---
id: release-0-9-0-docs-pass-release
level: task
title: "Release 0.9.0 — docs pass + release staging"
short_code: "CLOACI-T-0802"
created_at: 2026-06-24T10:50:05.024564+00:00
updated_at: 2026-06-24T11:16:00.307217+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Release 0.9.0 — docs pass + release staging

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Stage the **0.9.0** release: a docs pass over the unreleased work (90 commits since v0.8.0 — auth I-0118, workflow-legibility I-0126/0128, trigger fan-out, multi-arch fleet, per-tenant isolation, Aurora UI), plus release mechanics (CHANGELOG, version bump, OpenAPI spec). **Stage on the branch; do NOT tag/publish** (user's call).

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

**2026-06-24 — release mechanics + auth/trigger docs DONE; reference long-tail tracked.**

**Done:**
- **CHANGELOG**: comprehensive `[0.9.0]` entry across all three areas (auth; declared params; what/why docs; source retrieval; operator inject/fire; pause/resume; manual trigger fire; multi-arch fleet; per-tenant metrics/build; Aurora UI; SDK coverage; breaking = ABI v2→v3, ABAC authz + key-leak fix, trigger fan-out, param validation, paused-409; security; fixed = tenant-exec isolation, PyO3 deadlock, macros) + backfilled the missing `[0.8.0]` entry.
- **Version → 0.9.0**: workspace.package.version + all intra-workspace pins (`perl` sweep), `ui/package.json`, `clients/typescript/package.json`, Connect footer.
- **OpenAPI**: regenerated `docs/static/openapi.json` (only drift was the version string — surface already current); `angreal docs spec-check` = in sync. Build verified (spec-check ran the 0.9.0 binary).
- **Docs**: rewrote `security-model.md` (ABAC + identity providers + session lifecycle + multi-tenant individuals — the old `can_access_tenant` text was wrong); new `configure-local-accounts.md` + `configure-oidc-sso.md`; fixed `engine/scheduling/trigger.md` (named fan-out point).

**Remaining doc gaps (reference/prose long tail, NOT release-blocking — CHANGELOG + current OpenAPI spec cover the surface):** `engine/workflows/workflow.md` (params), `engine/workflows/task.md` + `python-api/task.md` (what/why; pure-Python persistence deferred T-0754), `reference/macros.md`, `python-api/trigger.md` (params), `reference/http-api.md` (source/pause/fire/interface + new fields), `reference/errors.md` + `api-error-envelope.md` (4 new codes), `reference/cli.md` (reactor fire / accumulator inject), `reference/package-manifest.md` (declared_params + what/why), `reference/ffi-vtable.md` (CloacinaPlugin v3, method idx 9), `execution-agent-fleet.md` (multi-arch), `multi-tenancy.md` (per-tenant build/exec/metrics isolation), `engine/explanation/subscription-fan-out.md` (cross-ref trigger-name vs reactor fan-out), `engine/computation-graphs/` (operator-injection how-to).

**Next:** clear the long tail (a focused reference-sync pass) or land the staged release first; tag/publish remains the user's call.

**2026-06-24 — COMPLETE (long tail cleared via 4 parallel agents, all source-verified + reviewed).** Spun up 4 agents over disjoint doc clusters; each verified claims against `crates/` and **caught real errors that had propagated from the investigation reports / my prompts** — corrected before commit: (1) there is **no `#[trigger] params(...)`** (abandoned in T-0777) — the trigger's typed surface is the *union of subscribed workflows' declared params* via `/interface` (fixed `python-api/trigger.md` **and the CHANGELOG**); (2) reactor `force_fire`/`fire_with` are values of a `mode` field, not separate fields, and the inject body key is `event`; (3) the CLI `fire` verb takes `--input` (not `force-fire`), and no workflow/trigger pause/resume CLI exists (HTTP-only). Files: authoring (task/macros/python-api task+trigger/package-manifest), ffi-vtable (ABI v3 + method 9), execution-agent-fleet (multi-arch), multi-tenancy (per-tenant build/exec/metrics), subscription-fan-out (trigger-name vs reactor + bidirectional cross-link), new how-to `drive-graph-surfaces-manually.md`, http-api (source/pause/fire/interface/health-reactors + WorkflowDetail fields + execute error codes), cli (reactor/accumulator nouns). **`angreal docs build` = clean Hugo build, no broken refs.** All committed. **Staged for review on `cloaci-i0118-abac-authz`; NOT tagged/published** (user's call).