---
title: "cloacina::execution_planner::trigger_rules"
description: "Trigger rules for conditional task execution."
editUrl: false
sidebar:
  order: 116
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Trigger rules for conditional task execution.

This module defines the trigger rule types and operators used to determine
when tasks should be executed based on various conditions.

## Enums

### <span id="enum.TriggerRule"></span><code style="font-size: 0.85em">enum TriggerRule</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Trigger rule definitions for conditional task execution.

Trigger rules determine when a task should be executed based on various conditions.
They can be combined to create complex execution logic.

**Performance**

Trigger rule evaluation:
- Is performed in memory
- Uses efficient context lookups
- Supports early termination for All/Any rules
- Caches context values where possible

**Thread Safety**

Trigger rules are:
- Immutable after creation
- Safe to share across threads
- Free of side effects

**Examples:**

```rust
use cloacina::execution_planner::{TriggerRule, TriggerCondition, ValueOperator};
use serde_json::json;

// Always execute
let always = TriggerRule::Always;

// Execute if all conditions are met
let all_conditions = TriggerRule::All {
    conditions: vec![
        TriggerCondition::TaskSuccess { task_name: "task1".to_string() },
        TriggerCondition::ContextValue {
            key: "status".to_string(),
            operator: ValueOperator::Equals,
            value: json!("ready")
        }
    ]
};

// Execute if any condition is met
let any_condition = TriggerRule::Any {
    conditions: vec![
        TriggerCondition::TaskFailed { task_name: "task1".to_string() },
        TriggerCondition::TaskSkipped { task_name: "task2".to_string() }
    ]
};

// Execute if no conditions are met
let none_condition = TriggerRule::None {
    conditions: vec![
        TriggerCondition::ContextValue {
            key: "skip".to_string(),
            operator: ValueOperator::Exists,
            value: json!(true)
        }
    ]
};
```

#### Variants

- **`Always`** - Always execute the task (default behavior).
- **`All`** - Execute only if all conditions are met.
- **`Any`** - Execute if any condition is met.
- **`None`** - Execute only if none of the conditions are met.

### <span id="enum.TriggerCondition"></span><code style="font-size: 0.85em">enum TriggerCondition</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Individual conditions that can be evaluated for trigger rules.

Conditions are the building blocks of trigger rules, allowing tasks to be
executed based on the state of other tasks or context values.

**Performance**

Condition evaluation:
- Task state conditions use efficient database lookups
- Context value conditions are evaluated in memory
- Results are cached where possible
- Supports early termination for complex conditions

**Thread Safety**

Conditions are:
- Immutable after creation
- Safe to share across threads
- Free of side effects

**Examples:**

```rust
use cloacina::execution_planner::{TriggerCondition, ValueOperator};
use serde_json::json;

// Task state conditions
let task_success = TriggerCondition::TaskSuccess { task_name: "task1".to_string() };
let task_failed = TriggerCondition::TaskFailed { task_name: "task2".to_string() };
let task_skipped = TriggerCondition::TaskSkipped { task_name: "task3".to_string() };

// Context value conditions
let context_equals = TriggerCondition::ContextValue {
    key: "status".to_string(),
    operator: ValueOperator::Equals,
    value: json!("ready")
};

let context_exists = TriggerCondition::ContextValue {
    key: "flag".to_string(),
    operator: ValueOperator::Exists,
    value: json!(true)
};
```

#### Variants

- **`TaskSuccess`** - Condition based on successful task completion.
- **`TaskFailed`** - Condition based on task failure.
- **`TaskSkipped`** - Condition based on task being skipped.
- **`ContextValue`** - Condition based on context value evaluation.

### <span id="enum.ValueOperator"></span><code style="font-size: 0.85em">enum ValueOperator</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Operators for evaluating context values in trigger conditions.

These operators define how context values should be compared and evaluated
in trigger conditions.

**Performance**

Operator evaluation:
- Uses efficient type-specific comparisons
- Supports early termination where possible
- Handles type coercion gracefully
- Caches results for repeated evaluations

**Thread Safety**

Operators are:
- Immutable
- Safe to share across threads
- Free of side effects

**Examples:**

```rust
use cloacina::execution_planner::ValueOperator;
use serde_json::json;

// Basic comparisons
let equals = ValueOperator::Equals;      // ==
let not_equals = ValueOperator::NotEquals; // !=
let greater = ValueOperator::GreaterThan;  // >
let less = ValueOperator::LessThan;       // <

// String operations
let contains = ValueOperator::Contains;     // "hello".contains("ell")
let not_contains = ValueOperator::NotContains; // !"hello".contains("xyz")

// Existence checks
let exists = ValueOperator::Exists;       // key exists
let not_exists = ValueOperator::NotExists; // key doesn't exist
```

#### Variants

- **`Equals`** - Exact equality comparison.
- **`NotEquals`** - Inequality comparison.
- **`GreaterThan`** - Greater than comparison (for numbers).
- **`LessThan`** - Less than comparison (for numbers).
- **`Contains`** - Contains check (for strings and arrays).
- **`NotContains`** - Does not contain check.
- **`Exists`** - Key exists in context.
- **`NotExists`** - Key does not exist in context.
