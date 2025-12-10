/*
 *  Copyright 2025 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! Trigger rules for conditional task execution.
//!
//! This module defines the trigger rule types and operators used to determine
//! when tasks should be executed based on various conditions.

use serde::{Deserialize, Serialize};

/// Trigger rule definitions for conditional task execution.
///
/// Trigger rules determine when a task should be executed based on various conditions.
/// They can be combined to create complex execution logic.
///
/// # Examples
///
/// ```rust,ignore
/// use cloacina::scheduler::{TriggerRule, TriggerCondition, ValueOperator};
/// use serde_json::json;
///
/// // Always execute
/// let always = TriggerRule::Always;
///
/// // Execute if all conditions are met
/// let all_conditions = TriggerRule::All {
///     conditions: vec![
///         TriggerCondition::TaskSuccess { task_name: "task1".to_string() },
///         TriggerCondition::ContextValue {
///             key: "status".to_string(),
///             operator: ValueOperator::Equals,
///             value: json!("ready")
///         }
///     ]
/// };
///
/// // Execute if any condition is met
/// let any_condition = TriggerRule::Any {
///     conditions: vec![
///         TriggerCondition::TaskFailed { task_name: "task1".to_string() },
///         TriggerCondition::TaskSkipped { task_name: "task2".to_string() }
///     ]
/// };
///
/// // Execute if no conditions are met
/// let none_condition = TriggerRule::None {
///     conditions: vec![
///         TriggerCondition::ContextValue {
///             key: "skip".to_string(),
///             operator: ValueOperator::Exists,
///             value: json!(true)
///         }
///     ]
/// };
/// ```
///
/// # Performance
///
/// Trigger rule evaluation:
/// - Is performed in memory
/// - Uses efficient context lookups
/// - Supports early termination for All/Any rules
/// - Caches context values where possible
///
/// # Thread Safety
///
/// Trigger rules are:
/// - Immutable after creation
/// - Safe to share across threads
/// - Free of side effects
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TriggerRule {
    /// Always execute the task (default behavior).
    Always,
    /// Execute only if all conditions are met.
    All { conditions: Vec<TriggerCondition> },
    /// Execute if any condition is met.
    Any { conditions: Vec<TriggerCondition> },
    /// Execute only if none of the conditions are met.
    None { conditions: Vec<TriggerCondition> },
}

/// Individual conditions that can be evaluated for trigger rules.
///
/// Conditions are the building blocks of trigger rules, allowing tasks to be
/// executed based on the state of other tasks or context values.
///
/// # Examples
///
/// ```rust,ignore
/// use cloacina::scheduler::{TriggerCondition, ValueOperator};
/// use serde_json::json;
///
/// // Task state conditions
/// let task_success = TriggerCondition::TaskSuccess { task_name: "task1".to_string() };
/// let task_failed = TriggerCondition::TaskFailed { task_name: "task2".to_string() };
/// let task_skipped = TriggerCondition::TaskSkipped { task_name: "task3".to_string() };
///
/// // Context value conditions
/// let context_equals = TriggerCondition::ContextValue {
///     key: "status".to_string(),
///     operator: ValueOperator::Equals,
///     value: json!("ready")
/// };
///
/// let context_exists = TriggerCondition::ContextValue {
///     key: "flag".to_string(),
///     operator: ValueOperator::Exists,
///     value: json!(true)
/// };
/// ```
///
/// # Performance
///
/// Condition evaluation:
/// - Task state conditions use efficient database lookups
/// - Context value conditions are evaluated in memory
/// - Results are cached where possible
/// - Supports early termination for complex conditions
///
/// # Thread Safety
///
/// Conditions are:
/// - Immutable after creation
/// - Safe to share across threads
/// - Free of side effects
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TriggerCondition {
    /// Condition based on successful task completion.
    TaskSuccess { task_name: String },
    /// Condition based on task failure.
    TaskFailed { task_name: String },
    /// Condition based on task being skipped.
    TaskSkipped { task_name: String },
    /// Condition based on context value evaluation.
    ContextValue {
        key: String,
        operator: ValueOperator,
        value: serde_json::Value,
    },
}

/// Operators for evaluating context values in trigger conditions.
///
/// These operators define how context values should be compared and evaluated
/// in trigger conditions.
///
/// # Examples
///
/// ```rust,ignore
/// use cloacina::scheduler::ValueOperator;
/// use serde_json::json;
///
/// // Basic comparisons
/// let equals = ValueOperator::Equals;      // ==
/// let not_equals = ValueOperator::NotEquals; // !=
/// let greater = ValueOperator::GreaterThan;  // >
/// let less = ValueOperator::LessThan;       // <
///
/// // String operations
/// let contains = ValueOperator::Contains;     // "hello".contains("ell")
/// let not_contains = ValueOperator::NotContains; // !"hello".contains("xyz")
///
/// // Existence checks
/// let exists = ValueOperator::Exists;       // key exists
/// let not_exists = ValueOperator::NotExists; // key doesn't exist
/// ```
///
/// # Performance
///
/// Operator evaluation:
/// - Uses efficient type-specific comparisons
/// - Supports early termination where possible
/// - Handles type coercion gracefully
/// - Caches results for repeated evaluations
///
/// # Thread Safety
///
/// Operators are:
/// - Immutable
/// - Safe to share across threads
/// - Free of side effects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueOperator {
    /// Exact equality comparison.
    Equals,
    /// Inequality comparison.
    NotEquals,
    /// Greater than comparison (for numbers).
    GreaterThan,
    /// Less than comparison (for numbers).
    LessThan,
    /// Contains check (for strings and arrays).
    Contains,
    /// Does not contain check.
    NotContains,
    /// Key exists in context.
    Exists,
    /// Key does not exist in context.
    NotExists,
}
