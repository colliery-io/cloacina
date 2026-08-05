---
id: flat-name-global-registries-cross
level: task
title: "Flat name-global registries cross tenant lines in-process — audit and key by tenant/package"
short_code: "CLOACI-T-0921"
created_at: 2026-08-02T17:44:00.940671+00:00
updated_at: 2026-08-04T05:05:01.821436+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Flat name-global registries cross tenant lines in-process — audit and key by tenant/package

## Objective

Audit every process-wide name-keyed registry against the tenant-is-THE-isolation-boundary doctrine and re-key the violators. The 2026-08-02 deep dive found registries keyed by bare name, so same-named entities from different packages/tenants collide inside one server process — a doctrine breach in a system whose entire authz design rests on tenant isolation being structural.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (important for user experience)

## Findings

1. EndpointRegistry keys accumulators by bare name process-wide (crates/cloacina/src/computation_graph/registry.rs:317-321): two packages (or two tenants) loading same-named accumulators broadcast into EACH OTHER'S boundary channels, and deregistering one tears both down.
2. Python-side GRAPH_EXECUTORS / ACCUMULATOR_REGISTRY are likewise bare-name keyed (crates/cloacina-python): two tenants' same-named graphs share one executor slot — last load wins, cross-tenant.
3. Audit scope beyond the two known: any other process-global map keyed by bare entity name (trigger registries, reactor maps, workflow name lookups in the shared runtime) — enumerate and classify each as (a) already tenant/package-scoped, (b) name-collision-safe by construction, or (c) violator.

Context: workflow task namespaces already carry tenant::package:: prefixes (TaskNamespace), so the pattern exists — these registries predate or bypassed it. Server multi-tenancy makes this reachable today: one server process hosts many tenants' packages.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Registry audit table (registry -> keying -> verdict) recorded in this task
- [ ] EndpointRegistry keyed by (tenant, package, name); cross-package same-name test proves isolation (inject into A, B receives nothing; unload A, B survives)
- [ ] Python graph/accumulator registries scoped the same way with an equivalent test
- [ ] Any further violators from the audit fixed or ticketed individually

## Status Updates

- 2026-08-02: Filed from the architecture deep dive (DEEPDIVE.md risk register #23; cg-runtime report §7.6 + python-integration report §7.4). Verified against main @ 5216e632.