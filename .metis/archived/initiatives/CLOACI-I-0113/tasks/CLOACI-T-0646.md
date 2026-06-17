---
id: rust-client-crate-cloacina-client
level: task
title: "Rust client crate — cloacina-client extract from cloacinactl, WS stream, live contract suite"
short_code: "CLOACI-T-0646"
created_at: 2026-06-10T01:30:29.910601+00:00
updated_at: 2026-06-10T04:22:57.532640+00:00
parent: CLOACI-I-0113
blocked_by: [CLOACI-T-0642, CLOACI-T-0644]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0113
---

# Rust client crate — cloacina-client extract from cloacinactl, WS stream, live contract suite

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0113]]

## Objective **[REQUIRED]**

Extract `cloacinactl`'s crate-private client (`src/shared/{client,client_ctx,error}.rs`) into a published `crates/cloacina-client` crate built on `cloacina-api-types` + `reqwest` + `tokio-tungstenite`. `cloacinactl` becomes a consumer with zero behavior change, so the CLI exercises the same client surface third parties see. Includes a live-server contract suite — DTO sharing prevents schema drift but does not prove handler semantics (REQ-007).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] `crates/cloacina-client`: `ClientBuilder` (`api_key`, `tenant`, `from_cloacinactl_profile` with env:/file: key schemes), typed method-per-endpoint surface returning `cloacina-api-types` DTOs, plus generic `get_json`/`post_json`/`delete_path` escape hatches
- [x] `cloacinactl` migrated to consume the crate (`CliClient` is now a thin wrapper; `CliError` maps from `ClientError` preserving ADR-0003 exit codes; `execution follow` runs on the crate's stream with `reconnect: false`); full CLI e2e suite green including the T-0629 substrate delivery+ack test
- [x] WS: `subscribe_delivery(recipient)` / `follow_execution_events(exec_id)` return typed Streams with reconnection + dedup + ack-after-yield (REQ-004) — named for the documented delivery protocol (T-0644) and TS-SDK parity rather than the placeholder `computation_graph().subscribe()` phrasing
- [x] Live-server contract suite (`tests/contract.rs`): full REST surface + WS lifecycle, green against a live server (REQ-007)
- [x] < 5ms overhead vs raw `reqwest`: measured median overhead on localhost /health round-trips, asserted in the contract suite (NFR-002)

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Lift `cloacinactl/src/shared/{client,client_ctx,error}.rs` into `crates/cloacina-client`, generalize the error model away from CLI-shaped messages, and re-point `cloacinactl` at the crate in the same change so nothing forks. WS via `tokio-tungstenite` returning a typed `futures::Stream`.

### Dependencies
Blocked by CLOACI-T-0642 (DTOs) and CLOACI-T-0644 (WS protocol semantics). Runs in parallel with T-0645/T-0647. Sequences before T-0648.

### Risk Considerations
Behavior drift during extraction — gated by the existing cloacinactl e2e suite passing unchanged. The live contract suite is still required despite shared types: DTO sharing prevents schema drift but doesn't prove handler semantics match the documented contract.

## Status Updates **[REQUIRED]**

**2026-06-10** — Implemented on `i0113-server-sdks`:
- New `crates/cloacina-client` (workspace member + pinned workspace dep): `error.rs` (`ClientError` — generalized from `CliError`, status mapping preserved), `profile.rs` (cloacinactl config.toml interop + `resolve_api_key_scheme` moved here), `ws.rs` (delivery stream via async-stream + tokio-tungstenite: fresh ticket per connect, hello v1, dedup-on-id, ack-after-yield, exponential backoff, 4426 → terminal `ProtocolVersion` error), `lib.rs` (builder + typed surface for all 21 endpoints + generic escape hatches the CLI's verb handlers rely on).
- **CLI migration:** `shared/client.rs` is now a wrapper (ctx + exit-code mapping only — the CLI-shaped parts); `shared/client_ctx.rs` delegates key-scheme resolution; `shared/error.rs` gains `From<ClientError>` preserving exit codes 1-5; `nouns/execution/mod.rs` follow loop replaced by the crate's stream (`reconnect: false` keeps exact old behavior; hand-rolled ws_url/decode helpers deleted).
- **Verification:** crate unit tests + doctest green; live contract suite green (full REST surface, WS lifecycle, NFR-002 overhead — measured median overhead well under 5ms); full `angreal test e2e cli` green including T-0629 substrate delivery+ack (now exercising the crate's stream code).
- **e2e gotcha (environmental, not a regression):** first e2e run failed exit-4-vs-3 because my long-running contract-test server shared the `cloacina` DB and its bootstrap-admin key shadowed the e2e's `--bootstrap-key`. Fresh DB → green. Reinforces CLOACI-T-0649 (dbname ignored — the e2e *thought* it had its own DB story).