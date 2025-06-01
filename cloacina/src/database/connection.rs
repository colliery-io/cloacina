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
//! This module provides a connection pool implementation using `r2d2` for managing
//! database connections efficiently. It handles connection pooling, connection lifecycle,
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

use diesel::r2d2::{ConnectionManager, Pool};
use tracing::info;

#[cfg(feature = "postgres")]
use diesel::r2d2::{CustomizeConnection, Error as R2D2Error};
#[cfg(feature = "postgres")]
use diesel::PgConnection;
#[cfg(feature = "postgres")]
use url::Url;

#[cfg(feature = "sqlite")]
use diesel::SqliteConnection;

/// Type alias for the connection type based on the selected backend
#[cfg(feature = "postgres")]
pub type DbConnection = PgConnection;

#[cfg(feature = "sqlite")]
pub type DbConnection = SqliteConnection;

/// Type alias for the connection manager based on the selected backend
pub type DbConnectionManager = ConnectionManager<DbConnection>;

/// Type alias for the connection pool
pub type DbPool = Pool<DbConnectionManager>;

#[cfg(feature = "postgres")]
/// Connection customizer that sets the PostgreSQL search_path for schema isolation
///
/// This customizer is applied to each connection when it's acquired from the pool,
/// ensuring that all database operations occur within the specified schema.
#[derive(Debug, Clone)]
struct SchemaCustomizer {
    schema: Option<String>,
}

#[cfg(feature = "postgres")]
impl CustomizeConnection<PgConnection, R2D2Error> for SchemaCustomizer {
    fn on_acquire(&self, conn: &mut PgConnection) -> Result<(), R2D2Error> {
        use diesel::prelude::*;

        if let Some(ref schema) = self.schema {
            // Set search_path to the specified schema, with public as fallback for extensions
            let sql = format!("SET search_path TO {}, public", schema);
            diesel::sql_query(&sql)
                .execute(conn)
                .map_err(R2D2Error::QueryError)?;
        }
        Ok(())
    }
}

/// Represents a pool of database connections.
///
/// This struct provides a thread-safe wrapper around a connection pool,
/// allowing multiple parts of the application to share database connections
/// efficiently.
///
/// # Thread Safety
///
/// The `Database` struct is `Clone` and can be safely shared between threads.
/// Each clone references the same underlying connection pool.
#[derive(Clone, Debug)]
pub struct Database {
    /// The actual connection pool.
    pool: DbPool,
    /// The PostgreSQL schema name (PostgreSQL only)
    #[cfg(feature = "postgres")]
    schema: Option<String>,
}

impl Database {
    /// Creates a new database connection pool.
    ///
    /// # Arguments
    ///
    /// For PostgreSQL:
    /// * `connection_string` - The base URL of the database server (e.g., "postgres://username:password@localhost:5432")
    /// * `database_name` - The name of the database to connect to
    /// * `max_size` - The maximum number of connections the pool should maintain
    ///
    /// For SQLite:
    /// * `connection_string` - The path to the database file or ":memory:" for in-memory database
    /// * `database_name` - Ignored for SQLite (pass empty string)
    /// * `max_size` - The maximum number of connections the pool should maintain
    ///
    /// # Returns
    ///
    /// Returns a `Database` instance containing the created connection pool.
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// * The connection string is invalid
    /// * The connection pool creation fails
    /// * The database server is unreachable (PostgreSQL)
    /// * The database file cannot be created or accessed (SQLite)
    pub fn new(connection_string: &str, database_name: &str, max_size: u32) -> Self {
        #[cfg(feature = "postgres")]
        {
            return Self::new_with_schema(connection_string, database_name, max_size, None);
        }
        #[cfg(feature = "sqlite")]
        {
            let connection_url = Self::build_connection_url(connection_string, database_name);
            let manager = ConnectionManager::<DbConnection>::new(connection_url);
            let pool = Pool::builder()
                .max_size(max_size)
                .build(manager)
                .expect("Failed to create connection pool");

            info!("Database connection pool initialized");
            return Self { pool };
        }
    }

