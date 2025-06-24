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

//! Data Access Layer with conditional backend support
//!
//! This module provides storage-specific DAL implementations:
//! - postgres_dal: For PostgreSQL backend using native types
//! - sqlite_dal: For SQLite backend using universal wrapper types
//! - filesystem_dal: For filesystem-based storage operations

// Conditional imports based on database backend
#[cfg(feature = "postgres")]
mod postgres_dal;

#[cfg(feature = "sqlite")]
mod sqlite_dal;

// Filesystem DAL is always available
mod filesystem_dal;

// Re-export the appropriate DAL implementation
#[cfg(feature = "postgres")]
pub use postgres_dal::*;

#[cfg(feature = "sqlite")]
pub use sqlite_dal::*;

// Always re-export filesystem DAL (but only the specific type to avoid conflicts)
pub use filesystem_dal::FilesystemRegistryStorage;
