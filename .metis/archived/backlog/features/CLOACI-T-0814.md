---
id: k8s-actuator-helm-rbac-per-tenant
level: task
title: "K8s actuator + Helm RBAC + per-tenant namespace (fast-follow)"
short_code: "CLOACI-T-0814"
created_at: 2026-06-27T14:43:41.700850+00:00
updated_at: 2026-06-27T18:04:06.625514+00:00
parent: CLOACI-I-0127
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# K8s actuator + Helm RBAC + per-tenant namespace (fast-follow)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Fast-follow production actuator: the Kubernetes `FleetActuator` impl (scale per-tenant agent Deployment replicas in the tenant's own namespace) + the Helm RBAC + per-tenant namespace. Same `FleetActuator` trait as the Docker dev actuator (T-0810).

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

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] K8s actuator scales a tenant agent pool on a real cluster (Deployment replicas) in the tenant's OWN namespace (REQ-007).
- [ ] Helm chart provisions the actuator RBAC: ServiceAccount + Role/RoleBinding to create/scale/delete agent workloads in tenant namespaces, least-privilege, NO cluster-admin (REQ-006).
- [ ] Substrate guard (REQ-008): the K8s actuator refuses to start when NOT in-cluster (no SA / API unreachable); fail-closed.

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

### 2026-06-27 — Implemented (local validation green; not committed)

**Kubernetes FleetActuator** (`crates/cloacina-server/src/actuator/kubernetes.rs`): mirrors `docker.rs` exactly — a mockable `KubeOps` trait (`ensure_namespace`, `ensure_secret`, `ensure_deployment`, `scale_deployment`, `count_ready_replicas`) with a real `KubeApiOps` impl (server-side apply for ns/secret/deployment; merge-patch for replicas; `get_opt` for ready count) + a test mock. Reuses the T-0810 `KeyMinter`/`DalKeyMinter`, `ReconcileOutcome`, `ActuatorError`. `reconcile` ensures the tenant namespace `cloacina-tenant-<sanitized>` (REQ-007), upserts a per-tenant `Secret` (agent key, referenced as `CLOACINA_API_KEY`), ensures the `cloacina-agent` Deployment, and scales replicas = desired; spawned/stopped computed from the ready-replica delta. Client construction is in-cluster + lazy (`Config::incluster`), sync — mirrors Docker's lazy connect.

**Guard wiring** (`actuator/guard.rs`): added `Decision::Kubernetes`; `kubernetes` arm now → in-cluster builds the actuator, NOT in-cluster → `Refused` (fail-closed). Removed the dead `NotImplemented` variant. `build_actuator` refactored to delegate to a new injectable `build_actuator_with(kind, substrate, build_docker, build_kubernetes)` seam so K8s/Docker construction is mockable in tests without a real cluster/daemon.

**Helm** (`charts/cloacina-server`): values-gated on `fleet.actuator` (default `none` → existing installs byte-unchanged). New `templates/fleet-rbac.yaml` (rendered only under `eq .Values.fleet.actuator "kubernetes"`): ServiceAccount + least-privilege ClusterRole (namespaces [create,get,list]; apps/deployments [create,get,list,patch,delete]; secrets [create,get,update,delete] — no `list` on secrets, no cluster-admin, no wildcards) + ClusterRoleBinding. Server Deployment binds the SA and sets `CLOACINA_FLEET_ACTUATOR=kubernetes` + `CLOACINA_AGENT_IMAGE` + `CLOACINA_AGENT_SERVER_URL` (defaults to release Service DNS), all under the same gate.

**Deps**: `kube = 2.0` (resolved 2.0.1, `default-features=false`, features `client`+`rustls-tls`) + `k8s-openapi = 0.26` (resolved 0.26.1, feature `v1_32`).

**Validation**: `angreal check crate crates/cloacina-server` green; `cargo test -p cloacina-server --lib -- actuator --test-threads=1` → 33 passed / 0 failed (20 existing + 13 new). `helm lint` clean; `helm template --set fleet.actuator=kubernetes` renders SA + ClusterRole + ClusterRoleBinding + actuator env + SA binding; default `helm template` renders zero fleet artifacts (unchanged).

**Not validated locally (needs a real cluster)**: actual namespace/Deployment creation + agent self-registration. Smoke test (kind/minikube): install with `--set fleet.actuator=kubernetes`, `POST /v1/tenants/{t}/fleet/provision`, watch a `cloacina-agent` Deployment scale up in `cloacina-tenant-<t>` and agents appear via `GET /v1/agents`.

**Flagged decisions**: single per-tenant Secret shared by all replicas (re-minted on scale-UP events, not every reconcile); key revocation on scale-down deferred (same as Docker). Not committed/pushed per instructions.