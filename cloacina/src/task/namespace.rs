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

//! Task namespace management for isolated task execution.
//!
//! This module provides hierarchical namespace support for tasks, enabling:
//! - Multi-tenant task isolation
//! - Packaged workflow separation
//! - Conflict resolution between workflows with same task IDs
//!
//! ## Namespace Format
//!
//! Namespaces follow the format: `tenant_id::package_name::workflow_id::task_id`
//!
//! - `tenant_id`: Default "public", can be tenant-specific for multi-tenancy
//! - `package_name`: Default "embedded", or name from .so file metadata
//! - `workflow_id`: From workflow! macro name field (required)
//! - `task_id`: From #[task] macro id parameter (required)
//!
//! ## Examples
//!
//! ```rust
//! use cloacina::TaskNamespace;
//!
//! // Embedded workflow (most common)
//! let ns = TaskNamespace::embedded("customer_etl", "extract_data");
//! assert_eq!(ns.to_string(), "public::embedded::customer_etl::extract_data");
//!
//! // Packaged workflow
//! let ns = TaskNamespace::packaged("analytics.so", "data_pipeline", "extract_data");
//! assert_eq!(ns.to_string(), "public::analytics.so::data_pipeline::extract_data");
//!
//! // Multi-tenant scenario
//! let ns = TaskNamespace::tenant("tenant_123", "embedded", "customer_etl", "extract_data");
//! assert_eq!(ns.to_string(), "tenant_123::embedded::customer_etl::extract_data");
//! ```

use std::fmt::{Display, Formatter, Result as FmtResult};

/// Hierarchical namespace for task identification and isolation.
///
/// Provides a structured way to identify tasks across different contexts:
/// multi-tenant environments, packaged workflows, and embedded workflows.
///
/// The namespace components form a hierarchy from most general (tenant) to
/// most specific (task), enabling precise task resolution while supporting
/// fallback strategies for compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskNamespace {
    /// Tenant identifier for multi-tenancy support.
    /// Default: "public" for single-tenant or public access
    pub tenant_id: String,

    /// Package or deployment context identifier.
    /// Default: "embedded" for tasks compiled into the binary
    /// For packaged workflows: name from .so file metadata
    pub package_name: String,

    /// Workflow identifier from workflow macro.
    /// Groups related tasks together within a package/tenant
    pub workflow_id: String,

    /// Individual task identifier from task macro.
    /// Unique within the workflow context
    pub task_id: String,
}

impl TaskNamespace {
    /// Create a namespace for embedded workflow tasks (most common case).
    ///
    /// This is the default namespace type for tasks defined directly in
    /// application code using the standard workflow! and #[task] macros.
    ///
    /// # Arguments
    ///
    /// * `workflow_id` - Name from workflow! macro
    /// * `task_id` - ID from #[task] macro
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina::TaskNamespace;
    ///
    /// let ns = TaskNamespace::embedded("customer_etl", "extract_data");
    /// assert_eq!(ns.tenant_id, "public");
    /// assert_eq!(ns.package_name, "embedded");
    /// assert_eq!(ns.workflow_id, "customer_etl");
    /// assert_eq!(ns.task_id, "extract_data");
    /// ```
    pub fn embedded(workflow_id: &str, task_id: &str) -> Self {
        Self {
            tenant_id: "public".to_string(),
            package_name: "embedded".to_string(),
            workflow_id: workflow_id.to_string(),
            task_id: task_id.to_string(),
        }
    }

    /// Create a namespace for packaged workflow tasks.
    ///
    /// Used for tasks loaded from .so files via the packaged workflow system.
    /// The package name typically comes from the .so file metadata.
    ///
    /// # Arguments
    ///
    /// * `package_name` - Name from packaged workflow metadata
    /// * `workflow_id` - Name from package_workflow! macro
    /// * `task_id` - ID from #[task] macro
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina::TaskNamespace;
    ///
    /// let ns = TaskNamespace::packaged("analytics.so", "data_pipeline", "extract_data");
    /// assert_eq!(ns.tenant_id, "public");
    /// assert_eq!(ns.package_name, "analytics.so");
    /// assert_eq!(ns.workflow_id, "data_pipeline");
    /// assert_eq!(ns.task_id, "extract_data");
    /// ```
    pub fn packaged(package_name: &str, workflow_id: &str, task_id: &str) -> Self {
        Self {
            tenant_id: "public".to_string(),
            package_name: package_name.to_string(),
            workflow_id: workflow_id.to_string(),
            task_id: task_id.to_string(),
        }
    }

