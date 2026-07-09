---
id: ci-hardening-for-0-9-0-auth-aurora
level: task
title: "CI hardening for 0.9.0 auth — aurora-dark https, auth integration lane, OIDC/Dex"
short_code: "CLOACI-T-0804"
created_at: 2026-06-24T15:11:36.400011+00:00
updated_at: 2026-06-24T15:30:10.460980+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# CI hardening for 0.9.0 auth — aurora-dark https, auth integration lane, OIDC/Dex

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Close the CI gaps the 0.9.0 auth/design-sync work opened: (1) the `aurora-dark` design-sync dep was pinned git+ssh → CI's `npm install` would fail with no deploy key; (2) the new auth surface (local accounts, whoami, the cross-tenant key-management leak fix) had no end-to-end CI gate; (3) OIDC had zero automated coverage.

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

**2026-06-24 — COMPLETE, all three verified locally.**
1. **aurora-dark https** — repinned `ui/package.json` to `git+https://…#commit` + rewrote the lockfile `resolved` from git+ssh→https (repo is public). Proved a clean re-fetch with `GIT_SSH_COMMAND=false` installs → CI needs no SSH secret.
2. **auth integration lane** — added group 8 to `.angreal/test/auth.py` (whoami roles, the **leak-fix regression** = tenant key→global /auth/keys GET+POST→403, local accounts create→login→whoami→refresh→logout) + wired `angreal test auth` into `cloacina.yml` (postgres matrix). Fixed the harness (`api_request` crashed on an empty 2xx body — was masking the whole suite) + 3 pre-existing failures (items envelope; a check that asserted the now-god-only /auth/keys; metrics counter needing a pipeline). **49/49 pass, exit 0.**
3. **OIDC/Dex** — added a `dex` service to `.angreal/docker-compose.yaml` + `.angreal/dex-config.yaml` (localhost issuer for the host-run test) + a CI step running the `#[ignore]`'d OIDC tests (discovery + begin_login) against it. **2/2 pass** locally (had to stop the demo dex — one owner of :5556).

**Coverage map for reviewers:** spec-drift + SDK-drift + UI typecheck/build already on PR; auth unit tests (52-route no-drift, mapping, argon2, mint) ride `angreal test integration`; now + auth e2e + OIDC. The full OIDC browser callback (token validation through Dex's login) is still only manual/nightly-e2e territory. All committed; not tagged.