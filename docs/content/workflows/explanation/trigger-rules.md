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
the workflow at evaluation time. There are two families of conditions: **task status conditions**
(inspecting whether an upstream task completed, failed, or was skipped) and **context value
conditions** (evaluating values stored in the shared workflow context).

Task status conditions use the form `task_success("task_id")`, `task_failed("task_id")`, or
`task_skipped("task_id")`. The referenced task must also appear in the `dependencies` array,
since trigger rules only evaluate after all declared dependencies reach a terminal state.

Context value conditions use the form `context_value("key", operator, value)`, where `operator`
is one of the comparison operators (`equals`, `not_equals`, `greater_than`, `less_than`,
`contains`, `not_contains`, `exists`, `not_exists`). Numeric operators compare values as 64-bit
floats; `contains` works on both strings (substring match) and arrays (element membership).

For the full syntax tables and operator details, see the
[Macro Reference]({{< ref "/workflows/reference/macros" >}}).

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

### Terminal States and Context Availability

Each terminal state (`Completed`, `Failed`, `Skipped`) maps to exactly one `true` condition
among `task_success`, `task_failed`, and `task_skipped`. See the
[Macro Reference]({{< ref "/workflows/reference/macros" >}}) for the full truth table.

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

## Common Patterns

The following patterns illustrate the key use cases for trigger rules. For implementation
details and working code, see
[Tutorial 04: Error Handling]({{< ref "/workflows/tutorials/library/04-error-handling" >}}).

- **Fallback** -- Use `task_failed("primary")` to activate a backup task when the primary
  path fails after retries. Both tasks write to the same context key so downstream consumers
  are agnostic to which path ran.

- **Conditional branching** -- Use `context_value` conditions to route execution based on
  runtime data (e.g., a quality score). Pair mutually exclusive `all(...)` rules on sibling
  tasks to create distinct processing paths. Be deliberate about boundary values to ensure
  exactly one branch fires.

- **Error notification** -- Use `all(task_failed("A"), task_failed("B"))` to fire an alerting
  task only when every recovery path has been exhausted, signaling a critical system issue.

- **Branch convergence** -- Use `any(task_success("branch_a"), task_success("branch_b"))` to
  converge multiple conditional branches into a single downstream task that runs as long as
  at least one path succeeded.

- **Always-run cleanup** -- Use `any(task_success("X"), task_failed("X"), task_skipped("X"))`
  to ensure a task runs regardless of the upstream outcome. This is useful for resource
  release, logging, or temporary file cleanup.

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

## Further Reading

- [Tutorial 04: Error Handling]({{< ref "/workflows/tutorials/library/04-error-handling" >}}) -- step-by-step walkthrough building a resilient pipeline with fallbacks, conditional branching, and error notification
- [Macro Reference]({{< ref "/workflows/reference/macros" >}}) -- full syntax tables for `trigger_rules`, condition types, operators, and combinators
- {{< api-link path="cloacina::execution_planner::TriggerRule" type="enum" display="TriggerRule" >}} -- Rust API docs
- {{< api-link path="cloacina::execution_planner::TriggerCondition" type="enum" display="TriggerCondition" >}}
- {{< api-link path="cloacina::execution_planner::ValueOperator" type="enum" display="ValueOperator" >}}
