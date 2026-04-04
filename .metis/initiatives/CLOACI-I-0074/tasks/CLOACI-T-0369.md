---
id: reactor-receiver-strategy-executor
level: task
title: "Reactor — receiver, strategy, executor"
short_code: "CLOACI-T-0369"
created_at: 2026-04-04T22:54:48.937061+00:00
updated_at: 2026-04-04T23:12:16.856516+00:00
parent: CLOACI-I-0074
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0074
---

# Reactor — receiver, strategy, executor

## Objective

Implement the Reactor — the long-lived process that receives boundaries from accumulators, maintains cache + dirty flags, evaluates reaction criteria, and calls the compiled graph function.

Spec: CLOACI-S-0005.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `Reactor` struct: holds compiled graph function, reaction criteria, input strategy, accumulator channel, manual channel, shutdown
- [ ] Receiver task: reads `(SourceName, Vec<u8>)` from accumulator mpsc channel, updates `InputCache` via `Arc<RwLock<>>`, sets dirty flags, sends `StrategySignal` to executor
- [ ] Executor task: reads `StrategySignal`, evaluates `when_any` criteria on dirty flags, snapshots cache, calls `graph.execute(&snapshot).await`, clears dirty flags
- [ ] `DirtyFlags` struct: `HashMap<SourceName, bool>` with `set()`, `any_set()`, `clear_all()`
- [ ] `when_any` reaction criteria: fire if any dirty flag set
- [ ] `latest` input strategy: cache overwritten on each boundary, snapshot taken before execution
- [ ] Manual channel: `ForceFire` command bypasses reaction criteria, executes with current cache
- [ ] `Reactor::run()` spawns receiver + executor tasks, waits for shutdown
- [ ] Shutdown cleanly stops both tasks
- [ ] Unit tests: dirty flags, reaction criteria evaluation, cache snapshot isolation, manual fire

Place in `crates/cloacina/src/computation_graph/reactor.rs`.

### Dependencies
T-0362 (InputCache, GraphResult), T-0367 (BoundarySender — defines the channel format the reactor reads from).

## Status Updates

**2026-04-04**: Completed.
- `reactor.rs`: Reactor struct, receiver task (Arc<RwLock<InputCache>> + DirtyFlags), executor task (strategy evaluation + graph call)
- ReactionCriteria (WhenAny/WhenAll), InputStrategy (Latest/Sequential), DirtyFlags, StrategySignal, ManualCommand
- CompiledGraphFn type alias for Arc<dyn Fn(InputCache) -> Pin<Box<Future<Output=GraphResult>>>>
- 7 tests: dirty flags (when_any, when_all, clear, empty), reactor fires on boundary, manual force fire, cache snapshot isolation
