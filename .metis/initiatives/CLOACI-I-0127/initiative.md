---
id: kubernetes-first-agent-runners-k8s
level: initiative
title: "Kubernetes-first agent runners — K8s-native fleet with UI-driven agent and tenant assignment"
short_code: "CLOACI-I-0127"
created_at: 2026-06-20T02:38:01.431843+00:00
updated_at: 2026-06-20T02:38:01.431843+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


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

### Non-Functional Requirements
- NFR-001: Agent identity/auth integrates with the server trust model
  (cf. CLOACI-A-0005 deployment-mode trust model, CLOACI-I-0118 tenant/auth).
- NFR-002: Fleet operations are observable (health, capacity, assignment state).

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

## Implementation Plan

This is an XL initiative and is **not yet decomposed**. Discovery first, then a
human check-in on architecture (packaging approach, source-of-truth/reconciliation
model, tenant-isolation enforcement) before decomposing into tasks. Likely task
seams:

1. K8s packaging (Helm/operator) for server + agent-runner fleet.
2. Agent registry + fleet-health API.
3. Tenant ↔ agent-pool assignment model + dispatch routing (on CLOACI-T-0722).
4. UI configuration surface (gated on the in-flight design review).
5. Reconciliation between UI-driven config and declarative K8s state.

## Related Work

- **CLOACI-T-0722** (backlog) — Execute computation graphs on the agent fleet;
  the execution model this scales.
- **CLOACI-I-0118** — server OIDC auth / tenant model (tenant identity for
  assignment).
- **CLOACI-A-0005** — Deployment-mode trust model (hobbyist daemon vs enterprise
  server); K8s-first is the enterprise end.
- **CLOACI-I-0117** — web UI (the eventual config surface; gated on design review).

## Child Tasks

- **CLOACI-T-0748** — Kubernetes-first deployment — configure and assign agents to
  tenant workloads from the UI. (Holds the original demo capture; to be split
  during decomposition.)
