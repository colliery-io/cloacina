---
id: rust-client-crate-cloacina-client
level: task
title: "Rust client crate — cloacina-client extract from cloacinactl, WS stream, live contract suite"
short_code: "CLOACI-T-0646"
created_at: 2026-06-10T01:30:29.910601+00:00
updated_at: 2026-06-10T01:30:29.910601+00:00
parent: CLOACI-I-0113
blocked_by: ["CLOACI-T-0642", "CLOACI-T-0644"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0113
---

# Rust client crate — cloacina-client extract from cloacinactl, WS stream, live contract suite

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0113]]

## Objective **[REQUIRED]**

Extract `cloacinactl`'s crate-private client (`src/shared/{client,client_ctx,error}.rs`) into a published `crates/cloacina-client` crate built on `cloacina-api-types` + `reqwest` + `tokio-tungstenite`. `cloacinactl` becomes a consumer with zero behavior change, so the CLI exercises the same client surface third parties see. Includes a live-server contract suite — DTO sharing prevents schema drift but does not prove handler semantics (REQ-007).

## Acceptance Criteria **[REQUIRED]**

- [ ] `crates/cloacina-client`: `ClientBuilder` (`api_key`, `tenant`, `profile_from_cloacinactl_config`), noun-shaped surface (`client.workflows()`, `.executions()`, …) returning `cloacina-api-types` DTOs
- [ ] `cloacinactl` migrated to consume the crate; zero behavior change — existing CLI e2e suite green unchanged
- [ ] `client.computation_graph().subscribe(...)` returns a typed Stream with reconnection handling (REQ-004)
- [ ] Live-server contract suite covering every endpoint + a WS lifecycle (REQ-007)
- [ ] < 5ms overhead vs raw `reqwest` for a localhost single round-trip (NFR-002)

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Lift `cloacinactl/src/shared/{client,client_ctx,error}.rs` into `crates/cloacina-client`, generalize the error model away from CLI-shaped messages, and re-point `cloacinactl` at the crate in the same change so nothing forks. WS via `tokio-tungstenite` returning a typed `futures::Stream`.

### Dependencies
Blocked by CLOACI-T-0642 (DTOs) and CLOACI-T-0644 (WS protocol semantics). Runs in parallel with T-0645/T-0647. Sequences before T-0648.

### Risk Considerations
Behavior drift during extraction — gated by the existing cloacinactl e2e suite passing unchanged. The live contract suite is still required despite shared types: DTO sharing prevents schema drift but doesn't prove handler semantics match the documented contract.

## Status Updates **[REQUIRED]**

*To be added during implementation*
