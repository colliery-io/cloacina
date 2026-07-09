---
id: ui-live-execution-stream-ws-backed
level: task
title: "UI live execution stream — WS-backed live-follow on execution detail"
short_code: "CLOACI-T-0656"
created_at: 2026-06-11T02:18:57.886708+00:00
updated_at: 2026-06-11T11:17:39.403621+00:00
parent: CLOACI-I-0117
blocked_by: [CLOACI-T-0653]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI live execution stream — WS-backed live-follow on execution detail

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

The real-time centerpiece (REQ-004 live half, NFR-002): on execution detail, stream events live over the delivery WS via `followExecutionEvents(execId)` as a run progresses, appending to the event log until terminal. This is the feature the whole UAT harness (T-0660) exists to exercise.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] Execution detail opens a live stream for in-flight executions via the SDK's `followExecutionEvents`; events render (in `sequence_num` order via `EventLog`) as they arrive. Status is polled (`livePoll`, 2s) so the badge transitions to terminal when the run finishes.
- [x] `useLiveExecutionEvents` bridges the SDK's async iterator into state, started only while non-terminal and **aborted via `AbortController` on unmount / when terminal** — no leaked WS (NFR-002).
- [x] Dedup + reconnect delegated to the SDK; the UI additionally dedups by `sequence_num` (live) and at the REST/live seam. No-dup/no-gap behavior is asserted by the T-0660/T-0661 harness (can't be exercised here — empty dev tenant).
- [x] **OQ-6 merge defined + implemented**: REST history is the backfill, the live tail layers on top, `mergeEvents` dedups on `sequence_num` (monotonic per execution), last-write-wins at the seam; on the terminal transition the REST log is refetched for the authoritative final history. Recorded below.
- [x] Terminal UX explicit: a "live"/"streaming…" indicator only while non-terminal; terminal executions never open a stream (the `enabled` gate is `!isTerminalStatus`), they just render REST history.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
`useLiveExecutionEvents(execId)` wraps `for await (const e of followExecutionEvents(...))`, pushing onto the normalized event array from T-0653 with a dedup key. Use an `AbortController`/cancellation tied to component lifecycle. Decide the merge: fetch REST history first (up to `max_known_id` semantics), then start the stream; the delivery WS resyncs unacked rows so the seam must dedup.

### Dependencies
Blocked by CLOACI-T-0653 (extends its execution-detail view + event-log model). Exercised end-to-end by T-0660 (workload generator produces the live runs) and T-0661 (Playwright asserts the stream).

### Risk Considerations
The merge seam (OQ-6) is the subtle part — at-least-once delivery means the WS may replay events already in the REST history; dedup must be correct. WS auth uses the SDK's ticket flow (no key in the URL beyond the single-use ticket). Cancellation correctness matters (navigating away mid-stream must not leak sockets).

## Status Updates **[REQUIRED]**

**2026-06-11** — Implemented on `i0117-web-ui` (the real-time centerpiece):
- `util/events.ts` `mergeEvents(...sources)` — the **OQ-6 seam**: dedup on `sequence_num`, last-write-wins, order delegated to `EventLog`'s sort.
- `api/executions.ts`: `useLiveExecutionEvents(id, enabled)` consumes `followExecutionEvents(client, id, { signal })` in a `useEffect`, dedups live frames by `sequence_num`, and aborts the `AbortController` on unmount / enabled-flip (clean WS teardown). `useExecution(id, { livePoll })` re-polls status every 2s while non-terminal via a terminal-aware `refetchInterval`.
- `routes/ExecutionDetail.tsx` rewritten: `enabled = !isTerminalStatus(status)` gates the stream; merges REST history + live tail; "live"/"streaming…" indicators while in progress; refetches REST events on the terminal transition for authoritative final history. Terminal executions never open a socket.
- **OQ-6 decision (recorded in the initiative's open questions too):** backfill-then-tail with `sequence_num` dedup. The at-least-once delivery WS may replay rows already in REST history — the merge collapses them. WS auth is the SDK's single-use ticket flow (no key in the URL).
- **Verified:** `npm run typecheck` clean, `npm test` 2/2. The live no-dup/no-gap behavior fundamentally needs *running executions* — exercised by T-0660 (workload generator) + T-0661 (Playwright UAT), as scoped. The dev tenant is empty so there's nothing to stream yet.