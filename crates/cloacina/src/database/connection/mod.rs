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
//! ```rust,ignore
//! use cloacina::database::connection::Database;
//!
//! // PostgreSQL
//! //! let db = Database::new(
//!     "postgres://username:password@localhost:5432",
//!     "my_database",
//!     10
//! );
//!
//! // SQLite
//! //! let db = Database::new(
//!     "path/to/database.db",
//!     "", // Not used for SQLite
//!     10
//! );
//! ```

mod backend;
mod schema_validation;

// Re-export all public types
pub use backend::{AnyConnection, AnyPool, BackendType};
pub use schema_validation::{validate_schema_name, SchemaError};

// Legacy type aliases - conditional on features
#[cfg(feature = "postgres")]
pub use backend::{DbConnection, DbConnectionManager, DbPool};

#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub use backend::{DbConnection, DbPool};

use thiserror::Error;
use tracing::info;
use url::Url;

#[cfg(feature = "postgres")]
use deadpool_diesel::postgres::{Manager as PgManager, Pool as PgPool, Runtime as PgRuntime};
#[cfg(feature = "sqlite")]
use deadpool_diesel::sqlite::{
    Manager as SqliteManager, Pool as SqlitePool, Runtime as SqliteRuntime,
};

/// Errors that can occur during database operations.
///
/// This error type covers connection pool creation, URL parsing,
/// migration execution, and schema validation failures.
#[derive(Debug, Error)]
pub enum DatabaseError {
    /// Failed to create connection pool
    #[error("Failed to create {backend} connection pool: {source}")]
    PoolCreation {
        backend: &'static str,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Failed to parse database URL
    #[error("Invalid database URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// Schema validation failed
    #[error("Schema validation failed: {0}")]
    Schema(#[from] SchemaError),

    /// Migration execution failed
    #[error("Migration failed: {0}")]
    Migration(String),
}

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
#[derive(Clone)]
pub struct Database {
    /// The connection pool (PostgreSQL or SQLite)
    pool: AnyPool,
    /// The detected backend type
    backend: BackendType,
    /// The PostgreSQL schema name for multi-tenant isolation (ignored for SQLite)
    schema: Option<String>,
}

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Database")
            .field("backend", &self.backend)
            .field("schema", &self.schema)
            .field("pool", &"<connection pool>")
            .finish()
    }
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
    ///
    /// # Panics
    ///
    /// Panics if connection pool creation fails or if the schema name is invalid.
    /// Use [`try_new_with_schema`](Self::try_new_with_schema) for fallible construction.
    pub fn new_with_schema(
        connection_string: &str,
        database_name: &str,
        max_size: u32,
        schema: Option<&str>,
    ) -> Self {
        Self::try_new_with_schema(connection_string, database_name, max_size, schema)
            .expect("Failed to create database connection pool")
    }

