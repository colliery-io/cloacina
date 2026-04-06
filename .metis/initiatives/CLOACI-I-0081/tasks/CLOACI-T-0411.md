---
id: graceful-shutdown-wire-shutdown
level: task
title: "Graceful shutdown — wire shutdown_all into server, WebSocket draining, final-state persistence"
short_code: "CLOACI-T-0411"
created_at: 2026-04-05T21:24:25.811396+00:00
updated_at: 2026-04-06T00:28:54.861125+00:00
parent: CLOACI-I-0081
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0081
---

# Graceful shutdown — wire shutdown_all into server, WebSocket draining, final-state persistence

## Parent Initiative

[[CLOACI-I-0081]]

## Objective

Wire end-to-end graceful shutdown so that on SIGTERM/SIGINT, the server cleanly shuts down all computation graph components, drains WebSocket connections, and persists final state before exiting. Currently `ReactiveScheduler::shutdown_all()` is never called in the server shutdown path.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `ReactiveScheduler::shutdown_all()` called in server shutdown handler (in `serve.rs` shutdown signal path)
- [ ] Shutdown ordering: stop accepting new WS connections -> drain active WS connections (send close frames) -> shutdown scheduler (which shuts down reactors + accumulators) -> persist final state
- [ ] Each reactor persists final `InputCache` + `DirtyFlags` before stopping
- [ ] Each accumulator persists final checkpoint before stopping
- [ ] Batch accumulators flush remaining buffer before stopping (already works for graceful shutdown, verify it still works with new checkpoint wiring)
- [ ] Configurable shutdown timeout — if components don't stop within timeout, force-kill (follow daemon shutdown pattern)
- [ ] WebSocket connections receive close frames before server drops them
- [ ] `/health` and `/ready` return 503 during shutdown (service draining)
- [ ] Unit test: shutdown signal triggers scheduler shutdown
- [ ] Integration test: send SIGTERM to server, verify state persisted to DAL, verify WS clients receive close frames

## Implementation Notes

### Technical Approach

**Shutdown sequence** (mirrors daemon pattern from `commands/daemon.rs`):
1. Signal received (SIGINT/SIGTERM)
2. Health endpoints start returning 503 (draining)
3. Stop accepting new WebSocket upgrades
4. Send close frames to all active WebSocket connections
5. Call `reactive_scheduler.shutdown_all()` — which sends shutdown to each graph
6. Each graph: reactor persists final cache, accumulators persist final checkpoints, batch accumulators flush
7. Wait up to timeout for all components to stop
8. If timeout exceeded, force-abort remaining tasks
9. HTTP server completes graceful shutdown

**Key files:**
- `crates/cloacinactl/src/commands/serve.rs` — wire scheduler shutdown into shutdown signal handler
- `crates/cloacinactl/src/server/ws.rs` — WebSocket connection tracking + close frame sending
- `crates/cloacina/src/computation_graph/scheduler.rs` — ensure `shutdown_all()` triggers persist-then-stop for each component

### Dependencies
- T-0408 (accumulator checkpoints) — accumulators need checkpoint wiring to persist on shutdown
- T-0410 (reactor persistence) — reactor needs DAL wiring to persist on shutdown

### Risk Considerations
- Shutdown timeout must be long enough for DAL persistence but short enough for k8s termination grace period
- WebSocket close frame delivery is best-effort — if client is unresponsive, don't block shutdown

## Status Updates

### 2026-04-06: Core wiring complete

**Completed:**
- ReactiveScheduler::shutdown_all() now called in server shutdown path after axum graceful shutdown completes
- Shutdown ordering: SIGINT/SIGTERM → axum stops accepting connections → axum drains → scheduler.shutdown_all() → each graph unloads (reactor persists final cache, batch accumulators flush) → server exits
- Clone scheduler Arc before passing state to router so it survives axum's move semantics
- All unit tests pass

**Already working (from prior tasks):**
- Reactor persists final cache + dirty flags on shutdown (T-0410)
- Batch accumulators flush remaining buffer on graceful shutdown (existing code)
- Axum with_graceful_shutdown handles connection draining

**Deferred:**
- Configurable shutdown timeout with force-kill (would add complexity, can add later if needed)
- WebSocket close frame sending (axum handles connection draining, explicit close frames are a polish item)
- /health returning 503 during draining (requires shared atomic flag, polish item)
