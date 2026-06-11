---
id: ui-seed-demo-harness-workload
level: task
title: "UI seed + demo harness — workload generator (seed + loop modes) + demo compose profile"
short_code: "CLOACI-T-0660"
created_at: 2026-06-11T02:19:03.526145+00:00
updated_at: 2026-06-11T02:19:03.526145+00:00
parent: CLOACI-I-0117
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI seed + demo harness — workload generator (seed + loop modes) + demo compose profile

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

The environment generator that makes the UI's live-streaming centerpiece testable *and* demoable. A small harness that, against a target `cloacina-server`, ensures a tenant, uploads fixture `.cloacina` packages, and drives executions — in a deterministic **seed mode** (for automated UAT) and a continuous **loop mode** (for "stand it up and watch it run"). Plus the fixtures it needs and a `docker compose` demo profile. Server-side tooling — independent of the UI build, so it can land early and in parallel.

## Acceptance Criteria **[REQUIRED]**

- [ ] Harness (language TBD — reuse the existing test stack; could be Python/angreal alongside the e2e harnesses, or a small Node script sharing the SDK) that, given a server URL + admin key: ensures a tenant, uploads fixtures, and triggers executions.
- [ ] **Seed mode** — produces a known, deterministic state: ≥1 completed run, ≥1 **failed** run, ≥1 **in-flight** run — so T-0661's Playwright assertions are stable.
- [ ] **Loop mode** — continuously fires executions on a configurable interval, mixing fast, slow-enough-to-watch (~10–30s), and intentionally-failing runs, so the UI always shows live activity.
- [ ] **Fixtures**: a **slow-streaming** workflow (emits a visible event sequence over ~10–30s) and a **failing** workflow, in addition to reusing `examples/fixtures/*` where they fit. Compiled `.cloacina` packages available to the harness.
- [ ] **Demo compose profile**: postgres + `cloacina-server` (CORS enabled) + the UI image (T-0659) + the harness in loop mode → `compose up` yields a UI with continuous live activity to watch.
- [ ] Documented invocation (seed vs loop) so both T-0661 and a human can drive it.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Prefer reusing the I-0113 `sdk-contract` server-boot harness for the server side. The driver itself can lean on `@cloacina/client` (Node) or the existing Python tooling. The slow-streaming fixture is the important new artifact — it must emit events gradually so the live view has something to animate; the failing fixture exercises the debug/failed-state UI.

### Dependencies
Independent of the UI feature tasks (it only needs a server). The **demo compose profile** depends on CLOACI-T-0659 (UI image). Consumed by CLOACI-T-0661 (automated UAT).

### Risk Considerations
The slow-streaming fixture must be slow enough to observe but not so slow it makes CI crawl — make the pacing configurable. Use fresh-DB isolation (per CLOACI-T-0649, the server ignores the dbname, so isolate at the DB-create level as the contract harness does).

## Status Updates **[REQUIRED]**

*To be added during implementation*