    /// Creates a new database connection pool with optional schema support.
    ///
    /// This is the fallible version of [`new_with_schema`](Self::new_with_schema).
    ///
    /// # Arguments
    ///
    /// * `connection_string` - The database connection URL or path
    /// * `database_name` - The database name (used for PostgreSQL, ignored for SQLite)
    /// * `max_size` - Maximum number of connections in the pool
    /// * `schema` - Optional schema name for PostgreSQL multi-tenant isolation
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The schema name is invalid (SQL injection prevention)
    /// - The connection pool cannot be created
    pub fn try_new_with_schema(
        connection_string: &str,
        _database_name: &str,
        max_size: u32,
        schema: Option<&str>,
    ) -> Result<Self, DatabaseError> {
        let backend = BackendType::from_url(connection_string);

        // Validate schema name at construction time to prevent SQL injection
        let validated_schema = schema
            .map(|s| validate_schema_name(s).map(|v| v.to_string()))
            .transpose()?;

        #[cfg(all(feature = "postgres", feature = "sqlite"))]
        match backend {
            BackendType::Postgres => {
                let connection_url = Self::build_postgres_url(connection_string, _database_name)?;
                let manager = PgManager::new(connection_url, PgRuntime::Tokio1);
                let pool = PgPool::builder(manager)
                    .max_size(max_size as usize)
                    .build()
                    .map_err(|e| DatabaseError::PoolCreation {
                        backend: "PostgreSQL",
                        source: Box::new(e),
                    })?;

                info!(
                    "PostgreSQL connection pool initialized{}",
                    validated_schema
                        .as_ref()
                        .map_or(String::new(), |s| format!(" with schema '{}'", s))
                );

                return Ok(Self {
                    pool: AnyPool::Postgres(pool),
                    backend,
                    schema: validated_schema,
                });
            }
            BackendType::Sqlite => {
                let connection_url = Self::build_sqlite_url(connection_string);
                let manager = SqliteManager::new(connection_url, SqliteRuntime::Tokio1);
                let sqlite_pool_size = 1;
                let pool = SqlitePool::builder(manager)
                    .max_size(sqlite_pool_size)
                    .build()
                    .map_err(|e| DatabaseError::PoolCreation {
                        backend: "SQLite",
                        source: Box::new(e),
                    })?;

                info!(
                    "SQLite connection pool initialized (size: {})",
                    sqlite_pool_size
                );

                return Ok(Self {
                    pool: AnyPool::Sqlite(pool),
                    backend,
                    schema: validated_schema,
                });
            }
        }

        #[cfg(all(feature = "postgres", not(feature = "sqlite")))]
        {
            let _ = backend; // suppress unused warning
            let connection_url = Self::build_postgres_url(connection_string, _database_name)?;
            let manager = PgManager::new(connection_url, PgRuntime::Tokio1);
            let pool = PgPool::builder(manager)
                .max_size(max_size as usize)
                .build()
                .map_err(|e| DatabaseError::PoolCreation {
                    backend: "PostgreSQL",
                    source: Box::new(e),
                })?;

            info!(
                "PostgreSQL connection pool initialized{}",
                validated_schema
                    .as_ref()
                    .map_or(String::new(), |s| format!(" with schema '{}'", s))
            );

            return Ok(Self {
                pool,
                backend: BackendType::Postgres,
                schema: validated_schema,
            });
        }

        #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
        {
            let _ = backend; // suppress unused warning
            let connection_url = Self::build_sqlite_url(connection_string);
            let manager = SqliteManager::new(connection_url, SqliteRuntime::Tokio1);
            let sqlite_pool_size = 1;
            let pool = SqlitePool::builder(manager)
                .max_size(sqlite_pool_size)
                .build()
                .map_err(|e| DatabaseError::PoolCreation {
                    backend: "SQLite",
                    source: Box::new(e),
                })?;

            info!(
                "SQLite connection pool initialized (size: {})",
                sqlite_pool_size
            );

            return Ok(Self {
                pool,
                backend: BackendType::Sqlite,
                schema: validated_schema,
            });
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

    /// Closes the connection pool, releasing all database connections.
    ///
    /// After calling this method, all current and future attempts to get
    /// connections from the pool will fail immediately. This should be called
    /// when shutting down to ensure connections are properly released back to
    /// the database server.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let db = Database::new("postgres://localhost/mydb", "mydb", 10)?;
    /// // ... use database ...
    /// db.close(); // Release all connections
    /// ```
    pub fn close(&self) {
        tracing::info!("Closing database connection pool");
        self.pool.close();
    }

    /// Builds a PostgreSQL connection URL.
    fn build_postgres_url(base_url: &str, database_name: &str) -> Result<String, url::ParseError> {
        let mut url = Url::parse(base_url)?;
        url.set_path(database_name);
        Ok(url.to_string())
    }

    /// Builds a SQLite connection URL.
    fn build_sqlite_url(connection_string: &str) -> String {
        // Strip sqlite:// prefix if present
        if let Some(path) = connection_string.strip_prefix("sqlite://") {
            path.to_string()
        } else {
            connection_string.to_string()
        }
    }

    /// Runs pending database migrations for the appropriate backend.
    ///
    /// This method detects the backend type and runs the corresponding migrations.
    pub async fn run_migrations(&self) -> Result<(), String> {
        use diesel_migrations::MigrationHarness;

        #[cfg(all(feature = "postgres", feature = "sqlite"))]
        match &self.pool {
            AnyPool::Postgres(pool) => {
                let conn = pool.get().await.map_err(|e| e.to_string())?;
                conn.interact(|conn| {
                    conn.run_pending_migrations(crate::database::POSTGRES_MIGRATIONS)
                        .map(|_| ())
                        .map_err(|e| format!("Failed to run PostgreSQL migrations: {}", e))
                })
                .await
                .map_err(|e| format!("Failed to run migrations: {}", e))?
                .map_err(|e| e)?;
            }
            AnyPool::Sqlite(pool) => {
                let conn = pool.get().await.map_err(|e| e.to_string())?;
                conn.interact(|conn| {
                    use diesel::prelude::*;

                    // Set SQLite pragmas for better concurrency before running migrations
                    // WAL mode allows concurrent reads during writes
                    diesel::sql_query("PRAGMA journal_mode=WAL;")
                        .execute(conn)
                        .map_err(|e| format!("Failed to set WAL mode: {}", e))?;
                    // busy_timeout makes SQLite wait 30s instead of immediately failing on locks
                    diesel::sql_query("PRAGMA busy_timeout=30000;")
                        .execute(conn)
                        .map_err(|e| format!("Failed to set busy_timeout: {}", e))?;

                    conn.run_pending_migrations(crate::database::SQLITE_MIGRATIONS)
                        .map(|_| ())
                        .map_err(|e| format!("Failed to run SQLite migrations: {}", e))
                })
                .await
                .map_err(|e| format!("Failed to run migrations: {}", e))?
                .map_err(|e| e)?;
            }
        }

        #[cfg(all(feature = "postgres", not(feature = "sqlite")))]
        {
            let conn = self.pool.get().await.map_err(|e| e.to_string())?;
            conn.interact(|conn| {
                conn.run_pending_migrations(crate::database::POSTGRES_MIGRATIONS)
                    .map(|_| ())
                    .map_err(|e| format!("Failed to run PostgreSQL migrations: {}", e))
            })
            .await
            .map_err(|e| format!("Failed to run migrations: {}", e))?
            .map_err(|e| e)?;
        }

        #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
        {
            let conn = self.pool.get().await.map_err(|e| e.to_string())?;
            conn.interact(|conn| {
                use diesel::prelude::*;

                diesel::sql_query("PRAGMA journal_mode=WAL;")
                    .execute(conn)
                    .map_err(|e| format!("Failed to set WAL mode: {}", e))?;
                diesel::sql_query("PRAGMA busy_timeout=30000;")
                    .execute(conn)
                    .map_err(|e| format!("Failed to set busy_timeout: {}", e))?;

                conn.run_pending_migrations(crate::database::SQLITE_MIGRATIONS)
                    .map(|_| ())
                    .map_err(|e| format!("Failed to run SQLite migrations: {}", e))
            })
            .await
            .map_err(|e| format!("Failed to run migrations: {}", e))?
            .map_err(|e| e)?;
        }

        Ok(())
    }

    /// Sets up the PostgreSQL schema for multi-tenant isolation.
    ///
    /// Creates the schema if it doesn't exist and runs migrations within it.
    /// Returns an error if called on a SQLite backend or if the schema name
    /// is invalid (to prevent SQL injection).
    ///
    /// # Security
    /// Schema names are validated to prevent SQL injection attacks.
    /// Only alphanumeric characters and underscores are allowed.
    #[cfg(feature = "postgres")]
    pub async fn setup_schema(&self, schema: &str) -> Result<(), String> {
        use diesel::prelude::*;

        // Validate schema name to prevent SQL injection
        let validated_schema = validate_schema_name(schema).map_err(|e| e.to_string())?;

        #[cfg(all(feature = "postgres", feature = "sqlite"))]
        let pool = match &self.pool {
            AnyPool::Postgres(pool) => pool,
            AnyPool::Sqlite(_) => {
                return Err("Schema setup is not supported for SQLite".to_string());
            }
        };

        #[cfg(all(feature = "postgres", not(feature = "sqlite")))]
        let pool = &self.pool;

        let conn = pool.get().await.map_err(|e| e.to_string())?;

        let schema_name = validated_schema.to_string();
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
                .map(|_| ())
                .map_err(|e| format!("Failed to run migrations: {}", e))
        })
        .await
        .map_err(|e| format!("Failed to run migrations in schema: {}", e))?
        .map_err(|e| e)?;

        info!("Schema '{}' set up successfully", schema);
        Ok(())
    }

