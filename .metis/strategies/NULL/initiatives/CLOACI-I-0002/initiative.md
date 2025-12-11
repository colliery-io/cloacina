---
id: introduce-dispatcher-layer-for
level: initiative
title: "Introduce Dispatcher Layer for Executor Decoupling"
short_code: "CLOACI-I-0002"
created_at: 2025-11-28T15:27:33.310692+00:00
updated_at: 2025-12-10T16:44:23.281489+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/ready"


exit_criteria_met: false
estimated_complexity: L
strategy_id: NULL
initiative_id: introduce-dispatcher-layer-for
---

# Introduce Dispatcher Layer for Executor Decoupling Initiative

## Context

The current cloacina architecture uses a 2-tier model where the TaskScheduler determines task readiness and writes state to the database, while executors poll the database for "Ready" tasks via `claim_ready_task()`. This creates tight coupling between the scheduler and executor through the shared database state pattern.

**Current Architecture:**
```
Scheduler ──mark_ready()──> Database <──poll/claim──> Executor
```

While the scheduler and executor have no direct code dependencies, they are coupled through:
1. The executor must implement DB-polling pattern (`claim_ready_task`)
2. All executors must understand the "Ready" state convention
3. Adding new executor backends requires implementing the same DB-polling logic
4. No clean abstraction for routing tasks to different execution paradigms

**Coupling Point:** `task_scheduler.rs:714-718` - when dependencies are satisfied, the scheduler calls `dal.task_execution().mark_ready()`. The executor independently polls for this state.

This architecture works well for the current single-executor model but creates friction for supporting multiple execution backends (Kubernetes jobs, serverless functions, message queues, remote workers).

## Goals & Non-Goals

**Goals:**
- Introduce a Dispatcher abstraction between scheduler and executor
- Define a clean event contract (`TaskReadyEvent`) that all executors consume
- Enable pluggable executor backends without DB-polling knowledge
- Maintain horizontal scheduler scaling (many schedulers, DB-coordinated)
- Preserve existing ThreadTaskExecutor behavior as default backend

**Non-Goals:**
- Changing scheduler internals or DB coordination logic
- Implementing additional executor backends (future initiative)
- Modifying task/workflow definition APIs
- Breaking existing public APIs

## Architecture

### Overview

Introduce a 3-tier architecture with the Dispatcher as a routing shim:

```
┌───────────────┐                    ┌────────────────┐              ┌───────────────┐
│   Scheduler   │ ── TaskReady ───> │   Dispatcher   │ ── event ──> │  Executor(s)  │
│  (DB-bound)   │      Event         │   (the shim)   │              │ (any paradigm)│
└───────────────┘                    └────────────────┘              └───────────────┘
       │
       │ still writes to DB for
       │ state, locking, audit
       v
┌───────────────┐
│   Database    │
└───────────────┘
```

The scheduler remains DB-backed and handles its own coordination via database locks. The only change is that instead of the executor polling for "Ready" tasks, the scheduler pushes through a Dispatcher that routes to the appropriate executor.

### Component Responsibilities

| Component | Responsibility |
|-----------|----------------|
| TaskScheduler | Determines task readiness, manages DB state, emits TaskReadyEvent via Dispatcher |
| Dispatcher | Routes events to appropriate executor based on configuration |
| Executor | Receives events and executes according to its paradigm (thread, k8s, queue, etc.) |

### Module Structure

```
cloacina/src/
├── dispatcher/
│   ├── mod.rs              # Module exports
│   ├── types.rs            # TaskReadyEvent, DispatchError, RoutingConfig
│   ├── traits.rs           # Dispatcher trait, Executor trait
│   ├── router.rs           # Default routing implementation
│   └── default.rs          # DefaultDispatcher implementation
├── executor/
│   ├── mod.rs              # Updated exports
│   ├── traits.rs           # Updated Executor trait (receives events)
│   ├── thread_executor.rs  # Renamed, implements new Executor trait
│   └── ... (existing)
```

## Detailed Design

### Core Types

