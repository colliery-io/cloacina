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

use thiserror::Error;
use tracing::info;

use deadpool_diesel::postgres::{Manager as PgManager, Pool as PgPool, Runtime as PgRuntime};
use diesel::PgConnection;
use url::Url;

use deadpool_diesel::sqlite::{
    Manager as SqliteManager, Pool as SqlitePool, Runtime as SqliteRuntime,
};
use diesel::SqliteConnection;

// =============================================================================
// Schema Validation
// =============================================================================

/// Maximum length for PostgreSQL schema names (NAMEDATALEN - 1).
const MAX_SCHEMA_NAME_LENGTH: usize = 63;

/// Reserved PostgreSQL schema names that cannot be used.
const RESERVED_SCHEMA_NAMES: &[&str] = &["public", "pg_catalog", "information_schema", "pg_temp"];

/// Errors that can occur during schema name validation.
///
/// These errors are returned when a schema name fails validation checks
/// designed to prevent SQL injection attacks.
#[derive(Debug, Error)]
pub enum SchemaError {
    /// Schema name is empty or exceeds the maximum length.
    #[error("Schema name length invalid: '{name}' (must be 1-{max} characters)")]
    InvalidLength { name: String, max: usize },

    /// Schema name does not start with a letter or underscore.
    #[error("Schema name must start with a letter or underscore: '{0}'")]
    InvalidStart(String),

    /// Schema name contains characters other than alphanumeric or underscore.
    #[error(
        "Schema name contains invalid characters (only alphanumeric and underscore allowed): '{0}'"
    )]
    InvalidCharacters(String),

    /// Schema name is a reserved PostgreSQL name.
    #[error("Schema name is reserved: '{0}'")]
    ReservedName(String),
}

/// Validates a PostgreSQL schema name to prevent SQL injection.
///
/// This function enforces PostgreSQL identifier naming rules:
/// - Length must be between 1 and 63 characters
/// - Must start with a letter (a-z, A-Z) or underscore
/// - Subsequent characters must be alphanumeric or underscore
/// - Cannot be a reserved PostgreSQL schema name
///
/// # Arguments
/// * `name` - The schema name to validate
///
/// # Returns
/// * `Ok(&str)` - The validated schema name (zero-copy)
/// * `Err(SchemaError)` - Description of the validation failure
///
/// # Example
/// ```
/// use cloacina::database::connection::validate_schema_name;
///
/// assert!(validate_schema_name("my_schema").is_ok());
/// assert!(validate_schema_name("tenant_123").is_ok());
/// assert!(validate_schema_name("public").is_err()); // Reserved
/// assert!(validate_schema_name("123abc").is_err()); // Starts with number
/// assert!(validate_schema_name("my-schema").is_err()); // Contains hyphen
/// ```
pub fn validate_schema_name(name: &str) -> Result<&str, SchemaError> {
    // Check length
    if name.is_empty() || name.len() > MAX_SCHEMA_NAME_LENGTH {
        return Err(SchemaError::InvalidLength {
            name: name.to_string(),
            max: MAX_SCHEMA_NAME_LENGTH,
        });
    }

    // Must start with letter or underscore
    let first_char = name.chars().next().unwrap(); // Safe: we checked non-empty above
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return Err(SchemaError::InvalidStart(name.to_string()));
    }

    // Only allow alphanumeric and underscore
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(SchemaError::InvalidCharacters(name.to_string()));
    }

    // Reject reserved names (case-insensitive)
    let lower_name = name.to_lowercase();
    if RESERVED_SCHEMA_NAMES.contains(&lower_name.as_str()) {
        return Err(SchemaError::ReservedName(name.to_string()));
    }

    Ok(name)
}

// =============================================================================
// Runtime Database Backend Selection
// =============================================================================

/// Represents the database backend type, detected at runtime from the connection URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    /// PostgreSQL backend
    Postgres,
    /// SQLite backend
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
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            return BackendType::Postgres;
        }

        // SQLite URLs can be:
        // - sqlite:// prefix
        // - file: URI format (e.g., file:test?mode=memory&cache=shared)
        // - file paths (relative or absolute)
        // - :memory: for in-memory databases
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
    Postgres(PgConnection),
    /// SQLite connection variant
    Sqlite(SqliteConnection),
}

