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
//! Uses PostgreSQL at localhost by default. Docker should be running
//! for integration tests.

use cloacina::database::connection::Database;
use cloacina::database::BackendType;
use diesel::deserialize::QueryableByName;
use diesel::prelude::*;
use diesel::sql_types::Text;
use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex, Once};
use tracing::info;
use uuid;

static INIT: Once = Once::new();
#[cfg(feature = "postgres")]
static POSTGRES_FIXTURE: OnceCell<Arc<Mutex<TestFixture>>> = OnceCell::new();
#[cfg(feature = "sqlite")]
static SQLITE_FIXTURE: OnceCell<Arc<Mutex<TestFixture>>> = OnceCell::new();

/// Default PostgreSQL connection URL
#[cfg(feature = "postgres")]
const DEFAULT_POSTGRES_URL: &str = "postgres://cloacina:cloacina@localhost:5432/cloacina";

/// Get the test schema name from environment variable or generate a unique one
/// This allows CI jobs to isolate their tests using different schemas
#[cfg(feature = "postgres")]
fn get_test_schema() -> String {
    std::env::var("CLOACINA_TEST_SCHEMA").unwrap_or_else(|_| {
        format!(
            "test_{}",
            uuid::Uuid::new_v4().to_string().replace("-", "_")
        )
    })
}

/// Default SQLite connection URL (in-memory with shared cache for testing)
#[cfg(feature = "sqlite")]
const DEFAULT_SQLITE_URL: &str = "file:cloacina_test?mode=memory&cache=shared";

/// Gets or initializes the PostgreSQL test fixture singleton
///
/// This function ensures only one PostgreSQL test fixture exists across all tests,
/// initializing it if necessary. Uses PostgreSQL at localhost with schema isolation.
///
/// Schema is determined by CLOACINA_TEST_SCHEMA env var, or a unique UUID-based schema.
/// This ensures CI jobs running in parallel don't interfere with each other.
///
/// IMPORTANT: Uses only connection pool (no raw PgConnection) to avoid SIGSEGV
/// crashes on Ubuntu CI caused by interaction between raw connections and pool.
///
/// # Returns
/// An Arc<Mutex<TestFixture>> pointing to the shared PostgreSQL test fixture instance
#[cfg(feature = "postgres")]
pub async fn get_or_init_postgres_fixture() -> Arc<Mutex<TestFixture>> {
    POSTGRES_FIXTURE
        .get_or_init(|| {
            let db_url = DEFAULT_POSTGRES_URL.to_string();
            let schema = get_test_schema();

            // Use Database::new_with_schema for schema isolation
            let db = Database::new_with_schema(
                "postgres://cloacina:cloacina@localhost:5432",
                "cloacina",
                5,
                Some(&schema),
            );

            info!(
                "PostgreSQL test fixture initialized with schema: {}",
                schema
            );

            Arc::new(Mutex::new(TestFixture::new_postgres(db, db_url, schema)))
        })
        .clone()
}

/// Gets or initializes the SQLite test fixture singleton
///
/// This function ensures only one SQLite test fixture exists across all tests,
/// initializing it if necessary. Uses an in-memory SQLite database.
///
/// IMPORTANT: SQLite uses a single-connection pool to avoid lock contention.
/// Do NOT create additional raw SqliteConnections - use the pool for all operations.
///
/// # Returns
/// An Arc<Mutex<TestFixture>> pointing to the shared SQLite test fixture instance
#[cfg(feature = "sqlite")]
pub async fn get_or_init_sqlite_fixture() -> Arc<Mutex<TestFixture>> {
    SQLITE_FIXTURE
        .get_or_init(|| {
            let db_url = DEFAULT_SQLITE_URL.to_string();
            let db = Database::new(&db_url, "", 5);
            // Note: SQLite fixture does NOT hold a raw connection.
            // The pool is limited to 1 connection to avoid lock contention,
            // so holding a separate connection would cause deadlocks.
            Arc::new(Mutex::new(TestFixture::new_sqlite(db, db_url)))
        })
        .clone()
}

/// Get the default fixture for the current backend configuration.
/// Returns PostgreSQL fixture when postgres is enabled, SQLite when only sqlite is enabled.
#[cfg(feature = "postgres")]
pub async fn get_or_init_fixture() -> Arc<Mutex<TestFixture>> {
    get_or_init_postgres_fixture().await
}

/// Get the default fixture for the current backend configuration.
/// Returns SQLite fixture when only sqlite is enabled.
#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub async fn get_or_init_fixture() -> Arc<Mutex<TestFixture>> {
    get_or_init_sqlite_fixture().await
}

/// Represents a test fixture for the Cloacina project.
///
/// The fixture supports both PostgreSQL and SQLite backends, determined
/// automatically from the DATABASE_URL.
///
/// All database operations use the connection pool to avoid interaction issues
/// between raw connections and pool connections (which caused SIGSEGV on Ubuntu CI).
#[allow(dead_code)]
pub struct TestFixture {
    /// Flag indicating if the fixture has been initialized
    initialized: bool,
    /// Database connection pool
    db: Database,
    /// The database URL used to create this fixture
    db_url: String,
    /// Schema name for PostgreSQL isolation
    schema: String,
}

