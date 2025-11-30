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

//! Database connection management module supporting both PostgreSQL and SQLite.
//!
//! This module provides an async connection pool implementation using `deadpool-diesel` for managing
//! database connections efficiently. It handles async connection pooling, connection lifecycle,
//! and provides a thread-safe way to access database connections.
//!
//! # Features
//!
//! - Connection pooling with configurable pool size
//! - Thread-safe connection management
//! - Automatic connection cleanup
//! - URL-based configuration for PostgreSQL
//! - File path or `:memory:` configuration for SQLite
//!
//! # Example
//!
//! ```rust
//! use cloacina::database::connection::Database;
//!
//! // PostgreSQL
//! #[cfg(feature = "postgres")]
//! let db = Database::new(
//!     "postgres://username:password@localhost:5432",
//!     "my_database",
//!     10
//! );
//!
//! // SQLite
//! #[cfg(feature = "sqlite")]
//! let db = Database::new(
//!     "path/to/database.db",
//!     "", // Not used for SQLite
//!     10
//! );
//! ```

use tracing::info;

#[cfg(feature = "postgres")]
use deadpool_diesel::postgres::{
    Manager as PgManager, Pool as PgPool, Runtime as PgRuntime,
};
#[cfg(feature = "postgres")]
use diesel::PgConnection;
#[cfg(feature = "postgres")]
use url::Url;

#[cfg(feature = "sqlite")]
use deadpool_diesel::sqlite::{
    Manager as SqliteManager, Pool as SqlitePool, Runtime as SqliteRuntime,
};
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
    pub fn from_url(url: &str) -> Self {
        #[cfg(feature = "postgres")]
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            return BackendType::Postgres;
        }

        #[cfg(feature = "sqlite")]
        {
            // SQLite URLs can be:
            // - sqlite:// prefix
            // - file paths (relative or absolute)
            // - :memory: for in-memory databases
            if url.starts_with("sqlite://")
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
        }

        panic!(
            "Unable to detect database backend from URL '{}'. \
             Expected postgres://, postgresql://, sqlite://, or a file path.",
            url
        );
    }
}

/// Multi-connection enum that wraps both PostgreSQL and SQLite connections.
///
/// This enum enables runtime database backend selection using Diesel's
/// `MultiConnection` derive macro. The actual connection type is determined
/// at runtime based on the connection URL.
#[derive(diesel::MultiConnection)]
pub enum AnyConnection {
    /// PostgreSQL connection variant
    #[cfg(feature = "postgres")]
    Postgres(PgConnection),
    /// SQLite connection variant
    #[cfg(feature = "sqlite")]
    Sqlite(SqliteConnection),
}

/// Pool enum that wraps both PostgreSQL and SQLite connection pools.
///
/// This enum enables runtime pool selection based on the detected backend.
#[derive(Clone)]
pub enum AnyPool {
    /// PostgreSQL connection pool
    #[cfg(feature = "postgres")]
    Postgres(PgPool),
    /// SQLite connection pool
    #[cfg(feature = "sqlite")]
    Sqlite(SqlitePool),
}

impl std::fmt::Debug for AnyPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "postgres")]
            AnyPool::Postgres(_) => write!(f, "AnyPool::Postgres(...)"),
            #[cfg(feature = "sqlite")]
            AnyPool::Sqlite(_) => write!(f, "AnyPool::Sqlite(...)"),
        }
    }
}

impl AnyPool {
    /// Returns a reference to the PostgreSQL pool if this is a PostgreSQL backend.
    #[cfg(feature = "postgres")]
    pub fn as_postgres(&self) -> Option<&PgPool> {
        match self {
            AnyPool::Postgres(pool) => Some(pool),
            #[cfg(feature = "sqlite")]
            _ => None,
        }
    }

    /// Returns a reference to the SQLite pool if this is a SQLite backend.
    #[cfg(feature = "sqlite")]
    pub fn as_sqlite(&self) -> Option<&SqlitePool> {
        match self {
            AnyPool::Sqlite(pool) => Some(pool),
            #[cfg(feature = "postgres")]
            _ => None,
        }
    }

