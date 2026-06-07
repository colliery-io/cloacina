---
id: hardening-fleet-soak-variant
level: task
title: "Hardening — fleet soak variant, Diataxis docs, optional Helm agent deployment"
short_code: "CLOACI-T-0635"
created_at: 2026-05-27T17:36:35.740756+00:00
updated_at: 2026-05-27T17:36:35.740756+00:00
parent: CLOACI-I-0114
blocked_by: ["CLOACI-T-0634"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0114
---

# Hardening — fleet soak variant, Diataxis docs, optional Helm agent deployment

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0114]].

## Objective **[REQUIRED]**

Make the fleet operable and durable over time. Add a sustained-load soak that runs a multi-agent fleet, ship the Diataxis docs for deploying/operating it, and (optionally) add a Helm agent deployment. Soak is first-class here — the prior server soak surfaced executor-deadlock and routing gaps ([[project_soak_test_gaps]]), so we exercise the fleet under sustained load, not just in unit/integration tests.

## Acceptance Criteria **[REQUIRED]**

- [ ] New `angreal test soak` fleet variant: server + N agents under sustained load, watching for roster leaks, stuck in-flight/outbox entries, and reconciliation drift over time.
- [ ] Soak stable over the target duration with no leaked work, no roster drift, flat outbox depth.
- [ ] Diataxis docs: how-to (deploy an agent, route a glob to the fleet, operate it), explanation (where the fleet sits in scaling), reference (agent config/flags, metrics).
- [ ] Optional: Helm chart agent deployment (server in DB trust zone; agents pointed at it with API key + tenant).
- [ ] Metrics doc updated with fleet/agent + outbox signals; scrape validated.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Extend the existing soak harness (`angreal test soak server`) to stand up agents. Docs follow the established Diataxis structure from the I-0112 refresh. Helm work builds on the existing chart (`angreal helm`).

### Dependencies
[[CLOACI-T-0634]] (churn/saturation correctness must land before soak is meaningful). Closes [[CLOACI-I-0114]].

### Risk Considerations
- Soak is where slow leaks (roster entries, outbox rows, file-cache growth on agents) surface — instrument before running, not after.
- Use fresh DBs per soak run ([[feedback_stale_db_testing]]).

## Status Updates **[REQUIRED]**

*To be added during implementation*
