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
//! - **Production-ready resilience patterns**
//!
//! ## Usage
//!
//! ```rust
//! use cloacina::retry::{RetryPolicy, BackoffStrategy, RetryCondition};
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
//!
//! ## Key Components
//!
//! ### RetryPolicy
//! The main configuration struct that defines how a task should behave when it fails.
//! It includes settings for retry attempts, backoff strategy, delays, and retry conditions.
//!
//! ### BackoffStrategy
//! Defines how the delay between retry attempts should increase. Available strategies:
//! - **Fixed**: Same delay for every retry
//! - **Linear**: Delay increases linearly with each attempt
//! - **Exponential**: Delay increases exponentially with each attempt
//! - **Custom**: Reserved for future extensibility
//!
//! ### RetryCondition
//! Determines whether a failed task should be retried based on:
//! - **AllErrors**: Retry on any error (default)
//! - **Never**: Never retry
//! - **TransientOnly**: Retry only for transient errors
//! - **ErrorPattern**: Retry based on error message patterns
//!
//! ## Best Practices
//!
//! 1. **Jitter**: Always enable jitter in production to prevent thundering herd problems
//! 2. **Max Delay**: Set a reasonable max_delay to prevent excessive wait times
//! 3. **Error Conditions**: Use specific retry conditions to avoid retrying on permanent failures
//! 4. **Backoff Strategy**: Choose exponential backoff for most cases, linear for predictable failures

// Re-export all retry types from cloacina_workflow
// This ensures type compatibility between macro-generated code and runtime
pub use cloacina_workflow::retry::{
    BackoffStrategy, RetryCondition, RetryPolicy, RetryPolicyBuilder,
};