    /// Returns the PostgreSQL pool, panicking if this is not a PostgreSQL backend.
    #[cfg(feature = "postgres")]
    pub fn expect_postgres(&self) -> &PgPool {
        match self {
            AnyPool::Postgres(pool) => pool,
            #[cfg(feature = "sqlite")]
            _ => panic!("Expected PostgreSQL pool but got SQLite"),
        }
    }

    /// Returns the SQLite pool, panicking if this is not a SQLite backend.
    #[cfg(feature = "sqlite")]
    pub fn expect_sqlite(&self) -> &SqlitePool {
        match self {
            AnyPool::Sqlite(pool) => pool,
            #[cfg(feature = "postgres")]
            _ => panic!("Expected SQLite pool but got PostgreSQL"),
        }
    }

    /// Gets a connection from the pool.
    ///
    /// This method is only available when exactly one backend is enabled.
    /// For dual-backend builds, use `expect_postgres().get()` or `expect_sqlite().get()`.
    #[cfg(all(feature = "postgres", not(feature = "sqlite")))]
    pub async fn get(
        &self,
    ) -> Result<
        deadpool::managed::Object<PgManager>,
        deadpool::managed::PoolError<deadpool_diesel::Error>,
    > {
        match self {
            AnyPool::Postgres(pool) => pool.get().await,
        }
    }

    /// Gets a connection from the pool.
    ///
    /// This method is only available when exactly one backend is enabled.
    /// For dual-backend builds, use `expect_postgres().get()` or `expect_sqlite().get()`.
    #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
    pub async fn get(
        &self,
    ) -> Result<
        deadpool::managed::Object<SqliteManager>,
        deadpool::managed::PoolError<deadpool_diesel::Error>,
    > {
        match self {
            AnyPool::Sqlite(pool) => pool.get().await,
        }
    }
}

// =============================================================================
// Legacy Type Aliases (for backward compatibility during migration)
// =============================================================================

/// Type alias for the connection type based on the selected backend
#[cfg(all(feature = "postgres", not(feature = "sqlite")))]
pub type DbConnection = PgConnection;

#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub type DbConnection = SqliteConnection;

/// Type alias for the connection manager based on the selected backend
#[cfg(all(feature = "postgres", not(feature = "sqlite")))]
pub type DbConnectionManager = PgManager;

#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub type DbConnectionManager = SqliteManager;

/// Type alias for the connection pool
#[cfg(all(feature = "postgres", not(feature = "sqlite")))]
pub type DbPool = PgPool;

#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub type DbPool = SqlitePool;

/// Represents a pool of database connections.
///
/// This struct provides a thread-safe wrapper around a connection pool,
/// allowing multiple parts of the application to share database connections
/// efficiently. Supports runtime backend selection between PostgreSQL and SQLite.
///
/// # Thread Safety
///
/// The `Database` struct is `Clone` and can be safely shared between threads.
/// Each clone references the same underlying connection pool.
#[derive(Clone, Debug)]
pub struct Database {
    /// The connection pool (PostgreSQL or SQLite)
    pool: AnyPool,
    /// The detected backend type
    backend: BackendType,
    /// The PostgreSQL schema name for multi-tenant isolation (ignored for SQLite)
    schema: Option<String>,
}

impl Database {
    /// Creates a new database connection pool with automatic backend detection.
    ///
    /// The backend is detected from the connection string:
    /// - `postgres://` or `postgresql://` -> PostgreSQL
    /// - `sqlite://`, file paths, or `:memory:` -> SQLite
    ///
    /// # Arguments
    ///
    /// * `connection_string` - The database connection URL or path
    /// * `database_name` - The database name (used for PostgreSQL, ignored for SQLite)
    /// * `max_size` - Maximum number of connections in the pool
    ///
    /// # Panics
    ///
    /// Panics if the connection pool cannot be created.
    pub fn new(connection_string: &str, database_name: &str, max_size: u32) -> Self {
        Self::new_with_schema(connection_string, database_name, max_size, None)
    }

