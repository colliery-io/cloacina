---
id: demo-fleet-actuator-variant-docker
level: task
title: "Demo fleet-actuator variant (Docker) + fleet control-plane soak platform"
short_code: "CLOACI-T-0816"
created_at: 2026-06-27T23:32:13.024183+00:00
updated_at: 2026-06-28T00:23:52.095896+00:00
parent: CLOACI-I-0127
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Demo fleet-actuator variant (Docker) + fleet control-plane soak platform

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

{Clear statement of what this task accomplishes}

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

### 2026-06-27 — Started (research complete)
Branch: i0127-agent-control-plane. Docker up. Researched the actuator + demo + soak idioms.

Key findings driving the design:
- Docker actuator (`actuator/docker.rs`) spawns `cloacina-agent` containers labelled
  `cloacina.managed=true` / `cloacina.tenant=<t>`, injecting `CLOACINA_SERVER` +
  `CLOACINA_API_KEY` env (agent clap supports `env=` for both — verified). Config from
  `CLOACINA_AGENT_IMAGE` / `CLOACINA_AGENT_NETWORK` / `CLOACINA_AGENT_SERVER_URL`.
- Guard (`guard.rs`): docker decision needs k8s NOT detected + `/var/run/docker.sock`
  reachable. Demo has neither k8s signal → mounting the socket → Decision::Docker.
- Control loop (lib.rs ~1091): runs only when actuator kind != none; leader-gated;
  (a) autoscale desired_count from utilization, (b) reconcile actual→desired via actuator.
  Tenant set = tenants with a desired_count row OR live agents. `CLOACINA_AUTOSCALE`
  kill-switch leaves reconcile running. Floor via `CLOACINA_AUTOSCALE_FLOOR` (default 0).
- Auto-provision: tenant CREATE sets desired_count=`CLOACINA_INITIAL_AGENTS` (default 1,
  clamped to max). The demo's `acme` tenant is created by harness-acme → auto-provisions.
  BUT `public` pre-exists (not created via POST /v1/tenants) → must be provisioned
  explicitly via `POST /v1/tenants/public/fleet/provision` to enter the tenant set.
- Fleet API: `GET /v1/tenants/{t}/fleet` → desired_count/actual_count/effective_limit;
  `POST .../fleet/provision` (+1), `.../fleet/deprovision` (-1). `GET /v1/agents` roster.
- Metrics: `cloacina_fleet_agents_evicted_total`, `cloacina_fleet_work_reassigned_total`.
  api_keys table in `public` schema; minted keys named `agent:<tenant>:<uuid>` (count via psql).

Design decisions:
- Override `docker/docker-compose.demo.fleet.yml`: sets project `name` + renames the
  default network to a FIXED name so actuator-spawned (out-of-project) agents resolve the
  `server` alias. Server gets actuator=docker + socket mount + agent env. Static `agent`/
  `agent-acme` set to `deploy.replicas: 0` (image still built so the actuator can spawn it).
- Keep AUTOSCALE on (the showcase) with FLOOR=1 so a provisioned realm doesn't drop to 0.
- angreal `ui fleet-up`/`fleet-down` brings it up + provisions public/acme.
- Soak `.angreal/test/soak/fleet_actuator.py` → `angreal test soak fleet-actuator`.

### 2026-06-27 — DELIVERABLE 1 VALIDATED (Docker actuator real-spawn proof)
Brought up `docker compose -f demo.yml -f demo.fleet.yml` (angreal `ui fleet-up`).
Had to fix ONE thing: the demo server runs as non-root uid 10001 but
`/var/run/docker.sock` is root:root 0660 → bollard `client error (Connect)`. Added
`user: "0:0"` to server in the override (dev-only; documented). After that:
- (a) `fleet actuator initialized actuator=docker` + `fleet control loop started`. PASS
- (b) actuator spawned `cloacina.tenant`-labelled containers (`docker ps`): acme + public,
  image `cloacina-agent:demo-fleet`. Logs: `spawned agent container ... tenant=acme/public`. PASS
