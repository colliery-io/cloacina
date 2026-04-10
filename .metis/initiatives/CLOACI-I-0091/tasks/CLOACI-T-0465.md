---
id: create-runtime-struct-wrapping
level: task
title: "Create Runtime struct wrapping task, workflow, trigger, and stream backend registries"
short_code: "CLOACI-T-0465"
created_at: 2026-04-09T16:59:29.647623+00:00
updated_at: 2026-04-09T17:25:35.127789+00:00
parent: CLOACI-I-0091
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0091
---

# Create Runtime struct wrapping task, workflow, trigger, and stream backend registries

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0091]]

## Objective

Create a `Runtime` struct that owns scoped instances of the 4 global registries currently at process-static scope. Provide `Runtime::from_global()` to snapshot the current globals into a scoped instance. This is the foundation — no existing behavior changes.

**Effort**: 1-2 days

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] New `pub struct Runtime` in `crates/cloacina/src/runtime.rs` (new module)
- [ ] Runtime holds: `task_registry: HashMap<TaskNamespace, TaskConstructor>`, `workflow_registry: HashMap<String, WorkflowConstructor>`, `trigger_registry: HashMap<String, TriggerConstructor>`, `stream_backend_registry: StreamBackendRegistry`
- [ ] `Runtime::new()` creates an empty runtime
- [ ] `Runtime::from_global()` snapshots all 4 global registries into a new scoped instance
- [ ] Accessor methods: `runtime.get_task(&namespace)`, `runtime.get_workflow(name)`, `runtime.register_task(namespace, constructor)`, etc.
- [ ] `Runtime` is `Clone` + `Send` + `Sync` (wrapped in `Arc` internally)
- [ ] Global functions (`global_task_registry()`, `get_task()`, etc.) continue to work unchanged
- [ ] Unit tests verify: `from_global()` captures registered tasks, scoped mutations don't affect globals
- [ ] All existing tests pass (this is purely additive)

## Implementation Notes

### Technical Approach

```rust
// crates/cloacina/src/runtime.rs
pub struct Runtime {
    tasks: Arc<RwLock<HashMap<TaskNamespace, TaskConstructor>>>,
    workflows: Arc<RwLock<HashMap<String, WorkflowConstructor>>>,
    triggers: Arc<RwLock<HashMap<String, TriggerConstructor>>>,
    stream_backends: Arc<Mutex<StreamBackendRegistry>>,
}

impl Runtime {
    pub fn new() -> Self { /* empty hashmaps */ }

    pub fn from_global() -> Self {
        // Snapshot current global registries
        let tasks = global_task_registry().read().clone();
        let workflows = global_workflow_registry().read().clone();
        let triggers = global_trigger_registry().read().clone();
        let stream_backends = global_stream_backend_registry().lock().clone();
        Self { /* wrap in Arc<RwLock> */ }
    }

    pub fn get_task(&self, ns: &TaskNamespace) -> Option<Arc<dyn Task>> { ... }
    pub fn register_task(&self, ns: TaskNamespace, ctor: impl Fn() -> Arc<dyn Task>) { ... }
    // etc.
}
```

Key types to reference:
- `GLOBAL_TASK_REGISTRY` in `task.rs:637` — `Lazy<Arc<RwLock<HashMap<TaskNamespace, Box<dyn Fn() -> Arc<dyn Task>>>>>>`
- `GLOBAL_WORKFLOW_REGISTRY` in `workflow/registry.rs:36`
- `GLOBAL_TRIGGER_REGISTRY` in `trigger/registry.rs:36`
- `GLOBAL_REGISTRY` (stream backends) in `computation_graph/stream_backend.rs:138`

### Dependencies
None. This is additive.

## Status Updates

- **2026-04-09**: Created `runtime.rs` with `Runtime` struct wrapping 3 registries (tasks, workflows, triggers) via `Arc<RuntimeInner>` with `parking_lot::RwLock<HashMap>` per registry. Stream backend registry omitted — not cloneable (`Box<dyn Fn>` factories) and not a source of test serialization issues. `from_global()` snapshots by calling each constructor once and capturing the instance in a clone-based closure. 7 unit tests pass: empty runtime, register/get, scoped isolation, clone sharing, from_global, workflow_names, debug format. Exported as `pub use runtime::Runtime` from lib.rs. Compiles clean.