/// Pool enum that wraps both PostgreSQL and SQLite connection pools.
///
/// This enum enables runtime pool selection based on the detected backend.
#[derive(Clone)]
pub enum AnyPool {
    /// PostgreSQL connection pool
    Postgres(PgPool),
    /// SQLite connection pool
    Sqlite(SqlitePool),
}

impl std::fmt::Debug for AnyPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnyPool::Postgres(_) => write!(f, "AnyPool::Postgres(...)"),
            AnyPool::Sqlite(_) => write!(f, "AnyPool::Sqlite(...)"),
        }
    }
}

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
}

// =============================================================================
// Legacy Type Aliases (for backward compatibility during migration)
// =============================================================================
// Note: With dual-backend support, use AnyConnection and AnyPool instead.
// These aliases default to PostgreSQL for backwards compatibility.

/// Type alias for the connection type (defaults to PostgreSQL)
pub type DbConnection = PgConnection;

/// Type alias for the connection manager (defaults to PostgreSQL)
pub type DbConnectionManager = PgManager;

/// Type alias for the connection pool (defaults to PostgreSQL)
pub type DbPool = PgPool;

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
    ///
    /// # Panics
    ///
    /// Panics if the schema name is invalid (to prevent SQL injection).
    /// Valid schema names must start with a letter or underscore, contain only
    /// alphanumeric characters and underscores, and not be a reserved name.
    pub fn new_with_schema(
        connection_string: &str,
        _database_name: &str,
        max_size: u32,
        schema: Option<&str>,
    ) -> Self {
        let backend = BackendType::from_url(connection_string);

        // Validate schema name at construction time to prevent SQL injection
        let validated_schema = schema.map(|s| {
            validate_schema_name(s)
                .expect("Invalid schema name provided to Database::new_with_schema")
                .to_string()
        });

        match backend {
            BackendType::Postgres => {
                let connection_url = Self::build_postgres_url(connection_string, _database_name);
                let manager = PgManager::new(connection_url, PgRuntime::Tokio1);
                let pool = PgPool::builder(manager)
                    .max_size(max_size as usize)
                    .build()
                    .expect("Failed to create PostgreSQL connection pool");

                info!(
                    "PostgreSQL connection pool initialized{}",
                    validated_schema
                        .as_ref()
                        .map_or(String::new(), |s| format!(" with schema '{}'", s))
                );

                Self {
                    pool: AnyPool::Postgres(pool),
                    backend,
                    schema: validated_schema,
                }
            }
            BackendType::Sqlite => {
                let connection_url = Self::build_sqlite_url(connection_string);
                let manager = SqliteManager::new(connection_url, SqliteRuntime::Tokio1);
                // SQLite has limited concurrent write support even with WAL mode.
                // Using a single connection avoids "database is locked" errors.
                // For read-heavy workloads, consider increasing this with proper
                // busy_timeout configuration on each connection.
                let sqlite_pool_size = 1;
                let pool = SqlitePool::builder(manager)
                    .max_size(sqlite_pool_size)
                    .build()
                    .expect("Failed to create SQLite connection pool");

                info!(
                    "SQLite connection pool initialized (size: {})",
                    sqlite_pool_size
                );

                Self {
                    pool: AnyPool::Sqlite(pool),
                    backend,
                    schema: validated_schema,
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
    fn build_postgres_url(base_url: &str, database_name: &str) -> String {
        let mut url = Url::parse(base_url).expect("Invalid PostgreSQL URL");
        url.set_path(database_name);
        url.to_string()
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

        match &self.pool {
            AnyPool::Postgres(pool) => {
                let conn = pool.get().await.map_err(|e| e.to_string())?;
                conn.interact(|conn| {
                    conn.run_pending_migrations(crate::database::POSTGRES_MIGRATIONS)
                        .expect("Failed to run PostgreSQL migrations");
                })
                .await
                .map_err(|e| format!("Failed to run migrations: {}", e))?;
            }
            AnyPool::Sqlite(pool) => {
                let conn = pool.get().await.map_err(|e| e.to_string())?;
                conn.interact(|conn| {
                    use diesel::prelude::*;

                    // Set SQLite pragmas for better concurrency before running migrations
                    // WAL mode allows concurrent reads during writes
                    diesel::sql_query("PRAGMA journal_mode=WAL;")
                        .execute(conn)
                        .expect("Failed to set WAL mode");
                    // busy_timeout makes SQLite wait 30s instead of immediately failing on locks
                    diesel::sql_query("PRAGMA busy_timeout=30000;")
                        .execute(conn)
                        .expect("Failed to set busy_timeout");

                    conn.run_pending_migrations(crate::database::SQLITE_MIGRATIONS)
                        .expect("Failed to run SQLite migrations");
                })
                .await
                .map_err(|e| format!("Failed to run migrations: {}", e))?;
            }
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
    pub async fn setup_schema(&self, schema: &str) -> Result<(), String> {
        use diesel::prelude::*;

        // Validate schema name to prevent SQL injection
        let validated_schema = validate_schema_name(schema).map_err(|e| e.to_string())?;

        let pool = match &self.pool {
            AnyPool::Postgres(pool) => pool,
            AnyPool::Sqlite(_) => {
                return Err("Schema setup is not supported for SQLite".to_string());
            }
        };

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
    ///
    /// # Security
    /// Schema names are validated before use in SQL to prevent injection attacks.
    pub async fn get_connection_with_schema(
        &self,
    ) -> Result<
        deadpool::managed::Object<PgManager>,
        deadpool::managed::PoolError<deadpool_diesel::Error>,
    > {
        use diesel::prelude::*;

        let pool = match &self.pool {
            AnyPool::Postgres(pool) => pool,
            AnyPool::Sqlite(_) => {
                panic!("get_connection_with_schema called on SQLite backend");
            }
        };

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
    pub async fn get_sqlite_connection(
        &self,
    ) -> Result<
        deadpool::managed::Object<SqliteManager>,
        deadpool::managed::PoolError<deadpool_diesel::Error>,
    > {
        let pool = match &self.pool {
            AnyPool::Sqlite(pool) => pool,
            AnyPool::Postgres(_) => {
                panic!("get_sqlite_connection called on PostgreSQL backend");
            }
        };

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

    // =========================================================================
    // Schema Validation Tests
    // =========================================================================

    #[test]
    fn test_valid_schema_names() {
        // Simple valid names
        assert!(validate_schema_name("my_schema").is_ok());
        assert!(validate_schema_name("tenant_123").is_ok());
        assert!(validate_schema_name("MySchema").is_ok());

        // Starting with underscore
        assert!(validate_schema_name("_private").is_ok());
        assert!(validate_schema_name("_123").is_ok());

        // Single character
        assert!(validate_schema_name("a").is_ok());
        assert!(validate_schema_name("_").is_ok());

        // Maximum length (63 characters)
        let max_name = "a".repeat(63);
        assert!(validate_schema_name(&max_name).is_ok());
    }

    #[test]
    fn test_sql_injection_attempts_rejected() {
        // Command injection with semicolon
        assert!(matches!(
            validate_schema_name("test; DROP TABLE users; --"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Quote injection
        assert!(matches!(
            validate_schema_name("test' OR '1'='1"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Comment injection
        assert!(matches!(
            validate_schema_name("test/*comment*/"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Double dash comment
        assert!(matches!(
            validate_schema_name("test--comment"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Parentheses
        assert!(matches!(
            validate_schema_name("test()"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Equals sign
        assert!(matches!(
            validate_schema_name("test=1"),
            Err(SchemaError::InvalidCharacters(_))
        ));
    }

    #[test]
    fn test_invalid_length() {
        // Empty string
        assert!(matches!(
            validate_schema_name(""),
            Err(SchemaError::InvalidLength { .. })
        ));

        // Too long (64 characters)
        let too_long = "a".repeat(64);
        assert!(matches!(
            validate_schema_name(&too_long),
            Err(SchemaError::InvalidLength { .. })
        ));

        // Way too long
        let way_too_long = "a".repeat(1000);
        assert!(matches!(
            validate_schema_name(&way_too_long),
            Err(SchemaError::InvalidLength { .. })
        ));
    }

    #[test]
    fn test_invalid_start_character() {
        // Starting with number
        assert!(matches!(
            validate_schema_name("123abc"),
            Err(SchemaError::InvalidStart(_))
        ));

        // Starting with hyphen
        assert!(matches!(
            validate_schema_name("-schema"),
            Err(SchemaError::InvalidStart(_))
        ));

        // Starting with dot
        assert!(matches!(
            validate_schema_name(".schema"),
            Err(SchemaError::InvalidStart(_))
        ));

        // Starting with space
        assert!(matches!(
            validate_schema_name(" schema"),
            Err(SchemaError::InvalidStart(_))
        ));
    }

    #[test]
    fn test_invalid_characters() {
        // Hyphen
        assert!(matches!(
            validate_schema_name("my-schema"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Dot
        assert!(matches!(
            validate_schema_name("my.schema"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Space
        assert!(matches!(
            validate_schema_name("my schema"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Special characters
        assert!(matches!(
            validate_schema_name("schema@test"),
            Err(SchemaError::InvalidCharacters(_))
        ));
        assert!(matches!(
            validate_schema_name("schema#1"),
            Err(SchemaError::InvalidCharacters(_))
        ));
        assert!(matches!(
            validate_schema_name("schema$"),
            Err(SchemaError::InvalidCharacters(_))
        ));
    }

    #[test]
    fn test_reserved_names() {
        // Reserved names (case-insensitive)
        assert!(matches!(
            validate_schema_name("public"),
            Err(SchemaError::ReservedName(_))
        ));
        assert!(matches!(
            validate_schema_name("PUBLIC"),
            Err(SchemaError::ReservedName(_))
        ));
        assert!(matches!(
            validate_schema_name("Public"),
            Err(SchemaError::ReservedName(_))
        ));

        assert!(matches!(
            validate_schema_name("pg_catalog"),
            Err(SchemaError::ReservedName(_))
        ));
        assert!(matches!(
            validate_schema_name("PG_CATALOG"),
            Err(SchemaError::ReservedName(_))
        ));

        assert!(matches!(
            validate_schema_name("information_schema"),
            Err(SchemaError::ReservedName(_))
        ));
        assert!(matches!(
            validate_schema_name("INFORMATION_SCHEMA"),
            Err(SchemaError::ReservedName(_))
        ));

        assert!(matches!(
            validate_schema_name("pg_temp"),
            Err(SchemaError::ReservedName(_))
        ));
    }

    #[test]
    fn test_schema_error_display() {
        // Verify error messages are informative
        let err = validate_schema_name("").unwrap_err();
        assert!(err.to_string().contains("length"));

        let err = validate_schema_name("123abc").unwrap_err();
        assert!(err.to_string().contains("start"));

        let err = validate_schema_name("my-schema").unwrap_err();
        assert!(err.to_string().contains("invalid characters"));

        let err = validate_schema_name("public").unwrap_err();
        assert!(err.to_string().contains("reserved"));
    }

    #[test]
    fn test_unicode_characters_rejected() {
        // Unicode Greek letter alpha
        assert!(matches!(
            validate_schema_name("schema_\u{03B1}"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Unicode snowman emoji
        assert!(matches!(
            validate_schema_name("schema_\u{2603}"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Non-ASCII e with acute accent (cafe with accented e)
        assert!(matches!(
            validate_schema_name("caf\u{00E9}"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Chinese character
        assert!(matches!(
            validate_schema_name("schema_\u{4E2D}"),
            Err(SchemaError::InvalidCharacters(_))
        ));
    }
}
