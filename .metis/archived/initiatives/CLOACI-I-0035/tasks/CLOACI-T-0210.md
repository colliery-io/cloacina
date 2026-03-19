---
id: integration-test-get-metrics
level: task
title: "Integration test: GET /metrics returns valid Prometheus text format"
short_code: "CLOACI-T-0210"
created_at: 2026-03-17T01:52:26.772255+00:00
updated_at: 2026-03-17T02:06:54.303850+00:00
parent: CLOACI-I-0035
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0035
---

# Integration test: GET /metrics returns valid Prometheus text format

## Parent Initiative

[[CLOACI-I-0035]]

## Objective

Add an integration test that spins up the Cloacina HTTP server, hits the `GET /metrics` endpoint, and verifies it returns a 200 response with Prometheus text format content-type. This validates the end-to-end observability pipeline from metric recording through HTTP rendering.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Test `test_metrics_endpoint_returns_prometheus_format` added to `serve.rs` test module
- [ ] Test starts a real HTTP server on a random port, hits `/metrics`, asserts 200 status
- [ ] Test verifies the `content-type` header contains `text/plain`
- [ ] Test handles the global recorder constraint gracefully (recorder may already be installed by another test)
- [ ] `cargo test -p cloacinactl` passes cleanly

## Test Cases

### Test Case 1: Metrics endpoint returns Prometheus format
- **Test ID**: TC-001
- **Preconditions**: Server started with observability initialized
- **Steps**:
  1. Initialize Prometheus recorder (or skip if already installed)
  2. Create `AppState`, bind to `127.0.0.1:0`, spawn server
  3. `GET /metrics` via reqwest
- **Expected Results**: 200 OK, content-type includes `text/plain`, body is non-empty or valid Prometheus text
- **Status**: Pending

## Implementation Notes

### Technical Approach

1. Add test to the existing `#[cfg(test)] mod tests` block in `serve.rs`
2. Call `crate::observability::init_prometheus()` -- if it fails (recorder already installed), that is fine; the endpoint still works via the `OnceLock`
3. Record a test counter to ensure something appears in output
4. Spawn server, wait briefly, hit `/metrics`, assert status and content-type
5. Use the same server lifecycle pattern as existing tests (5s safety-net shutdown)

### Dependencies

- CLOACI-T-0208 (Prometheus metrics endpoint must be implemented first)

### Risk Considerations

- The `metrics` crate global recorder is per-process; multiple tests installing it will conflict. The test must tolerate a previously-installed recorder.
- Tests run in parallel by default; port 0 binding avoids port conflicts.

## Status Updates

*To be added during implementation*