    /// Creates a new database connection pool with PostgreSQL schema support
    ///
    /// # Arguments
    /// * `connection_string` - The base URL of the database server
    /// * `database_name` - The name of the database to connect to
    /// * `max_size` - The maximum number of connections the pool should maintain
    /// * `schema` - Optional schema name for multi-tenant isolation
    ///
    /// # Returns
    /// Returns a `Database` instance with schema support
    ///
    /// # Note
    /// This method is only available with the "postgres" feature enabled.
    #[cfg(feature = "postgres")]
    pub fn new_with_schema(
        connection_string: &str,
        database_name: &str,
        max_size: u32,
        schema: Option<&str>,
    ) -> Self {
        let connection_url = Self::build_connection_url(connection_string, database_name);
        let manager = ConnectionManager::<DbConnection>::new(connection_url);

        // Build the connection pool with schema customizer
        let pool = Pool::builder()
            .max_size(max_size)
            .connection_customizer(Box::new(SchemaCustomizer {
                schema: schema.map(String::from),
            }))
            .build(manager)
            .expect("Failed to create connection pool");

        info!(
            "Database connection pool initialized{}",
            schema.map_or(String::new(), |s| format!(" with schema '{}'", s))
        );

        Self {
            pool,
            schema: schema.map(String::from),
        }
    }

    /// Sets up the PostgreSQL schema for multi-tenant isolation
    ///
    /// This method creates the schema if it doesn't exist and runs migrations within it.
    ///
    /// # Arguments
    /// * `schema` - The schema name to set up
    ///
    /// # Returns
    /// * `Result<(), String>` - Success or error message
    #[cfg(feature = "postgres")]
    pub fn setup_schema(&self, schema: &str) -> Result<(), String> {
        use diesel::prelude::*;

        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        // Create schema if it doesn't exist
        let create_schema_sql = format!("CREATE SCHEMA IF NOT EXISTS {}", schema);
        diesel::sql_query(&create_schema_sql)
            .execute(&mut conn)
            .map_err(|e| format!("Failed to create schema: {}", e))?;

        // Set search path for migrations
        let set_search_path_sql = format!("SET search_path TO {}, public", schema);
        diesel::sql_query(&set_search_path_sql)
            .execute(&mut conn)
            .map_err(|e| format!("Failed to set search path: {}", e))?;

        // Run migrations in the schema
        crate::database::run_migrations(&mut conn)
            .map_err(|e| format!("Failed to run migrations in schema: {}", e))?;

        info!("Schema '{}' set up successfully", schema);
        Ok(())
    }

    /// Gets the current schema name (PostgreSQL only)
    #[cfg(feature = "postgres")]
    pub fn schema(&self) -> Option<&str> {
        self.schema.as_deref()
    }

    /// Builds the connection URL based on the backend
    #[cfg(feature = "postgres")]
    fn build_connection_url(base_url: &str, database_name: &str) -> String {
        // Parse the base URL and set the database name
        let mut url = Url::parse(base_url).expect("Invalid PostgreSQL URL");
        url.set_path(database_name);
        url.to_string()
    }

    #[cfg(feature = "sqlite")]
    fn build_connection_url(connection_string: &str, _database_name: &str) -> String {
        // For SQLite, just return the connection string as-is
        // It can be a file path or ":memory:"
        connection_string.to_string()
    }

    /// Gets a connection from the pool.
    ///
    /// This method returns a clone of the connection pool, which can be used
    /// to acquire individual connections as needed.
    ///
    /// # Returns
    ///
    /// Returns a pooled connection from the pool.
    pub fn get_connection(&self) -> DbPool {
        self.pool.clone()
    }

    /// Gets the connection pool.
    ///
    /// This method returns a clone of the connection pool. It is functionally
    /// identical to `get_connection()` and exists for API clarity.
    ///
    /// # Returns
    ///
    /// Returns a reference to the connection pool.
    pub fn pool(&self) -> DbPool {
        self.pool.clone()
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
        let url = Database::build_connection_url("/path/to/database.db", "");
        assert_eq!(url, "/path/to/database.db");

        // Test in-memory database
        let url = Database::build_connection_url(":memory:", "");
        assert_eq!(url, ":memory:");

        // Test relative path
        let url = Database::build_connection_url("./database.db", "");
        assert_eq!(url, "./database.db");
    }
}
