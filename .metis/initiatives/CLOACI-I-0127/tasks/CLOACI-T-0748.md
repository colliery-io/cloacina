---
id: kubernetes-first-deployment
level: task
title: "Kubernetes-first deployment — configure and assign agents to tenant workloads from the UI"
short_code: "CLOACI-T-0748"
created_at: 2026-06-20T02:26:30.274166+00:00
updated_at: 2026-06-20T02:26:30.274166+00:00
parent: CLOACI-I-0127
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0127
---

# Kubernetes-first deployment — configure and assign agents to tenant workloads from the UI

## Origin

Surfaced during a live demo (2026-06-18/19). The deployment story for the
server/agent topology is currently hand-rolled (compose, manual wiring). For
multi-tenant, scaled deployments we want to lean into Kubernetes as the
first-class deployment target, and let an operator configure agents and assign
them to tenant workloads from within the UI rather than via out-of-band config.

> **Scope note:** this is initiative-sized, not a single task. It is captured
> here as a backlog feature so the demo finding isn't lost; it should be promoted
> to (or folded into) an initiative and decomposed before any implementation.
> Do not start as a single task.

## Objective

Make Kubernetes the primary, well-supported deployment mode for the
server + agent fleet, and give operators a UI surface to: provision/configure
agents, and assign agents (pools) to specific tenants / tenant workloads — so
tenant compute can be shaped and isolated without editing manifests by hand.

## Backlog Item Details

### Type
- [x] Feature — deployment / platform (initiative-scale)

### Priority
- [x] P2 — Medium (strategic platform direction; not blocking current users, but
      central to the scaled/multi-tenant story)

### Business Justification
- **User Value**: Operators run Cloacina on K8s with a supported path and manage
  agent → tenant assignment from the UI instead of YAML surgery.
- **Business Value**: Unlocks the multi-tenant, horizontally-scaled deployment
  story; makes per-tenant compute isolation and capacity a product capability.
- **Effort Estimate**: XL (initiative — multiple tasks across server, agent
  fleet, Helm/K8s packaging, and UI).

## Scope sketch (to be refined during decomposition)

- **K8s-first packaging**: first-class Helm/operator path for server + agent
  fleet; agents as schedulable, scalable workloads.
- **UI agent management**: register/configure agents, view fleet health, set
  capacity/labels from the UI.
- **Tenant ↔ agent assignment**: bind agent pools to tenants / tenant workloads
  so a tenant's computation graphs run on its assigned compute; enforce isolation.
- **Config provenance**: UI-driven config must reconcile with declarative K8s
  state (avoid UI/manifest drift).

## Related work

- **CLOACI-T-0722** (backlog) — Execute computation graphs on the agent fleet
  (whole-graph dispatch per reactor firing). The fleet execution model this
  builds on.
- **CLOACI-I-0117** — web UI (the surface that grows the agent/tenant config
  views).
- **CLOACI-I-0118** — server OIDC auth / tenant model (tenant identity this
  assignment hangs off of).
- **CLOACI-A-0005** — Deployment-mode trust model (hobbyist daemon vs enterprise
  server) — the K8s-first mode is the enterprise end of this.
- Embedded-first remains a permanent, production-legitimate end-state; K8s-first
  is the *scaled* deployment target, not a replacement for embedded.

## Acceptance Criteria (initiative-level — refine on decomposition)

- [ ] Promoted to / folded into an initiative and decomposed into tasks before
      implementation.
- [ ] Supported Kubernetes deployment path for server + agent fleet (Helm/operator).
- [ ] UI can configure agents and view fleet status.
- [ ] UI can assign agents (pools) to tenants / tenant workloads with enforced
      isolation.
- [ ] UI-driven agent/tenant config reconciles with declarative K8s state without
      drift.

## Status Updates

*To be added during implementation*
