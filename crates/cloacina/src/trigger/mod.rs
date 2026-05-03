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

//! # Trigger System
//!
//! This module provides event-based workflow triggering functionality.
//! Triggers are user-defined polling functions that can fire workflows
//! when specific conditions are met.
//!
//! ## Overview
//!
//! Triggers enable event-driven workflow execution by:
//! - Polling user-defined conditions at configurable intervals
//! - Firing workflows with optional context when conditions are met
//! - Deduplicating concurrent executions based on context hash
//!
//! ## Example
//!
//! ```rust,ignore
//! use cloacina::*;
//!
//! #[trigger(
//!     name = "file_watcher",
//!     poll_interval = "5s",
//!     allow_concurrent = false,
//! )]
//! async fn file_watcher() -> TriggerResult {
//!     if let Some(path) = check_for_new_file("/inbox/").await {
//!         let mut ctx = Context::new();
//!         ctx.insert("file_path", serde_json::json!(path))?;
//!         TriggerResult::Fire(Some(ctx))
//!     } else {
//!         TriggerResult::Skip
//!     }
//! }
//! ```

pub mod registry;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use thiserror::Error;

use crate::Context;

/// Errors that can occur during trigger operations.
#[derive(Debug, Error)]
pub enum TriggerError {
    /// Error during trigger polling
    #[error("Trigger poll error: {message}")]
    PollError { message: String },

    /// Error creating context for workflow
    #[error("Context creation error: {0}")]
    ContextError(#[from] crate::error::ContextError),

    /// Trigger not found in registry
    #[error("Trigger not found: {name}")]
    TriggerNotFound { name: String },

    /// Database error during trigger operations
    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),

    /// Connection pool error
    #[error("Connection pool error: {0}")]
    ConnectionPool(String),

    /// Workflow scheduling failed
    #[error("Failed to schedule workflow '{workflow}': {message}")]
    WorkflowSchedulingFailed { workflow: String, message: String },
}

impl From<deadpool::managed::PoolError<deadpool_diesel::Error>> for TriggerError {
    fn from(err: deadpool::managed::PoolError<deadpool_diesel::Error>) -> Self {
        TriggerError::ConnectionPool(err.to_string())
    }
}

impl From<cloacina_workflow::TriggerError> for TriggerError {
    fn from(err: cloacina_workflow::TriggerError) -> Self {
        match err {
            cloacina_workflow::TriggerError::PollError { message } => {
                TriggerError::PollError { message }
            }
            cloacina_workflow::TriggerError::ContextError(e) => TriggerError::PollError {
                message: format!("Context error: {}", e),
            },
        }
    }
}

// T-0552 / I-0102 follow-up: TriggerResult relocated to cloacina-workflow
// so the leaf-crate `Trigger` trait can use it directly. Re-exported here
// so existing engine paths (`cloacina::trigger::TriggerResult`) keep
// resolving.
pub use cloacina_workflow::TriggerResult;

/// Configuration for a trigger.
///
/// This is typically set via macro attributes and stored in the database
/// for persistence across restarts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    /// Unique name identifying this trigger
    pub name: String,

    /// Name of the workflow to fire when the trigger activates
    pub workflow_name: String,

    /// How often to poll the trigger function
    pub poll_interval: Duration,

    /// Whether to allow concurrent executions with the same context
    pub allow_concurrent: bool,

    /// Whether this trigger is enabled
    pub enabled: bool,
}

impl TriggerConfig {
    /// Creates a new trigger configuration.
    pub fn new(name: &str, workflow_name: &str, poll_interval: Duration) -> Self {
        Self {
            name: name.to_string(),
            workflow_name: workflow_name.to_string(),
            poll_interval,
            allow_concurrent: false,
            enabled: true,
        }
    }

    /// Sets whether concurrent executions are allowed.
    pub fn with_allow_concurrent(mut self, allow: bool) -> Self {
        self.allow_concurrent = allow;
        self
    }

