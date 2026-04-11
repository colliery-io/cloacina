---
title: "Trigger Rules"
description: "How trigger rules control conditional task execution based on workflow state and context data"
date: 2024-03-19
weight: 3
---

## What Are Trigger Rules?

Trigger rules are conditional execution criteria that control **when** a task runs, beyond
simple dependency completion. Every task in a workflow has dependencies (structural connections)
that define the execution graph, but trigger rules add a second layer of logic: even after
all dependencies finish, the task only executes if its trigger rule evaluates to `true`.

Think of it this way:

- **Dependencies** answer: "What must finish before I can run?"
- **Trigger rules** answer: "Given what happened upstream, *should* I run?"

This distinction enables powerful patterns such as fallback tasks that activate only when a
primary task fails, conditional branching based on runtime data, and cleanup tasks that run
regardless of upstream outcomes.

### Default Behavior

When a task has no explicit `trigger_rules` attribute, it receives the implicit rule `Always`.
This means the task will execute whenever all of its dependencies reach a terminal state
(Completed, Failed, or Skipped) --- which, for the common case where all dependencies succeed,
is equivalent to "run when ready."

```rust
// No trigger_rules attribute: uses implicit Always rule
#[task(
    id = "process",
    dependencies = ["fetch"]
)]
pub async fn process(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // Runs whenever "fetch" finishes (success, failure, or skip)
    Ok(())
}
```

## Rule Types

Trigger rules are built from **conditions** --- individual predicates that inspect the state of
the workflow at evaluation time. There are two families of conditions: task status conditions
and context value conditions.

### Task Status Conditions

These conditions inspect the terminal state of another task in the same workflow execution.

| Condition | Syntax | Evaluates to `true` when... |
|-----------|--------|----------------------------|
| Task Success | `task_success("task_id")` | The named task completed successfully |
| Task Failed | `task_failed("task_id")` | The named task failed (after all retries exhausted) |
| Task Skipped | `task_skipped("task_id")` | The named task was skipped |

The `task_id` string must match the `id` attribute of another task in the workflow. The
referenced task must also be listed in the `dependencies` array — trigger rules only
evaluate after all declared dependencies reach a terminal state.

```rust
// This task runs only when fetch_data has FAILED
#[task(
    id = "cached_data",
    dependencies = ["fetch_data"],
    trigger_rules = task_failed("fetch_data")
)]
pub async fn cached_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("Using cached data as fallback");
    // ...
    Ok(())
}
```

### Context Value Conditions

These conditions evaluate values stored in the workflow context --- the shared key-value store
that tasks read from and write to during execution. Context value conditions use a
three-argument form:

```
context_value("key", operator, value)
```

Where `key` is the context key to look up, `operator` is one of the comparison operators,
and `value` is the expected value to compare against.

#### Operators

| Operator | Syntax | Description |
|----------|--------|-------------|
| Equals | `equals` | Exact equality (`==`) |
| Not Equals | `not_equals` | Inequality (`!=`) |
| Greater Than | `greater_than` | Numeric greater-than (`>`) |
| Less Than | `less_than` | Numeric less-than (`<`) |
| Contains | `contains` | Substring check for strings, element check for arrays |
| Not Contains | `not_contains` | Negation of Contains |
| Exists | `exists` | Key is present in context (value argument is ignored) |
| Not Exists | `not_exists` | Key is absent from context (value argument is ignored) |

Numeric operators (`greater_than`, `less_than`) compare values as 64-bit floats. If either
value is not numeric, the condition evaluates to `false`. The `contains` operator works on
both strings (substring match) and arrays (element membership).

```rust
// Runs only when process_data succeeded AND quality score exceeds 80
#[task(
    id = "high_quality_processing",
    dependencies = ["process_data"],
    trigger_rules = all(
        task_success("process_data"),
        context_value("data_quality_score", greater_than, 80)
    )
)]
pub async fn high_quality_processing(
    context: &mut Context<serde_json::Value>,
) -> Result<(), TaskError> {
    info!("Processing high quality data");
    // ...
    Ok(())
}
```

## Combinators

Individual conditions are composed into rules using logical combinators.

### `all(...)` --- AND Logic

All conditions must evaluate to `true` for the rule to pass. Evaluation uses short-circuit
semantics: the first `false` condition stops evaluation immediately.

```rust
trigger_rules = all(
    task_success("process_data"),
    context_value("data_quality_score", greater_than, 80)
)
```

Both conditions must hold: the upstream task must have succeeded AND the quality score in
context must exceed 80.

### `any(...)` --- OR Logic

Any single condition evaluating to `true` is sufficient for the rule to pass. Evaluation
short-circuits on the first `true` condition.

```rust
trigger_rules = any(
    task_success("high_quality_processing"),
    task_success("low_quality_processing")
)
```

