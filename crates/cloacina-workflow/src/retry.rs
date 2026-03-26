/*
 *  Copyright 2025-2026 Colliery Software
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

//! # Retry Policy System
//!
//! This module provides a comprehensive retry policy system for Cloacina tasks,
//! including configurable backoff strategies, jitter, and conditional retry logic.
//!
//! ## Overview
//!
//! The retry system allows tasks to define sophisticated retry behavior:
//! - **Configurable retry limits** with per-task policies
//! - **Multiple backoff strategies** including exponential, linear, and custom
//! - **Jitter support** to prevent thundering herd problems
//! - **Conditional retries** based on error types and conditions
//!
//! ## Usage
//!
//! ```rust
//! use cloacina_workflow::retry::{RetryPolicy, BackoffStrategy, RetryCondition};
//! use std::time::Duration;
//!
//! let policy = RetryPolicy::builder()
//!     .max_attempts(5)
//!     .backoff_strategy(BackoffStrategy::Exponential {
//!         base: 2.0,
//!         multiplier: 1.0
//!     })
//!     .initial_delay(Duration::from_millis(100))
//!     .max_delay(Duration::from_secs(30))
//!     .with_jitter(true)
//!     .retry_condition(RetryCondition::AllErrors)
//!     .build();
//! ```

use crate::error::TaskError;
use chrono::NaiveDateTime;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Comprehensive retry policy configuration for tasks.
///
/// This struct defines how a task should behave when it fails, including
/// the number of retry attempts, backoff strategy, delays, and conditions
/// under which retries should be attempted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (not including the initial attempt)
    pub max_attempts: i32,

    /// The backoff strategy to use for calculating delays between retries
    pub backoff_strategy: BackoffStrategy,

    /// Initial delay before the first retry attempt
    pub initial_delay: Duration,

    /// Maximum delay between retry attempts (caps exponential growth)
    pub max_delay: Duration,

    /// Whether to add random jitter to delays to prevent thundering herd
    pub jitter: bool,

    /// Conditions that determine whether a retry should be attempted
    pub retry_conditions: Vec<RetryCondition>,
}

/// Different backoff strategies for calculating retry delays.
///
/// Each strategy defines how the delay between retry attempts should increase.
/// The actual delay is calculated based on the attempt number and the strategy's parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum BackoffStrategy {
    /// Fixed delay - same delay for every retry attempt
    Fixed,

    /// Linear backoff - delay increases linearly with each attempt
    /// delay = initial_delay * attempt * multiplier
    Linear {
        /// Multiplier for linear growth (default: 1.0)
        multiplier: f64,
    },

    /// Exponential backoff - delay increases exponentially with each attempt
    /// delay = initial_delay * multiplier * (base ^ attempt)
    Exponential {
        /// Base for exponential growth (default: 2.0)
        base: f64,
        /// Multiplier for the exponential function (default: 1.0)
        multiplier: f64,
    },

    /// Custom backoff function (reserved for future extensibility)
    Custom {
        /// Name of the custom function to use
        function_name: String,
    },
}

/// Conditions that determine whether a failed task should be retried.
///
/// These conditions are used to evaluate whether a task should be retried
/// based on the type of error or specific error patterns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum RetryCondition {
    /// Retry on all errors (default behavior)
    AllErrors,

    /// Never retry (equivalent to max_attempts = 0)
    Never,

    /// Retry only for transient errors (network, timeout, etc.)
    TransientOnly,

    /// Retry only if error message contains any of the specified patterns
    ErrorPattern { patterns: Vec<String> },
}

impl Default for RetryPolicy {
    /// Creates a default retry policy with reasonable production settings.
    ///
    /// Default configuration:
    /// - 3 retry attempts
    /// - Exponential backoff (base 2.0, multiplier 1.0)
    /// - 1 second initial delay
    /// - 60 seconds maximum delay
    /// - Jitter enabled
    /// - Retry on all errors
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff_strategy: BackoffStrategy::Exponential {
                base: 2.0,
                multiplier: 1.0,
            },
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            jitter: true,
            retry_conditions: vec![RetryCondition::AllErrors],
        }
    }
}

impl RetryPolicy {
    /// Creates a new RetryPolicyBuilder for fluent configuration.
    pub fn builder() -> RetryPolicyBuilder {
        RetryPolicyBuilder::new()
    }

    /// Calculates the delay before the next retry attempt.
    ///
    /// # Arguments
    ///
    /// * `attempt` - The current attempt number (1-based)
    ///
    /// # Returns
    ///
    /// The duration to wait before the next retry attempt.
    pub fn calculate_delay(&self, attempt: i32) -> Duration {
        let base_delay = match &self.backoff_strategy {
            BackoffStrategy::Fixed => self.initial_delay,

            BackoffStrategy::Linear { multiplier } => {
                let millis = self.initial_delay.as_millis() as f64 * attempt as f64 * multiplier;
                Duration::from_millis(millis as u64)
            }

            BackoffStrategy::Exponential { base, multiplier } => {
                let millis =
                    self.initial_delay.as_millis() as f64 * multiplier * base.powi(attempt - 1);
                Duration::from_millis(millis as u64)
            }

            BackoffStrategy::Custom { .. } => {
                // For now, fall back to exponential backoff for custom functions
                let millis = self.initial_delay.as_millis() as f64 * 2.0_f64.powi(attempt - 1);
                Duration::from_millis(millis as u64)
            }
        };

        // Cap the delay at max_delay
        let capped_delay = std::cmp::min(base_delay, self.max_delay);

        // Add jitter if enabled
        if self.jitter {
            self.add_jitter(capped_delay)
        } else {
            capped_delay
        }
    }

    /// Determines whether a retry should be attempted based on the error and retry conditions.
    ///
    /// # Arguments
    ///
    /// * `error` - The error that caused the task to fail
    /// * `attempt` - The current attempt number
    ///
    /// # Returns
    ///
    /// `true` if the task should be retried, `false` otherwise.
    pub fn should_retry(&self, error: &TaskError, attempt: i32) -> bool {
        // Check if we've exceeded the maximum number of attempts
        if attempt >= self.max_attempts {
            return false;
        }

        // Check retry conditions
        self.retry_conditions
            .iter()
            .any(|condition| match condition {
                RetryCondition::AllErrors => true,
                RetryCondition::Never => false,
                RetryCondition::TransientOnly => self.is_transient_error(error),
                RetryCondition::ErrorPattern { patterns } => {
                    let error_msg = error.to_string().to_lowercase();
                    patterns
                        .iter()
                        .any(|pattern| error_msg.contains(&pattern.to_lowercase()))
                }
            })
    }

    /// Calculates the absolute timestamp when the next retry should occur.
    ///
    /// # Arguments
    ///
    /// * `attempt` - The current attempt number
    /// * `now` - The current timestamp
    ///
    /// # Returns
    ///
    /// A NaiveDateTime representing when the retry should be attempted.
    pub fn calculate_retry_at(&self, attempt: i32, now: NaiveDateTime) -> NaiveDateTime {
        let delay = self.calculate_delay(attempt);
        now + chrono::Duration::from_std(delay).unwrap_or_default()
    }

    /// Adds random jitter to a delay to prevent thundering herd problems.
    ///
    /// Uses +/-25% jitter by default.
    fn add_jitter(&self, delay: Duration) -> Duration {
        let mut rng = rand::thread_rng();
        let jitter_factor = rng.gen_range(0.75..=1.25); // +/-25% jitter
        let jittered_millis = (delay.as_millis() as f64 * jitter_factor) as u64;
        Duration::from_millis(jittered_millis)
    }

    /// Determines if an error is transient (network, timeout, temporary failures).
    fn is_transient_error(&self, error: &TaskError) -> bool {
        match error {
            TaskError::Timeout { .. } => true,
            TaskError::ExecutionFailed { message, .. } | TaskError::Unknown { message, .. } => {
                Self::message_matches_transient_patterns(message)
            }
            _ => false,
        }
    }

    /// Checks whether an error message contains any known transient error patterns.
    fn message_matches_transient_patterns(message: &str) -> bool {
        const TRANSIENT_PATTERNS: &[&str] = &[
            "connection",
            "network",
            "timeout",
            "temporary",
            "unavailable",
            "busy",
            "overloaded",
            "rate limit",
        ];
        let error_msg = message.to_lowercase();
        TRANSIENT_PATTERNS
            .iter()
            .any(|pattern| error_msg.contains(pattern))
    }
}

/// Builder for creating RetryPolicy instances with a fluent API.
#[derive(Debug)]
pub struct RetryPolicyBuilder {
    policy: RetryPolicy,
}

impl RetryPolicyBuilder {
    /// Creates a new RetryPolicyBuilder with default values.
    pub fn new() -> Self {
        Self {
            policy: RetryPolicy::default(),
        }
    }

    /// Sets the maximum number of retry attempts.
    pub fn max_attempts(mut self, max_attempts: i32) -> Self {
        self.policy.max_attempts = max_attempts;
        self
    }

    /// Sets the backoff strategy.
    pub fn backoff_strategy(mut self, strategy: BackoffStrategy) -> Self {
        self.policy.backoff_strategy = strategy;
        self
    }

    /// Sets the initial delay before the first retry.
    pub fn initial_delay(mut self, delay: Duration) -> Self {
        self.policy.initial_delay = delay;
        self
    }

    /// Sets the maximum delay between retries.
    pub fn max_delay(mut self, delay: Duration) -> Self {
        self.policy.max_delay = delay;
        self
    }

    /// Enables or disables jitter.
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.policy.jitter = jitter;
        self
    }

    /// Adds a retry condition.
    pub fn retry_condition(mut self, condition: RetryCondition) -> Self {
        self.policy.retry_conditions = vec![condition];
        self
    }

    /// Adds multiple retry conditions.
    pub fn retry_conditions(mut self, conditions: Vec<RetryCondition>) -> Self {
        self.policy.retry_conditions = conditions;
        self
    }

    /// Builds the RetryPolicy.
    pub fn build(self) -> RetryPolicy {
        self.policy
    }
}

impl Default for RetryPolicyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_retry_policy() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.initial_delay, Duration::from_secs(1));
        assert_eq!(policy.max_delay, Duration::from_secs(60));
        assert!(policy.jitter);
        assert!(matches!(
            policy.backoff_strategy,
            BackoffStrategy::Exponential { .. }
        ));
    }

    #[test]
    fn test_retry_policy_builder() {
        let policy = RetryPolicy::builder()
            .max_attempts(5)
            .initial_delay(Duration::from_millis(500))
            .max_delay(Duration::from_secs(30))
            .with_jitter(false)
            .backoff_strategy(BackoffStrategy::Linear { multiplier: 1.5 })
            .retry_condition(RetryCondition::TransientOnly)
            .build();

        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.initial_delay, Duration::from_millis(500));
        assert_eq!(policy.max_delay, Duration::from_secs(30));
        assert!(!policy.jitter);
        assert_eq!(policy.retry_conditions, vec![RetryCondition::TransientOnly]);
    }

    #[test]
    fn test_fixed_backoff_calculation() {
        let policy = RetryPolicy::builder()
            .backoff_strategy(BackoffStrategy::Fixed)
            .initial_delay(Duration::from_secs(2))
            .with_jitter(false)
            .build();

        assert_eq!(policy.calculate_delay(1), Duration::from_secs(2));
        assert_eq!(policy.calculate_delay(2), Duration::from_secs(2));
        assert_eq!(policy.calculate_delay(3), Duration::from_secs(2));
    }

    #[test]
    fn test_linear_backoff_calculation() {
        let policy = RetryPolicy::builder()
            .backoff_strategy(BackoffStrategy::Linear { multiplier: 1.0 })
            .initial_delay(Duration::from_secs(1))
            .with_jitter(false)
            .build();

        assert_eq!(policy.calculate_delay(1), Duration::from_secs(1));
        assert_eq!(policy.calculate_delay(2), Duration::from_secs(2));
        assert_eq!(policy.calculate_delay(3), Duration::from_secs(3));
    }

    #[test]
    fn test_exponential_backoff_calculation() {
        let policy = RetryPolicy::builder()
            .backoff_strategy(BackoffStrategy::Exponential {
                base: 2.0,
                multiplier: 1.0,
            })
            .initial_delay(Duration::from_secs(1))
            .with_jitter(false)
            .build();

        assert_eq!(policy.calculate_delay(1), Duration::from_secs(1));
        assert_eq!(policy.calculate_delay(2), Duration::from_secs(2));
        assert_eq!(policy.calculate_delay(3), Duration::from_secs(4));
        assert_eq!(policy.calculate_delay(4), Duration::from_secs(8));
    }

    #[test]
    fn test_max_delay_capping() {
        let policy = RetryPolicy::builder()
            .backoff_strategy(BackoffStrategy::Exponential {
                base: 2.0,
                multiplier: 1.0,
            })
            .initial_delay(Duration::from_secs(10))
            .max_delay(Duration::from_secs(15))
            .with_jitter(false)
            .build();

        assert_eq!(policy.calculate_delay(1), Duration::from_secs(10));
        assert_eq!(policy.calculate_delay(2), Duration::from_secs(15)); // Capped
        assert_eq!(policy.calculate_delay(3), Duration::from_secs(15)); // Capped
    }

    // --- is_transient_error tests ---

    fn make_execution_error(msg: &str) -> TaskError {
        TaskError::ExecutionFailed {
            message: msg.to_string(),
            task_id: "test".to_string(),
            timestamp: chrono::Utc::now(),
        }
    }

    fn make_unknown_error(msg: &str) -> TaskError {
        TaskError::Unknown {
            task_id: "test".to_string(),
            message: msg.to_string(),
        }
    }

    #[test]
    fn test_timeout_is_transient() {
        let policy = RetryPolicy::default();
        let error = TaskError::Timeout {
            task_id: "test".to_string(),
            timeout_seconds: 30,
        };
        assert!(policy.is_transient_error(&error));
    }

    #[test]
    fn test_connection_error_is_transient() {
        let policy = RetryPolicy::default();
        assert!(policy.is_transient_error(&make_execution_error("Connection refused")));
        assert!(policy.is_transient_error(&make_execution_error("network unreachable")));
        assert!(policy.is_transient_error(&make_execution_error("service temporarily unavailable")));
        assert!(policy.is_transient_error(&make_execution_error("server busy")));
        assert!(policy.is_transient_error(&make_execution_error("overloaded")));
        assert!(policy.is_transient_error(&make_execution_error("rate limit exceeded")));
    }

    #[test]
    fn test_unknown_error_with_transient_message_is_transient() {
        let policy = RetryPolicy::default();
        assert!(policy.is_transient_error(&make_unknown_error("Connection reset by peer")));
        assert!(policy.is_transient_error(&make_unknown_error("TIMEOUT waiting for response")));
    }

    #[test]
    fn test_permanent_errors_are_not_transient() {
        let policy = RetryPolicy::default();
        assert!(!policy.is_transient_error(&make_execution_error("invalid input format")));
        assert!(!policy.is_transient_error(&make_execution_error("permission denied")));
        assert!(!policy.is_transient_error(&make_unknown_error("null pointer")));
    }

    #[test]
    fn test_non_retryable_error_variants_are_not_transient() {
        let policy = RetryPolicy::default();
        assert!(!policy.is_transient_error(&TaskError::ContextError {
            task_id: "t".to_string(),
            error: crate::error::ContextError::KeyNotFound("k".to_string()),
        }));
        assert!(
            !policy.is_transient_error(&TaskError::DependencyNotSatisfied {
                dependency: "dep".to_string(),
                task_id: "t".to_string(),
            })
        );
        assert!(!policy.is_transient_error(&TaskError::ValidationFailed {
            message: "bad".to_string(),
        }));
        assert!(
            !policy.is_transient_error(&TaskError::ReadinessCheckFailed {
                task_id: "t".to_string(),
            })
        );
        assert!(!policy.is_transient_error(&TaskError::TriggerRuleFailed {
            task_id: "t".to_string(),
        }));
    }

    #[test]
    fn test_transient_pattern_matching_is_case_insensitive() {
        let policy = RetryPolicy::default();
        assert!(policy.is_transient_error(&make_execution_error("CONNECTION REFUSED")));
        assert!(policy.is_transient_error(&make_execution_error("Network Error")));
        assert!(policy.is_transient_error(&make_execution_error("TIMEOUT")));
    }

    // --- should_retry tests ---

    #[test]
    fn test_should_retry_all_errors_within_limit() {
        let policy = RetryPolicy::builder()
            .max_attempts(3)
            .retry_condition(RetryCondition::AllErrors)
            .build();

        let error = make_execution_error("anything");
        assert!(policy.should_retry(&error, 1));
        assert!(policy.should_retry(&error, 2));
        assert!(!policy.should_retry(&error, 3)); // at max
        assert!(!policy.should_retry(&error, 4)); // over max
    }

    #[test]
    fn test_should_retry_never_condition() {
        let policy = RetryPolicy::builder()
            .max_attempts(10)
            .retry_condition(RetryCondition::Never)
            .build();

        assert!(!policy.should_retry(&make_execution_error("anything"), 1));
    }

    #[test]
    fn test_should_retry_transient_only() {
        let policy = RetryPolicy::builder()
            .max_attempts(3)
            .retry_condition(RetryCondition::TransientOnly)
            .build();

        assert!(policy.should_retry(&make_execution_error("connection refused"), 1));
        assert!(!policy.should_retry(&make_execution_error("invalid input"), 1));
    }

    #[test]
    fn test_should_retry_error_pattern() {
        let policy = RetryPolicy::builder()
            .max_attempts(3)
            .retry_condition(RetryCondition::ErrorPattern {
                patterns: vec!["deadlock".to_string(), "lock timeout".to_string()],
            })
            .build();

        assert!(policy.should_retry(&make_execution_error("deadlock detected"), 1));
        assert!(policy.should_retry(&make_execution_error("Lock Timeout on table"), 1));
        assert!(!policy.should_retry(&make_execution_error("invalid input"), 1));
    }

    #[test]
    fn test_should_retry_zero_max_attempts() {
        let policy = RetryPolicy::builder()
            .max_attempts(0)
            .retry_condition(RetryCondition::AllErrors)
            .build();

        assert!(!policy.should_retry(&make_execution_error("anything"), 0));
    }

    #[test]
    fn test_custom_backoff_falls_back_to_exponential() {
        let policy = RetryPolicy::builder()
            .backoff_strategy(BackoffStrategy::Custom {
                function_name: "my_func".to_string(),
            })
            .initial_delay(Duration::from_secs(1))
            .with_jitter(false)
            .build();

        assert_eq!(policy.calculate_delay(1), Duration::from_secs(1));
        assert_eq!(policy.calculate_delay(2), Duration::from_secs(2));
        assert_eq!(policy.calculate_delay(3), Duration::from_secs(4));
    }

    #[test]
    fn test_jitter_stays_within_bounds() {
        let policy = RetryPolicy::builder()
            .backoff_strategy(BackoffStrategy::Fixed)
            .initial_delay(Duration::from_secs(10))
            .with_jitter(true)
            .build();

        // Run multiple times to check jitter range (+/-25%)
        for _ in 0..100 {
            let delay = policy.calculate_delay(1);
            let millis = delay.as_millis();
            assert!(millis >= 7500, "jitter too low: {}ms", millis);
            assert!(millis <= 12500, "jitter too high: {}ms", millis);
        }
    }

    #[test]
    fn test_message_matches_transient_patterns_directly() {
        assert!(RetryPolicy::message_matches_transient_patterns(
            "connection reset"
        ));
        assert!(RetryPolicy::message_matches_transient_patterns(
            "NETWORK error"
        ));
        assert!(!RetryPolicy::message_matches_transient_patterns(
            "invalid input"
        ));
        assert!(!RetryPolicy::message_matches_transient_patterns(""));
    }
}