    /// Sets whether the trigger is enabled.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

// T-0552 / I-0102 follow-up: Trigger trait relocated to cloacina-workflow
// so packaged cdylibs can collect TriggerEntry inventory entries. Engine
// paths re-export.
pub use cloacina_workflow::Trigger;

pub use registry::TriggerConstructor;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestTrigger {
        name: String,
        should_fire: bool,
    }

    #[async_trait]
    impl Trigger for TestTrigger {
        fn name(&self) -> &str {
            &self.name
        }

        fn poll_interval(&self) -> Duration {
            Duration::from_secs(1)
        }

        fn allow_concurrent(&self) -> bool {
            false
        }

        async fn poll(&self) -> Result<TriggerResult, cloacina_workflow::TriggerError> {
            if self.should_fire {
                Ok(TriggerResult::Fire(None))
            } else {
                Ok(TriggerResult::Skip)
            }
        }
    }

    #[test]
    fn test_trigger_result_should_fire() {
        assert!(!TriggerResult::Skip.should_fire());
        assert!(TriggerResult::Fire(None).should_fire());
        assert!(TriggerResult::Fire(Some(Context::new())).should_fire());
    }

    #[test]
    fn test_trigger_result_into_context() {
        assert!(TriggerResult::Skip.into_context().is_none());
        assert!(TriggerResult::Fire(None).into_context().is_none());

        let ctx = Context::new();
        let result = TriggerResult::Fire(Some(ctx));
        assert!(result.into_context().is_some());
    }

    #[test]
    fn test_trigger_result_context_hash() {
        // Skip always returns same hash
        assert_eq!(TriggerResult::Skip.context_hash(), "skip");

        // Fire with no context returns same hash
        assert_eq!(TriggerResult::Fire(None).context_hash(), "fire_no_context");

        // Fire with context returns hash based on context data
        let mut ctx1 = Context::new();
        ctx1.insert("key", serde_json::json!("value1")).unwrap();
        let hash1 = TriggerResult::Fire(Some(ctx1)).context_hash();

        let mut ctx2 = Context::new();
        ctx2.insert("key", serde_json::json!("value2")).unwrap();
        let hash2 = TriggerResult::Fire(Some(ctx2)).context_hash();

        // Different contexts should have different hashes
        assert_ne!(hash1, hash2);

        // Same context data should have same hash
        let mut ctx3 = Context::new();
        ctx3.insert("key", serde_json::json!("value1")).unwrap();
        let hash3 = TriggerResult::Fire(Some(ctx3)).context_hash();
        assert_eq!(hash1, hash3);
    }

    #[test]
    fn test_trigger_config() {
        let config = TriggerConfig::new("test", "my_workflow", Duration::from_secs(5));
        assert_eq!(config.name, "test");
        assert_eq!(config.workflow_name, "my_workflow");
        assert_eq!(config.poll_interval, Duration::from_secs(5));
        assert!(!config.allow_concurrent);
        assert!(config.enabled);

        let config = config.with_allow_concurrent(true).with_enabled(false);
        assert!(config.allow_concurrent);
        assert!(!config.enabled);
    }

    #[tokio::test]
    async fn test_trigger_trait() {
        let trigger = TestTrigger {
            name: "test_trigger".to_string(),
            should_fire: false,
        };

        assert_eq!(trigger.name(), "test_trigger");
        assert_eq!(trigger.poll_interval(), Duration::from_secs(1));
        assert!(!trigger.allow_concurrent());

        let result = trigger.poll().await.unwrap();
        assert!(!result.should_fire());
    }

    #[tokio::test]
    async fn test_trigger_fires() {
        let trigger = TestTrigger {
            name: "firing_trigger".to_string(),
            should_fire: true,
        };

        let result = trigger.poll().await.unwrap();
        assert!(result.should_fire());
    }
}
