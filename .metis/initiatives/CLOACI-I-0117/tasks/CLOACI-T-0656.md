---
id: ui-live-execution-stream-ws-backed
level: task
title: "UI live execution stream — WS-backed live-follow on execution detail"
short_code: "CLOACI-T-0656"
created_at: 2026-06-11T02:18:57.886708+00:00
updated_at: 2026-06-11T02:18:57.886708+00:00
parent: CLOACI-I-0117
blocked_by: ["CLOACI-T-0653"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI live execution stream — WS-backed live-follow on execution detail

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

The real-time centerpiece (REQ-004 live half, NFR-002): on execution detail, stream events live over the delivery WS via `followExecutionEvents(execId)` as a run progresses, appending to the event log until terminal. This is the feature the whole UAT harness (T-0660) exists to exercise.

## Acceptance Criteria **[REQUIRED]**

- [ ] Execution detail opens a live stream for in-flight executions via the SDK's `followExecutionEvents`; events render in order as they arrive; the view updates to terminal state when the run finishes.
- [ ] A React hook bridges the SDK's async iterator into component state, starting on mount of a live execution and **cancelling cleanly on unmount/navigation** (no leaked WS).
- [ ] Dedup + reconnect are delegated to the SDK; the UI shows no duplicated events and no gap across a transient disconnect (NFR-002) — verified by the T-0660/T-0661 harness.
- [ ] **History-vs-live merge defined (OQ-6)**: on open, backfill the historical event log (REST events endpoint) then tail forward via the WS, deduping on `sequence_num`/`id` at the seam; no duplicate or missing events at the boundary. Record the chosen merge in the status update.
- [ ] Stream-closed / terminal UX is explicit (not a spinner that never resolves); already-terminal executions show history without opening a stream.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
`useLiveExecutionEvents(execId)` wraps `for await (const e of followExecutionEvents(...))`, pushing onto the normalized event array from T-0653 with a dedup key. Use an `AbortController`/cancellation tied to component lifecycle. Decide the merge: fetch REST history first (up to `max_known_id` semantics), then start the stream; the delivery WS resyncs unacked rows so the seam must dedup.

### Dependencies
Blocked by CLOACI-T-0653 (extends its execution-detail view + event-log model). Exercised end-to-end by T-0660 (workload generator produces the live runs) and T-0661 (Playwright asserts the stream).

### Risk Considerations
The merge seam (OQ-6) is the subtle part — at-least-once delivery means the WS may replay events already in the REST history; dedup must be correct. WS auth uses the SDK's ticket flow (no key in the URL beyond the single-use ticket). Cancellation correctness matters (navigating away mid-stream must not leak sockets).

## Status Updates **[REQUIRED]**

*To be added during implementation*
