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

//! Database connection management module for PostgreSQL using Diesel ORM.
//!
//! This module provides a connection pool implementation using `r2d2` for managing
//! PostgreSQL database connections efficiently. It handles connection pooling,
//! connection lifecycle, and provides a thread-safe way to access database connections.
//!
//! # Features
//!
//! - Connection pooling with configurable pool size
//! - Thread-safe connection management
//! - Automatic connection cleanup
//! - URL-based configuration
//!
//! # Example
//!
//! ```rust
//! use cloacina::database::connection::Database;
//!
//! let db = Database::new(
//!     "postgres://username:password@localhost:5432",
//!     "my_database",
//!     10
//! );
//!
//! // Get a connection from the pool
//! let pool = db.get_connection();
//! ```
//!
//! # Error Handling
//!
//! The module uses panic-based error handling for connection pool creation
//! as this is typically a fatal error that should be handled at application startup.
//! Connection acquisition from the pool is handled through the `r2d2` pool's
//! error handling mechanisms.

use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use tracing::info;
use url::Url;

/// Represents a pool of PostgreSQL database connections.
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
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl Database {
    /// Creates a new database connection pool.
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL of the database server (e.g., "postgres://username:password@localhost:5432")
    /// * `database_name` - The name of the database to connect to
    /// * `max_size` - The maximum number of connections the pool should maintain
    ///
    /// # Returns
    ///
    /// Returns a `Database` instance containing the created connection pool.
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// * The base URL is invalid
    /// * The connection pool creation fails
    /// * The database server is unreachable
    /// * The credentials are invalid
    ///
    /// # Example
    ///
    /// ```rust
    /// use cloacina::database::connection::Database;
    ///
    /// let db = Database::new(
    ///     "postgres://postgres:postgres@localhost:5432",
    ///     "my_database",
    ///     10
    /// );
    /// ```
    pub fn new(base_url: &str, database_name: &str, max_size: u32) -> Self {
        // Parse the base URL and set the database name
        let mut url = Url::parse(base_url).expect("Invalid base URL");
        url.set_path(database_name);

        // Create a connection manager
        let manager = ConnectionManager::<PgConnection>::new(url.as_str());

        // Build the connection pool
        let pool = Pool::builder()
            .max_size(max_size)
            .build(manager)
            .expect("Failed to create connection pool");

        info!("Database connection pool initialized");

        Self { pool }
    }

    /// Gets a connection from the pool.
    ///
    /// This method returns a clone of the connection pool, which can be used
    /// to acquire individual connections as needed.
    ///
    /// # Returns
    ///
    /// Returns a pooled connection from the pool.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::RunQueryDsl;
    ///
    /// let pool = db.get_connection();
    /// let conn = pool.get().expect("Failed to get connection");
    /// ```
    pub fn get_connection(&self) -> Pool<ConnectionManager<PgConnection>> {
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
    ///
    /// # Example
    ///
    /// ```rust
    /// let pool = db.pool();
    /// ```
    pub fn pool(&self) -> Pool<ConnectionManager<PgConnection>> {
        self.pool.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_parsing_scenarios() {
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
}
