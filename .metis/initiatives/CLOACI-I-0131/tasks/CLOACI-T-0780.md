---
id: multi-arch-artifacts-per-target
level: task
title: "Multi-arch artifacts — per-target cdylibs with triple-matched dispatch"
short_code: "CLOACI-T-0780"
created_at: 2026-06-23T02:04:15.730326+00:00
updated_at: 2026-06-23T02:04:45.054809+00:00
parent: CLOACI-I-0131
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0131
---

# Multi-arch artifacts — per-target cdylibs with triple-matched dispatch

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0131]]

## Objective **[REQUIRED]**

A package can hold only ONE cdylib today (workflow_packages, unique per
package+version, single compiled_data), and dispatch stamps build_target_triple
from the SERVER host. Heterogeneous fleets can't work: a non-matching agent
fail-closed refuses with no alternate artifact. Add per-target artifacts +
triple-matched dispatch so a package can carry an x86_64 AND aarch64 cdylib and
each agent is handed the one matching its target_triple. Extends the per-tenant
compiler pattern (CLOACI-T-0779) to per-target.

## Plan (additive — host path untouched, zero risk to current execution)

- **Storage:** new `package_artifacts(content_hash, package_name, version,
  tenant_id, target_triple, compiled_data, created_at; unique(name,version,tenant,
  triple))` — EXTRA per-triple cdylibs. workflow_packages stays the primary
  (host) build. sqlite + postgres migration.
- **Dispatch:** select artifact for (package, tenant, AGENT triple) =
  package_artifacts[triple] ?? workflow_packages primary; stamp build_target_triple
  from the chosen artifact's actual triple (not server host). Host agents keep
  hitting the primary (unchanged).
- **Compiler:** `--build-target <triple>` flag (alongside --tenant-schema); when
  set + != host, store into package_artifacts tagged with the triple. Actual
  cross-cargo-build needs cross toolchains in the image — DEFERRED (no demo value
  single-arch); wire the flag + storage now.
- **Agent fetch:** unchanged (content-addressed by digest).
- **VERIFY (single-arch):** insert a synthetic 2nd-triple artifact; assert
  get_dispatch(package, "aarch64-…") returns it while host triple returns the
  primary — proves triple-matched selection. Real cross-exec needs a 2nd-arch
  runner (out of scope).

## Status Updates **[REQUIRED]**

- 2026-06-23: Scoped off the per-tenant compiler (T-0779). User: wire up multi-arch
  now. Additive package_artifacts + triple-matched dispatch; cross-toolchain
  deferred. Building.

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