```rust
// dispatcher/types.rs

/// Event emitted when a task becomes ready for execution
#[derive(Debug, Clone)]
pub struct TaskReadyEvent {
    /// Unique task execution ID
    pub task_execution_id: Uuid,
    /// Parent pipeline execution ID
    pub pipeline_execution_id: Uuid,
    /// Fully qualified task name (namespace::task)
    pub task_name: String,
    /// Current attempt number
    pub attempt: i32,
    /// Pre-resolved execution context from dependencies
    pub context: Context<serde_json::Value>,
}

/// Configuration for task routing
#[derive(Debug, Clone)]
pub struct RoutingConfig {
    /// Default executor key when no rules match
    pub default_executor: String,
    /// Routing rules evaluated in order
    pub rules: Vec<RoutingRule>,
}

#[derive(Debug, Clone)]
pub struct RoutingRule {
    /// Glob pattern to match task names (e.g., "ml::*", "heavy::*")
    pub task_pattern: String,
    /// Executor key to route matching tasks to
    pub executor: String,
}
```

### Dispatcher Trait

```rust
// dispatcher/traits.rs

/// Dispatcher routes task-ready events to appropriate executors
#[async_trait]
pub trait Dispatcher: Send + Sync {
    /// Dispatch a ready task to the appropriate executor
    async fn dispatch(&self, event: TaskReadyEvent) -> Result<(), DispatchError>;

    /// Register an executor backend
    fn register_executor(&mut self, key: &str, executor: Arc<dyn Executor>);

    /// Check if dispatcher has capacity for more work
    fn has_capacity(&self) -> bool;
}
```

### Executor Trait (Updated)

```rust
// executor/traits.rs

/// Executor receives task-ready events and executes them
#[async_trait]
pub trait Executor: Send + Sync {
    /// Execute a task from a ready event
    ///
    /// The executor is responsible for:
    /// - Running the task according to its paradigm
    /// - Reporting completion/failure back to the DAL
    /// - Managing its own concurrency limits
    async fn execute(&self, event: TaskReadyEvent) -> Result<ExecutionResult, ExecutorError>;

    /// Check if this executor can accept more work
    fn has_capacity(&self) -> bool;

    /// Get executor metrics for monitoring
    fn metrics(&self) -> ExecutorMetrics;
}

/// Result of task execution
#[derive(Debug)]
pub struct ExecutionResult {
    pub task_execution_id: Uuid,
    pub status: TaskState,
    pub output_context: Option<Context<serde_json::Value>>,
    pub error: Option<String>,
    pub duration: Duration,
}
```

### Default Dispatcher Implementation

```rust
// dispatcher/default.rs

pub struct DefaultDispatcher {
    executors: HashMap<String, Arc<dyn Executor>>,
    routing: RoutingConfig,
    dal: DAL,
}

impl DefaultDispatcher {
    pub fn new(dal: DAL, routing: RoutingConfig) -> Self {
        Self {
            executors: HashMap::new(),
            routing,
            dal,
        }
    }

    fn resolve_executor(&self, task_name: &str) -> &str {
        for rule in &self.routing.rules {
            if glob_match(&rule.task_pattern, task_name) {
                return &rule.executor;
            }
        }
        &self.routing.default_executor
    }
}

#[async_trait]
impl Dispatcher for DefaultDispatcher {
    async fn dispatch(&self, event: TaskReadyEvent) -> Result<(), DispatchError> {
        let executor_key = self.resolve_executor(&event.task_name);
        let executor = self.executors.get(executor_key)
            .ok_or(DispatchError::ExecutorNotFound(executor_key.to_string()))?;

        // Mark task as running before dispatch
        self.dal.task_execution().mark_running(event.task_execution_id).await?;

        // Dispatch to executor (fire-and-forget or await based on config)
        let result = executor.execute(event).await?;

        // Update task state based on result
        match result.status {
            TaskState::Completed => {
                if let Some(ctx) = result.output_context {
                    self.dal.context().create(&ctx).await?;
                }
                self.dal.task_execution().mark_completed(result.task_execution_id).await?;
            }
            TaskState::Failed => {
                self.dal.task_execution().mark_failed(
                    result.task_execution_id,
                    result.error.as_deref().unwrap_or("Unknown error")
                ).await?;
            }
            _ => {}
        }

        Ok(())
    }

    fn register_executor(&mut self, key: &str, executor: Arc<dyn Executor>) {
        self.executors.insert(key.to_string(), executor);
    }

    fn has_capacity(&self) -> bool {
        self.executors.values().any(|e| e.has_capacity())
    }
}
```

