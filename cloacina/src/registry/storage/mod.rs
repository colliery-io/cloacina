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
//! This module provides concrete implementations of the `RegistryStorage` trait
//! for different storage backends, enabling flexible deployment configurations.
//!
//! ## Available Backends
//!
//! - **PostgreSQL**: Stores binary data in the database using BYTEA columns
//! - **SQLite**: Stores binary data in the database using BLOB columns
//! - **Filesystem**: Stores binary data as files on the local filesystem
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use cloacina::registry::storage::{PostgresRegistryStorage, FilesystemRegistryStorage};
//! use cloacina::registry::RegistryStorage;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // PostgreSQL backend
//! let postgres_storage = PostgresRegistryStorage::new(db_pool).await?;
//!
//! // Filesystem backend
//! let fs_storage = FilesystemRegistryStorage::new("/var/lib/cloacina/registry")?;
//!
//! // Both implement the same RegistryStorage trait
//! let data = b"compiled workflow binary data";
//! let id = postgres_storage.store_binary(data.to_vec()).await?;
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "sqlite")]
pub mod sqlite;

pub mod filesystem;

// Re-export storage implementations for convenience
#[cfg(feature = "postgres")]
pub use postgres::PostgresRegistryStorage;

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteRegistryStorage;

pub use filesystem::FilesystemRegistryStorage;
