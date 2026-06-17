---
id: cut-v0-7-0-release-fleet-default
level: task
title: "Cut v0.7.0 release (fleet + default-executor)"
short_code: "CLOACI-T-0641"
created_at: 2026-06-09T23:22:22.622107+00:00
updated_at: 2026-06-10T03:31:29.980769+00:00
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

# Cut v0.7.0 release (fleet + default-executor)

## Objective

Cut the `v0.7.0` release. The fleet + delivery-substrate work (I-0114/I-0115) and the
server-level default-executor change (CLOACI-T-0640) landed on `main` but were never
released — workspace is still pinned at `0.6.1` (== last tag), and `unified_release.yml`
hard-gates `Cargo.toml version == tag`, so no release could fire. 32 commits sit on `main`
past `v0.6.1`.

**Version rationale (0.7.0, minor bump):** T-0640 removed `Router` / `RoutingConfig` /
`RoutingRule` from the public prelude — a breaking change for library consumers, which
pre-1.0 maps to a minor bump. Fleet is also a substantial new feature.

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

- [x] Workspace version bumped `0.6.1 → 0.7.0` (workspace.package + all cloacina-internal path-dep pins)
- [x] `Cargo.lock` regenerated (workspace members only; `deadpool-diesel` 0.6.1 left untouched)
- [x] CHANGELOG `[Unreleased]` rolled to `[0.7.0] - 2026-06-09` with fleet (I-0114/I-0115) + default-executor (T-0640) entries
- [x] Install snippets bumped to 0.7.0 (README + docs)
- [x] Commit on `main`, tag `v0.7.0`, push
- [x] Published: crates.io ✓, PyPI ✓, GHCR Docker ✓, Helm ✓, GitHub Release ✓ (2/4 install binaries)
- [~] Full 4-target binary matrix — 2/4 shipped; remaining 2 (aarch64-linux, x86_64-darwin) carved out to follow-up [[fix-cloacinactl-release-binary]] (CLOACI-T-0650, lands in v0.7.1). Two CI bugs found + fixed en route (nested-workflow perms 4397eb7f; Release-upload perms + idempotent PyPI 9059c6ca).

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

## Status Updates

**2026-06-09** — Release prep done:
- Bumped 20 cloacina-internal Cargo.toml pins `0.6.1 → 0.7.0`; `cargo update --workspace --offline` synced the 12 workspace members in `Cargo.lock` (deadpool-diesel 0.6.1 left untouched). `cargo metadata` confirms cloacina @ 0.7.0.
- CHANGELOG `[Unreleased]` rolled to `[0.7.0] - 2026-06-09`; added fleet (I-0114/I-0115) + default-executor (T-0640) entries, plus the breaking glob-routing-removal note.
- Bumped 46 install-snippet refs across README + docs to 0.7.0 (prior 0.6.1 bump was non-exhaustive; cleared the drift).
- Committed `Release 0.7.0` (c5225734) on main; tagged `v0.7.0`; pushed.

**2026-06-09 (release-pipeline startup_failure → fixed):**
- First two `unified_release.yml` runs (incl. a re-run) died at `startup_failure` in ~3s — zero jobs. No logs persisted; GitHub's API exposes no annotation for startup failures. actionlint clean, valid `needs` graph, depth 3 (≤4), 54-job nightly (well under the 256 cap) — all ruled out.
- Pulled the real message from the run's web page: `The nested job 'nightly-docker' is requesting 'packages: write', but is only allowed 'packages: none'. ...'prune-caches' is requesting 'actions: write', but is only allowed 'actions: none'.`
- **Root cause:** the caller's `permissions:` block is the ceiling for a called reusable workflow. `unified_release.yml`'s workflow-level block is minimal (contents:read, issues:write, id-token:write). Since v0.6.1, `nightly.yml` grew `nightly-docker` (packages:write) + `prune-caches` (actions:write), which exceed that ceiling. Nightly runs fine standalone (no caller cap); v0.7.0 is the first tag since those jobs landed, so it had never surfaced.
- **Fix (commit 4397eb7f):** job-level `permissions:` on the `nightly-suite` call granting the nested superset (contents:read, packages:write, actions:write, issues:write) — keeps the release default token minimal.
- Nothing was ever published (crates.io `updated_at` still 2026-05-09), so safe to move the tag. Force-moved `v0.7.0` → 4397eb7f, force-pushed.
- New run `27242885790` cleared startup; nightly gate fully green.

**2026-06-09 (first full run: published, but binaries 403'd):**
- Run `27242885790` published **crates.io ✓, PyPI ✓, Docker (multi-arch) ✓, Helm ✓** — `cloacina 0.7.0` is live and immutable (crates.io `updated_at` now 2026-06-10).
- The 4 `Build Binary` jobs FAILED — only at `Upload to GitHub Release`: `403 Resource not accessible by integration`. Binaries *built* fine. Cause: `build-release-binaries` has no job-level `permissions`, so inherited the workflow default `contents:read`; creating a Release + uploading assets needs `contents:write`. (Same class as the nested-perms bug; also a never-run f8afd92a job.) So the GitHub Release object + `cloacinactl` install tarballs never got created.
- **Fix (commit 9059c6ca):** (1) `permissions: contents:write` on `build-release-binaries`; (2) `skip-existing:true` on PyPI publish so a same-tag re-run skips instead of 400ing (crates/Docker/Helm already idempotent).
- User OK'd re-running + re-pushing as 0.7.0. Force-moved `v0.7.0` → 9059c6ca, force-pushed.

**2026-06-10 (re-run 27246082223: Release created, 2/4 binaries):**
- Idempotent publishes all skipped/succeeded; `contents:write` fix worked — **GitHub Release v0.7.0 created (published, not draft)** with 2 of 4 tarballs: `x86_64-unknown-linux-gnu` ✓, `aarch64-apple-darwin` ✓.
- **2 binary targets failed with real build errors (not perms):**
  - `aarch64-unknown-linux-gnu` (cross): `pyo3-build-config` → "no Python 3.x interpreter found" — the `cross` container has no Python; cloacinactl pulls pyo3 transitively.
  - `x86_64-apple-darwin` (x86 on arm64 runner): `ld: symbol(s) not found for architecture x86_64` (`_rd_kafka_*`) — no x86_64 librdkafka on the Apple-Silicon runner.
- **Root cause:** `crates/cloacinactl/Cargo.toml` → `[features] default = ["postgres","sqlite","kafka"]`. The release build runs `cargo build --bin cloacinactl` without `--no-default-features`, so every binary links librdkafka (kafka) + pulls pyo3. Native-arch builds tolerate it; cross/x-arch builds don't.

**RELEASE OUTCOME: v0.7.0 is fully cut** — crates.io, PyPI, GHCR Docker, Helm, + published GitHub Release with install binaries for the 2 primary platforms (x86_64-linux, aarch64-darwin). Remaining: 2 of 4 convenience binaries (aarch64-linux, x86_64-darwin) need a cloacinactl build-portability fix. **RESOLVED — user chose to defer** to follow-up [[fix-cloacinactl-release-binary]] (CLOACI-T-0650), to land in v0.7.1. Task closed completed.