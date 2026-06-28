---
id: kubernetes-first-agent-runners-k8s
level: initiative
title: "Kubernetes-first agent runners — K8s-native fleet with UI-driven agent and tenant assignment"
short_code: "CLOACI-I-0127"
created_at: 2026-06-20T02:38:01.431843+00:00
updated_at: 2026-06-28T16:28:06.161622+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: kubernetes-first-agent-runners-k8s
---

# Kubernetes-first agent runners — K8s-native fleet with UI-driven agent and tenant assignment

## Context

Surfaced during a live demo (2026-06-18/19). The scaled deployment story for the
server + agent-runner topology is currently hand-rolled (compose, manual wiring).
For multi-tenant, horizontally-scaled deployments we want to lean into Kubernetes
as the first-class deployment target, run agent runners as K8s-native
workloads, and let an operator configure agents and assign them to tenants /
tenant workloads from the UI rather than via out-of-band manifests.

This builds directly on the agent-fleet execution model — computation graphs are
dispatched to the agent fleet per reactor firing (CLOACI-T-0722). This initiative
is about making that fleet **deployable, scalable, and operable on Kubernetes**,
with tenant-aware assignment.

> **Embedded-first remains a permanent, production-legitimate end-state.** This
> K8s-first work is the *scaled / enterprise* deployment target, not a
> replacement for embedded and not a signal that embedded is "training wheels."

## Goals & Non-Goals

**Goals:**
- A supported, first-class Kubernetes deployment path (Helm and/or an operator)
  for the server + agent-runner fleet, with agents as schedulable, scalable
  workloads.
- Operators can register/configure agents and view fleet health.
- Operators can assign agent pools to tenants / tenant workloads so a tenant's
  computation graphs run on its assigned compute, with enforced isolation.
- UI-driven agent/tenant configuration reconciles with declarative K8s state
  (no UI-vs-manifest drift).

**Non-Goals:**
- Not removing or deprioritizing the embedded deployment mode.
- Not a generic "run anything on K8s" platform — scope is the Cloacina
  server + agent-runner fleet and its tenant model.
- The full UI build-out is gated on the design review (see Open Questions); the
  *data/control-plane* contracts can land ahead of polished UI.

## Requirements

### Functional Requirements
- REQ-001: Helm chart and/or operator deploys the server + agent-runner fleet on
  Kubernetes; agent runners scale horizontally as K8s workloads.
- REQ-002: An API surface to register/configure agents and report fleet health
  (capacity, labels, liveness).
- REQ-003: An API + model to bind agent pools to tenants / tenant workloads, so
  CG dispatch routes a tenant's graphs to its assigned agents.