    /// Gets a PostgreSQL connection with the schema search path set.
    ///
    /// For PostgreSQL, this sets the search path to the configured schema.
    /// For SQLite, this is a no-op and returns an error.
    ///
    /// # Security
    /// Schema names are validated before use in SQL to prevent injection attacks.
    #[cfg(feature = "postgres")]
    pub async fn get_connection_with_schema(
        &self,
    ) -> Result<
        deadpool::managed::Object<PgManager>,
        deadpool::managed::PoolError<deadpool_diesel::Error>,
    > {
        use diesel::prelude::*;

        #[cfg(all(feature = "postgres", feature = "sqlite"))]
        let pool = match &self.pool {
            AnyPool::Postgres(pool) => pool,
            AnyPool::Sqlite(_) => {
                panic!("get_connection_with_schema called on SQLite backend");
            }
        };

        #[cfg(all(feature = "postgres", not(feature = "sqlite")))]
        let pool = &self.pool;

        let conn = pool.get().await?;

        if let Some(ref schema) = self.schema {
            // Validate schema name to prevent SQL injection
            // This should already be validated at construction time, but we validate
            // again here for defense in depth
            if let Ok(validated) = validate_schema_name(schema) {
                let schema_name = validated.to_string();
                let _ = conn
                    .interact(move |conn| {
                        let set_search_path_sql =
                            format!("SET search_path TO {}, public", schema_name);
                        diesel::sql_query(&set_search_path_sql).execute(conn)
                    })
                    .await;
            }
        }

        Ok(conn)
    }

