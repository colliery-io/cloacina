---
id: ui-automated-uat-ship-readiness
level: task
title: "UI automated UAT + ship-readiness — Playwright e2e, type-drift gate, version lockstep, Diataxis docs"
short_code: "CLOACI-T-0661"
created_at: 2026-06-11T02:19:04.924145+00:00
updated_at: 2026-06-11T02:19:04.924145+00:00
parent: CLOACI-I-0117
blocked_by: ["CLOACI-T-0656", "CLOACI-T-0657", "CLOACI-T-0658", "CLOACI-T-0659", "CLOACI-T-0660"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI automated UAT + ship-readiness — Playwright e2e, type-drift gate, version lockstep, Diataxis docs

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

Make the UI releasable: an automated UAT pass (Playwright driving the SPA against the seeded environment from T-0660), the generated-types drift gate, version lockstep with the server, and Diataxis docs. The acceptance-scenario gate the whole initiative is measured by.

## Acceptance Criteria **[REQUIRED]**

- [ ] **Playwright `ui-e2e` lane**: builds + serves the SPA, boots `cloacina-server` on a fresh DB (reuse the I-0113 `sdk-contract` harness), runs T-0660 in **seed mode**, then asserts the acceptance scenarios end-to-end:
  - connect (manual API-key path) → land on overview
  - executions list reflects the seeded runs; filter by `status=Failed` works
  - **follow a live (in-flight) run to completion** — events stream in, view reaches terminal (the centerpiece, NFR-002: no dup/gap)
  - open the failed run → its error/event log is visible (UC-2)
  - upload: success path + rejected-package error path (UC-3)
  - key create (one-time plaintext) → revoke (UC-4)
- [ ] **Type-drift gate**: the UI's generated/SDK types are checked against the committed `openapi.json` in CI (mirrors the SDK gate); fails on drift.
- [ ] **Version lockstep**: UI/image version stamped from the workspace version; CI asserts it (extends `scripts/check_sdk_versions.py` or a sibling).
- [ ] **Diataxis docs**: a tutorial (connect → execute → watch) + how-to (deploy the UI container, configure CORS, the demo profile) under `docs/content/`.
- [ ] `ui-e2e` wired into CI (PR smoke subset acceptable; full pass on nightly, mirroring the SDK-contract cadence).

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
A new `angreal test ui-e2e` (or extension of the e2e group) that orchestrates: build UI → boot server (fresh DB) → seed (T-0660) → Playwright. The live-follow assertion is the high-value one — drive a slow-streaming fixture run and assert events accrue to terminal. Keep a fast PR subset vs. full nightly to bound CI wall-time (same trade as the SDK matrix).

### Dependencies
Blocked by CLOACI-T-0656/0657/0658 (the scenarios exercise live-follow, write ops, keys), CLOACI-T-0659 (deployable UI to serve), and CLOACI-T-0660 (the seeded environment). The closing task of the initiative's non-OIDC scope.

### Risk Considerations
Live-stream assertions can be flaky if timing-sensitive — assert on *eventual* terminal state with generous waits, not exact event timing. Reuse the contract-server fresh-DB isolation. Browser-engine smoke (the I-0117 NFR) rides here via Playwright's real browser.

## Status Updates **[REQUIRED]**

*To be added during implementation*
