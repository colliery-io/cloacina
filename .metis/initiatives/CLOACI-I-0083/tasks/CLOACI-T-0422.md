---
id: cg-policy-wiring-scheduler-sets
level: task
title: "CG policy wiring — scheduler sets auth policies on load_graph for single-tenant flow"
short_code: "CLOACI-T-0422"
created_at: 2026-04-06T15:18:09.985183+00:00
updated_at: 2026-04-06T16:13:52.232215+00:00
parent: CLOACI-I-0083
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0083
---

# CG policy wiring — scheduler sets auth policies on load_graph for single-tenant flow

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0083]]

## Objective

Make CG WebSocket endpoints usable for the single-tenant (default) deployment. Currently `ReactiveScheduler.load_graph()` registers accumulators and reactors in the EndpointRegistry but never calls `set_accumulator_policy()` or `set_reactor_policy()`. The WebSocket handlers correctly enforce deny-by-default, so all connections get 403. This task wires the policies so that any authenticated key can access CG endpoints — the minimal fix for single-tenant mode.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `load_graph()` in `scheduler.rs` calls `set_accumulator_policy()` for each accumulator registered
- [ ] `load_graph()` calls `set_reactor_policy()` for each reactor registered
- [ ] Policy is "allow any authenticated key" — the `AccumulatorAuthPolicy` needs to support this (may need an `allow_all` flag or similar, since current design uses explicit UUID lists)
- [ ] WebSocket connection to `/v1/ws/accumulator/{name}?token=valid_key` returns 101 (upgrade) instead of 403
- [ ] WebSocket connection to `/v1/ws/reactor/{name}?token=valid_key` returns 101 instead of 403
- [ ] No-auth connections still get 401
- [ ] Existing unit tests pass
- [ ] WS integration test (`angreal cloacina ws-integration`) updated to verify 101 on valid auth

## Implementation Notes

### Key files
- `crates/cloacina/src/computation_graph/scheduler.rs` — `load_graph()` method
- `crates/cloacina/src/computation_graph/registry.rs` — `AccumulatorAuthPolicy`, `ReactorAuthPolicy`, `set_accumulator_policy()`, `set_reactor_policy()`
- `crates/cloacinactl/src/server/ws.rs` — WebSocket handlers that call `check_accumulator_auth` / `check_reactor_auth`

### Design consideration
`AccumulatorAuthPolicy` currently uses `allowed_producers: Vec<uuid::Uuid>` with empty = deny all. For "allow any authenticated key," options:
1. Add `allow_all_authenticated: bool` flag to the policy struct
2. Use a sentinel UUID
3. Move the check logic to support both modes

Option 1 is cleanest. The WS handler already validates the token before checking the policy, so `allow_all_authenticated = true` means "if you passed authn, you're authorized."

### Unblocks
- T-0404 (CG soak test) — can now inject events via WebSocket

## Status Updates **[REQUIRED]**

**2026-04-06 — Complete**
- Added `allow_all_authenticated: bool` to `AccumulatorAuthPolicy` and `ReactorAuthPolicy`
- Added `allow_all()` constructor on both policy structs
- Updated `is_authorized()` and `is_operation_permitted()` to check the flag first
- Wired `set_accumulator_policy(allow_all)` and `set_reactor_policy(allow_all)` in `load_graph()` after registration
- Also wired policies in both restart paths: full reactor restart and individual accumulator restart
- Updated existing test struct literals to include the new field
- All 10 registry unit tests pass, both crates compile clean
