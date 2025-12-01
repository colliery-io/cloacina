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
//! This module provides storage backends for persisting workflow binaries:
//! - `UnifiedRegistryStorage` - Database storage (PostgreSQL or SQLite)
//! - `FilesystemRegistryStorage` - Filesystem-based storage
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use cloacina::dal::{UnifiedRegistryStorage, FilesystemRegistryStorage};
//! use cloacina::registry::RegistryStorage;
//! use cloacina::database::Database;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Database storage (unified - works with both PostgreSQL and SQLite)
//! let database = Database::new("sqlite::memory:", "test", 10);
//! let db_storage = UnifiedRegistryStorage::new(database);
//!
//! // Filesystem storage
//! let fs_storage = FilesystemRegistryStorage::new("/var/lib/cloacina/registry")?;
//!
//! // Both implement RegistryStorage trait
//! let data = b"compiled workflow binary data";
//! // let id = db_storage.store_binary(data.to_vec()).await?;
//! # Ok(())
//! # }
//! ```

// Re-export DAL implementations
pub use crate::dal::FilesystemRegistryStorage;
pub use crate::dal::UnifiedRegistryStorage;
