---
id: k3s-functional-test-harness-full
level: task
title: "k3s functional-test harness — full e2e for the Kubernetes fleet actuator"
short_code: "CLOACI-T-0815"
created_at: 2026-06-27T20:59:15.835327+00:00
updated_at: 2026-06-27T21:49:55.327486+00:00
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

# k3s functional-test harness — full e2e for the Kubernetes fleet actuator

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

### 2026-06-27 — Harness built; stale-image trap found; fresh-build run in progress

**Delivered** (mirrors brokkr's k3s-in-docker-compose pattern, fits cloacina `.angreal` idioms):
- `.angreal/files/docker-compose.k8s.yaml` — single-node k3s (`rancher/k3s:v1.30.10-k3s1`, traefik/servicelb/metrics-server disabled) + `registry:2` + kubeconfig-translation (host `localhost:6443`, docker-net `k3s:6443`) copied to a per-run host dir.
- `.angreal/files/k3s-registries.yaml` — mirrors `registry:5000` (+ localhost spellings) so the cluster pulls locally-built images.
- `.angreal/test/e2e/k8s_fleet.py` — angreal command `angreal test e2e k8s-fleet` (`--no-cleanup`, `--skip-build`, `--agents N`, `--tag`). Orchestrates: k3s up → build/push images → helm install (`fleet.actuator=kubernetes`, agent image = local registry) → port-forward → 5 assertions → teardown. Registered in `.angreal/test/__init__.py`.

**Wiring**: server runs IN-cluster via helm, bound to the chart's fleet SA (`<release>-cloacina-server-fleet`) so the REAL `templates/fleet-rbac.yaml` ClusterRole is exercised. Bundled postgres (persistence off). extraEnv: `CLOACINA_AUTOSCALE=false` (keep reconcile, kill util-autoscale so floor=0 doesn't undo our provision), `CLOACINA_AUTOSCALE_INTERVAL_S=5`, `CLOACINA_INITIAL_AGENTS=0`. Bootstrap key via a pre-created Secret → `apiKeySecretRef`. API reached by `kubectl port-forward`; cluster state asserted with host `kubectl`.

**Two harness bugs found+fixed during validation:**
1. `docker compose up --wait` fails because the one-shot kubeconfig containers exit(0) and `--wait` treats any exit as failure → split: `--wait` only on registry+k3s, bring up the one-shots without `--wait`, poll for the kubeconfig file.
2. **STALE IMAGE TRAP (key finding):** the running demo stack's `docker-server:latest` was built 2026-06-26 from `docker/Dockerfile.demo`, but the actuator + substrate guard landed 2026-06-27 (T-0810 `5562ea09`, T-0814 `57c65509`). The stale binary has ZERO actuator code (`strings | grep 'fleet actuator initialized'` = 0) — server booted fine but NEVER initialized the actuator and ignored `CLOACINA_BOOTSTRAP_KEY` (env wiring is also newer, main.rs:49). So `--skip-build` reuse of demo images is INVALID for the server. Fixed the build path: server ALWAYS builds fresh from the root `Dockerfile` (cloacina-server bin only = fastest correct recipe); the stable agent image is reused when present.

**Verified live before fix**: pod env had `CLOACINA_FLEET_ACTUATOR=kubernetes`, `CLOACINA_AGENT_IMAGE=registry:5000/cloacina-agent:...`, and `serviceAccountName=fleet-e2e-cloacina-server-fleet` — chart plumbing is correct; only the binary was stale.

**Now running**: fresh-build run (`angreal test e2e k8s-fleet --no-cleanup --agents 2`) — cold cloacina-server release build in progress. Results to follow.

### 2026-06-27 — FINAL: all 5 assertions GREEN (real chart RBAC exercised + passes)

Validated end-to-end against live k3s with the freshly-built server (`cloacina-server:k8s-e2e` id `54a0c969edd1`, contains T-0810/T-0814 actuator code), bound to the chart's fleet ServiceAccount.

