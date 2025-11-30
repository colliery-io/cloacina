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
//! - unified: Runtime backend selection (PostgreSQL or SQLite)
//! - postgres_dal: Legacy PostgreSQL backend (being migrated)
//! - sqlite_dal: Legacy SQLite backend (being migrated)
//! - filesystem_dal: For filesystem-based storage operations
//!
//! # Migration Status
//!
//! The codebase is transitioning from compile-time backend selection
//! (postgres_dal/sqlite_dal) to runtime backend selection (unified).
//! During the transition, both approaches are available.

// Unified DAL with runtime backend selection (new approach)
#[cfg(any(feature = "postgres", feature = "sqlite"))]
pub mod unified;

// Legacy DAL implementations (being migrated to unified)
#[cfg(feature = "postgres")]
mod postgres_dal;

#[cfg(feature = "sqlite")]
mod sqlite_dal;

// Filesystem DAL is always available
mod filesystem_dal;

// Re-export the appropriate legacy DAL implementation for backward compatibility
#[cfg(all(feature = "postgres", not(feature = "sqlite")))]
pub use postgres_dal::*;

#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub use sqlite_dal::*;

// When both features are enabled, export under qualified names
#[cfg(all(feature = "postgres", feature = "sqlite"))]
pub mod legacy_postgres {
    pub use super::postgres_dal::*;
}

#[cfg(all(feature = "postgres", feature = "sqlite"))]
pub mod legacy_sqlite {
    pub use super::sqlite_dal::*;
}

// When both features are enabled, export unified DAL as the primary DAL
#[cfg(all(feature = "postgres", feature = "sqlite"))]
pub use unified::DAL;

// Export CronExecutionStats from the unified module
#[cfg(all(feature = "postgres", feature = "sqlite"))]
pub use unified::cron_execution::CronExecutionStats;

// Re-export registry storage types for dual-backend builds
#[cfg(all(feature = "postgres", feature = "sqlite"))]
pub use postgres_dal::PostgresRegistryStorage;

#[cfg(all(feature = "postgres", feature = "sqlite"))]
pub use sqlite_dal::SqliteRegistryStorage;

// Always re-export filesystem DAL
pub use filesystem_dal::FilesystemRegistryStorage;

// Re-export unified DAL types for convenience
#[cfg(any(feature = "postgres", feature = "sqlite"))]
pub use unified::DAL as UnifiedDAL;
