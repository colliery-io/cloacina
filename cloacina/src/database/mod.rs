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

//! # Database Layer
//!
//! This module provides database connectivity, schema management, and migration support
//! for persisting Cloacina execution context and metadata.
//!
//! ## Components
//!
//! - [`connection`]: Database connection management and pooling
//! - [`schema`]: Diesel schema definitions
//!
//! ## Database Support
//!
//! Supports both PostgreSQL and SQLite through compile-time feature flags:
//! - PostgreSQL: Full-featured with native UUID and timestamp types
//! - SQLite: File-based or in-memory with type conversions
//!
//! The database layer handles:
//! - Context persistence and retrieval
//! - Task execution state tracking
//! - Automatic schema migrations
//!
//! ## Connection Pooling
//!
//! The module uses r2d2 for connection pooling with the following default settings:
//! - Maximum pool size: 10 connections
//! - Connection timeout: 30 seconds
//! - Idle timeout: 10 minutes
//!
//! These settings can be customized when creating a new pool.
//!
//! ## Error Handling
//!
//! The module provides a custom `Result` type alias that standardizes error handling
//! across database operations. All database operations return this type, which wraps
//! `diesel::result::Error` for consistent error handling.
//!
//! ## Migration System
//!
//! The migration system is built on top of Diesel's migration framework and supports:
//! - Automatic migration detection and application
//! - Version tracking in the database
//! - Rollback support
//! - Transaction-safe migrations
//!
//! Migrations are stored in the `src/database/migrations` directory and are embedded
//! into the binary at compile time.
//!
//! ## Usage
//!
//! ```rust
//! use cloacina::runner::DefaultRunner;
//! use cloacina::database::{DbPool, Result};
//!
//! // Initialize executor (automatically runs migrations)
//! let executor = DefaultRunner::new("postgresql://user:pass@localhost/cloacina").await?;
//!
//! // Manual database access example
//! let pool = DbPool::builder()
//!     .max_size(15)
//!     .build(ConnectionManager::new("postgresql://user:pass@localhost/cloacina"))?;
//!
//! // Get a connection from the pool
//! let conn = pool.get()?;
//!
//! // Run migrations manually if needed
//! run_migrations(&mut conn)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Public Types
//!
//! - [`DbPool`]: The connection pool type for managing database connections
//! - [`Result<T>`]: Type alias for database operation results
//!
//! ## Public Functions
//!
//! - [`run_migrations`]: Manually runs pending database migrations
//!
//! Migrations are automatically applied when using `DefaultRunner`. For lower-level
//! database access, migrations can be run manually using `run_migrations()`.

pub mod admin;
pub mod connection;
pub mod schema;
pub mod universal_types;

use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

// Re-export connection types from the connection module
pub use connection::{AnyConnection, BackendType, Database};

// Legacy type aliases - only available when exactly one backend is enabled
#[cfg(any(
    all(feature = "postgres", not(feature = "sqlite")),
    all(feature = "sqlite", not(feature = "postgres"))
))]
pub use connection::{DbConnection, DbConnectionManager, DbPool};

// Re-export admin types for tenant management
pub use admin::{AdminError, DatabaseAdmin, TenantConfig, TenantCredentials};

/// Type alias for database operation results.
///
/// Standardizes error handling across database operations.
pub type Result<T> = std::result::Result<T, diesel::result::Error>;

// Re-export universal types for convenience
pub use universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};

/// Embedded migrations for PostgreSQL.
#[cfg(feature = "postgres")]
pub const POSTGRES_MIGRATIONS: EmbeddedMigrations =
    embed_migrations!("src/database/migrations/postgres");

/// Embedded migrations for SQLite.
#[cfg(feature = "sqlite")]
pub const SQLITE_MIGRATIONS: EmbeddedMigrations =
    embed_migrations!("src/database/migrations/sqlite");

/// Embedded migrations for automatic schema management.
///
/// Contains all SQL migration files for setting up the database schema.
/// When only one backend is enabled, this aliases to that backend's migrations.
#[cfg(all(feature = "postgres", not(feature = "sqlite")))]
pub const MIGRATIONS: EmbeddedMigrations = POSTGRES_MIGRATIONS;

#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub const MIGRATIONS: EmbeddedMigrations = SQLITE_MIGRATIONS;

/// Runs pending database migrations.
///
/// This function applies any pending migrations to bring the database
/// schema up to date with the current version.
///
/// # Arguments
///
/// * `conn` - Mutable reference to a database connection (PostgreSQL or SQLite)
///
/// # Returns
///
/// * `Ok(())` - If migrations complete successfully
/// * `Err(_)` - If migration fails
///
/// # Examples
///
/// ```rust
/// use cloacina::database::{run_migrations, DbConnection};
///
/// # fn example(mut conn: DbConnection) -> Result<(), diesel::result::Error> {
/// run_migrations(&mut conn)?;
/// # Ok(())
/// # }
/// ```
pub fn run_migrations(conn: &mut DbConnection) -> Result<()> {
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run migrations");
    Ok(())
}