- REQ-004: Tenant compute isolation is enforced (a tenant's work does not run on
  another tenant's pool).
- REQ-005: UI-driven config reconciles with declarative K8s state without drift
  (a clear source-of-truth / reconciliation model).

- REQ-006: The **Helm chart provisions the Kubernetes RBAC** the K8s actuator
  needs to create/scale/delete agent workloads in tenant namespaces
  (ServiceAccount + Role/RoleBinding), **least-privilege — no cluster-admin**.
  (Explicit ask, 2026-06-27.)
- REQ-007: A tenant's agents/workloads run in the **tenant's own Kubernetes
  namespace** (operational isolation — network/resource boundaries per tenant).
  (Explicit ask, 2026-06-27.)
- REQ-008 (**misconfig guard — fail-closed, 2026-06-27**): the fleet actuator is
  **explicitly selected** (`CLOACINA_FLEET_ACTUATOR=docker|kubernetes|none`) and
  **validated against the detected substrate at startup**. The **Docker-container
  actuator REFUSES to start when Kubernetes is detected** (`KUBERNETES_SERVICE_HOST`
  / the in-cluster service-account mount) — it must never `docker run` containers
  inside a cluster, bypassing K8s scheduling/namespacing/RBAC. Symmetrically: the
  K8s actuator refuses when not in-cluster (no SA / API unreachable), and the
  Docker actuator refuses with no Docker socket. Any mismatch is a **loud boot-time
  failure, never a silent wrong-scaling.** The Helm chart sets `=kubernetes`, the
  compose stack sets `=docker`; the guard catches overrides/mistakes.

### Non-Functional Requirements
- NFR-001: Agent identity/auth integrates with the server trust model
  (cf. CLOACI-A-0005 deployment-mode trust model, CLOACI-I-0118 tenant/auth).
- NFR-002: Fleet operations are observable (health, capacity, assignment state).
- NFR-003: Tenant-admins self-serve agent provisioning bounded by an
  admin-set limit; god sets the default limit + per-tenant exceptions.
- NFR-004 (**security tenet — belt & suspenders, 2026-06-27**): the execution
  substrate (K8s namespace / NetworkPolicy / RBAC) is **defense-in-depth, NOT the
  security boundary**. We do **not** trust the substrate to keep a tenant in its
  lane. The **server independently enforces tenant scope on every control-plane
  and CRUD operation** — provision/deprovision, limits, desired-count, agent
  registry, fleet dispatch — via the fail-closed ABAC route table (CLOACI-I-0118).
  Any attempt to CRUD *across* a caller's tenant scope is **denied server-side
  regardless of how the substrate is configured.** Belt = per-tenant namespace
  (REQ-007); suspenders = server-side authZ as the real boundary. Every
  control-plane task below carries an explicit cross-scope-denial AC.

## Architecture

### Overview
- **K8s packaging:** Helm chart (and possibly a lightweight operator/CRDs) for
  server + agent-runner Deployments/StatefulSets; agents as a scalable pool.
- **Fleet control plane:** server-side registry of agents (identity, labels,
  capacity, health) and the tenant→pool assignment model.
- **Dispatch routing:** CG dispatch (per CLOACI-T-0722) consults the assignment
  model to place a tenant's graph on its assigned pool.
- **Reconciliation:** decide the source of truth between UI-driven config and
  declarative K8s state and reconcile (operator pattern vs. server-as-authority).

These are open design questions to resolve in discovery — do not commit to an
approach without a human check-in.

## Use Cases

### Use Case 1: Operator scales a tenant's compute
- **Actor:** Platform operator
- **Scenario:** Assigns an additional agent pool to a tenant in the UI; the fleet
  scales and the tenant's graphs begin dispatching to it.
- **Expected Outcome:** Tenant throughput increases; isolation preserved.

### Use Case 2: Operator stands up Cloacina on K8s
- **Actor:** Platform operator
- **Scenario:** Installs the Helm chart; server + agent runners come up; agents
  self-register and report health.
- **Expected Outcome:** A working, scalable deployment without hand-wiring.

## Alternatives Considered

- **Keep compose / hand-rolled deployment.** Rejected as the *scaled* target —
  doesn't meet multi-tenant scaling/isolation needs (still fine for embedded /
  small deployments).
- **Config via manifests only, no UI.** Rejected — the demo ask is explicitly to
  configure agents and tenant assignment *from the UI*; manifest-only is the
  status quo we're improving on.
- **Server-as-sole-authority vs. K8s-operator reconciliation.** Open — to be
  decided in discovery with a human check-in.

## Design Decision (2026-06-27 — human check-in)

**Architecture: control-plane / pluggable-actuator split.** The server owns the
**control plane** (deployment-agnostic): per-tenant *desired agent count*, the
*limits model* (admin default + per-tenant exceptions), and the *back-pressure
autoscaler*. A pluggable **actuator** reconciles desired→actual by spawning/killing
agent runtimes — a **docker/local actuator first** (proves the loop on the compose
stack), with the **K8s actuator as a fast-follow** (the production scaler).

**Why this fits what already exists:** agents are already **tenant-scoped**
(`fleet/protocol.rs` `tenant_id`; `reject_cross_tenant_agent`; `fleet_executor`
selects same-tenant agents → isolation enforced), and the **back-pressure signal**
already exists (`NoCapacity` in the scheduler). So this work is a control plane on
top of a working tenant-scoped fleet — agents still self-register once spawned;
the existing dispatch path is untouched. "Provision from the UI" = bump the
tenant's desired count within its limit → actuator spawns it → it self-registers.

## Implementation Plan — first slice: control plane + dev actuator

Decomposition (proposed; vertical, compose-demoable):