    /// Create a namespace for multi-tenant scenarios.
    ///
    /// Allows tenant-specific task isolation while maintaining the same
    /// package and workflow structure. Useful for SaaS applications where
    /// different tenants may have customized versions of the same workflows.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - Unique identifier for the tenant
    /// * `package_name` - Package context ("embedded" or package name)
    /// * `workflow_id` - Workflow identifier
    /// * `task_id` - Task identifier
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina::TaskNamespace;
    ///
    /// let ns = TaskNamespace::tenant("customer_123", "embedded", "order_processing", "validate_order");
    /// assert_eq!(ns.tenant_id, "customer_123");
    /// assert_eq!(ns.package_name, "embedded");
    /// assert_eq!(ns.workflow_id, "order_processing");
    /// assert_eq!(ns.task_id, "validate_order");
    /// ```
    pub fn tenant(tenant_id: &str, package_name: &str, workflow_id: &str, task_id: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            package_name: package_name.to_string(),
            workflow_id: workflow_id.to_string(),
            task_id: task_id.to_string(),
        }
    }

    /// Create a complete namespace from all components.
    ///
    /// This is the most general constructor, useful when all namespace
    /// components are known and need to be specified explicitly.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - Tenant identifier
    /// * `package_name` - Package identifier
    /// * `workflow_id` - Workflow identifier
    /// * `task_id` - Task identifier
    pub fn new(tenant_id: &str, package_name: &str, workflow_id: &str, task_id: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            package_name: package_name.to_string(),
            workflow_id: workflow_id.to_string(),
            task_id: task_id.to_string(),
        }
    }

    /// Check if this is a public (non-tenant-specific) namespace.
    ///
    /// # Returns
    ///
    /// `true` if this namespace uses the default "public" tenant
    pub fn is_public(&self) -> bool {
        self.tenant_id == "public"
    }

    /// Check if this is an embedded (non-packaged) namespace.
    ///
    /// # Returns
    ///
    /// `true` if this namespace uses the default "embedded" package
    pub fn is_embedded(&self) -> bool {
        self.package_name == "embedded"
    }

    /// Get the workflow-scoped portion of the namespace.
    ///
    /// Returns a namespace that identifies the workflow but not the specific task,
    /// useful for workflow-level operations.
    ///
    /// # Returns
    ///
    /// A new TaskNamespace with "workflow" as the task_id
    pub fn workflow_scope(&self) -> Self {
        Self {
            tenant_id: self.tenant_id.clone(),
            package_name: self.package_name.clone(),
            workflow_id: self.workflow_id.clone(),
            task_id: "workflow".to_string(),
        }
    }

    /// Create a fallback namespace for resolution hierarchy.
    ///
    /// Returns a namespace with the tenant_id changed to "public" for
    /// fallback resolution when tenant-specific tasks are not found.
    ///
    /// # Returns
    ///
    /// A new TaskNamespace with "public" as tenant_id, or None if already public
    pub fn public_fallback(&self) -> Option<Self> {
        if self.is_public() {
            None
        } else {
            Some(Self {
                tenant_id: "public".to_string(),
                package_name: self.package_name.clone(),
                workflow_id: self.workflow_id.clone(),
                task_id: self.task_id.clone(),
            })
        }
    }

    /// Parse a namespace from its string representation.
    ///
    /// # Arguments
    ///
    /// * `namespace_str` - String in format "tenant_id::package_name::workflow_id::task_id"
    ///
    /// # Returns
    ///
    /// * `Ok(TaskNamespace)` - If parsing succeeds
    /// * `Err(String)` - If format is invalid
    pub fn from_string(namespace_str: &str) -> Result<Self, String> {
        let parts: Vec<&str> = namespace_str.split("::").collect();
        if parts.len() != 4 {
            return Err(format!("Invalid namespace format: '{}'. Expected format: 'tenant_id::package_name::workflow_id::task_id'", namespace_str));
        }

        Ok(Self::new(parts[0], parts[1], parts[2], parts[3]))
    }
}

impl Display for TaskNamespace {
    /// Format the namespace as a string using the standard format.
    ///
    /// Format: `tenant_id::package_name::workflow_id::task_id`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina::TaskNamespace;
    ///
    /// let ns = TaskNamespace::embedded("etl", "extract");
    /// assert_eq!(ns.to_string(), "public::embedded::etl::extract");
    ///
    /// let ns = TaskNamespace::tenant("tenant_1", "pkg.so", "analytics", "process");
    /// assert_eq!(ns.to_string(), "tenant_1::pkg.so::analytics::process");
    /// ```
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "{}::{}::{}::{}",
            self.tenant_id, self.package_name, self.workflow_id, self.task_id
        )
    }
}

