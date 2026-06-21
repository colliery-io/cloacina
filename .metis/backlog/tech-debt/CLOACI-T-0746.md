---
id: release-pipeline-gaps-npm-cloacina
level: task
title: "Release pipeline gaps — npm @cloacina scope/token + cross-build PyO3 binaries (2 of 4 targets)"
short_code: "CLOACI-T-0746"
created_at: 2026-06-18T14:58:55.143483+00:00
updated_at: 2026-06-18T14:58:55.143483+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Release pipeline gaps — npm @cloacina scope/token + cross-build PyO3 binaries (2 of 4 targets)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Fix two pre-existing release-pipeline gaps surfaced (not caused) by the 0.8.0
release so future releases publish completely and cleanly. The 0.8.0 release
itself shipped fine on the primary channels (crates.io, PyPI cloacina +
cloacina-client, Docker server+ui images, Helm) — these two are the only gaps,
and both are infra/config, not code regressions.

## The two gaps (from the v0.8.0 `unified_release` run 27756956471)

1. **npm `@cloacina/client` publish — 404, never published.**
   `Publish @cloacina/client to npm` fails: `npm error 404 ... '@cloacina/client@0.8.0'
   is not in this registry`. 0.7.0 was never on npm either (confirmed via the
   registry). The workflow already does `npm publish --access public` with
   `NODE_AUTH_TOKEN=${{ secrets.NPM_TOKEN }}` (`.github/workflows/unified_release.yml:323,327`),
   so the failure is an **npm account/scope issue**: the `@cloacina` org/scope
   doesn't exist or `NPM_TOKEN` lacks publish rights to it. Needs a maintainer to
   create the `@cloacina` npm org + a granular token with publish rights and set
   the `NPM_TOKEN` secret. (Python client is on PyPI, so this only affects the
   TypeScript client's npm distribution.)

2. **2 of 4 release binaries fail cross-compile — `error: no Python 3.x interpreter found`.**
   `Build Binary (aarch64-unknown-linux-gnu)` and `(x86_64-apple-darwin)` fail;
   `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin` succeed. The binary
   (`cloacinactl`) pulls in `cloacina` → `cloacina-python` (PyO3), which needs a
   Python interpreter at build time, and the `cross` image (aarch64-linux) +
   the darwin cross env don't have one. Fix: provision Python 3 in those cross
   build environments (e.g., install python3 + set `PYO3_PYTHON`/
   `PYO3_CROSS_*` in the cross Dockerfile/job, or build `cloacinactl` with
   `--no-default-features` to drop the PyO3 path if the CLI doesn't need it).

## Backlog Item Details
### Type
- [x] Tech Debt — release/CI infrastructure
### Priority
- [x] P2 — not blocking (0.8.0 shipped on the main channels); fix before the next release

## Acceptance Criteria **[REQUIRED]**

- [ ] `@cloacina` npm org/scope exists and `NPM_TOKEN` has publish rights; the npm
      publish job succeeds (TS client published).
- [ ] All four `cloacinactl` binary targets build + attach to the GitHub release
      (Python provisioned for the cross/darwin targets, or PyO3 dropped from the CLI).
- [ ] A dry-run / next release publishes all artifacts with zero failed jobs.

## Notes
- 0.8.0 is live and immutable on crates.io + PyPI (cloacina, cloacina-client) +
  Docker + Helm; the GitHub release carries 2 of 4 platform binaries.
- Optional: backfill the 2 missing v0.8.0 binaries by building locally and
  uploading to the v0.8.0 GitHub release, or just land them in the next tag.
- Decision recorded (2026-06-18): accept 0.8.0 as shipped ("good enough"); these
  fixes are a follow-up, not a 0.8.0 blocker.

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

*To be added during implementation*