1. **Limits model** — admin default `max_agents` + per-tenant exceptions (god-set,
   on I-0118 ABAC); effective-limit lookup. AC: default enforced; exception honored
   (e.g. default 4, acme 6).
2. **Desired-count + provision/deprovision API** — per-tenant desired-count state;
   tenant-admin provisions/deprovisions for *their own* tenant, bounded by the
   effective limit. AC: rejected past limit; tenant-scoped (no cross-tenant).
3. **Pluggable actuator + Docker dev actuator** — `FleetActuator` trait
   (reconcile desired→actual) + a **Docker-container impl** (`bollard` / Docker API):
   `docker run` N tenant-keyed `cloacina-agent` containers (labelled
   `cloacina.tenant=<t>`), stop the surplus on scale-down. **First pass = containers
   only** — no local-process flavor, no K8s. Needs a per-tenant **agent registration
   key** (tenant-scoped, `agent` provenance) injected as `CLOACINA_API_KEY`. AC: bump
   desired → a container's agent self-registers into the tenant pool on the compose
   stack; lower → it drains/stops; actuator never spawns/targets another tenant's
   containers (cross-scope-denial). **AC (REQ-008): the actuator framework
   validates the selected actuator against the detected substrate at boot,
   fail-closed — the Docker actuator REFUSES to start when Kubernetes is detected
   (`KUBERNETES_SERVICE_HOST` / SA mount), the K8s actuator refuses when not
   in-cluster, the Docker actuator refuses with no socket — loud boot error, never
   a silent wrong-scaling.**
4. **Back-pressure autoscaler** — control loop: per-tenant pressure (NoCapacity
   rate / Ready-backlog) → adjust desired within [floor, effective-limit] with
   scale-up/down thresholds + cooldown. AC: sustained pressure scales up to the
   limit (never beyond); idle scales to the floor.
5. **Auto-provision on tenant create** — POST /tenants sets the initial desired
   count (within the default limit). AC: a new tenant comes up with its agent(s).
6. **UI — tenant agent management** — provision/deprovision + pool/limit/autoscaler
   state; role-gated (tenant-admin write, read sees state). AC: tenant-admin
   provisions from the UI and sees the agent join; read users see but can't change.

**Fast-follow (production actuator):**

7. **K8s actuator + Helm RBAC** — K8s `FleetActuator` impl (scale per-tenant agent
   Deployment replicas). **AC (REQ-006): the Helm chart creates the RBAC the
   actuator needs — ServiceAccount + Role/RoleBinding to create/scale/delete agent
   workloads in tenant namespaces, least-privilege, no cluster-admin** — and the
   actuator uses it to scale a tenant's pool on a real cluster.

## Related Work

- **CLOACI-T-0722** (backlog) — Execute computation graphs on the agent fleet;
  the execution model this scales.
- **CLOACI-I-0118** — server OIDC auth / tenant model (tenant identity for
  assignment).
- **CLOACI-A-0005** — Deployment-mode trust model (hobbyist daemon vs enterprise
  server); K8s-first is the enterprise end.
- **CLOACI-I-0117** — web UI (the eventual config surface; gated on design review).

## Child Tasks

**First slice — control plane + Docker dev actuator (2026-06-27):**
- **CLOACI-T-0808** — Agent limits (admin default + per-tenant exceptions); model + API.
- **CLOACI-T-0809** — Per-tenant desired-count + provision/deprovision REST API (+ cross-scope-denial).
- **CLOACI-T-0810** — Pluggable `FleetActuator` + Docker-container dev actuator + substrate guard (REQ-008).
- **CLOACI-T-0811** — Back-pressure autoscaler, leader-elected, within limits.
- **CLOACI-T-0812** — Auto-provision agent(s) on tenant create.
- **CLOACI-T-0813** — UI: tenant agent management (role-gated).

**Fast-follow — production actuator:**
- **CLOACI-T-0814** — K8s actuator + Helm RBAC (REQ-006) + per-tenant namespace (REQ-007).

Per-task objective + acceptance criteria are in the *Implementation Plan* above
(items 1–7); each control-plane task also carries the NFR-004 cross-scope-denial AC.

- **CLOACI-T-0748** — original demo capture; **superseded** by T-0808–T-0814 above
  (fold/archive once these are populated).