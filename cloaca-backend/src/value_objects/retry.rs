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

use pyo3::prelude::*;
use std::time::Duration;

/// Python wrapper for RetryPolicy
#[pyclass(name = "RetryPolicy")]
#[derive(Clone)]
pub struct PyRetryPolicy {
    inner: cloacina::retry::RetryPolicy,
}

/// Python wrapper for BackoffStrategy
#[pyclass(name = "BackoffStrategy")]
#[derive(Clone)]
pub struct PyBackoffStrategy {
    inner: cloacina::retry::BackoffStrategy,
}

/// Python wrapper for RetryCondition
#[pyclass(name = "RetryCondition")]
#[derive(Clone)]
pub struct PyRetryCondition {
    inner: cloacina::retry::RetryCondition,
}

/// Python wrapper for RetryPolicy::Builder
#[pyclass(name = "RetryPolicyBuilder")]
#[derive(Clone, Debug)]
pub struct PyRetryPolicyBuilder {
    max_attempts: Option<i32>,
    backoff_strategy: Option<cloacina::retry::BackoffStrategy>,
    initial_delay: Option<Duration>,
    max_delay: Option<Duration>,
    retry_condition: Option<cloacina::retry::RetryCondition>,
    with_jitter: Option<bool>,
}

#[pymethods]
impl PyRetryPolicy {
    /// Create a builder for constructing RetryPolicy
    #[staticmethod]
    pub fn builder() -> PyRetryPolicyBuilder {
        PyRetryPolicyBuilder {
            max_attempts: None,
            backoff_strategy: None,
            initial_delay: None,
            max_delay: None,
            retry_condition: None,
            with_jitter: None,
        }
    }

    /// Create a default RetryPolicy
    #[staticmethod]
    pub fn default() -> Self {
        Self {
            inner: cloacina::retry::RetryPolicy::default(),
        }
    }

    /// Check if a retry should be attempted
    pub fn should_retry(&self, attempt: i32, _error_type: &str) -> bool {
        // For now, use a simple retry condition check
        // In the future, this could be enhanced to parse error_type
        attempt < self.inner.max_attempts
    }

    /// Calculate delay for a given attempt
    pub fn calculate_delay(&self, attempt: i32) -> f64 {
        let duration = self.inner.calculate_delay(attempt);
        duration.as_secs_f64()
    }

    /// Get maximum number of attempts
    #[getter]
    pub fn max_attempts(&self) -> i32 {
        self.inner.max_attempts
    }

    /// Get initial delay in seconds
    #[getter]
    pub fn initial_delay(&self) -> f64 {
        self.inner.initial_delay.as_secs_f64()
    }

    /// Get maximum delay in seconds
    #[getter]
    pub fn max_delay(&self) -> f64 {
        self.inner.max_delay.as_secs_f64()
    }

    /// Check if jitter is enabled
    #[getter]
    pub fn with_jitter(&self) -> bool {
        self.inner.jitter
    }

    /// String representation
    pub fn __repr__(&self) -> String {
        format!(
            "RetryPolicy(max_attempts={}, initial_delay={}s, max_delay={}s, jitter={})",
            self.max_attempts(),
            self.initial_delay(),
            self.max_delay(),
            self.with_jitter()
        )
    }
}

#[pymethods]
impl PyBackoffStrategy {
    /// Fixed delay strategy
    #[staticmethod]
    pub fn fixed() -> Self {
        Self {
            inner: cloacina::retry::BackoffStrategy::Fixed,
        }
    }

    /// Linear backoff strategy
    #[staticmethod]
    pub fn linear(multiplier: f64) -> Self {
        Self {
            inner: cloacina::retry::BackoffStrategy::Linear { multiplier },
        }
    }

    /// Exponential backoff strategy
    #[staticmethod]
    pub fn exponential(base: f64, multiplier: Option<f64>) -> Self {
        Self {
            inner: cloacina::retry::BackoffStrategy::Exponential {
                base,
                multiplier: multiplier.unwrap_or(1.0),
            },
        }
    }

    /// String representation
    pub fn __repr__(&self) -> String {
        match &self.inner {
            cloacina::retry::BackoffStrategy::Fixed => "BackoffStrategy.Fixed".to_string(),
            cloacina::retry::BackoffStrategy::Linear { multiplier } => {
                format!("BackoffStrategy.Linear(multiplier={})", multiplier)
            }
            cloacina::retry::BackoffStrategy::Exponential { base, multiplier } => {
                format!(
                    "BackoffStrategy.Exponential(base={}, multiplier={})",
                    base, multiplier
                )
            }
            cloacina::retry::BackoffStrategy::Custom { function_name } => {
                format!("BackoffStrategy.Custom(function_name='{}')", function_name)
            }
        }
    }
}

