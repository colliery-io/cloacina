---
id: leader-election-adr-multi-replica
level: task
title: "Leader-election ADR + multi-replica validation (scheduler claiming + single-writer fleet loop)"
short_code: "CLOACI-T-0818"
created_at: 2026-06-28T02:02:39.919445+00:00
updated_at: 2026-06-28T04:30:26.519451+00:00
parent: CLOACI-I-0127
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Leader-election ADR + multi-replica validation (scheduler claiming + single-writer fleet loop)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Record the leader-election design as an ADR and validate the multi-replica server path.

**Decision (ADR):** the fleet control loop (autoscaler + reconcile, T-0811) is leader-gated via a Postgres advisory lock because it is an irreducibly-singleton, global per-tenant control action with no per-task claim granularity. The API (stateless) and the **scheduler/dispatch** (per-task DB claiming — N schedulers claim disjoint Ready tasks, no double-dispatch) scale freely across replicas; only the fleet loop is serialized. The server is a **deployment-layer** service — embedded is the *library* layer only and runs no fleet loop — so the leader is the **HA enabler** for a multi-replica server, not embedded-driven. Chose (A) in-process loop + advisory lock (one deployment, self-gating) over (B) a separate singleton fleet-controller deployment.

**Validation (the gap):** all of this only ever runs at 1 replica today (the T-0815 k8s soak showed `advisory_holder=null`). Stand up a **2-replica** server soak and prove: (1) two schedulers claim disjoint work — no double-dispatch/re-run; (2) exactly one replica holds the fleet lock — provisioning/autoscaling stays single-writer (no double-provision, no desired_count races); (3) failover — kill the leader, another acquires the lock and the loop resumes. Make the chart's `replicas` a documented, supported multi-replica story. Surfaced by [[CLOACI-I-0127]].

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

### 2026-06-28 — Multi-replica validation built + run green (ADR CLOACI-A-0008)

Built the multi-replica validation as a functional e2e lane: **`angreal test e2e k8s-leader`**
(`.angreal/test/e2e/k8s_leader.py`, registered in the e2e group via
`.angreal/test/__init__.py`). It reuses the T-0815/T-0816 k3s platform verbatim
(`bring_up_cluster`, `_prepare_images`, `start_port_forward`, `_server_values`,
`docker-compose.k8s.yaml`) and deploys the chart at **`replicaCount=2`** with
`fleet.actuator=kubernetes` on a distinct compose project + port-forward 18096
(no clash with e2e 18092 / soak 18094). Control tick set to 1s for observability.

**Validated with a REAL run** (`angreal test e2e k8s-leader --skip-build`, reusing the
retained `localhost:5050/cloacina-{server,agent}:k8s-soak` images). Result: **4/5 green,
0 failures**, platform torn down, exit 0.

- **[PASS] 1 — both replicas Ready.** `readyReplicas==2`, 2 server pods Running,
  `/ready` healthy through the Service.
- **[PASS] 2 — single leader.** Sampled the fleet advisory lock (key **8110127**) at
  ~10 Hz for 60s via `kubectl exec` into the postgresql pod. **Exact query:**
  `SELECT a.client_addr, a.pid FROM pg_locks l JOIN pg_stat_activity a ON l.pid=a.pid WHERE l.locktype='advisory' AND l.objid=8110127 AND l.granted;`
  Observed the lock held in 14 samples; **max simultaneous holders = 1**; `client_addr`
  mapped (via `kubectl get pods -o wide`) to the two server pods **evenly (7/7)** —
  confirming the per-tick election the ADR describes ("exactly one replica leads per
  tick"): leadership legitimately alternates tick-to-tick, never two at once.
- **[PASS] 3 — single-writer provisioning.** Created tenant `acme`, set its limit,
  provisioned N=3 via REST → the (leader-only) reconcile scaled the tenant
  `cloacina-agent` Deployment to **exactly 3** (one Deployment, not 2×N). Deprovision → 0.
- **[BLOCKED] 4 — disjoint claiming.** NOT validated end-to-end: a helm-only
  cloacina-server deploy ships **no compiler** (`charts/cloacina-server` has no compiler
  template), so uploaded source `.cloacina` packages stay `build_status='pending'` and
  never execute; the dist fixtures also carry host-absolute path-deps the in-cluster
  compiler does not rewrite. The property itself is enforced in cloacina-core by the
  `task_outbox` claim `DELETE … FOR UPDATE SKIP LOCKED` + `claimed_by` CAS
  (`crates/cloacina/src/dal/unified/task_execution/claiming.rs`), and BOTH replicas run
  the per-tenant scheduler unconditionally (NOT leader-gated:
  `services.rs` "Always: per-runner task scheduler"; global runner per replica at
  `lib.rs:689`). An opt-in `--claiming` path (deploy a compiler, upload, await build,
  drive M executions, assert no `(workflow_execution_id, task_name)` has >1
  `task_executions` row) is scaffolded for when a matching-ABI compiler is available.
- **[PASS] 5 — failover.** Caught the lock holder, `kubectl delete pod` on it, then polled
  the lock query until a **different** pod acquired it. Hand-off observed:
  old holder `…-lh44g` (pid 120) → killed → re-acquired by `…-ztbgp` (pid 1412, addr
  10.42.0.40); provisioning still worked under the new leader (scaled to 3); the killed
  replica rescheduled and rejoined → **2/2 Ready**, lock holders stayed ≤1. (First run
  exposed a harness bug — the `port-forward svc/…` pins to one pod and died with the
  killed leader; fixed by re-establishing the forward after the kill.)

Run command: `angreal test e2e k8s-leader --skip-build` (flags: `--no-cleanup`,
`--claiming`, `--agents N`, `--tag`). Default tears the platform down.

Leaving task **active**: assertions 1/2/3/5 (the ADR A-0008 leadership core) are proven on
a real 2-replica cluster; assertion 4 remains the one gap (needs an in-cluster compiler).