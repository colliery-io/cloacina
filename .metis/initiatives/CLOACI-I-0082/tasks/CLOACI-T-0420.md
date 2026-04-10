---
id: persistence-completion-batch
level: task
title: "Persistence completion — batch buffer, polling checkpoint, sequential queue durability"
short_code: "CLOACI-T-0420"
created_at: 2026-04-06T01:05:53.395973+00:00
updated_at: 2026-04-06T09:21:29.925269+00:00
parent: CLOACI-I-0082
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0082
---

# Persistence completion — batch buffer, polling checkpoint, sequential queue durability

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0082]]

## Objective

Add crash-resilient persistence to the three accumulator/reactor components that currently lack it: batch accumulator buffer, polling accumulator checkpoint, and sequential reactor queue.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] **PERSIST-1 (Batch):** Batch accumulator periodically snapshots its buffer to DAL via CheckpointHandle (not just on graceful shutdown). Configurable snapshot interval.
- [ ] **PERSIST-1 (Batch):** On startup, batch accumulator restores buffered events from DAL checkpoint if available
- [ ] **PERSIST-2 (Polling):** Polling accumulator saves poll state via CheckpointHandle after each successful poll that returns Some
- [ ] **PERSIST-2 (Polling):** On startup, polling accumulator restores last poll state from checkpoint if available
- [ ] **PERSIST-3 (Sequential):** Reactor persists the sequential queue BEFORE draining (snapshot of pending items), not after (when queue is empty)
- [ ] **PERSIST-3 (Sequential):** On startup, reactor loads queued items from DAL and replays them before accepting new boundaries
- [ ] State accumulator serialization normalized to use `types::serialize`/`types::deserialize` instead of hardcoded `serde_json` (MINOR-2 consistency fix)
- [ ] All existing tests pass

## Implementation Notes

### PERSIST-1: Batch buffer crash resilience (`accumulator.rs:521-581`)
- Add an optional snapshot timer to `batch_accumulator_runtime` (e.g., every 5 seconds or configurable)
- On timer tick: serialize buffer via CheckpointHandle and save to DAL
- On startup: load from checkpoint, deserialize into buffer
- On graceful shutdown: persist final buffer state before draining

### PERSIST-2: Polling accumulator checkpoint (`accumulator.rs:416-465`)
- After each successful `poll()` that returns `Some(output)`, call `checkpoint.save(&poll_state)` where poll_state is the accumulator's internal state
- This requires the `PollingAccumulator` trait to either expose serializable state or use CheckpointHandle in the runtime
- On startup: load checkpoint, pass to accumulator via init/context

### PERSIST-3: Sequential queue durability (`reactor.rs:461-484`)
- Before the drain loop, persist the current queue state to DAL
- After each item is successfully processed, update the persisted queue (remove processed item)
- On startup: load queue from DAL, prepend to any new items

### Dependencies
- T-0417 (scheduler wiring) must land first — accumulators need CheckpointHandle from the scheduler

## Status Updates

*To be added during implementation*
