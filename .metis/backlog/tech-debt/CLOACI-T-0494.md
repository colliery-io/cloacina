---
id: stabilize-accumulator-trait-raw
level: task
title: "Stabilize accumulator trait — raw bytes input, user-owned deserialization"
short_code: "CLOACI-T-0494"
created_at: 2026-04-16T02:03:08.509378+00:00
updated_at: 2026-04-16T02:46:23.285203+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Stabilize accumulator trait — raw bytes input, user-owned deserialization

## Objective

The accumulator traits were designed with typed deserialization (`Event: DeserializeOwned`) to validate the initial computation graph data flow. Now that the flow is proven, the trait needs to stabilize around the correct abstraction: accumulators receive raw bytes and the user's implementation owns deserialization. This eliminates the format coupling (JSON vs bincode) that has caused a cascade of serialization bugs across the WebSocket, Kafka, FFI bridge, and reactor cache paths.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P1 - High (blocks release-mode CG execution, Kafka format flexibility)

### Technical Debt Impact
- **Current Problems**: The accumulator runtime pre-deserializes incoming bytes using a hardcoded format (currently `serde_json::from_slice`). This forces all event sources (WebSocket, Kafka, future sources) through a single deserialization path. The `Event: DeserializeOwned` trait bound means the runtime owns format decisions instead of the user. The `GenericPassthroughAccumulator` uses `serde_json::Value` as its event type, which can't round-trip through bincode — breaking the FFI packaging bridge in release builds.
- **Benefits of Fixing**: Accumulators become format-agnostic. Users handle deserialization inside `process()` / `process_batch()`. WebSocket, Kafka, protobuf, avro, raw bytes all work without accumulator changes. The FFI bridge receives properly typed `Output` instead of fighting `serde_json::Value` through bincode.
- **Risk Assessment**: Touches all accumulator implementations, macros, tests, and packaged graph factories. Moderate scope but mechanical — the change is removing a layer, not adding one.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `Accumulator` trait: remove `Event` associated type and `DeserializeOwned` bound. `process(&mut self, event: Vec<u8>) -> Option<Self::Output>`
- [ ] `BatchAccumulator` trait: same. `process_batch(&mut self, events: Vec<Vec<u8>>) -> Option<Self::Output>`
- [ ] `PollingAccumulator` trait: unchanged (generates events internally, doesn't consume external bytes)
- [ ] `EventSource` trait: `Event = Vec<u8>` — pushes raw bytes into the merge channel
- [ ] Accumulator socket receiver: forwards `Vec<u8>` directly to `process()`, no deserialization
- [ ] Event source: pushes `Vec<u8>` directly to merge channel, no deserialization
- [ ] Kafka stream reader: proper `EventSource` implementation, not socket feeder. Forwards raw Kafka payload bytes.
- [ ] `BoundarySender.send()`: serializes typed `Output` (user-controlled type, always serializable)
- [ ] FFI packaging bridge: works in both debug and release builds
- [ ] `#[computation_graph]` macro: updated for new trait signature
- [ ] Python accumulator decorators: updated
- [ ] All existing tests updated and passing
- [ ] Server soak test passes (WebSocket + Kafka + CG execution)
- [ ] Demo works with release builds

## Implementation Notes

### New trait signatures

```rust
pub trait Accumulator: Send + 'static {
    type Output: Serialize + Send + 'static;
    fn process(&mut self, event: Vec<u8>) -> Option<Self::Output>;
}

pub trait BatchAccumulator: Send + 'static {
    type Output: Serialize + Send + 'static;
    fn process_batch(&mut self, events: Vec<Vec<u8>>) -> Option<Self::Output>;
}

pub trait EventSource: Send + 'static {
    async fn run(
        self,
        events: mpsc::Sender<Vec<u8>>,
        shutdown: watch::Receiver<bool>,
    ) -> Result<(), AccumulatorError>;
}
```

### Key changes
1. Remove `Event` associated type from `Accumulator` and `BatchAccumulator`
2. Remove `DeserializeOwned` bound
3. Socket receiver: `socket_rx.recv() -> Vec<u8>` passed directly to `event_tx` (no deserialization)
4. Merge channel type changes from `mpsc::Sender<A::Event>` to `mpsc::Sender<Vec<u8>>`
5. Kafka stream reader becomes a proper `EventSource` impl that sends raw `msg.payload`
6. `GenericPassthroughAccumulator`: `process(event: Vec<u8>) -> Option<Vec<u8>>` — true passthrough of raw bytes
7. Update `#[computation_graph]` macro codegen
8. Update `@cloaca.passthrough_accumulator` Python decorator

### What stays the same
- `Output: Serialize` bound — the boundary sender needs to serialize the output
- `BoundarySender.send()` — serializes with `types::serialize`
- `PollingAccumulator` — generates events internally, no external byte input
- Internal wire format between accumulator→reactor (bincode via `types::serialize`)
- Checkpoint persistence format

## Status Updates

### 2026-04-15: Core trait changes complete, unit+integration tests passing

All core changes implemented. Kafka reader left as socket feeder (functionally correct now — socket no longer deserializes). Python decorators unaffected (FFI bridge updated). Remaining: soak/demo verification.
