---
id: pluggable-fleetactuator-trait
level: task
title: "Pluggable FleetActuator trait + Docker-container dev actuator (+ substrate guard)"
short_code: "CLOACI-T-0810"
created_at: 2026-06-27T14:43:36.899333+00:00
updated_at: 2026-06-27T17:14:05.488598+00:00
parent: CLOACI-I-0127
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Pluggable FleetActuator trait + Docker-container dev actuator (+ substrate guard)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

The pluggable `FleetActuator` trait (reconcile desired->actual) + the Docker-container dev actuator (slice 1 #3). The Docker impl (bollard / Docker API) runs N tenant-keyed `cloacina-agent` containers labelled `cloacina.tenant=<t>` and stops the surplus; the spawned agent self-registers via its injected per-tenant `CLOACINA_API_KEY`. First pass = containers only (no local-process, no K8s).

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

## Acceptance Criteria **[REQUIRED]**

- [ ] Bumping `desired_count` -> a container's agent self-registers into the tenant pool on the compose stack; lowering it drains/stops one; reconcile = count tenant-labelled containers vs desired, actuate the delta.
- [ ] Cross-scope-denial: the actuator never spawns or targets another tenant's containers (NFR-004).
- [ ] Substrate guard (REQ-008): actuator selection is explicit (`CLOACINA_FLEET_ACTUATOR`) and validated at boot, fail-closed — the Docker actuator REFUSES to start when Kubernetes is detected (`KUBERNETES_SERVICE_HOST` / SA mount) and refuses with no Docker socket; loud boot error, never silent wrong-scaling.

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

### 2026-06-27 — Implemented (not committed)

Pluggable `FleetActuator` + Docker dev actuator + fail-closed substrate guard landed on `i0127-agent-control-plane`. Compile + unit tests green locally; the real Docker spawn path is implemented but cannot be CI-tested (needs a live daemon + `cloacina-agent` image) — flagged below.

**New module `crates/cloacina-server/src/actuator/`:**
- `mod.rs` — `#[async_trait] FleetActuator` trait (`reconcile`, `kind`), `ReconcileOutcome`, `ActuatorError`, `NoopActuator` (kind `"none"`), and pure `reconcile_delta(running, desired) -> ReconcileDelta` (saturating).
- `guard.rs` — REQ-008 substrate guard. `Substrate` trait (injectable K8s / Docker-socket probes) + `HostSubstrate`; pure `evaluate(kind, &dyn Substrate)`; `build_actuator(...)`; `actuator_kind_from_env()`. Branches: none→Noop, docker+K8s→Refused, docker+no-socket→Unavailable, docker+socket→Docker, kubernetes+not-in-cluster→Refused, kubernetes+in-cluster→NotImplemented (T-0814), unknown→Unknown.
- `docker.rs` — `DockerActuator` over `ContainerOps` (bollard) + `KeyMinter` traits (both mockable). `BollardOps` (bollard 0.21) does label-scoped list / create+start / stop+remove. `DalKeyMinter` mints a tenant-scoped `read` key per spawn.

**Wiring (`lib.rs`):** `pub mod actuator;`; `AppState.fleet_actuator: Arc<dyn FleetActuator>` (added to runtime + `test_state`); fail-closed `build_actuator` at boot (server refuses to start on guard error); reconcile loop spawned only when `kind != "none"` (interval `CLOACINA_RECONCILE_INTERVAL_S`, default 15) with the T-0811 leader-election TODO.

**Additive DAL (`crates/cloacina/src/dal/unified/agent_desired/mod.rs`):** `list_all() -> Vec<(tenant, u32)>` for the reconcile loop (read-only; T-0809 logic untouched).

**Tests:** 20 passing (`cargo test -p cloacina-server --lib -- actuator`) — guard 8, delta 5, docker mock-reconcile 6, noop 1. `angreal check crate crates/cloacina-server` green.

**Deps:** `bollard = "0.21"` added; `async-trait` already present.

**Agent-key role = `read`:** `routes/authz.rs` authorizes `POST /agent/register` at `Access::any(Level::Read)`, and `register_agent` registers the agent under the caller key's `tenant_id` — so a tenant-scoped `read` key is the minimal self-registration credential.

**Flagged decisions / cannot-validate:**
- Mint one fresh key per spawned container; key-reuse + revoke-on-stop deferred → keys accumulate in `api_keys` (future cleanup task).
- `list_managed` counts running containers only (`all=false`).
- Reconcile loop is single-writer; multi-replica needs leader election (T-0811) — TODO in place.
- Real Docker spawn NOT validated locally. Smoke-test on compose stack: build `cloacina-agent:latest`; set on server `CLOACINA_FLEET_ACTUATOR=docker`, `CLOACINA_AGENT_NETWORK=<compose net>`, `CLOACINA_AGENT_SERVER_URL=http://server:8080`, mount `/var/run/docker.sock`; `POST /v1/tenants/{t}/fleet/provision`; within the reconcile interval a labelled `cloacina-agent` container starts and self-registers (`GET /v1/agents`); deprovision drains one.

NOT committed / pushed per task instruction.