/// Parse a namespace string back into a TaskNamespace.
///
/// Supports parsing namespace strings in the standard format back into
/// structured TaskNamespace objects.
///
/// # Arguments
///
/// * `namespace_str` - String in format "tenant::package::workflow::task"
///
/// # Returns
///
/// * `Ok(TaskNamespace)` - Successfully parsed namespace
/// * `Err(String)` - Parse error message
///
/// # Examples
///
/// ```rust
/// use cloacina::parse_namespace;
///
/// let ns = parse_namespace("public::embedded::etl::extract").unwrap();
/// assert_eq!(ns.tenant_id, "public");
/// assert_eq!(ns.task_id, "extract");
///
/// // Invalid format
/// assert!(parse_namespace("invalid_format").is_err());
/// ```
pub fn parse_namespace(namespace_str: &str) -> Result<TaskNamespace, String> {
    let parts: Vec<&str> = namespace_str.split("::").collect();

    if parts.len() != 4 {
        return Err(format!(
            "Invalid namespace format '{}'. Expected 'tenant::package::workflow::task'",
            namespace_str
        ));
    }

    Ok(TaskNamespace::new(parts[0], parts[1], parts[2], parts[3]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_namespace() {
        let ns = TaskNamespace::embedded("customer_etl", "extract_data");

        assert_eq!(ns.tenant_id, "public");
        assert_eq!(ns.package_name, "embedded");
        assert_eq!(ns.workflow_id, "customer_etl");
        assert_eq!(ns.task_id, "extract_data");

        assert!(ns.is_public());
        assert!(ns.is_embedded());
    }

    #[test]
    fn test_packaged_namespace() {
        let ns = TaskNamespace::packaged("analytics.so", "data_pipeline", "extract_data");

        assert_eq!(ns.tenant_id, "public");
        assert_eq!(ns.package_name, "analytics.so");
        assert_eq!(ns.workflow_id, "data_pipeline");
        assert_eq!(ns.task_id, "extract_data");

        assert!(ns.is_public());
        assert!(!ns.is_embedded());
    }

    #[test]
    fn test_tenant_namespace() {
        let ns = TaskNamespace::tenant(
            "customer_123",
            "embedded",
            "order_processing",
            "validate_order",
        );

        assert_eq!(ns.tenant_id, "customer_123");
        assert_eq!(ns.package_name, "embedded");
        assert_eq!(ns.workflow_id, "order_processing");
        assert_eq!(ns.task_id, "validate_order");

        assert!(!ns.is_public());
        assert!(ns.is_embedded());
    }

    #[test]
    fn test_namespace_display() {
        let ns = TaskNamespace::embedded("etl", "extract");
        assert_eq!(ns.to_string(), "public::embedded::etl::extract");

        let ns = TaskNamespace::tenant("tenant_1", "pkg.so", "analytics", "process");
        assert_eq!(ns.to_string(), "tenant_1::pkg.so::analytics::process");
    }

    #[test]
    fn test_workflow_scope() {
        let ns = TaskNamespace::embedded("customer_etl", "extract_data");
        let workflow_ns = ns.workflow_scope();

        assert_eq!(workflow_ns.tenant_id, "public");
        assert_eq!(workflow_ns.package_name, "embedded");
        assert_eq!(workflow_ns.workflow_id, "customer_etl");
        assert_eq!(workflow_ns.task_id, "workflow");
    }

    #[test]
    fn test_public_fallback() {
        let tenant_ns = TaskNamespace::tenant("customer_123", "embedded", "etl", "extract");
        let fallback = tenant_ns.public_fallback().unwrap();

        assert_eq!(fallback.tenant_id, "public");
        assert_eq!(fallback.package_name, "embedded");
        assert_eq!(fallback.workflow_id, "etl");
        assert_eq!(fallback.task_id, "extract");

        // Public namespace has no fallback
        let public_ns = TaskNamespace::embedded("etl", "extract");
        assert!(public_ns.public_fallback().is_none());
    }

    #[test]
    fn test_parse_namespace() {
        let ns = parse_namespace("public::embedded::etl::extract").unwrap();
        assert_eq!(ns.tenant_id, "public");
        assert_eq!(ns.package_name, "embedded");
        assert_eq!(ns.workflow_id, "etl");
        assert_eq!(ns.task_id, "extract");

        let ns = parse_namespace("tenant_123::analytics.so::data_pipeline::transform").unwrap();
        assert_eq!(ns.tenant_id, "tenant_123");
        assert_eq!(ns.package_name, "analytics.so");
        assert_eq!(ns.workflow_id, "data_pipeline");
        assert_eq!(ns.task_id, "transform");

        // Test error cases
        assert!(parse_namespace("invalid").is_err());
        assert!(parse_namespace("too::few::parts").is_err());
        assert!(parse_namespace("too::many::parts::here::extra").is_err());
    }

    #[test]
    fn test_namespace_equality_and_hashing() {
        let ns1 = TaskNamespace::embedded("etl", "extract");
        let ns2 = TaskNamespace::embedded("etl", "extract");
        let ns3 = TaskNamespace::embedded("etl", "transform");

        assert_eq!(ns1, ns2);
        assert_ne!(ns1, ns3);

        // Test that they can be used as HashMap keys
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(ns1.clone(), "task1");
        map.insert(ns3.clone(), "task2");

        assert_eq!(map.get(&ns2), Some(&"task1"));
        assert_eq!(map.len(), 2);
    }
}