- (c) `GET /v1/agents` → 2 agents, tenant_id=acme + tenant_id=public, self-registered. PASS
- (d) acme_billing + acme_payroll EXECUTED on the actuator agent: `agent reported result
  ... tenant_id=acme ... outcome=success`. PASS (tenant-scoped path)
- (e) actuator `stopped + removed agent container` on scale-down (public 2→1, stopped=1). PASS

GAP (surfaced): the demo's `public`/cron workload dispatches in the NULL realm
(`tenant None`); the actuator mints TENANT-SCOPED keys, so its public agent registers
as tenant_id="public" and can't serve null-realm tasks → `no available fleet agent in
tenant None`. Named tenants (acme) work end-to-end; the null/default realm (which the
static bootstrap-key `agent` used to serve) is NOT covered by the per-tenant actuator.
Remediation (out of scope): seed "public" as a real tenant, or add null-realm
provisioning, or retain one bootstrap agent for the default+cron realm.

Soak design note: stopping a container leaves a stale heartbeat → the sweeper evicts it,
so `cloacina_fleet_agents_evicted_total` legitimately rises with deprovision churn. The
meaningful drift check is per-tenant: a STEADY loaded fleet's actual_count never drops.
Soak: steady acme fleet under acme_billing load (no false eviction) + churn a scratch
tenant for convergence/key-growth. Soak override sets AUTOSCALE=false, FLOOR=0,
INTERVAL_S=10 for deterministic desired→actual convergence.

### 2026-06-27 — DELIVERABLE 2 VALIDATED + cleanup. BOTH DONE.
`angreal test soak fleet-actuator --duration 60 --no-build` → PASS. Observed:
- 222 acme_billing executions over 60s (real fleet workflow load on steady acme=2).
- steady-fleet drift samples = 0 (acme stayed desired=2/actual=2 the whole run — no
  false eviction of a loaded healthy fleet).
- convergence: managed(acme+churn)=2 == sum_desired=2 after settle (churn cycled
  0→2→0→2→0, peak 5 total managed, fully reaped down — no leaked/orphaned containers).
- work_reassigned_total delta = 0; /ready failures = 0.
- agent_keys 25→29 (+4 == ~1.00/spawn) — bounded, exactly the flagged per-spawn
  DalKeyMinter tradeoff (keys not reclaimed on stop); reported, not failed.
Three iterations to get the harness right (all surfaced real edges): (1) ANSI color
codes split the `actuator=docker` field → check `fleet control loop started` instead;
(2) convergence mis-scoped — leftover `public` desired rows in the reused DB volume
kept a public agent up, so scope managed-count to the soak's own tenants; (3) the
execute endpoint needs a JSON context body (submits were 0 without it).

Files: `docker/docker-compose.demo.fleet.yml` (new), `.angreal/test/soak/fleet_actuator.py`
(new), `.angreal/task_ui.py` (+`ui fleet-up`/`fleet-down`), `.angreal/test/__init__.py`
(register soak module). Override adds `user: "0:0"` to server for socket access (dev-only).

Cleanup DONE: `docker compose -f demo.yml -f demo.fleet.yml down -v` + reaped all
`cloacina.managed` containers (0 remain); 19GB free. Also brought down a PRE-EXISTING
base demo stack (project `docker`, 32h up) at the start to free ports 8080/8082.
Left the pre-existing k8s-e2e cluster (not mine) alone. NOT committed/pushed.
Cargo.lock got the actuator deps (bollard/kube) materialized by a session tool — left as-is.

GAP recap (the one thing to flag): pure-actuator can't serve the demo's NULL/default
realm (`public` + server cron triggers) — only named tenants (acme works end-to-end).
A coherent public dashboard would need public seeded as a real tenant, or a retained
bootstrap-key agent for the null realm, or null-realm provisioning support. Out of scope.