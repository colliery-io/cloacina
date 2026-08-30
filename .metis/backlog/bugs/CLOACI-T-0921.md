---
id: flat-name-global-registries-cross
level: task
title: "Flat name-global registries cross tenant lines in-process — audit and key by tenant/package"
short_code: "CLOACI-T-0921"
created_at: 2026-08-02T17:44:00.940671+00:00
updated_at: 2026-08-06T13:46:27.576794+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


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

- [x] Registry audit table recorded (26 process-global name-keyed maps enumerated and classified — see the 2026-08-04 entry below)
- [x] EndpointRegistry (all 8 inner maps) keyed by EndpointKey { tenant_id, name } with EndpointOwner stamped per entry; cross-tenant isolation test proves inject-A/B-silent and deregister-A/B-survives
- [x] Python GRAPH_EXECUTORS + ACCUMULATOR_REGISTRY keyed by GraphKey { tenant_id, package, name } via the new registration_scope.rs seam, with equivalent tests
- [x] Further violators fixed or ticketed: T-0924 (Runtime + CG scheduler re-keying), T-0925 (PROVIDER_SEARCH_PATH tenant-blindness)

## Status Updates

- 2026-08-02: Filed from the architecture deep dive (DEEPDIVE.md risk register #23; cg-runtime report §7.6 + python-integration report §7.4). Verified against main @ 5216e632.
- 2026-08-06: DONE — merged to main in PR #235 (squash). AUDIT: 26 process-global name-keyed maps classified. Fixed here: EndpointRegistry's 8 inner maps (accumulators, reactors/handles, both policy maps, health, freshness, meta, injects) and the two Python CG registries. VERIFIED SAFE (not assumed): RuntimeInner.tasks is already TaskNamespace-keyed — the pattern the rest should copy; DeliverySink.by_key is already (recipient, tenant_id) and served as the reference shape; stream_backends keys by backend KIND not entity name; the proc-macro registry is compile-time. DEFERRED and now ticketed: Runtime + scheduler maps (T-0924), PROVIDER_SEARCH_PATH (T-0925), plus EndpointOwner.package provenance and the Python drain buffers (GIL-serialized on today's import path) recorded here as low-priority residuals. DESIGN: resolution goes own-tenant -> untenanted (embedded/pre-tenancy) -> admin-only UNIQUE cross-tenant match; two tenants owning a name yields AmbiguousEndpoint, never a guess; a different owner claiming a live (tenant, name) is a loud EndpointOwnershipConflict; load_reactor unwinds the whole load on rejection. ROUTES UNCHANGED — /v1/ws/{accumulator,reactor}/{name} keep bare-name paths because AuthenticatedKey (tenant + is_admin) was ALREADY in scope in both handlers for the policy check and merely needed threading into resolution. BONUS SECURITY FIX found during the audit: list_accumulators_with_health_for_key fell back to allow_all for accumulators with no policy row, letting tenant B ENUMERATE tenant A's accumulator names — now gated on tenant before the policy check. Deregistration is owner-scoped and also clears the health/freshness/policy/inject side tables that previously leaked past unload. LANDING NOTE: the constructors-wasm suite caught two stale callers of the changed send_to_accumulator signature — that suite had run in NO ci lane until T-0917 gave it one hours earlier, so its first act was catching this. A second fix corrected the reconciler e2e test to send in EndpointScope::tenant("public") rather than untenanted(), matching the tenant its reconciler registers under.