#[pymethods]
impl PyRetryCondition {
    /// Never retry
    #[staticmethod]
    pub fn never() -> Self {
        Self {
            inner: cloacina::retry::RetryCondition::Never,
        }
    }

    /// Retry only on transient errors
    #[staticmethod]
    pub fn transient_only() -> Self {
        Self {
            inner: cloacina::retry::RetryCondition::TransientOnly,
        }
    }

    /// Retry on all errors
    #[staticmethod]
    pub fn all_errors() -> Self {
        Self {
            inner: cloacina::retry::RetryCondition::AllErrors,
        }
    }

    /// Retry on specific error patterns
    #[staticmethod]
    pub fn error_pattern(patterns: Vec<String>) -> Self {
        Self {
            inner: cloacina::retry::RetryCondition::ErrorPattern { patterns },
        }
    }

    /// String representation
    pub fn __repr__(&self) -> String {
        match &self.inner {
            cloacina::retry::RetryCondition::Never => "RetryCondition.Never".to_string(),
            cloacina::retry::RetryCondition::TransientOnly => {
                "RetryCondition.TransientOnly".to_string()
            }
            cloacina::retry::RetryCondition::AllErrors => "RetryCondition.AllErrors".to_string(),
            cloacina::retry::RetryCondition::ErrorPattern { patterns } => {
                format!("RetryCondition.ErrorPattern(patterns={:?})", patterns)
            }
        }
    }
}

#[pymethods]
impl PyRetryPolicyBuilder {
    /// Set maximum number of retry attempts
    pub fn max_attempts(&self, attempts: i32) -> Self {
        let mut new_builder = self.clone();
        new_builder.max_attempts = Some(attempts);
        new_builder
    }

    /// Set initial delay
    pub fn initial_delay(&self, delay_seconds: f64) -> Self {
        let mut new_builder = self.clone();
        new_builder.initial_delay = Some(Duration::from_secs_f64(delay_seconds));
        new_builder
    }

    /// Set maximum delay
    pub fn max_delay(&self, delay_seconds: f64) -> Self {
        let mut new_builder = self.clone();
        new_builder.max_delay = Some(Duration::from_secs_f64(delay_seconds));
        new_builder
    }

    /// Set backoff strategy
    pub fn backoff_strategy(&self, strategy: PyBackoffStrategy) -> Self {
        let mut new_builder = self.clone();
        new_builder.backoff_strategy = Some(strategy.inner);
        new_builder
    }

    /// Set retry condition
    pub fn retry_condition(&self, condition: PyRetryCondition) -> Self {
        let mut new_builder = self.clone();
        new_builder.retry_condition = Some(condition.inner);
        new_builder
    }

    /// Enable/disable jitter
    pub fn with_jitter(&self, jitter: bool) -> Self {
        let mut new_builder = self.clone();
        new_builder.with_jitter = Some(jitter);
        new_builder
    }

    /// Build the RetryPolicy
    pub fn build(&self) -> PyRetryPolicy {
        let mut builder = cloacina::retry::RetryPolicy::builder();

        if let Some(attempts) = self.max_attempts {
            builder = builder.max_attempts(attempts);
        }
        if let Some(strategy) = &self.backoff_strategy {
            builder = builder.backoff_strategy(strategy.clone());
        }
        if let Some(delay) = self.initial_delay {
            builder = builder.initial_delay(delay);
        }
        if let Some(delay) = self.max_delay {
            builder = builder.max_delay(delay);
        }
        if let Some(condition) = &self.retry_condition {
            builder = builder.retry_condition(condition.clone());
        }
        if let Some(jitter) = self.with_jitter {
            builder = builder.with_jitter(jitter);
        }

        PyRetryPolicy {
            inner: builder.build(),
        }
    }
}

impl PyRetryPolicy {
    /// Convert from Rust RetryPolicy (for internal use)
    pub fn from_rust(policy: cloacina::retry::RetryPolicy) -> Self {
        Self { inner: policy }
    }

    /// Convert to Rust RetryPolicy (for internal use)
    pub fn to_rust(&self) -> cloacina::retry::RetryPolicy {
        self.inner.clone()
    }
}