impl TestFixture {
    /// Creates a new TestFixture instance for PostgreSQL
    ///
    /// Uses only the connection pool (no raw connection) to avoid interaction
    /// issues that caused SIGSEGV on Ubuntu CI.
    #[cfg(feature = "postgres")]
    pub fn new_postgres(db: Database, db_url: String, schema: String) -> Self {
        INIT.call_once(|| {
            cloacina::init_logging(None);
        });

        info!(
            "Test fixture created (PostgreSQL) with URL: {}, schema: {}",
            db_url, schema
        );

        TestFixture {
            initialized: false,
            db,
            db_url,
            schema,
        }
    }

    /// Creates a new TestFixture instance for SQLite
    ///
    /// SQLite fixtures use a single-connection pool to avoid lock contention.
    #[cfg(feature = "sqlite")]
    pub fn new_sqlite(db: Database, db_url: String) -> Self {
        INIT.call_once(|| {
            cloacina::init_logging(None);
        });

        info!("Test fixture created (SQLite) with URL: {}", db_url);

        TestFixture {
            initialized: false,
            db,
            db_url,
            schema: "main".to_string(), // SQLite uses "main" as default schema
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
        self.db_url.clone()
    }

    /// Get the schema name for this fixture
    pub fn get_schema(&self) -> String {
        self.schema.clone()
    }

    /// Get the name of the current backend (postgres or sqlite)
    pub fn get_current_backend(&self) -> &'static str {
        #[cfg(all(feature = "postgres", feature = "sqlite"))]
        {
            match self.db.backend() {
                BackendType::Postgres => "postgres",
                BackendType::Sqlite => "sqlite",
            }
        }
        #[cfg(all(feature = "postgres", not(feature = "sqlite")))]
        {
            "postgres"
        }
        #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
        {
            "sqlite"
        }
    }

    /// Create a unified storage backend using this fixture's database (primary storage method)
    pub fn create_storage(&self) -> cloacina::dal::UnifiedRegistryStorage {
        cloacina::dal::UnifiedRegistryStorage::new(self.db.clone())
    }

    /// Create storage backend matching the current database backend
    pub fn create_backend_storage(&self) -> Box<dyn cloacina::registry::traits::RegistryStorage> {
        Box::new(cloacina::dal::UnifiedRegistryStorage::new(self.db.clone()))
    }

    /// Create a unified storage backend using this fixture's database
    pub fn create_unified_storage(&self) -> cloacina::dal::UnifiedRegistryStorage {
        cloacina::dal::UnifiedRegistryStorage::new(self.db.clone())
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
        #[cfg(feature = "postgres")]
        if self.db.backend() == BackendType::Postgres {
            // Use setup_schema which creates schema, sets search_path, and runs migrations
            self.db
                .setup_schema(&self.schema)
                .await
                .expect("Failed to setup PostgreSQL schema");
            self.initialized = true;
            return;
        }

        // For SQLite, run migrations through the pool connection
        #[cfg(feature = "sqlite")]
        if self.db.backend() == BackendType::Sqlite {
            let conn = self
                .db
                .get_sqlite_connection()
                .await
                .expect("Failed to get SQLite connection from pool");
            conn.interact(|conn| {
                cloacina::database::run_migrations_sqlite(conn)
                    .expect("Failed to run SQLite migrations");
            })
            .await
            .expect("Failed to run SQLite migrations");
            self.initialized = true;
        }
    }

    /// Reset the database by truncating all tables in the test schema
    pub async fn reset_database(&mut self) {
        // Use the pool for PostgreSQL reset operations to avoid interaction issues
        // between raw connections and pool connections (fixes SIGSEGV on Ubuntu CI)
        #[cfg(feature = "postgres")]
        if self.db.backend() == BackendType::Postgres {
            let schema = self.schema.clone();
            let conn = self
                .db
                .get_postgres_connection()
                .await
                .expect("Failed to get PostgreSQL connection from pool");

            conn.interact(move |conn| {
                use diesel::sql_query;
                use diesel::RunQueryDsl;

                // Define a struct for the query result
                #[derive(QueryableByName)]
                struct TableName {
                    #[diesel(sql_type = Text)]
                    tablename: String,
                }

                // Get list of all user tables in the test schema (excluding migrations table)
                let tables_result: Result<Vec<TableName>, _> = sql_query(&format!(
                    "SELECT tablename FROM pg_tables WHERE schemaname = '{}' AND tablename != '__diesel_schema_migrations'",
                    schema
                ))
                .load::<TableName>(conn);

                if let Ok(table_rows) = tables_result {
                    // Truncate all user tables with CASCADE to handle foreign keys
                    for table_row in &table_rows {
                        let _ = sql_query(&format!(
                            "TRUNCATE TABLE \"{}\".\"{}\" CASCADE",
                            schema, table_row.tablename
                        ))
                        .execute(conn);
                    }
                }
            })
            .await
            .expect("Failed to reset PostgreSQL database");

            return;
        }

        // For SQLite, use the pool connection
        #[cfg(feature = "sqlite")]
        if self.db.backend() == BackendType::Sqlite {
            let conn = self
                .db
                .get_sqlite_connection()
                .await
                .expect("Failed to get SQLite connection from pool");

            conn.interact(|conn| {
                use diesel::sql_query;
                use diesel::RunQueryDsl;

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
                        let _ = sql_query(&format!("DELETE FROM {}", table_row.name)).execute(conn);
                    }
                }

                // Run migrations to ensure schema is up to date
                cloacina::database::run_migrations_sqlite(conn).expect("Failed to run migrations");
            })
            .await
            .expect("Failed to reset SQLite database");
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

    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
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
