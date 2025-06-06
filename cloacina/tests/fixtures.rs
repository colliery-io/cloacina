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

use cloacina::database::connection::{Database, DbConnection};
use diesel::deserialize::QueryableByName;
use diesel::prelude::*;
use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex, Once};
use tracing::info;

#[cfg(feature = "postgres")]
use diesel::pg::PgConnection;
#[cfg(feature = "sqlite")]
use diesel::sqlite::SqliteConnection;

static INIT: Once = Once::new();
static FIXTURE: OnceCell<Arc<Mutex<TestFixture>>> = OnceCell::new();

/// Gets or initializes a test fixture singleton
///
/// This function ensures only one test fixture exists across all tests,
/// initializing it if necessary.
///
/// # Returns
/// An Arc<Mutex<TestFixture>> pointing to the shared test fixture instance
pub async fn get_or_init_fixture() -> Arc<Mutex<TestFixture>> {
    FIXTURE
        .get_or_init(|| {
            #[cfg(feature = "postgres")]
            {
                let db =
                    Database::new("postgres://cloacina:cloacina@localhost:5432", "cloacina", 5);
                let conn =
                    PgConnection::establish("postgres://cloacina:cloacina@localhost:5432/cloacina")
                        .expect("Failed to connect to PostgreSQL database");
                Arc::new(Mutex::new(TestFixture::new(db, conn)))
            }
            #[cfg(feature = "sqlite")]
            {
                // Use file:memdb1?mode=memory&cache=shared for shared in-memory database
                // This ensures all connections share the same database
                let db_url = "file:memdb1?mode=memory&cache=shared";
                let db = Database::new(db_url, "", 5);
                let conn = SqliteConnection::establish(db_url)
                    .expect("Failed to connect to SQLite database");
                Arc::new(Mutex::new(TestFixture::new(db, conn)))
            }
        })
        .clone()
}

/// Represents a test fixture for the Cloacina project.
#[allow(dead_code)]
pub struct TestFixture {
    /// Flag indicating if the fixture has been initialized
    initialized: bool,
    /// Database connection pool
    db: Database,
    /// Direct database connection for migrations
    conn: DbConnection,
}

impl TestFixture {
    /// Creates a new TestFixture instance.
    pub fn new(db: Database, conn: DbConnection) -> Self {
        INIT.call_once(|| {
            // Initialize logging
            cloacina::init_logging(None);
        });

        info!("Test fixture created");

        TestFixture {
            initialized: false,
            db,
            conn,
        }
    }

    /// Get a mutable reference to the database connection
    pub fn get_connection(&mut self) -> &mut DbConnection {
        &mut self.conn
    }

    /// Get a DAL instance using the database connection pool
    pub fn get_dal(&self) -> cloacina::dal::DAL {
        cloacina::dal::DAL::new(self.db.get_connection())
    }

    /// Get a clone of the database instance
    pub fn get_database(&self) -> Database {
        self.db.clone()
    }

    /// Initialize the fixture with additional setup
    pub async fn initialize(&mut self) {
        // Initialize the database schema
        cloacina::database::run_migrations(&mut self.conn).expect("Failed to run migrations");
        self.initialized = true;
    }

    /// Reset the database by dropping and recreating it
    pub async fn reset_database(&mut self) {
        // For PostgreSQL, we need to properly handle connection termination
        #[cfg(feature = "postgres")]
        {
            use diesel::Connection;
            
            // Connect to the 'postgres' database to perform admin operations
            let mut admin_conn = PgConnection::establish("postgres://cloacina:cloacina@localhost:5432/postgres")
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
            let mut conn = PgConnection::establish("postgres://cloacina:cloacina@localhost:5432/cloacina")
                .expect("Failed to connect to PostgreSQL database");

            // Run migrations
            cloacina::database::run_migrations(&mut conn)
                .expect("Failed to run migrations");

            // Update the fixture's connections
            self.db = db;
            self.conn = conn;
        }

        #[cfg(feature = "sqlite")]
        {
            // For SQLite, we just need to run migrations
            cloacina::database::run_migrations(&mut self.conn)
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
    #[diesel(sql_type = diesel::sql_types::Bigint)]
    count: i64,
}

#[cfg(test)]
pub mod fixtures {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    #[cfg(feature = "postgres")]
    async fn test_migration_function_postgres() {
        let mut conn =
            PgConnection::establish("postgres://cloacina:cloacina@localhost:5432/cloacina")
                .expect("Failed to connect to database");

        // Test that our migration function works
        let result = cloacina::database::run_migrations(&mut conn);
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
    #[cfg(feature = "sqlite")]
    async fn test_migration_function_sqlite() {
        let mut conn = SqliteConnection::establish("file:test_memdb?mode=memory&cache=shared")
            .expect("Failed to connect to database");

        // Test that our migration function works
        let result = cloacina::database::run_migrations(&mut conn);
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