The task runs if either the high-quality or low-quality processing path completed successfully.

### `none(...)` --- NOR Logic

The rule passes only if **none** of the conditions evaluate to `true`. This is the logical
negation of `any()`.

```rust
trigger_rules = none(
    task_failed("validation"),
    context_value("skip_processing", equals, true)
)
```

The task runs only if validation did not fail AND the skip flag is not set.

### Nesting Combinators

Combinators can be nested to express complex logic:

```rust
trigger_rules = all(
    task_success("fetch"),
    any(
        context_value("mode", equals, "fast"),
        context_value("priority", greater_than, 5)
    )
)
```

This reads: "Run if fetch succeeded AND (mode is fast OR priority is above 5)."

### Standalone Conditions as Rules

When you use a single condition without an explicit combinator, the macro wraps it in an
`all(...)` with one element. These two are equivalent:

```rust
trigger_rules = task_failed("fetch_data")
// is equivalent to:
trigger_rules = all(task_failed("fetch_data"))
```

## Evaluation Semantics

Understanding when and how trigger rules are evaluated is critical for building correct
workflows.

### Evaluation Timing

Trigger rules are evaluated **after all dependencies reach a terminal state**. A dependency
is in a terminal state when its status is one of: `Completed`, `Failed`, or `Skipped`. Until
every dependency has reached one of these states, the task remains in `Pending` and its
trigger rules are not evaluated.

The scheduling loop checks pending tasks on each iteration:

1. For each pending task, check whether all dependencies are in terminal states.
2. If yes, evaluate the task's trigger rule against the current workflow state and context.
3. If the rule evaluates to `true`, mark the task as `Ready` for execution.
4. If the rule evaluates to `false`, mark the task as `Skipped`.

### Skip Propagation

When a task is skipped (because its trigger rule evaluated to `false`), that skip status
propagates downstream. Any task whose dependency was skipped will have its own trigger rules
evaluated once all its dependencies reach terminal states --- but because the dependency
is `Skipped` rather than `Completed`, conditions like `task_success("skipped_task")` will
return `false`.

This means that if a task in the middle of a chain is skipped, downstream tasks will also
be skipped unless they have trigger rules that explicitly handle the skip case (e.g., using
`task_skipped("upstream")` or `any(...)` with alternative conditions).

### Terminal State Summary

| Dependency State | `task_success` | `task_failed` | `task_skipped` |
|-----------------|----------------|---------------|----------------|
| Completed | `true` | `false` | `false` |
| Failed | `false` | `true` | `false` |
| Skipped | `false` | `false` | `true` |

### Context Availability

When a `context_value` condition is evaluated, the context is loaded from the task's
dependencies:

- **No dependencies**: The initial pipeline context is used.
- **Single dependency**: The context saved by that dependency task is used.
- **Multiple dependencies**: Contexts from all dependencies are merged (later dependencies
  override earlier ones for conflicting keys).

This means context value conditions can only inspect values that were written by upstream tasks
or provided in the initial workflow context.

## How Trigger Rules Interact with Retries

Cloacina supports configurable retry policies on tasks (exponential backoff, fixed delay, etc.).
The interaction between retries and trigger rules follows a clear rule:

**`task_failed` fires only after ALL retry attempts are exhausted.**

During retries, the failing task remains in-progress from the perspective of downstream tasks.
Its dependents stay in `Pending` state because the dependency has not reached a terminal state
yet. Only once the maximum retry attempts are exceeded does the task transition to `Failed`,
which then:

1. Moves the task status to `Failed` (terminal state).
2. Unblocks downstream tasks whose dependencies now all have terminal states.
3. Allows `task_failed("that_task")` conditions to evaluate to `true`.

```rust
// This task retries up to 3 times with exponential backoff
#[task(
    id = "fetch_data",
    dependencies = [],
    retry_attempts = 3,
    retry_delay_ms = 1000,
    retry_backoff = "exponential"
)]
pub async fn fetch_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // If this fails, it will be retried up to 3 times before being marked Failed
    // ...
    Ok(())
}

// This fallback ONLY runs after all 3 retries of fetch_data are exhausted
#[task(
    id = "cached_data",
    dependencies = ["fetch_data"],
    trigger_rules = task_failed("fetch_data")
)]
pub async fn cached_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // Will not be evaluated until fetch_data has attempted 3 times and given up
    // ...
    Ok(())
}
```

## Design Patterns

### Fallback Pattern

The most common trigger rule pattern: a primary task attempts an operation, and if it fails
(after retries), a fallback task provides an alternative path forward.