- **[1] GREEN** server in-cluster + actuator init + `/ready`: logs `fleet actuator initialized actuator=kubernetes` and `fleet control loop started (leader-gated) actuator=kubernetes interval_s=5 autoscale_enabled=false`. Substrate guard passed in-cluster.
- **[2] GREEN** auth + tenant + provision: bootstrap key (from `CLOACINA_BOOTSTRAP_KEY`) authed; `POST /v1/tenants` (acme)=201; `POST /v1/tenants/acme/limits {max_agents:5}`; `provision`×2 → `desired_count=2`.
- **[3] GREEN — THE RBAC PROOF**: under the chart's fleet ClusterRole the actuator created `namespace/cloacina-tenant-acme` + `deployment.apps/cloacina-agent` (replicas=2) + `secret/cloacina-agent-key` (key `api-key`). Server log: `fleet actuator reconciled tenant agents (kubernetes) tenant=acme namespace=cloacina-tenant-acme desired=2 spawned=2`. **ZERO RBAC denials** (no `forbidden`/`cannot create`/`cannot patch`). The fixed `create`/`patch` verbs on namespaces/deployments/secrets WORK against a real API server.
- **[4] GREEN (after a real-bug fix)** agents self-register: initially BLOCKED — agents Running but `agent register failed`. Root-caused: the actuator places agents in `cloacina-tenant-<t>`, but the chart's default `fleet.agentServerUrl` uses the SHORT service name (`http://<fullname>:<port>`), which is **NXDOMAIN from a tenant namespace** (verified: short name fails, FQDN resolves). The minted key + register endpoint themselves are fine (manual replay of `POST /v1/agent/register` with the minted key = 200). Fixed by pinning the cross-namespace FQDN `http://<fullname>.<release-ns>.svc.cluster.local:8080` → agents log `agent registered with server` + `substrate WS connected`; `GET /v1/agents` shows 2 acme agents.
- **[5] GREEN** deprovision×2 → `desired_count=0` and `cloacina-agent` Deployment replicas→0.

**FLAGGED (product finding, not changed):** the chart default `fleet.agentServerUrl` should be the FQDN, not the short service name — otherwise actuator-spawned agents in tenant namespaces can never reach the server. Currently worked around in the harness via a values override; recommend fixing `charts/cloacina-server/values.yaml`/`deployment.yaml` default to render the FQDN.

**Infra note:** hit k3s `DiskPressure` (Docker VM ~95% full) mid-run → mass pod eviction; reclaimed via `docker builder prune`. Harness hardening rec: pre-flight a disk check + raise helm `--wait` timeout / add a postgres-ready gate (server crash-loops on DB-not-ready, which is slow under pressure).

**Harness fixes landed this session:** server always builds fresh (root Dockerfile); `--skip-build` reuses harness-tagged `localhost:5050/...:tag` images (not the stale demo image); FQDN `agentServerUrl` baked into values; agent-pod label selector corrected to `app.kubernetes.io/name=cloacina-agent`.

**Run it:** `angreal test e2e k8s-fleet` (add `--no-cleanup`, `--skip-build` after a first build, `--agents N`). Cluster left up this run; teardown: `docker compose -f .angreal/files/docker-compose.k8s.yaml -p <project> down -v`.

**Files:** `.angreal/files/docker-compose.k8s.yaml`, `.angreal/files/k3s-registries.yaml`, `.angreal/test/e2e/k8s_fleet.py`, +1 line in `.angreal/test/__init__.py`. No chart/actuator logic changed.

**CI-lane shape (proposed):** a dedicated nightly/manual `k8s-fleet-e2e` job (not the PR fast lane — needs Docker-in-Docker/privileged k3s + a ~25-min cold server build). Steps: checkout → `docker build` server (cache the cargo layer via buildx registry cache to cut the cold build) → `angreal test e2e k8s-fleet` (cleanup on). Gate on assertions 1-3 (RBAC proof) as required; 4-5 as required once the chart FQDN default is fixed. Pre-flight disk guard to avoid the DiskPressure flake.