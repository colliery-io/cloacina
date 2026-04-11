# cloacina::execution_planner::trigger_rules <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Trigger rules for conditional task execution.

This module defines the trigger rule types and operators used to determine
when tasks should be executed based on various conditions.

## Enums

### `cloacina::execution_planner::trigger_rules::TriggerRule` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Trigger rule definitions for conditional task execution.

Trigger rules determine when a task should be executed based on various conditions.
They can be combined to create complex execution logic.

**Examples:**

```rust,ignore
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



### `cloacina::execution_planner::trigger_rules::TriggerCondition` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Individual conditions that can be evaluated for trigger rules.

Conditions are the building blocks of trigger rules, allowing tasks to be
executed based on the state of other tasks or context values.

**Examples:**

```rust,ignore
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



### `cloacina::execution_planner::trigger_rules::ValueOperator` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Operators for evaluating context values in trigger conditions.

These operators define how context values should be compared and evaluated
in trigger conditions.

**Examples:**

```rust,ignore
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
