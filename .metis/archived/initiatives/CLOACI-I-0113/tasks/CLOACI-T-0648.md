---
id: cross-sdk-release-angreal-sdk
level: task
title: "Cross-SDK release — angreal sdk-contract matrix, Diataxis docs, lockstep release tooling"
short_code: "CLOACI-T-0648"
created_at: 2026-06-10T01:30:42.525161+00:00
updated_at: 2026-06-10T11:53:29.578137+00:00
parent: CLOACI-I-0113
blocked_by: [CLOACI-T-0645, CLOACI-T-0646, CLOACI-T-0647]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0113
---

# Cross-SDK release — angreal sdk-contract matrix, Diataxis docs, lockstep release tooling

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0113]]

## Objective **[REQUIRED]**

Aggregate and ship: an `angreal test sdk-contract` matrix running all three SDK contract suites against a live server in CI, Diataxis docs per language (tutorial, how-to, reference), version-lockstep release tooling so SDK versions stamp from the workspace version, and the first tagged SDK release riding the next cloacina release (REQ-006/REQ-008). This phase is an aggregation — each SDK's contract suite was already green when its own task exited.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] `angreal test sdk-contract` + `sdk-contract-rust`/`-python`/`-ts` boot `cloacina-server` on a **fresh DB** via the existing compose and run the suites; full matrix passed locally; `sdk-contract` job added to nightly.yml (which is also the release gate)
- [x] Coverage rule enforced by `scripts/check_sdk_coverage.py` (every spec operation reachable from every SDK + all 4 delivery-WS variants handled per SDK — it immediately caught a missing `ready()` in the Python shim); live round-trips are the suites themselves. Reactor WS variants are schema-documented but unwrapped pending a graph fixture — noted in the script header as follow-up
- [x] Diataxis docs: new `docs/content/sdks/` section — overview (`_index.md`: lockstep policy, auth/error/pagination/WS shared concepts, service-vs-embedded distinction) + per-language pages each with Tutorial / How-to (auth, pagination, errors, WS, browser+CORS for TS) / Reference linking `/openapi.json` and the WS protocol page (REQ-006)
- [x] Lockstep tooling: `scripts/check_sdk_versions.py` (workspace vs package.json vs pyproject vs `__version__` vs spec `info.version`) runs in `verify-version` and in the sdk-contract matrix; `publish-cargo` gains `cloacina-api-types` (tier 1) + `cloacina-client` (before cloacinactl); new idempotent `publish-npm` (skip when version exists) and `publish-pypi-client` (skip-existing) jobs with explicit job-level permissions (REQ-008)
- [x] First tagged SDK release rides the next cloacina release — all wiring is on the tag path (verify-version → publishes); requires the `NPM_TOKEN` secret to exist before that tag (flagged to user)

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
New angreal task group `test sdk-contract` (+ `sdk-contract-rust`/`-python`/`-ts`) reusing the existing e2e compose to boot the server, then running each suite. Coverage rule enforced by a script that diffs exercised endpoints/WS variants against the spec. Docs go into the existing Diataxis tree. Version stamping: SDK package versions derive from `workspace.package.version` at release time in `unified_release.yml`.

### Dependencies
Blocked by CLOACI-T-0645, CLOACI-T-0646, CLOACI-T-0647 — this phase aggregates; each SDK's suite was green at its own task exit.

### Risk Considerations
Release-workflow permissions: the v0.7.0 release hit two rounds of GitHub Actions permission ceilings (nested reusable-workflow perms, missing `contents:write`) — new npm/PyPI publish jobs need explicit job-level `permissions` from the start. CI wall-time for three live-server suites — consider full matrix on nightly with a smoke subset on PR.

## Status Updates **[REQUIRED]**

**2026-06-10** — Implemented on `i0113-server-sdks`:
- `.angreal/test/e2e/sdk_contract.py` (registered in `test/__init__.py`): shared `_sdk_server()` context (build via cli.py helpers, compose postgres, **DROP/CREATE the `cloacina` DB** — isolation must happen at this level until CLOACI-T-0649 is fixed, dedicated port 18084), runs version check → coverage check → suites. Per-language commands reuse the same harness.
- `scripts/check_sdk_versions.py` + `scripts/check_sdk_coverage.py`. Coverage detection per SDK: TS = literal spec path in client.ts; Rust = wildcarded path skeleton among lib.rs string literals; Python = generated operationId module imported in _client.py; WS variants searched case-insensitively across impl + suite. Caught a real gap on first run (Python `ready()` missing — added with test).
- nightly.yml: `sdk-contract` job (rust+node+uv+angreal, runs the matrix) — nightly is the release gate, so SDK drift is now release-blocking.
- unified_release.yml: lockstep check in `verify-version`; `cloacina-api-types` published tier-1 (cloacina depends on it) and `cloacina-client` before `cloacinactl`; new `publish-npm` (idempotent via npm-view check, **requires NPM_TOKEN secret**) and `publish-pypi-client` (uv build + skip-existing) jobs, both with explicit job-level permissions per the v0.7.0 lesson. actionlint clean on both workflows.
- Docs: `docs/content/sdks/` (overview + rust + python + typescript), weight 35.
- **Local verification: full `angreal test sdk-contract` matrix passed** (lockstep + coverage + Rust 3 tests + Python 19 + TS 27 against one fresh live server).
- **User action needed before next tagged release:** create the `NPM_TOKEN` repo secret (PYPI_TOKEN/CARGO_REGISTRY_TOKEN already exist). → **Done 2026-06-10** (user added the secret). Additionally `publish-npm` is `continue-on-error: true` until the first scoped publish succeeds — first publish of a scoped npm package often needs one-time org/2FA setup; drop the soft-fail after it lands (commit e1bf28de).