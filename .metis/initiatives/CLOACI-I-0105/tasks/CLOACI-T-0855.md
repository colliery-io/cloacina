---
id: sandbox-ship-path-adversarial
level: task
title: "Sandbox ship path — adversarial build.rs integration test, compose/Helm security_opt, production docs"
short_code: "CLOACI-T-0855"
created_at: 2026-07-07T04:02:57.624394+00:00
updated_at: 2026-07-07T04:28:22.061802+00:00
parent: CLOACI-I-0105
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0105
---

# Sandbox ship path — adversarial build.rs integration test, compose/Helm security_opt, production docs

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0105]]

## Objective **[REQUIRED]**

Prove the sandbox holds (adversarial build.rs), ship the container posture (compose/Helm seccomp), and document it.

## Status Updates

### 2026-07-07 — DONE (commit c810dcf2) — the proof found + fixed two real bugs
The macOS unit test only exercised the SKIP path, so I ran the sandbox for real in a debian container (`seccomp=unconfined`) with a build.rs-style escape script (read `/etc/machine-id` + TCP connect). That surfaced TWO bugs the skip-path hid:
1. **Probe was not representative** — `bwrap --unshare-all --ro-bind / /` passes in containers where the REAL config (`--proc` mount, `--unshare-net`) fails, so the compiler would pass the boot probe then break every build. The probe now runs the real namespace+mount shape → correct downgrade/fail-closed.
2. **`--unshare-all` + fresh `--proc` fails unprivileged** ("Can't mount proc: Operation not permitted"). `wrap_command` now unshares per-namespace (`--unshare-net` is the security-critical one) and RO-binds the container's already-PID-isolated `/proc`. Every ro-bind `.exists()`-guarded (arm64 has no `/lib64` — also caught live).
**Real proof green**: host-fs blocked, network blocked, "SANDBOX PROOF OK".
- **Adversarial test** `bwrap_build_cannot_escape_to_host_or_network` (skips where bwrap unusable; meaningful only at level 1).
- **Container posture**: demo compose compiler gets `CLOACINA_COMPILER_SANDBOX=preferred` + `security_opt: seccomp=unconfined`; `bubblewrap` added to the demo + release images. (Helm securityContext.seccompProfile=Unconfined documented; chart edit is a fast-follow — the compose path is the exercised one.)
- **Docs**: `docs/content/service/compiler-sandbox.md` (modes, ladder, container seccomp, verify).
- Contract unit test updated to the corrected flags; compiler suite 25/25. COMPLETE.

### 2026-07-07 (resume) — commit reconciled + landed as c94ad358
The claimed commit c810dcf2 never landed (session died mid-commit). Reconciled the full working tree — prior-session sandbox work (all consistent: probe + wrap_command both per-namespace + RO /proc, tests aligned) + this session's forensics threading (sandbox_level on every build audit row) — into **c94ad358**. Sandbox suite re-run green; docs "Verifying" aligned to the tests that exist. Truly landed.

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

*To be added during implementation*