### Scheduler Integration

The scheduler will be modified to accept an optional Dispatcher. When present, it dispatches events instead of just marking ready:

```rust
// task_scheduler.rs changes

impl TaskScheduler {
    // New field
    dispatcher: Option<Arc<dyn Dispatcher>>,

    // Modified method
    async fn update_pipeline_task_readiness(...) -> Result<(), ValidationError> {
        for task_execution in pending_tasks {
            if dependencies_satisfied && trigger_rules_satisfied {
                // Always mark ready in DB (for state tracking/audit)
                self.dal.task_execution().mark_ready(task_execution.id).await?;

                // If dispatcher present, dispatch immediately
                if let Some(dispatcher) = &self.dispatcher {
                    let context = self.build_task_context(task_execution).await?;
                    let event = TaskReadyEvent {
                        task_execution_id: task_execution.id.into(),
                        pipeline_execution_id: task_execution.pipeline_execution_id.into(),
                        task_name: task_execution.task_name.clone(),
                        attempt: task_execution.attempt,
                        context,
                    };
                    dispatcher.dispatch(event).await?;
                }
            }
        }
    }
}
```

### Backward Compatibility

The existing `ThreadTaskExecutor` with DB polling remains available:
- If no Dispatcher is configured, executors poll as before
- `DefaultRunner` can be configured with or without Dispatcher
- Gradual migration path for existing deployments

## Alternatives Considered

### 1. Message Queue as Coordinator
**Approach:** Scheduler publishes to Kafka/SQS, executors subscribe.
**Rejected:** Adds infrastructure dependency. Dispatcher abstraction allows message queue as a future executor backend without mandating it.

### 2. Event Deduplication at Coordinator
**Approach:** Coordinator deduplicates events from multiple schedulers.
**Rejected:** Unnecessary - scheduler already handles coordination via DB locking. The scheduler that wins the lock dispatches; others don't see the task as pending.

### 3. Partitioned Schedulers
**Approach:** Each scheduler owns a subset of pipelines by hash.
**Rejected:** Over-engineering for current needs. Can be added later if redundant work becomes problematic at scale.

### 4. Direct Executor Injection into Scheduler
**Approach:** Scheduler directly calls executor methods.
**Rejected:** Creates tight coupling. Dispatcher provides clean separation and routing flexibility.

## Implementation Plan

### Phase 1: Dispatcher Foundation
- Create `dispatcher/` module structure
- Define `TaskReadyEvent`, `DispatchError`, `RoutingConfig` types
- Define `Dispatcher` and updated `Executor` traits
- Implement `DefaultDispatcher` with single-executor routing

### Phase 2: ThreadExecutor Adaptation
- Adapt `ThreadTaskExecutor` to implement new `Executor` trait
- Add `execute(TaskReadyEvent)` method alongside existing `run()` loop
- Ensure backward compatibility with polling mode

### Phase 3: Scheduler Integration
- Add optional `Dispatcher` field to `TaskScheduler`
- Modify `update_pipeline_task_readiness` to dispatch when available
- Add context building for dispatch events

### Phase 4: DefaultRunner Integration
- Add dispatcher configuration to `DefaultRunnerConfig`
- Wire dispatcher into runner initialization
- Add routing configuration options

### Phase 5: Testing & Documentation
- Unit tests for dispatcher routing logic
- Integration tests for scheduler-dispatcher-executor flow
- Update module documentation
- Add examples for custom executor implementation

## Success Criteria

- [ ] Dispatcher trait and default implementation complete
- [ ] ThreadExecutor implements new Executor trait
- [ ] Scheduler dispatches through Dispatcher when configured
- [ ] Existing polling mode continues to work (backward compatible)
- [ ] All existing tests pass
- [ ] New tests cover dispatcher routing and execution flow
- [ ] Documentation updated with dispatcher usage examples
