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

//! Database backend types and runtime backend selection.

// Conditional imports based on enabled features
#[cfg(feature = "postgres")]
use deadpool_diesel::postgres::{Manager as PgManager, Pool as PgPool};
#[cfg(feature = "postgres")]
use diesel::PgConnection;

#[cfg(feature = "sqlite")]
use deadpool_diesel::sqlite::Pool as SqlitePool;
#[cfg(feature = "sqlite")]
use diesel::SqliteConnection;

// =============================================================================
// Runtime Database Backend Selection
// =============================================================================

/// Represents the database backend type, detected at runtime from the connection URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    /// PostgreSQL backend
    #[cfg(feature = "postgres")]
    Postgres,
    /// SQLite backend
    #[cfg(feature = "sqlite")]
    Sqlite,
}

impl BackendType {
    /// Detect the backend type from a connection URL.
    ///
    /// # Arguments
    /// * `url` - The database connection URL
    ///
    /// # Returns
    /// The detected `BackendType`
    ///
    /// # Panics
    /// Panics if the URL scheme doesn't match any enabled backend.
    #[allow(unused_variables)]
    pub fn from_url(url: &str) -> Self {
        #[cfg(feature = "postgres")]
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            return BackendType::Postgres;
        }

        // SQLite URLs can be:
        // - sqlite:// prefix
        // - file: URI format (e.g., file:test?mode=memory&cache=shared)
        // - file paths (relative or absolute)
        // - :memory: for in-memory databases
        #[cfg(feature = "sqlite")]
        if url.starts_with("sqlite://")
            || url.starts_with("file:")
            || url.starts_with("/")
            || url.starts_with("./")
            || url.starts_with("../")
            || url == ":memory:"
            || url.ends_with(".db")
            || url.ends_with(".sqlite")
            || url.ends_with(".sqlite3")
        {
            return BackendType::Sqlite;
        }

        #[cfg(all(feature = "postgres", feature = "sqlite"))]
        panic!(
            "Unable to detect database backend from URL '{}'. \
             Expected postgres://, postgresql://, sqlite://, or a file path.",
            url
        );

        #[cfg(all(feature = "postgres", not(feature = "sqlite")))]
        panic!(
            "Unable to detect database backend from URL '{}'. \
             Expected postgres:// or postgresql:// (sqlite feature not enabled).",
            url
        );

        #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
        panic!(
            "Unable to detect database backend from URL '{}'. \
             Expected sqlite://, file path, or :memory: (postgres feature not enabled).",
            url
        );

        #[cfg(not(any(feature = "postgres", feature = "sqlite")))]
        panic!("No database backend enabled. Enable either 'postgres' or 'sqlite' feature.");
    }
}

// =============================================================================
// AnyConnection - Multi-backend connection type
// =============================================================================
// When both backends are enabled, use an enum with MultiConnection derive.
// When only one backend is enabled, use a type alias for simpler code.

/// Multi-connection enum that wraps both PostgreSQL and SQLite connections.
///
/// This enum enables runtime database backend selection using Diesel's
/// `MultiConnection` derive macro. The actual connection type is determined
/// at runtime based on the connection URL.
#[cfg(all(feature = "postgres", feature = "sqlite"))]
#[derive(diesel::MultiConnection)]
pub enum AnyConnection {
    /// PostgreSQL connection variant
    Postgres(PgConnection),
    /// SQLite connection variant
    Sqlite(SqliteConnection),
}

/// When only PostgreSQL is enabled, AnyConnection is just a PgConnection.
#[cfg(all(feature = "postgres", not(feature = "sqlite")))]
pub type AnyConnection = PgConnection;

/// When only SQLite is enabled, AnyConnection is just a SqliteConnection.
#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub type AnyConnection = SqliteConnection;

// =============================================================================
// AnyPool - Multi-backend connection pool type
// =============================================================================
// When both backends are enabled, use an enum.
// When only one backend is enabled, use a type alias.

/// Pool enum that wraps both PostgreSQL and SQLite connection pools.
///
/// This enum enables runtime pool selection based on the detected backend.
#[cfg(all(feature = "postgres", feature = "sqlite"))]
#[derive(Clone)]
pub enum AnyPool {
    /// PostgreSQL connection pool
    Postgres(PgPool),
    /// SQLite connection pool
    Sqlite(SqlitePool),
}

