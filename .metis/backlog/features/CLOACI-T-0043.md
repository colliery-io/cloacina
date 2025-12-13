---
id: callbacks-add-callback-hooks-for
level: task
title: "Callbacks - Add callback hooks for task and workflow state transitions"
short_code: "CLOACI-T-0043"
created_at: 2025-12-13T14:58:58.132159+00:00
updated_at: 2025-12-13T15:23:40.018803+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/todo"


exit_criteria_met: false
strategy_id: NULL
initiative_id: NULL
---

# Callbacks - Add callback hooks for task and workflow state transitions

## Objective

Allow tasks and workflows to specify callback functions (already in scope) that are invoked on state transitions, enabling alerting, monitoring, and custom integrations. No registration system - just direct function references resolved at compile time.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (nice to have)

### Business Justification
- **User Value**: Enables notifications on workflow failures/completions, external monitoring integration, and custom cleanup operations
- **Business Value**: Better observability, faster incident response, extensible integration points
- **Effort Estimate**: M

## Acceptance Criteria

## Acceptance Criteria

- [ ] Task-level callbacks: `on_failure`, `on_success` parameters in `#[task()]` macro accept function paths
- [ ] Workflow-level callbacks: `on_workflow_complete`, `on_workflow_failure` in `workflow!` macro accept function paths
- [ ] `on_failure` callback signature receives the error: `async fn(task_id: &str, error: &TaskError, context: &Context<Value>)`
- [ ] `on_success` callback signature: `async fn(task_id: &str, context: &Context<Value>)`
- [ ] Callbacks are async and non-blocking to workflow execution
- [ ] Callback failures do not fail the workflow (errors logged but isolated)
- [ ] Documentation with examples for alerting, monitoring, and cleanup use cases

## Implementation Notes

### Proposed Interface

```rust
// Task-level callbacks - just reference functions in scope
#[task(
    id = "critical_task",
    dependencies = [],
    on_failure = alerts::notify_failure,  // module path
    on_success = log_completion           // local function
)]
async fn critical_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    // implementation
}

// Workflow-level callbacks
let workflow = workflow! {
    name: "data_pipeline",
    description: "Critical data processing",
    tasks: [extract, transform, load],
    on_workflow_complete: notify_completion,
    on_workflow_failure: escalate_alert
};

// Callback signatures - on_failure receives the error
async fn notify_failure(task_id: &str, error: &TaskError, context: &Context<Value>) {
    // send alert with error details
}

async fn log_completion(task_id: &str, context: &Context<Value>) {
    // log success
}
```

### Technical Approach
- Extend `#[task()]` macro to accept function path parameters (`on_failure`, `on_success`)
- Macro resolves function references at compile time - no runtime registration
- Generated code calls the function directly at appropriate state transitions
- Wrap callback invocation in error isolation (catch panics/errors, log, continue)

### Dependencies
- Task state machine
- Workflow executor

### Risk Considerations
- Callback error handling (must not fail workflow - wrap in catch/log)
- Compile-time validation of function signatures

## Status Updates

- **2025-12-13**: Created from GitHub issue #2
- **2025-12-13**: Simplified design - removed registration system, callbacks are just function references resolved at compile time. Clarified `on_failure` signature must receive error.
