---
id: hardening-fleet-soak-variant
level: task
title: "Hardening — fleet soak variant, Diataxis docs, optional Helm agent deployment"
short_code: "CLOACI-T-0635"
created_at: 2026-05-27T17:36:35.740756+00:00
updated_at: 2026-06-08T13:12:06.413157+00:00
parent: CLOACI-I-0114
blocked_by: [CLOACI-T-0634]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0114
---

# Hardening — fleet soak variant, Diataxis docs, optional Helm agent deployment

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0114]].

## Objective **[REQUIRED]**

Make the fleet operable and durable over time. Add a sustained-load soak that runs a multi-agent fleet, ship the Diataxis docs for deploying/operating it, and (optionally) add a Helm agent deployment. Soak is first-class here — the prior server soak surfaced executor-deadlock and routing gaps ([[project_soak_test_gaps]]), so we exercise the fleet under sustained load, not just in unit/integration tests.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] New `angreal test soak` fleet variant: server + N agents under sustained load, watching for roster leaks, stuck in-flight/outbox entries, and reconciliation drift over time. **Authored 2026-06-08** (`angreal test soak fleet`).
- [x] Soak stable over the target duration with no leaked work, no roster drift, flat outbox depth. **Confirmed 2026-06-08** — first run (229 execs) passed all stability checks; reworked to apply real sustained load (slow workflow + closed-loop saturation, peak ~12 in-flight) and re-run looked good (operator-confirmed).
- [x] Diataxis docs: how-to (deploy an agent, route a glob to the fleet, operate it), explanation (where the fleet sits in scaling), reference (agent config/flags, metrics). **Done 2026-06-07.**
- [x] Optional: Helm chart agent deployment (server in DB trust zone; agents pointed at it with API key + tenant). **Done 2026-06-08** (`charts/cloacina-agent`).
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

### 2026-06-08 — soak harness + agent Helm chart DONE

**`charts/cloacina-agent`** — Helm chart for the DB-less agent fleet. Deployment
(no Service/ingress/HTTP probes — outbound worker), `_helpers.tpl`, optional
inline-`apiKey` Secret vs BYO `apiKeySecretRef`, fail-closed validation
(requires `server.url` + a key), readOnlyRootFilesystem + writable home/tmp
emptyDirs, README + NOTES. `helm lint` + template (both key paths) + fail-closed
all pass; wired into `angreal helm lint` (server + agent).

**`angreal test soak fleet`** (`.angreal/test/soak/fleet.py`) — boots Postgres +
server (`**=fleet`) + compiler + N agents (host subprocesses, reusing the e2e
boot helpers), compiles the fixture, warm-up proves the fleet path, then drives
sustained load for `DURATION_S` while sampling `/metrics`. Stability gate at the
end: every submitted execution completed (no lost work), `active_tasks/_workflows`
+ `delivery_outbox_open` drain to 0 (no stuck in-flight/outbox), `fleet_agents_
evicted_total` == 0 (no roster drift), `fleet_work_reassigned_total` == 0, all
agent procs alive. Tunable via `CLOACINA_SOAK_FLEET_*` env. Registered;
py_compile + ruff clean. **Needs one real run** (full stack, a few minutes) to
tick the "stable over duration" criterion — same as how the e2e is run.

Committing both into a T-0635 PR off main. With these, T-0635 is complete pending
the soak run; that closes I-0114.

### 2026-06-08 — soak reworked to real load + merged; T-0635 complete

The first soak run passed every stability check but didn't actually load the
fleet (noop completed between samples → active=0, outbox=0). Reworked: run the
`fleet-slow-rust` workload (per-task sleep) and submit closed-loop to hold
~`TARGET_INFLIGHT` outstanding, keeping the fleet saturated for the whole run;
track in-flight via `active_workflows` (fleet tasks don't surface as Running at
the task level); added a no-load guard (peak active_workflows ≥ agents×conc).
Operator confirmed it now applies genuine sustained load.

PR #119 squash-merged to main (57090485): `charts/cloacina-agent` + the fleet
soak. All acceptance criteria met → T-0635 done. This was the last open task
under I-0114 ("Closes I-0114") — initiative closure pending human sign-off.