#[cfg(all(feature = "postgres", feature = "sqlite"))]
impl std::fmt::Debug for AnyPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnyPool::Postgres(_) => write!(f, "AnyPool::Postgres(...)"),
            AnyPool::Sqlite(_) => write!(f, "AnyPool::Sqlite(...)"),
        }
    }
}

#[cfg(all(feature = "postgres", feature = "sqlite"))]
impl AnyPool {
    /// Returns a reference to the PostgreSQL pool if this is a PostgreSQL backend.
    pub fn as_postgres(&self) -> Option<&PgPool> {
        match self {
            AnyPool::Postgres(pool) => Some(pool),
            _ => None,
        }
    }

    /// Returns a reference to the SQLite pool if this is a SQLite backend.
    pub fn as_sqlite(&self) -> Option<&SqlitePool> {
        match self {
            AnyPool::Sqlite(pool) => Some(pool),
            _ => None,
        }
    }

    /// Returns the PostgreSQL pool, panicking if this is not a PostgreSQL backend.
    pub fn expect_postgres(&self) -> &PgPool {
        match self {
            AnyPool::Postgres(pool) => pool,
            _ => panic!("Expected PostgreSQL pool but got SQLite"),
        }
    }

    /// Returns the SQLite pool, panicking if this is not a SQLite backend.
    pub fn expect_sqlite(&self) -> &SqlitePool {
        match self {
            AnyPool::Sqlite(pool) => pool,
            _ => panic!("Expected SQLite pool but got PostgreSQL"),
        }
    }

    /// Closes the connection pool, releasing all connections.
    ///
    /// After calling this method, all current and future attempts to get
    /// connections from the pool will fail immediately.
    pub fn close(&self) {
        match self {
            AnyPool::Postgres(pool) => pool.close(),
            AnyPool::Sqlite(pool) => pool.close(),
        }
    }
}

/// When only PostgreSQL is enabled, AnyPool is just a PgPool.
#[cfg(all(feature = "postgres", not(feature = "sqlite")))]
pub type AnyPool = PgPool;

/// When only SQLite is enabled, AnyPool is just a SqlitePool.
#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub type AnyPool = SqlitePool;

// =============================================================================
// Legacy Type Aliases (for backward compatibility during migration)
// =============================================================================
// Note: With dual-backend support, use AnyConnection and AnyPool instead.
// These aliases default to PostgreSQL for backwards compatibility.

/// Type alias for the connection type (defaults to PostgreSQL)
#[cfg(feature = "postgres")]
pub type DbConnection = PgConnection;

/// Type alias for the connection type (SQLite when postgres not enabled)
#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub type DbConnection = SqliteConnection;

/// Type alias for the connection manager (defaults to PostgreSQL)
#[cfg(feature = "postgres")]
pub type DbConnectionManager = PgManager;

/// Type alias for the connection pool (defaults to PostgreSQL)
#[cfg(feature = "postgres")]
pub type DbPool = PgPool;

/// Type alias for the connection pool (SQLite when postgres not enabled)
#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub type DbPool = SqlitePool;

// =============================================================================
// Backend Dispatch Macro
// =============================================================================
// This macro handles conditional compilation for backend dispatch patterns.
// When both backends are enabled, it uses a match statement.
// When only one backend is enabled, it compiles only that branch.

/// Dispatches to backend-specific code based on compile-time features.
///
/// Usage:
/// ```ignore
/// dispatch_backend!(
///     backend_type_expr,
///     postgres_expr,
///     sqlite_expr
/// )
/// ```
///
/// When both postgres and sqlite features are enabled, this expands to a match statement.
/// When only one feature is enabled, this compiles only the relevant branch.
#[macro_export]
macro_rules! dispatch_backend {
    ($backend:expr, $pg_branch:expr, $sqlite_branch:expr) => {{
        // Both backends enabled: use match
        #[cfg(all(feature = "postgres", feature = "sqlite"))]
        {
            match $backend {
                $crate::database::BackendType::Postgres => $pg_branch,
                $crate::database::BackendType::Sqlite => $sqlite_branch,
            }
        }

        // Only postgres enabled: directly use postgres branch
        #[cfg(all(feature = "postgres", not(feature = "sqlite")))]
        {
            let _ = $backend; // suppress unused warning
            $pg_branch
        }

        // Only sqlite enabled: directly use sqlite branch
        #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
        {
            let _ = $backend; // suppress unused warning
            $sqlite_branch
        }
    }};
}