```rust
// Primary: fetch from external API
#[task(
    id = "fetch_data",
    dependencies = [],
    retry_attempts = 3,
    retry_delay_ms = 1000,
    retry_backoff = "exponential",
    on_failure = on_data_fetch_failure
)]
pub async fn fetch_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("Attempting to fetch data from external source");
    // ... network call that might fail
    context.insert("raw_data", data)?;
    Ok(())
}

// Fallback: load from local cache when API is unavailable
#[task(
    id = "cached_data",
    dependencies = ["fetch_data"],
    trigger_rules = task_failed("fetch_data")
)]
pub async fn cached_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("Using cached data as fallback");
    let cached_data = load_from_cache().await?;
    context.insert("raw_data", cached_data)?;
    Ok(())
}
```

Downstream tasks that depend on both `fetch_data` and `cached_data` will receive the `raw_data`
context key regardless of which path was taken, because both tasks write to the same key.

### Conditional Branching

Use context values to route execution down different paths based on runtime data:

```rust
// Processing step writes a quality score to context
#[task(
    id = "process_data",
    dependencies = ["fetch_data", "cached_data"]
)]
pub async fn process_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // Calculate quality score and write to context
    context.insert("data_quality_score", json!(avg_quality))?;
    Ok(())
}

// High-quality path: expensive processing for good data
#[task(
    id = "high_quality_processing",
    dependencies = ["process_data"],
    trigger_rules = all(
        task_success("process_data"),
        context_value("data_quality_score", greater_than, 80)
    )
)]
pub async fn high_quality_processing(
    context: &mut Context<serde_json::Value>,
) -> Result<(), TaskError> {
    info!("Processing high quality data");
    // Advanced validation, premium processing...
    Ok(())
}

// Low-quality path: basic processing for poor data
#[task(
    id = "low_quality_processing",
    dependencies = ["process_data"],
    trigger_rules = all(
        task_success("process_data"),
        context_value("data_quality_score", less_than, 81)
    )
)]
pub async fn low_quality_processing(
    context: &mut Context<serde_json::Value>,
) -> Result<(), TaskError> {
    info!("Processing low quality data");
    // Basic validation only...
    Ok(())
}
```

Note the overlapping boundary (>80 vs <81): this ensures exactly one branch is taken for
any integer quality score. For floating-point scores, you may need to handle the boundary
case explicitly.

### Error Notification

A notification task that fires only when critical operations fail, useful for alerting:

```rust
#[task(
    id = "failure_notification",
    dependencies = ["fetch_data", "cached_data"],
    trigger_rules = all(
        task_failed("fetch_data"),
        task_failed("cached_data")
    )
)]
pub async fn failure_notification(
    context: &mut Context<serde_json::Value>,
) -> Result<(), TaskError> {
    error!("Critical failure: Both fetch and cache operations failed");
    context.insert("failure_notification", json!({
        "status": "critical_failure",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "message": "Both data sources failed",
        "alert_level": "high"
    }))?;
    Ok(())
}
```

This task only runs when **both** the primary fetch and the cache fallback have failed, indicating
a critical system issue that requires operator attention.

### Convergent Final Report

Use `any(...)` to converge multiple conditional branches into a single final task:

```rust
#[task(
    id = "final_report",
    dependencies = ["high_quality_processing", "low_quality_processing"],
    trigger_rules = any(
        task_success("high_quality_processing"),
        task_success("low_quality_processing")
    ),
    on_success = on_task_success
)]
pub async fn final_report(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("Generating final execution report");
    // This runs regardless of which quality path was taken,
    // as long as one of them succeeded.
    Ok(())
}
```

### Always-Run Cleanup

For tasks that must run regardless of upstream outcomes (logging, resource release, temporary
file cleanup), use dependencies to ensure ordering but rely on the implicit evaluation
behavior:

```rust
#[task(
    id = "cleanup",
    dependencies = ["processing"],
    trigger_rules = any(
        task_success("processing"),
        task_failed("processing"),
        task_skipped("processing")
    )
)]
pub async fn cleanup(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // Runs no matter what happened to "processing"
    release_resources().await?;
    Ok(())
}
```

By listing all three terminal states in an `any(...)`, the cleanup task runs regardless of
the outcome. This is effectively equivalent to `always` but scoped to a specific dependency
relationship.

## Complete Example: Resilient Pipeline

The following is the full workflow from [Tutorial 04: Error Handling](/tutorials/workflows/library/04-error-handling/), demonstrating how trigger rules compose to create a resilient
data pipeline with fallbacks, conditional branching, and error notification:

```rust
#[workflow(
    name = "resilient_pipeline",
    description = "Pipeline demonstrating error handling patterns"
)]
pub mod resilient_pipeline {
    // Task 1: Fetch from external source with retries
    #[task(
        id = "fetch_data",
        dependencies = [],
        retry_attempts = 3,
        retry_delay_ms = 1000,
        retry_backoff = "exponential",
        on_failure = on_data_fetch_failure
    )]
    pub async fn fetch_data(ctx: &mut Context<Value>) -> Result<(), TaskError> { /* ... */ }

    // Task 2: Fallback to cache (only when fetch fails)
    #[task(
        id = "cached_data",
        dependencies = ["fetch_data"],
        trigger_rules = task_failed("fetch_data")
    )]
    pub async fn cached_data(ctx: &mut Context<Value>) -> Result<(), TaskError> { /* ... */ }

    // Task 3: Process whatever data we got
    #[task(
        id = "process_data",
        dependencies = ["fetch_data", "cached_data"],
        on_success = on_task_success,
        on_failure = on_task_failure
    )]
    pub async fn process_data(ctx: &mut Context<Value>) -> Result<(), TaskError> { /* ... */ }

    // Task 4: High-quality path (quality > 80)
    #[task(
        id = "high_quality_processing",
        dependencies = ["process_data"],
        trigger_rules = all(
            task_success("process_data"),
            context_value("data_quality_score", greater_than, 80)
        )
    )]
    pub async fn high_quality_processing(ctx: &mut Context<Value>) -> Result<(), TaskError> { /* ... */ }

    // Task 5: Low-quality path (quality < 81)
    #[task(
        id = "low_quality_processing",
        dependencies = ["process_data"],
        trigger_rules = all(
            task_success("process_data"),
            context_value("data_quality_score", less_than, 81)
        )
    )]
    pub async fn low_quality_processing(ctx: &mut Context<Value>) -> Result<(), TaskError> { /* ... */ }

    // Task 6: Alert on total failure
    #[task(
        id = "failure_notification",
        dependencies = ["fetch_data", "cached_data"],
        trigger_rules = all(
            task_failed("fetch_data"),
            task_failed("cached_data")
        )
    )]
    pub async fn failure_notification(ctx: &mut Context<Value>) -> Result<(), TaskError> { /* ... */ }

    // Task 7: Final report (converges branches)
    #[task(
        id = "final_report",
        dependencies = ["high_quality_processing", "low_quality_processing"],
        trigger_rules = any(
            task_success("high_quality_processing"),
            task_success("low_quality_processing")
        ),
        on_success = on_task_success
    )]
    pub async fn final_report(ctx: &mut Context<Value>) -> Result<(), TaskError> { /* ... */ }
}
```

The execution flow looks like:

```
fetch_data (retries 3x)
    |
    +-- [success] --> process_data --> [quality > 80] --> high_quality_processing --> final_report
    |                      |
    |                      +---------> [quality < 81] --> low_quality_processing  --> final_report
    |
    +-- [failure] --> cached_data
    |                    |
    |                    +-- [success] --> process_data --> ...
    |                    |
    |                    +-- [failure] --> failure_notification
```

## Operational Considerations

### Rule Design Guidelines

1. **Start simple.** Use the implicit `Always` rule by default. Add trigger rules only when
   you need conditional execution.

2. **Keep rules focused.** Each trigger rule should express a single, clear condition. Prefer
   readable rules over clever compositions.

3. **Document boundaries.** When using context value conditions for branching, clearly document
   the expected value ranges and ensure branches are mutually exclusive or intentionally
   overlapping.

4. **Test both paths.** Every trigger rule creates at least two execution paths (rule passes,
   rule fails). Ensure both paths are tested.

### Dependencies vs. Trigger Rules

Do not use trigger rules as a substitute for proper dependencies. Dependencies define the
structural graph and determine execution ordering. Trigger rules add conditional logic
on top of that ordering.

If you need task B to always run after task A, make A a dependency of B. If you need task B
to run after A only when A succeeds, make A a dependency of B and add
`trigger_rules = task_success("A")`.

### Context Key Conventions

When using `context_value` conditions, maintain consistent key naming across your workflow.
Document which tasks write which keys and what values are expected. Consider using a shared
constants module for key names to avoid typos:

```rust
const QUALITY_SCORE_KEY: &str = "data_quality_score";
const HIGH_QUALITY_THRESHOLD: i64 = 80;
```

### Debugging Trigger Rule Evaluation

The scheduler logs trigger rule evaluation at the `DEBUG` level. Enable debug logging
for the `cloacina` module to see detailed evaluation traces:

```
RUST_LOG=cloacina=debug
```

This produces output showing each condition evaluation and the final rule result:

```
Trigger rule evaluation: All(2 conditions) (task: high_quality_processing)
  └─ Condition 1: TaskSuccess { task_name: "process_data" } -> true
  └─ Condition 2: ContextValue { key: "data_quality_score", ... } -> true
Trigger rule result: All -> true (all conditions passed)
```

## API Reference

{{< api-link path="cloacina::execution_planner::TriggerRule" type="enum" display="TriggerRule" >}}

{{< api-link path="cloacina::execution_planner::TriggerCondition" type="enum" display="TriggerCondition" >}}

{{< api-link path="cloacina::execution_planner::ValueOperator" type="enum" display="ValueOperator" >}}
