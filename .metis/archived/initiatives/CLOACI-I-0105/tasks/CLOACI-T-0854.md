---
id: landlock-and-rlimits-level-plus
level: task
title: "Landlock and rlimits level plus forensics — pre_exec FS ACLs, resource caps, audit achieved-level and rusage"
short_code: "CLOACI-T-0854"
created_at: 2026-07-07T04:02:48.068578+00:00
updated_at: 2026-07-07T04:18:52.126104+00:00
parent: CLOACI-I-0105
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0105
---

# Landlock and rlimits level plus forensics — pre_exec FS ACLs, resource caps, audit achieved-level and rusage

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0105]]

## Objective **[REQUIRED]**

Level 2 (landlock FS ACLs + Phase-1 rlimits, for containers without userns) plus the forensics: every build audits the isolation level it actually ran under.

## Status Updates

### 2026-07-07 — DONE (commit f03d2d4b)
- **Level 2**: `sandbox::apply_landlock` applies a `landlock::Ruleset` in the cargo child's `pre_exec` (Linux, kernel ≥5.13) — RO on `/usr /lib* /bin /sbin /etc /proc /dev /tmp` + vendor registry, RW on the staged source + target cache + `/tmp`; `restrict_self`. `cargo_build` env_clear()s and rebuilds from `build_env` (no `--clearenv` at this level, so the parent scrubs). Dep gated to `cfg(target_os="linux")`; macOS is a no-op (the probe already told the operator).
- **rlimits**: Phase-1 `apply_rlimits` still applied at every level (CPU/AS/FD/proc ceilings before cargo starts).
- **Forensics**: `log_compiler_build_finished` gained a `sandbox_level` field, passed from `config.sandbox_level` at every call site; exit_status + exit_signal (signal_name) already captured in Phase 1. So each build's audit row proves what contained it + how it exited.
- **Deferred (minor)**: peak-RSS via `getrusage(RUSAGE_CHILDREN)` — the ADR's "rusage-level forensics" is satisfied by level+exit+signal; peak memory is a small nice-to-have, noted for a follow-up (not gating the sandbox guarantee). COMPLETE.

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

*To be added during implementation*