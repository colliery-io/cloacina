---
id: ui-automated-uat-ship-readiness
level: task
title: "UI automated UAT + ship-readiness — Playwright e2e, type-drift gate, version lockstep, Diataxis docs"
short_code: "CLOACI-T-0661"
created_at: 2026-06-11T02:19:04.924145+00:00
updated_at: 2026-06-11T18:36:15.384264+00:00
parent: CLOACI-I-0117
blocked_by: [CLOACI-T-0656, CLOACI-T-0657, CLOACI-T-0658, CLOACI-T-0659, CLOACI-T-0660]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI automated UAT + ship-readiness — Playwright e2e, type-drift gate, version lockstep, Diataxis docs

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

Make the UI releasable: an automated UAT pass (Playwright driving the SPA against the seeded environment from T-0660), the generated-types drift gate, version lockstep with the server, and Diataxis docs. The acceptance-scenario gate the whole initiative is measured by.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] **Playwright `ui-e2e` lane** (`angreal test ui-e2e [--smoke]`, `.angreal/test/e2e/ui_e2e.py`): builds server + compiler + cloacinactl, boots `cloacina-server` on a **fresh DB** + `cloacina-compiler`, packs fixtures, builds + serves the SPA (`vite preview`), seeds T-0660, then runs the Playwright suite. **Ran green end-to-end** (6/6 full; 3/3 smoke via the orchestrator). Scenarios (`ui/e2e/`):
  - [x] connect (manual API-key) → overview (`connect.spec.ts`, @smoke)
  - [x] executions list reflects seeded runs; `?status=Failed` filter (@smoke)
  - [x] **follow an in-flight run to terminal** — asserts on the status badge (testid) flipping to terminal; the live test genuinely ran **~49s** following a streaming run (NFR-002 centerpiece)
  - [x] open the failed run → status + event log visible (UC-2, @smoke)
  - [x] upload success + rejected-package error (UC-3)
  - [x] key create (one-time plaintext) → revoke (UC-4)
- [x] **Type-drift gate**: the SDK's `check:generated` (committed `generated/types.ts` vs a fresh emit from `openapi.json`) + the UI `typecheck`/`build` against the SDK types — both in the new `ui-checks` CI job (and the existing `spec-check`). Verified locally (no drift).
- [x] **Version lockstep**: extended `scripts/check_sdk_versions.py` to assert `ui/package.json` + `ui/harness/package.json` match the workspace version; run in the `ui-checks` PR job. Passes at 0.7.0.
- [x] **Diataxis docs**: tutorial `docs/content/platform/tutorials/02-the-web-ui.md` (connect → seed → watch live → upload/execute) + how-to `docs/content/platform/how-to-guides/deploy-the-web-ui.md` (image, runtime config, CORS, compose, Helm, version lockstep).
- [x] **CI wiring**: PR gate = cheap Node-only `ui-checks` (version + drift + UI typecheck/build) on `ui`/`api`/`workflows` changes; **full Playwright lane on nightly** (`ui-e2e` job → `angreal test ui-e2e`), mirroring the SDK fast-PR / full-nightly cadence. Both workflow YAMLs validated.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
A new `angreal test ui-e2e` (or extension of the e2e group) that orchestrates: build UI → boot server (fresh DB) → seed (T-0660) → Playwright. The live-follow assertion is the high-value one — drive a slow-streaming fixture run and assert events accrue to terminal. Keep a fast PR subset vs. full nightly to bound CI wall-time (same trade as the SDK matrix).

### Dependencies
Blocked by CLOACI-T-0656/0657/0658 (the scenarios exercise live-follow, write ops, keys), CLOACI-T-0659 (deployable UI to serve), and CLOACI-T-0660 (the seeded environment). The closing task of the initiative's non-OIDC scope.

### Risk Considerations
Live-stream assertions can be flaky if timing-sensitive — assert on *eventual* terminal state with generous waits, not exact event timing. Reuse the contract-server fresh-DB isolation. Browser-engine smoke (the I-0117 NFR) rides here via Playwright's real browser.

## Status Updates **[REQUIRED]**

**2026-06-11 — Implemented + ran the full acceptance suite green.**
- **Playwright suite** `ui/`: `playwright.config.ts` (chromium = the I-0117 real-browser smoke), `e2e/env.ts` (E2E_* wiring + sessionStorage pre-seed for authenticated specs), `e2e/connect.spec.ts`, `e2e/scenarios.spec.ts`. Added `@playwright/test` devDep + `e2e` script. Added `data-testid="execution-status"` to ExecutionDetail so the live-follow test targets the badge, not stray text.
- **Orchestrator** `.angreal/test/e2e/ui_e2e.py` (`angreal test ui-e2e [--smoke]`): builds server+compiler+cloacinactl, fresh DB, boots server then compiler, packs fixtures, builds+serves the SPA, seeds (harness writes a summary file → exec IDs forwarded to Playwright), runs the suite, tears everything down. Registered in `.angreal/test/__init__.py`. Harness gained a `HARNESS_SUMMARY_FILE` JSON output.
- **Gates:** `scripts/check_sdk_versions.py` now covers `ui` + `ui/harness`. New `ui-checks` PR job (Node-only: version lockstep + SDK `check:generated` + UI typecheck/build). Full Playwright lane added to `nightly.yml`. `ui` path filter added to ci `changes`.
- **Docs:** platform tutorial `02-the-web-ui.md` + how-to `deploy-the-web-ui.md`.
- **Verification:** ran the suite manually against a hand-built stack → first pass 4/6 (two selector bugs: a too-broad upload matcher and a flaky nav assertion — both fixed, not product bugs), then **6/6 green**, with the live-follow test taking **49s** (genuinely followed a streaming run to terminal). Then ran `angreal test ui-e2e --smoke` → orchestrator built/seeded/served and **3/3 smoke green**, clean teardown. Version + drift + UI typecheck all pass locally. Test stack + tmp cleaned up.
- **CI-cost call:** PR runs only the cheap Node gates; the Docker+Rust+browser Playwright lane runs nightly (same fast-PR/full-nightly split as the SDK matrix) — the task explicitly allows this.