    /// Gets a PostgreSQL connection.
    ///
    /// Returns an error if this is a SQLite backend.
    #[cfg(feature = "postgres")]
    pub async fn get_postgres_connection(
        &self,
    ) -> Result<
        deadpool::managed::Object<PgManager>,
        deadpool::managed::PoolError<deadpool_diesel::Error>,
    > {
        self.get_connection_with_schema().await
    }

    /// Gets a SQLite connection.
    ///
    /// Returns an error if this is a PostgreSQL backend.
    #[cfg(feature = "sqlite")]
    pub async fn get_sqlite_connection(
        &self,
    ) -> Result<
        deadpool::managed::Object<SqliteManager>,
        deadpool::managed::PoolError<deadpool_diesel::Error>,
    > {
        #[cfg(all(feature = "postgres", feature = "sqlite"))]
        let pool = match &self.pool {
            AnyPool::Sqlite(pool) => pool,
            AnyPool::Postgres(_) => {
                panic!("get_sqlite_connection called on PostgreSQL backend");
            }
        };

        #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
        let pool = &self.pool;

        pool.get().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
            assert_eq!(
                BackendType::from_url("postgres://localhost/db"),
                BackendType::Postgres
            );
            assert_eq!(
                BackendType::from_url("postgresql://localhost/db"),
                BackendType::Postgres
            );
        }

        #[cfg(feature = "sqlite")]
        {
            assert_eq!(
                BackendType::from_url("sqlite:///path/to/db"),
                BackendType::Sqlite
            );
            assert_eq!(
                BackendType::from_url("/absolute/path.db"),
                BackendType::Sqlite
            );
            assert_eq!(
                BackendType::from_url("./relative/path.db"),
                BackendType::Sqlite
            );
            assert_eq!(BackendType::from_url(":memory:"), BackendType::Sqlite);
            assert_eq!(
                BackendType::from_url("database.sqlite"),
                BackendType::Sqlite
            );
            assert_eq!(
                BackendType::from_url("database.sqlite3"),
                BackendType::Sqlite
            );
            // SQLite URI format with mode and cache options
            assert_eq!(
                BackendType::from_url("file:test?mode=memory&cache=shared"),
                BackendType::Sqlite
            );
            assert_eq!(
                BackendType::from_url("file:cloacina_test?mode=memory&cache=shared"),
                BackendType::Sqlite
            );
        }
    }
}
