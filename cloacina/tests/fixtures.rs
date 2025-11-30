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

/*
 * Copyright (c) 2025 Dylan Storey
 * Licensed under the Elastic License 2.0.
 * See LICENSE file in the project root for full license text.
 */

//! This module provides a test fixture for the Cloacina project.
//!
//! It includes basic functionality to set up test contexts for testing,
//! similar to brokkr's ergonomic testing framework.
//!
//! # Dual-Backend Support
//!
//! When both PostgreSQL and SQLite features are enabled, the test fixture
//! defaults to PostgreSQL. Set the environment variable `TEST_DATABASE_BACKEND=sqlite`
//! to use SQLite instead.

use cloacina::database::connection::Database;
use diesel::deserialize::QueryableByName;
use diesel::prelude::*;
use diesel::sql_types::Text;
use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex, Once};
use tracing::info;
use uuid;

use diesel::pg::PgConnection;
use diesel::sqlite::SqliteConnection;

static INIT: Once = Once::new();
static FIXTURE: OnceCell<Arc<Mutex<TestFixture>>> = OnceCell::new();

/// Gets or initializes a test fixture singleton
///
/// This function ensures only one test fixture exists across all tests,
/// initializing it if necessary.
///
/// # Backend Selection
///
/// - In single-backend builds, uses the enabled backend
/// - In dual-backend builds, defaults to PostgreSQL unless `TEST_DATABASE_BACKEND=sqlite`
///
/// # Returns
/// An Arc<Mutex<TestFixture>> pointing to the shared test fixture instance
pub async fn get_or_init_fixture() -> Arc<Mutex<TestFixture>> {
    FIXTURE
        .get_or_init(|| {
            // Check environment variable for backend selection
            let backend = std::env::var("TEST_DATABASE_BACKEND")
                .unwrap_or_else(|_| "postgres".to_string());

            if backend == "sqlite" {
                let db_url = "file:memdb1?mode=memory&cache=shared";
                let db = Database::new(db_url, "", 5);
                let conn = SqliteConnection::establish(db_url)
                    .expect("Failed to connect to SQLite database");
                Arc::new(Mutex::new(TestFixture::new_sqlite(db, conn)))
            } else {
                let db = Database::new("postgres://cloacina:cloacina@localhost:5432", "cloacina", 5);
                let conn = PgConnection::establish("postgres://cloacina:cloacina@localhost:5432/cloacina")
                    .expect("Failed to connect to PostgreSQL database");
                Arc::new(Mutex::new(TestFixture::new_postgres(db, conn)))
            }
        })
        .clone()
}

/// Represents a test fixture for the Cloacina project.
///
/// The fixture supports both PostgreSQL and SQLite backends. In dual-backend builds,
/// it stores the connection in a backend-specific variant.
#[allow(dead_code)]
pub struct TestFixture {
    /// Flag indicating if the fixture has been initialized
    initialized: bool,
    /// Database connection pool
    db: Database,
    /// PostgreSQL connection (when using PostgreSQL backend)
    pg_conn: Option<PgConnection>,
    /// SQLite connection (when using SQLite backend)
    sqlite_conn: Option<SqliteConnection>,
}

impl TestFixture {
    /// Creates a new TestFixture instance for PostgreSQL
    pub fn new_postgres(db: Database, conn: PgConnection) -> Self {
        INIT.call_once(|| {
            cloacina::init_logging(None);
        });

        info!("Test fixture created (PostgreSQL)");

        TestFixture {
            initialized: false,
            db,
            pg_conn: Some(conn),
            sqlite_conn: None,
        }
    }

    /// Creates a new TestFixture instance for SQLite
    pub fn new_sqlite(db: Database, conn: SqliteConnection) -> Self {
        INIT.call_once(|| {
            cloacina::init_logging(None);
        });

        info!("Test fixture created (SQLite)");

        TestFixture {
            initialized: false,
            db,
            pg_conn: None,
            sqlite_conn: Some(conn),
        }
    }

    /// Get a DAL instance using the database
    pub fn get_dal(&self) -> cloacina::dal::DAL {
        cloacina::dal::DAL::new(self.db.clone())
    }

    /// Get a clone of the database instance
    pub fn get_database(&self) -> Database {
        self.db.clone()
    }

    /// Get the database URL for this fixture
    pub fn get_database_url(&self) -> String {
        match self.db.backend() {
            cloacina::database::BackendType::Postgres => {
                "postgres://cloacina:cloacina@localhost:5432/cloacina".to_string()
            }
            cloacina::database::BackendType::Sqlite => {
                "file:memdb1?mode=memory&cache=shared".to_string()
            }
        }
    }

