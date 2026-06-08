---
id: hardening-fleet-soak-variant
level: task
title: "Hardening — fleet soak variant, Diataxis docs, optional Helm agent deployment"
short_code: "CLOACI-T-0635"
created_at: 2026-05-27T17:36:35.740756+00:00
updated_at: 2026-06-07T16:51:26.846634+00:00
parent: CLOACI-I-0114
blocked_by: [CLOACI-T-0634]
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0114
---

# Hardening — fleet soak variant, Diataxis docs, optional Helm agent deployment

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0114]].

## Objective **[REQUIRED]**

Make the fleet operable and durable over time. Add a sustained-load soak that runs a multi-agent fleet, ship the Diataxis docs for deploying/operating it, and (optionally) add a Helm agent deployment. Soak is first-class here — the prior server soak surfaced executor-deadlock and routing gaps ([[project_soak_test_gaps]]), so we exercise the fleet under sustained load, not just in unit/integration tests.

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] New `angreal test soak` fleet variant: server + N agents under sustained load, watching for roster leaks, stuck in-flight/outbox entries, and reconciliation drift over time.
- [ ] Soak stable over the target duration with no leaked work, no roster drift, flat outbox depth.
- [x] Diataxis docs: how-to (deploy an agent, route a glob to the fleet, operate it), explanation (where the fleet sits in scaling), reference (agent config/flags, metrics). **Done 2026-06-07.**
- [ ] Optional: Helm chart agent deployment (server in DB trust zone; agents pointed at it with API key + tenant).
- [x] Metrics doc updated with fleet/agent + outbox signals; scrape validated. **Done 2026-06-07** (fleet counters + delivery_outbox gauge in metrics-catalog; Hugo build green).

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Extend the existing soak harness (`angreal test soak server`) to stand up agents. Docs follow the established Diataxis structure from the I-0112 refresh. Helm work builds on the existing chart (`angreal helm`).

### Dependencies
[[CLOACI-T-0634]] (churn/saturation correctness must land before soak is meaningful). Closes [[CLOACI-I-0114]].

### Risk Considerations
- Soak is where slow leaks (roster entries, outbox rows, file-cache growth on agents) surface — instrument before running, not after.
- Use fresh DBs per soak run ([[feedback_stale_db_testing]]).

## Status Updates **[REQUIRED]**

### 2026-06-07 — Diataxis fleet docs DONE (soak + helm agent chart still open)

Documented the fleet (I-0114) end to end — it had zero doc coverage. Hugo build
green (446 pages, no ref errors; `angreal docs build`).

New pages:
- `explanation/execution-agent-fleet.md` — register → route → claim → deliver →
  fetch+dlopen → execute → reconcile, dead-agent reclaim, tenant isolation, OQ-6
  triple/profile match, when to use fleet vs single-DB multi-runner vs
  in-process, observability.
- `how-to-guides/deploy-an-execution-agent-fleet.md` — route a glob, run agents,
  verify, tune liveness, operate.

Reference drift fixed: `reference/cli.md` (server fleet/liveness flags + new
`## agent` section), `reference/environment-variables.md` (fleet + agent vars +
summary), `reference/metrics-catalog.md` (`cloacina_fleet_*` +
`cloacina_delivery_outbox_open`), `explanation/horizontal-scaling.md`
(See-Also cross-link). (`--context` was already documented; the CLI fix made the
code match.)

**Still open under T-0635:** fleet soak variant (`angreal test soak` fleet) and
the optional `charts/cloacina-agent` Helm chart. Docs deliverable complete.
