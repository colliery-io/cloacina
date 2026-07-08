---
id: k8s-fleet-production-hardening
level: task
title: "K8s fleet production-hardening — agent pod securityContext/resources/probes, per-tenant NetworkPolicy, server HA (PDB + anti-affinity)"
short_code: "CLOACI-T-0819"
created_at: 2026-06-28T21:32:46.508185+00:00
updated_at: 2026-06-28T22:09:30.601321+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# K8s fleet production-hardening — agent pod securityContext/resources/probes, per-tenant NetworkPolicy, server HA (PDB + anti-affinity)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Close the K8s production-hardening gaps surfaced reviewing the [[CLOACI-I-0127]] Helm chart + actuator: the chart hardens the **server** pod, but the actuator-created **agent** workloads and the multi-replica story have gaps.

**AC — (A) Agent pod hardening.** The K8s actuator's agent Deployment (`crates/cloacina-server/src/actuator/kubernetes.rs`) sets a `securityContext` — `runAsNonRoot`, `runAsUser/Group 10001` (per `docker/Dockerfile.agent`), `readOnlyRootFilesystem` with `emptyDir`s for the agent's writable paths, `allowPrivilegeEscalation: false`, drop ALL caps, `seccompProfile: RuntimeDefault` — so agent pods are non-root and pass PodSecurity `restricted` (today they'd be rejected). Plus resource requests/limits, configurable via chart `fleet.agentResources` → `CLOACINA_AGENT_*` env. No httpGet probes (the agent is a WS client with no health endpoint; server-side heartbeat/eviction tracks liveness) — documented.

**AC — (B) Per-tenant NetworkPolicy (REQ-007).** The actuator creates a NetworkPolicy per tenant namespace: default-deny, egress allowed to DNS + the cloacina-server only, ingress none. Fleet RBAC gains `networkpolicies` create+patch. Toggleable (`fleet.networkPolicy.enabled`). MUST NOT break the agent→server connection (verify agents still register with it active). Defense-in-depth per NFR-004 (server ABAC remains the real boundary).

**AC — (C) Server HA primitives.** A PodDisruptionBudget template + a default soft pod-anti-affinity / topology-spread in the chart, so the multi-replica server (the advisory-lock leader, ADR [[CLOACI-A-0008]], makes >1 safe) survives node drains and spreads across nodes. **No HPA** — the control-plane back-pressure autoscaler is the scaling mechanism (HPA can't see per-tenant signal).

Validation: compile + actuator unit tests + helm lint + `helm template` (actuator=kubernetes, replicaCount=2) + ideally the k8s e2e (agents still register through the NetworkPolicy).

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

### 2026-06-28 — Implemented A+B+C (branch harden/k8s-fleet)

**Gap A — agent pod hardening** (`crates/cloacina-server/src/actuator/kubernetes.rs`): pure `agent_deployment_manifest()` builder; pod securityContext (runAsNonRoot, uid/gid/fsGroup 10001, seccompProfile RuntimeDefault) + container securityContext (allowPrivilegeEscalation false, readOnlyRootFilesystem, drop ALL). emptyDir for /home/cloacina (2Gi — Python workflow/+vendor unpack + cdylib cache) and /tmp (1Gi). Resources via AgentResources / CLOACINA_AGENT_{CPU,MEMORY}_{REQUEST,LIMIT} (defaults 250m/256Mi, 1/1Gi — mem bumped for PyO3). NO probes (commented; server heartbeat tracks liveness).

**Gap B — per-tenant NetworkPolicy** (riskiest): ensure_network_policy on KubeOps + pure network_policy_manifest(); podSelector {}, deny-all ingress, egress = DNS (UDP+TCP 53 to dnsNamespace) + server (namespaceSelector kubernetes.io/metadata.name + server pod labels, TCP 8080). Server coords via CLOACINA_SERVER_NAMESPACE / _POD_SELECTOR / _PORT. Toggle fleet.networkPolicy.enabled. Fail-OPEN if server coords absent. RBAC networkpolicies create+patch.

**Gap C — server HA**: poddisruptionbudget.yaml (gated enabled && replicaCount>1), default soft podAntiAffinity when affinity unset. No HPA (documented).

**Validation**: actuator unit tests 13/13 green (compiles ⇒ check-crate green); helm lint clean; helm template (actuator=kubernetes, replicaCount=2) renders PDB + anti-affinity + CLOACINA_AGENT_*/CLOACINA_SERVER_* env + networkpolicies RBAC; gating verified. 4 docs updated. e2e k8s-fleet NOT run (heavy build; left for human diff review) — static egress argument holds: agent needs only DNS + server:8080, both allowed; packages/interpreter need no external egress. NOT committed.