    /// Get the name of the current backend (postgres or sqlite)
    pub fn get_current_backend(&self) -> &'static str {
        match self.db.backend() {
            cloacina::database::BackendType::Postgres => "postgres",
            cloacina::database::BackendType::Sqlite => "sqlite",
        }
    }

    /// Create a PostgreSQL storage backend using this fixture's database (primary storage method)
    pub fn create_storage(&self) -> cloacina::dal::PostgresRegistryStorage {
        cloacina::dal::PostgresRegistryStorage::new(self.db.clone())
    }

    /// Create storage backend matching the current database backend
    pub fn create_backend_storage(&self) -> Box<dyn cloacina::registry::traits::RegistryStorage> {
        match self.db.backend() {
            cloacina::database::BackendType::Postgres => {
                Box::new(cloacina::dal::PostgresRegistryStorage::new(self.db.clone()))
            }
            cloacina::database::BackendType::Sqlite => {
                Box::new(cloacina::dal::SqliteRegistryStorage::new(self.db.clone()))
            }
        }
    }

    /// Create a PostgreSQL storage backend using this fixture's database
    pub fn create_postgres_storage(&self) -> cloacina::dal::PostgresRegistryStorage {
        cloacina::dal::PostgresRegistryStorage::new(self.db.clone())
    }

    /// Create a SQLite storage backend using this fixture's database
    pub fn create_sqlite_storage(&self) -> cloacina::dal::SqliteRegistryStorage {
        cloacina::dal::SqliteRegistryStorage::new(self.db.clone())
    }

    /// Create a filesystem storage backend for testing
    pub fn create_filesystem_storage(&self) -> cloacina::dal::FilesystemRegistryStorage {
        let temp_dir =
            std::env::temp_dir().join(format!("cloacina_test_storage_{}", uuid::Uuid::new_v4()));
        cloacina::dal::FilesystemRegistryStorage::new(temp_dir)
            .expect("Failed to create filesystem storage")
    }

    /// Initialize the fixture with additional setup
    pub async fn initialize(&mut self) {
        // Initialize the database schema based on the backend
        if let Some(ref mut conn) = self.pg_conn {
            cloacina::database::run_migrations_postgres(conn)
                .expect("Failed to run PostgreSQL migrations");
            self.initialized = true;
            return;
        }

        if let Some(ref mut conn) = self.sqlite_conn {
            cloacina::database::run_migrations_sqlite(conn)
                .expect("Failed to run SQLite migrations");
            self.initialized = true;
            return;
        }
    }

    /// Reset the database by dropping and recreating it
    pub async fn reset_database(&mut self) {
        if self.pg_conn.is_some() {
            use diesel::Connection;

            // Connect to the 'postgres' database to perform admin operations
            let mut admin_conn =
                PgConnection::establish("postgres://cloacina:cloacina@localhost:5432/postgres")
                    .expect("Failed to connect to postgres database for admin operations");

            // Terminate existing connections to 'cloacina'
            diesel::sql_query(
                "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = 'cloacina' AND pid <> pg_backend_pid()"
            )
            .execute(&mut admin_conn)
            .expect("Failed to terminate existing connections");

            // Drop and recreate the database
            diesel::sql_query("DROP DATABASE IF EXISTS cloacina")
                .execute(&mut admin_conn)
                .expect("Failed to drop database");

            diesel::sql_query("CREATE DATABASE cloacina")
                .execute(&mut admin_conn)
                .expect("Failed to create database");

            // Create new connections
            let db = Database::new("postgres://cloacina:cloacina@localhost:5432", "cloacina", 5);
            let mut conn =
                PgConnection::establish("postgres://cloacina:cloacina@localhost:5432/cloacina")
                    .expect("Failed to connect to PostgreSQL database");

            // Run migrations
            cloacina::database::run_migrations_postgres(&mut conn)
                .expect("Failed to run migrations");

            // Update the fixture's connections
            self.db = db;
            self.pg_conn = Some(conn);
            return;
        }

        if let Some(ref mut conn) = self.sqlite_conn {
            // For SQLite, clear all tables first, then run migrations
            use diesel::sql_query;

            // Define a struct for the query result
            #[derive(QueryableByName)]
            struct TableName {
                #[diesel(sql_type = Text)]
                name: String,
            }

            // Get list of all user tables (excluding sqlite system tables and migrations)
            let tables_result: Result<Vec<TableName>, _> = sql_query(
                "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != '__diesel_schema_migrations'"
            )
            .load::<TableName>(conn);

            if let Ok(table_rows) = tables_result {
                // Clear all user tables
                for table_row in table_rows {
                    let _ = sql_query(&format!("DELETE FROM {}", table_row.name))
                        .execute(conn);
                }
            }

            // Run migrations to ensure schema is up to date
            cloacina::database::run_migrations_sqlite(conn)
                .expect("Failed to run migrations");
        }
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        // No need to reset the database here - tests should manage their own cleanup
        // This prevents interference with other tests that might still be running
    }
}

#[derive(QueryableByName)]
struct TableCount {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

#[cfg(test)]
pub mod fixtures {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_migration_function_postgres() {
        let mut conn =
            PgConnection::establish("postgres://cloacina:cloacina@localhost:5432/cloacina")
                .expect("Failed to connect to database");

        // Test that our migration function works
        let result = cloacina::database::run_migrations_postgres(&mut conn);
        assert!(
            result.is_ok(),
            "Migration function should succeed: {:?}",
            result
        );

        // Verify the contexts table was created
        let table_count: Result<TableCount, diesel::result::Error> = diesel::sql_query(
            "SELECT COUNT(*) as count FROM information_schema.tables WHERE table_name = 'contexts'",
        )
        .get_result(&mut conn);

        assert!(
            table_count.is_ok(),
            "Contexts table should exist after migrations"
        );
        assert!(
            table_count.unwrap().count > 0,
            "Contexts table should be found in information_schema"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_migration_function_sqlite() {
        let mut conn = SqliteConnection::establish("file:test_memdb?mode=memory&cache=shared")
            .expect("Failed to connect to database");

        // Test that our migration function works
        let result = cloacina::database::run_migrations_sqlite(&mut conn);
        assert!(
            result.is_ok(),
            "Migration function should succeed: {:?}",
            result
        );

        // Verify the contexts table was created
        let table_count: Result<TableCount, diesel::result::Error> = diesel::sql_query(
            "SELECT COUNT(*) as count FROM sqlite_master WHERE type='table' AND name='contexts'",
        )
        .get_result(&mut conn);

        assert!(
            table_count.is_ok(),
            "Contexts table should exist after migrations"
        );
        assert!(
            table_count.unwrap().count > 0,
            "Contexts table should be found in sqlite_master"
        );
    }
}
