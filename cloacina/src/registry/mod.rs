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

//! # Workflow Registry
//!
//! This module provides a registry system for dynamically loading and managing
//! packaged workflows (.so files) created by cloacina-ctl. The registry enables
//! runtime distribution of workflows with proper isolation and versioning.
//!
//! ## Architecture
//!
//! The registry uses a two-table design:
//! - `workflow_registry`: Simple key-value storage for binary .so data
//! - `workflow_packages`: Rich metadata with foreign key to registry
//!
//! This separation allows efficient metadata queries without loading binaries
//! and enables future migration to object storage for the binary data.
//!
//! ## Key Components
//!
//! - [`traits`]: Core trait definitions for registry and storage
//! - [`types`]: Data types for workflows, metadata, and errors
//! - [`error`]: Error types for registry operations
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use cloacina::registry::{WorkflowRegistry, WorkflowPackage};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create registry
//! let mut registry = WorkflowRegistryImpl::new(database, storage)?;
//!
//! // Register a packaged workflow
//! let package = WorkflowPackage::from_file("analytics.cloacina")?;
//! let package_id = registry.register_workflow(package).await?;
//!
//! // List registered workflows
//! let workflows = registry.list_workflows().await?;
//!
//! // Load and execute
//! let loaded = registry.get_workflow("analytics", "1.0.0").await?;
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod traits;
pub mod types;

// Re-export commonly used types
pub use error::{RegistryError, StorageError};
pub use traits::{RegistryStorage, WorkflowRegistry};
pub use types::{LoadedWorkflow, WorkflowMetadata, WorkflowPackage, WorkflowPackageId};