    /// Creates a new database connection pool with optional schema support.
    ///
    /// The backend is detected from the connection string. Schema support is only
    /// effective for PostgreSQL; the schema parameter is stored but ignored for SQLite.
    ///
    /// # Arguments
    ///
    /// * `connection_string` - The database connection URL or path
    /// * `database_name` - The database name (used for PostgreSQL, ignored for SQLite)
    /// * `max_size` - Maximum number of connections in the pool
    /// * `schema` - Optional schema name for PostgreSQL multi-tenant isolation
    pub fn new_with_schema(
        connection_string: &str,
        _database_name: &str,
        max_size: u32,
        schema: Option<&str>,
    ) -> Self {
        let backend = BackendType::from_url(connection_string);

        match backend {
            #[cfg(feature = "postgres")]
            BackendType::Postgres => {
                let connection_url = Self::build_postgres_url(connection_string, _database_name);
                let manager = PgManager::new(connection_url, PgRuntime::Tokio1);
                let pool = PgPool::builder(manager)
                    .max_size(max_size as usize)
                    .build()
                    .expect("Failed to create PostgreSQL connection pool");

                info!(
                    "PostgreSQL connection pool initialized{}",
                    schema.map_or(String::new(), |s| format!(" with schema '{}'", s))
                );

                Self {
                    pool: AnyPool::Postgres(pool),
                    backend,
                    schema: schema.map(String::from),
                }
            }
            #[cfg(feature = "sqlite")]
            BackendType::Sqlite => {
                let connection_url = Self::build_sqlite_url(connection_string);
                let manager = SqliteManager::new(connection_url, SqliteRuntime::Tokio1);
                let pool = SqlitePool::builder(manager)
                    .max_size(max_size as usize)
                    .build()
                    .expect("Failed to create SQLite connection pool");

                info!("SQLite connection pool initialized");

                Self {
                    pool: AnyPool::Sqlite(pool),
                    backend,
                    schema: schema.map(String::from),
                }
            }
        }
    }

    /// Returns the detected backend type.
    pub fn backend(&self) -> BackendType {
        self.backend
    }

    /// Returns the schema name if set.
    pub fn schema(&self) -> Option<&str> {
        self.schema.as_deref()
    }

    /// Returns a clone of the connection pool.
    pub fn pool(&self) -> AnyPool {
        self.pool.clone()
    }

    /// Alias for `pool()` for backward compatibility.
    pub fn get_connection(&self) -> AnyPool {
        self.pool.clone()
    }

    /// Builds a PostgreSQL connection URL.
    #[cfg(feature = "postgres")]
    fn build_postgres_url(base_url: &str, database_name: &str) -> String {
        let mut url = Url::parse(base_url).expect("Invalid PostgreSQL URL");
        url.set_path(database_name);
        url.to_string()
    }

    /// Builds a SQLite connection URL.
    #[cfg(feature = "sqlite")]
    fn build_sqlite_url(connection_string: &str) -> String {
        // Strip sqlite:// prefix if present
        if let Some(path) = connection_string.strip_prefix("sqlite://") {
            path.to_string()
        } else {
            connection_string.to_string()
        }
    }

    /// Sets up the PostgreSQL schema for multi-tenant isolation.
    ///
    /// Creates the schema if it doesn't exist and runs migrations within it.
    /// Returns an error if called on a SQLite backend.
    #[cfg(feature = "postgres")]
    pub async fn setup_schema(&self, schema: &str) -> Result<(), String> {
        use diesel::prelude::*;

        let pool = match &self.pool {
            AnyPool::Postgres(pool) => pool,
            #[cfg(feature = "sqlite")]
            AnyPool::Sqlite(_) => {
                return Err("Schema setup is not supported for SQLite".to_string());
            }
        };

        let conn = pool.get().await.map_err(|e| e.to_string())?;

        let schema_name = schema.to_string();
        let schema_name_clone = schema_name.clone();

        // Create schema if it doesn't exist
        conn.interact(move |conn| {
            let create_schema_sql = format!("CREATE SCHEMA IF NOT EXISTS {}", schema_name);
            diesel::sql_query(&create_schema_sql).execute(conn)
        })
        .await
        .map_err(|e| format!("Failed to create schema: {}", e))?
        .map_err(|e| format!("Failed to create schema: {}", e))?;

        // Set search path for migrations
        conn.interact(move |conn| {
            let set_search_path_sql = format!("SET search_path TO {}, public", schema_name_clone);
            diesel::sql_query(&set_search_path_sql).execute(conn)
        })
        .await
        .map_err(|e| format!("Failed to set search path: {}", e))?
        .map_err(|e| format!("Failed to set search path: {}", e))?;

        // Run migrations in the schema
        conn.interact(|conn| {
            use diesel_migrations::MigrationHarness;
            conn.run_pending_migrations(crate::database::POSTGRES_MIGRATIONS)
                .expect("Failed to run migrations");
        })
        .await
        .map_err(|e| format!("Failed to run migrations in schema: {}", e))?;

        info!("Schema '{}' set up successfully", schema);
        Ok(())
    }

