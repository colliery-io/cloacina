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

//! Workflow builder for fluent workflow construction.
//!
//! This module provides the `WorkflowBuilder` struct for constructing
//! workflows using a chainable, fluent API.

use std::sync::Arc;

use crate::error::{ValidationError, WorkflowError};
use crate::task::Task;

use super::Workflow;

/// Builder pattern for convenient and fluent Workflow construction.
///
/// The WorkflowBuilder provides a chainable interface for constructing Workflows,
/// making it easy to set metadata, add tasks, and validate the structure
/// before finalizing the Workflow.
///
/// # Fields
///
/// * `workflow`: Workflow - The workflow being constructed
///
/// # Implementation Details
///
/// The builder pattern provides:
/// - Fluent interface for workflow construction
/// - Automatic validation during build
/// - Content-based version calculation
/// - Metadata management
///
/// # Examples
///
/// ```rust,ignore
/// use cloacina::*;
///
/// # struct TestTask { id: String, deps: Vec<String> }
/// # impl TestTask { fn new(id: &str, deps: Vec<&str>) -> Self { Self { id: id.to_string(), deps: deps.into_iter().map(|s| s.to_string()).collect() } } }
/// # use async_trait::async_trait;
/// # #[async_trait]
/// # impl Task for TestTask {
/// #     async fn execute(&self, context: Context<serde_json::Value>) -> Result<Context<serde_json::Value>, TaskError> { Ok(context) }
/// #     fn id(&self) -> &str { &self.id }
/// #     fn dependencies(&self) -> &[String] { &self.deps }
/// # }
/// let workflow = Workflow::builder("etl-pipeline")
///     .description("Customer data ETL pipeline")
///     .tag("environment", "staging")
///     .tag("owner", "data-team")
///     .add_task(TestTask::new("extract_customers", vec![]))?
///     .add_task(TestTask::new("validate_data", vec!["extract_customers"]))?
///     .validate()?
///     .build()?;
///
/// assert_eq!(workflow.name(), "etl-pipeline");
/// assert!(!workflow.metadata().version.is_empty());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
pub struct WorkflowBuilder {
    workflow: Workflow,
}

impl WorkflowBuilder {
    /// Create a new workflow builder
    pub fn new(name: &str) -> Self {
        Self {
            workflow: Workflow::new(name),
        }
    }

    /// Get the workflow name
    pub fn name(&self) -> &str {
        self.workflow.name()
    }

    /// Set the workflow description
    pub fn description(mut self, description: &str) -> Self {
        self.workflow.set_description(description);
        self
    }

    /// Set the workflow tenant
    pub fn tenant(mut self, tenant: &str) -> Self {
        self.workflow.tenant = tenant.to_string();
        self
    }

    /// Add a tag to the workflow metadata
    pub fn tag(mut self, key: &str, value: &str) -> Self {
        self.workflow.add_tag(key, value);
        self
    }

    /// Add a task to the workflow
    pub fn add_task(mut self, task: Arc<dyn Task>) -> Result<Self, WorkflowError> {
        self.workflow.add_task(task)?;
        Ok(self)
    }

    /// Validate the workflow structure
    pub fn validate(self) -> Result<Self, ValidationError> {
        self.workflow.validate()?;
        Ok(self)
    }

    /// Build the final workflow with automatic version calculation
    pub fn build(self) -> Result<Workflow, ValidationError> {
        self.workflow.validate()?;
        // Auto-calculate version when building
        Ok(self.workflow.finalize())
    }
}
