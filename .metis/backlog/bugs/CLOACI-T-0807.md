---
id: release-binary-build-fix-aarch64
level: task
title: "Release binary build — fix aarch64-linux (pyo3 cross) + x86_64-darwin (libpq arch) legs"
short_code: "CLOACI-T-0807"
created_at: 2026-06-27T13:09:48.069830+00:00
updated_at: 2026-07-05T15:36:12.857767+00:00
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

# Release binary build — fix aarch64-linux (pyo3 cross) + x86_64-darwin (libpq arch) legs

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

The v0.9.0 release pipeline (`unified_release.yml`, "Build Binary") publishes downloadable `cloacinactl` binaries for 4 targets; **2 legs failed** (the core release — crates.io/PyPI/Docker/Helm — was unaffected and shipped):
- **aarch64-unknown-linux-gnu** — built via Docker `cross`, whose image has **no Python interpreter** for `pyo3-build-config` (`cloacinactl` pulls pyo3 transitively through `cloacina-workflow-plugin`). `error: no Python 3.x interpreter found`.
- **x86_64-apple-darwin** — built on `macos-latest` (Apple Silicon), so `brew install libpq` yields **ARM64 libpq** and the x86_64 link fails: `ld: undefined _PQ* symbols` (the `postgres` feature needs libpq).

Both are runner-architecture mismatches, not code bugs. Fix = build every leg natively on the right-arch runner.

### Type
- [x] Bug — broken release binary build (degraded artifact, not a code defect)

### Priority
- [x] P2 — only the *downloadable GitHub-release binaries* for 2 targets; cargo/pip/Docker cover distribution.

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

- [x] aarch64-linux builds on native **`ubuntu-24.04-arm`** (cross:false) — Python + apt libpq present, no Docker `cross`.
- [x] x86_64-darwin builds on the Intel **`macos-13`** runner — `brew` libpq is x86_64, link succeeds.
- [x] Dead `cross` install/build steps removed; the single `Build` step runs for all legs; existing libpq/strip steps still apply (`matrix.cross == false`).
- [x] YAML validates.
- [ ] Verified by an actual release run (next tag) producing all 4 binaries — the fix is on `unified_release.yml`, which runs at the *release tag*, so it takes effect on the next release.
- [ ] **Open decision (v0.9.0 backfill):** re-running v0.9.0's failed jobs uses the OLD workflow at tag `6bd2e15c` and would still fail. Backfilling v0.9.0's two missing binaries requires either moving the published tag (risky) or a manual cross-arch build + `gh release upload`. Default: leave them (other channels cover v0.9.0); next release gets all 4.

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

**2026-06-27 — fix implemented (in PR).** `unified_release.yml` Build-Binary matrix: aarch64-linux → `ubuntu-24.04-arm` (cross:false), x86_64-darwin → `macos-13`; removed the now-dead `Install cross` / `Build (cross)` steps and collapsed to a single native `Build` step. Diagnosis confirmed locally: `cargo tree -p cloacinactl --no-default-features --features postgres,sqlite` includes `pyo3-build-config` (via cloacina-workflow-plugin), and the darwin failure is `_PQ*` (libpq) — both arch-mismatch, not code. YAML validated (ruby). Pending: next-release verification + the v0.9.0-backfill decision (see acceptance criteria).

**2026-07-05 — CLOSING.** Current `unified_release.yml` state (verified): aarch64-linux runs natively on `ubuntu-24.04-arm` (:389-394, cross:false) ✓; **x86_64-darwin was subsequently RETIRED rather than fixed** (:395-396) — the matrix is 3 legs (x86_64-linux, aarch64-linux, aarch64-darwin), all native, no continue-on-error, `fail_on_unmatched_files: true`. Residuals dispositioned: next-tag verification of the binary legs folds into [[CLOACI-T-0746]] (the remaining release-pipeline task — it must watch the next release anyway for npm); v0.9.0 backfill decision = default taken (leave it; cargo/pip/Docker cover v0.9.0). COMPLETE.
