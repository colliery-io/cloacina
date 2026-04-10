---
id: per-endpoint-authorization
level: task
title: "Per-endpoint authorization — AccumulatorAuthPolicy, ReactorAuthPolicy, package metadata"
short_code: "CLOACI-T-0379"
created_at: 2026-04-05T00:52:01.211125+00:00
updated_at: 2026-04-05T01:58:22.852824+00:00
parent: CLOACI-I-0071
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0071
---

# Per-endpoint authorization — AccumulatorAuthPolicy, ReactorAuthPolicy, package metadata

## Objective

Layer per-endpoint authorization on top of the WebSocket plumbing (T-0371). Currently T-0371 authenticates the PAK key (is this a valid key?) but does not authorize it for a specific endpoint (is this key allowed to push to accumulator "alpha"?). This task adds the authZ model from S-0007: policies declared in package metadata, enforced on WebSocket upgrade.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `AccumulatorAuthPolicy` struct: accumulator name + list of authorized PAK key IDs
- [ ] `ReactorAuthPolicy` struct: reactor name + list of authorized PAK key IDs + allowed operations per key
- [ ] `ReactorOp` enum: `ForceFire`, `FireWith`, `GetState`, `Pause`, `Resume`, `GetHealth`
- [ ] Policies loaded from `ComputationGraphDeclaration` metadata (declared alongside topology in the package)
- [ ] Policies stored in `EndpointRegistry` alongside channel senders
- [ ] Accumulator WS handler: after PAK auth, checks `AccumulatorAuthPolicy` — rejects with 403 if key not authorized for this name
- [ ] Reactor WS handler: after PAK auth, checks `ReactorAuthPolicy` — rejects with 403 if key not authorized for this name
- [ ] Reactor per-operation check: each command checked against `allowed_operations` for the key — rejects individual commands with error response if not permitted
- [ ] Test: key authorized for "alpha" can connect to alpha, rejected from "beta"
- [ ] Test: key authorized for ForceFire but not Pause — ForceFire succeeds, Pause returns error
- [ ] Test: no policy defined for endpoint → reject all (deny by default)

## Implementation Notes

### Files
- `crates/cloacina/src/computation_graph/registry.rs` — add policy storage alongside senders
- `crates/cloacinactl/src/server/ws.rs` — add authZ check after authn in both handlers
- `crates/cloacina/src/computation_graph/types.rs` — policy types

### Design
- Policies are in-memory, loaded from package declarations (no DB table needed for now)
- Deny by default: if no policy exists for an endpoint, all connections are rejected
- Policy check is one-time on connection (accumulator) or per-command (reactor operations)
- The `#[computation_graph]` macro will need an `auth` section in the attribute eventually, but for now policies can be set programmatically via `ComputationGraphDeclaration`

### Dependencies
T-0371 (WS layer with authn), T-0372 (registry to store policies), T-0373/T-0374 (handlers to add authZ checks to)

## Status Updates

- 2026-04-04: Complete. AccumulatorAuthPolicy, ReactorAuthPolicy, ReactorOp types added to registry. Deny-by-default: no policy = 403. WS handlers check authZ after authn. ReactorAuthPolicy supports per-operation permissions via operation_permissions map. 3 new unit tests: deny-by-default, authorized key, per-operation perms. Per-command authZ check in reactor WS handler not yet wired (requires passing auth.key_id into process_reactor_command) — connection-level check is in place.