    /// Gets a PostgreSQL connection with the schema search path set.
    ///
    /// For PostgreSQL, this sets the search path to the configured schema.
    /// For SQLite, this is a no-op and returns an error.
    #[cfg(feature = "postgres")]
    pub async fn get_connection_with_schema(
        &self,
    ) -> Result<
        deadpool::managed::Object<PgManager>,
        deadpool::managed::PoolError<deadpool_diesel::Error>,
    > {
        use diesel::prelude::*;

        let pool = match &self.pool {
            AnyPool::Postgres(pool) => pool,
            #[cfg(feature = "sqlite")]
            AnyPool::Sqlite(_) => {
                panic!("get_connection_with_schema called on SQLite backend");
            }
        };

        let conn = pool.get().await?;

        if let Some(ref schema) = self.schema {
            let schema_name = schema.clone();
            let _ = conn
                .interact(move |conn| {
                    let set_search_path_sql = format!("SET search_path TO {}, public", schema_name);
                    diesel::sql_query(&set_search_path_sql).execute(conn)
                })
                .await;
        }

        Ok(conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "postgres")]
    fn test_postgres_url_parsing_scenarios() {
        // Test complete URL with credentials and port
        let mut url = Url::parse("postgres://postgres:postgres@localhost:5432").unwrap();
        url.set_path("test_db");
        assert_eq!(url.path(), "/test_db");
        assert_eq!(url.scheme(), "postgres");
        assert_eq!(url.host_str(), Some("localhost"));
        assert_eq!(url.port(), Some(5432));
        assert_eq!(url.username(), "postgres");
        assert_eq!(url.password(), Some("postgres"));

        // Test URL without port
        let mut url = Url::parse("postgres://postgres:postgres@localhost").unwrap();
        url.set_path("test_db");
        assert_eq!(url.port(), None);

        // Test URL without credentials
        let mut url = Url::parse("postgres://localhost:5432").unwrap();
        url.set_path("test_db");
        assert_eq!(url.username(), "");
        assert_eq!(url.password(), None);

        // Test invalid URL
        assert!(Url::parse("not-a-url").is_err());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn test_sqlite_connection_strings() {
        // Test file path
        let url = Database::build_sqlite_url("/path/to/database.db");
        assert_eq!(url, "/path/to/database.db");

        // Test in-memory database
        let url = Database::build_sqlite_url(":memory:");
        assert_eq!(url, ":memory:");

        // Test relative path
        let url = Database::build_sqlite_url("./database.db");
        assert_eq!(url, "./database.db");

        // Test sqlite:// prefix stripping
        let url = Database::build_sqlite_url("sqlite:///path/to/db.sqlite");
        assert_eq!(url, "/path/to/db.sqlite");
    }

    #[test]
    fn test_backend_type_detection() {
        #[cfg(feature = "postgres")]
        {
            assert_eq!(BackendType::from_url("postgres://localhost/db"), BackendType::Postgres);
            assert_eq!(BackendType::from_url("postgresql://localhost/db"), BackendType::Postgres);
        }

        #[cfg(feature = "sqlite")]
        {
            assert_eq!(BackendType::from_url("sqlite:///path/to/db"), BackendType::Sqlite);
            assert_eq!(BackendType::from_url("/absolute/path.db"), BackendType::Sqlite);
            assert_eq!(BackendType::from_url("./relative/path.db"), BackendType::Sqlite);
            assert_eq!(BackendType::from_url(":memory:"), BackendType::Sqlite);
            assert_eq!(BackendType::from_url("database.sqlite"), BackendType::Sqlite);
            assert_eq!(BackendType::from_url("database.sqlite3"), BackendType::Sqlite);
        }
    }
}
