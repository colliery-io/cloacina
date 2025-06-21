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

//! Storage backend implementations for the workflow registry.
//!
//! **DEPRECATED**: Storage backends have been moved to the DAL module.
//!
//! Please use the new DAL-based storage implementations:
//! - `crate::dal::PostgresWorkflowRegistryDAL`
//! - `crate::dal::SqliteWorkflowRegistryDAL`
//! - `crate::dal::FilesystemWorkflowRegistryDAL`
//!
//! ## Migration Example
//!
//! ```rust,no_run
//! // Old way (deprecated):
//! // use cloacina::registry::storage::FilesystemRegistryStorage;
//!
//! // New way:
//! use cloacina::dal::FilesystemWorkflowRegistryDAL;
//! use cloacina::registry::RegistryStorage;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Use DAL instead
//! let dal = FilesystemWorkflowRegistryDAL::new("/var/lib/cloacina/registry")?;
//!
//! // Same RegistryStorage trait interface
//! let data = b"compiled workflow binary data";
//! let id = dal.store_binary(data.to_vec()).await?;
//! # Ok(())
//! # }
//! ```

// Re-export DAL implementations for backward compatibility
#[cfg(feature = "postgres")]
pub use crate::dal::PostgresWorkflowRegistryDAL as PostgresRegistryStorage;

#[cfg(feature = "sqlite")]
pub use crate::dal::SqliteWorkflowRegistryDAL as SqliteRegistryStorage;

pub use crate::dal::FilesystemWorkflowRegistryDAL as FilesystemRegistryStorage;
