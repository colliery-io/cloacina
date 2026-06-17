/*
 *  Copyright 2025-2026 Colliery Software
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
pub use schema_validation::{
    escape_password, validate_schema_name, validate_username, SchemaError, UsernameError,
};

// Legacy type aliases - conditional on features
#[cfg(feature = "postgres")]
pub use backend::{DbConnection, DbConnectionManager, DbPool};

#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub use backend::{DbConnection, DbPool};

#[cfg(feature = "sqlite")]
use std::sync::Arc;
#[cfg(feature = "sqlite")]
use tempfile::NamedTempFile;
use thiserror::Error;
use tracing::info;
use url::Url;

#[cfg(feature = "postgres")]
use deadpool_diesel::postgres::{Manager as PgManager, Pool as PgPool, Runtime as PgRuntime};
#[cfg(feature = "sqlite")]
use deadpool_diesel::sqlite::{
    Manager as SqliteManager, Pool as SqlitePool, Runtime as SqliteRuntime,
};

/// SQLite connection-pool size, unified across the sqlite-only and
/// postgres+sqlite builds.
///
/// **CLOACI-T-0741: set back to 1 (serialize).** CLOACI-T-0622 had bumped this
/// to 4, but a multi-connection sqlite pool under WAL is the source of the
/// chronic integration flake — both `database is locked` errors (concurrent
/// writers) and 180s hangs (WAL/lock contention). SQLite is single-writer; a
/// pool of 1 serialises access and removes the contention entirely. The
/// original reason T-0622 left 1 ("looked like a deadlock" on macOS CI) was an
/// *unbounded* wait on a reentrant/contended checkout — now bounded by
/// `POOL_WAIT_TIMEOUT`, so a true reentrant checkout surfaces as a fast error
/// instead of an infinite hang rather than forcing us onto a contended pool.
#[cfg(feature = "sqlite")]
const SQLITE_POOL_SIZE: usize = 1;

/// Bounded wait for a pool connection. Without it, an exhausted or contended
/// pool (especially the small SQLite pool) waits *indefinitely* for a
/// connection — a reentrant/contended checkout then stalls until some outer
/// timeout kills the process (the 180s scenario-kill behind the sqlite flake).
/// With a bounded wait the same condition surfaces as a fast, actionable
/// `pool timeout` error that names the checkout site instead of hanging.
const POOL_WAIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

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

/// Process-wide strict-search-path flag. CLOACI-T-0582.
///
/// When `true`, `Database::get_connection_with_schema` runs a
/// `SELECT current_schemas(false)` defense-in-depth check after the
/// `SET search_path` — even a successful SET that somehow landed in
/// the wrong schema is caught.
///
/// Set by `cloacina-server` at boot; the daemon (single-tenant per
/// ADR-0005) leaves it `false` to avoid the per-acquire round-trip.
/// SET-failure propagation is unconditional and does not depend on
/// this flag — strict-mode only gates the defense-in-depth check.
static STRICT_SEARCH_PATH: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Set the process-wide strict-search-path flag. Idempotent; safe to call
/// at any point during process startup. CLOACI-T-0582.
pub fn set_strict_search_path(enabled: bool) {
    STRICT_SEARCH_PATH.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

/// Read the process-wide strict-search-path flag. CLOACI-T-0582.
pub fn is_strict_search_path() -> bool {
    STRICT_SEARCH_PATH.load(std::sync::atomic::Ordering::Relaxed)
}

/// Row shape for the `SELECT current_schema()` defense-in-depth probe.
/// `current_schema()` returns NULL when no schema is set; we model
/// that as `Option<String>`. CLOACI-T-0582.
#[cfg(feature = "postgres")]
#[derive(diesel::QueryableByName, Debug)]
struct CurrentSchemaRow {
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    s: Option<String>,
}

/// Construct a `PoolError::Backend` carrying a CLOACI-T-0582 search_path
/// failure so callers see a typed error rather than a silent fallback.
/// The error message names the tenant schema and the underlying cause.
#[cfg(feature = "postgres")]
fn search_path_pool_error(
    tenant_schema: &str,
    cause: &str,
) -> deadpool::managed::PoolError<deadpool_diesel::Error> {
    // Encode as `Ping(QueryBuilderError)` so the public PoolError surface
    // stays unchanged while still surfacing our message. `QueryBuilderError`
    // takes `Box<dyn Error + Send + Sync>` which gives us free-form text.
    let inner = diesel::result::Error::QueryBuilderError(
        format!(
            "search_path setup failed for tenant '{}': {} (CLOACI-T-0582)",
            tenant_schema, cause
        )
        .into(),
    );
    deadpool::managed::PoolError::Backend(deadpool_diesel::Error::Ping(inner))
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
    /// Backing tempfile when the user requested `:memory:` (or
    /// `sqlite://:memory:`). Held via Arc so every Database clone keeps the
    /// file alive; when the last clone drops, NamedTempFile::Drop deletes
    /// the file. See `materialize_sqlite_connection` for why we substitute
    /// a real tempfile for in-memory requests.
    #[cfg(feature = "sqlite")]
    _memory_tempfile: Option<Arc<NamedTempFile>>,
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
                    .runtime(PgRuntime::Tokio1)
                    .wait_timeout(Some(POOL_WAIT_TIMEOUT))
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

                Ok(Self {
                    pool: AnyPool::Postgres(pool),
                    backend,
                    schema: validated_schema,
                    #[cfg(feature = "sqlite")]
                    _memory_tempfile: None,
                })
            }
            BackendType::Sqlite => {
                let (connection_url, memory_tempfile) =
                    Self::materialize_sqlite_connection(connection_string)?;
                let manager = SqliteManager::new(connection_url, SqliteRuntime::Tokio1);
                // CLOACI-T-0622: bumped from 1 to 4. The original `=1` was
                // a workaround for diesel's sqlite open path not passing
                // SQLITE_OPEN_URI, which made every new connection to
                // `:memory:` open a fresh private DB. T-0608 fixed that
                // by materialising `:memory:` as a per-Database tempfile,
                // so every pool connection now opens the same real file.
                // Combined with WAL + `busy_timeout=30000` (applied on
                // every checkout, see `get_sqlite_connection`), multi-
                // connection sqlite is safe. A pool of 1 serialises the
                // executor against the unified scheduler tick (reactor
                // poll + firings pruner, both added under I-0100/T-0602),
                // which on macOS CI was producing contention severe
                // enough to look like a deadlock.
                let sqlite_pool_size = SQLITE_POOL_SIZE;
                let pool = SqlitePool::builder(manager)
                    .max_size(sqlite_pool_size)
                    .runtime(SqliteRuntime::Tokio1)
                    .wait_timeout(Some(POOL_WAIT_TIMEOUT))
                    .build()
                    .map_err(|e| DatabaseError::PoolCreation {
                        backend: "SQLite",
                        source: Box::new(e),
                    })?;

                info!(
                    "SQLite connection pool initialized (size: {})",
                    sqlite_pool_size
                );

                Ok(Self {
                    pool: AnyPool::Sqlite(pool),
                    backend,
                    schema: validated_schema,
                    _memory_tempfile: memory_tempfile,
                })
            }
        }

        #[cfg(all(feature = "postgres", not(feature = "sqlite")))]
        {
            let _ = backend; // suppress unused warning
            let connection_url = Self::build_postgres_url(connection_string, _database_name)?;
            let manager = PgManager::new(connection_url, PgRuntime::Tokio1);
            let pool = PgPool::builder(manager)
                .max_size(max_size as usize)
                .runtime(PgRuntime::Tokio1)
                .wait_timeout(Some(POOL_WAIT_TIMEOUT))
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
                #[cfg(feature = "sqlite")]
                _memory_tempfile: None,
            });
        }

        #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
        {
            let _ = backend; // suppress unused warning
            let (connection_url, memory_tempfile) =
                Self::materialize_sqlite_connection(connection_string)?;
            let manager = SqliteManager::new(connection_url, SqliteRuntime::Tokio1);
            let sqlite_pool_size = SQLITE_POOL_SIZE;
            let pool = SqlitePool::builder(manager)
                .max_size(sqlite_pool_size)
                .runtime(SqliteRuntime::Tokio1)
                .wait_timeout(Some(POOL_WAIT_TIMEOUT))
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
                _memory_tempfile: memory_tempfile,
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
    ///
    /// Respects an explicit database name already present in `base_url` (e.g.
    /// `postgres://host/mydb`); only falls back to the `database_name`
    /// parameter when the URL carries no database (empty path or just `/`).
    ///
    /// Previously this unconditionally called `set_path(database_name)`, so a
    /// `--database-url postgres://…/mydb` silently connected to the
    /// caller-supplied `database_name` (the server hardcodes `"cloacina"`)
    /// while logging `mydb` — data landed in the wrong database (CLOACI-T-0649).
    fn build_postgres_url(base_url: &str, database_name: &str) -> Result<String, url::ParseError> {
        let mut url = Url::parse(base_url)?;
        let has_explicit_db = !url.path().trim_start_matches('/').is_empty();
        if !has_explicit_db {
            url.set_path(database_name);
        }
        Ok(url.to_string())
    }

    /// Resolve a SQLite connection string into (url, optional tempfile owner).
    ///
    /// `:memory:` (with or without the `sqlite://` prefix) is substituted
    /// for a per-Database tempfile on disk. This is the only reliable way
    /// to get multi-connection sharing under diesel — the standard
    /// `file::memory:?cache=shared` form requires `SQLITE_OPEN_URI`, which
    /// diesel's open path doesn't set. Without that flag, sqlite silently
    /// creates a file literally named `:memory:` in CWD and the supposed
    /// "shared cache" never happens.
    ///
    /// The returned `NamedTempFile` must be held for the lifetime of the
    /// Database (we wrap it in `Arc` and stash it on `Self` so Clone'd
    /// Databases share ownership and the file is deleted only when the
    /// last clone drops).
    ///
    /// Returns `(url, Some(handle))` for `:memory:` requests; otherwise
    /// `(url, None)` and the file path passes through unchanged.
    #[cfg(feature = "sqlite")]
    fn materialize_sqlite_connection(
        connection_string: &str,
    ) -> Result<(String, Option<Arc<NamedTempFile>>), DatabaseError> {
        let stripped = connection_string
            .strip_prefix("sqlite://")
            .unwrap_or(connection_string);
        if stripped != ":memory:" {
            return Ok((stripped.to_string(), None));
        }

        // Build a tempfile in the system temp dir. We use NamedTempFile so
        // the path is stable across pool connection opens. The file gets
        // unlinked on Drop.
        let tempfile = NamedTempFile::new().map_err(|e| DatabaseError::PoolCreation {
            backend: "SQLite",
            source: Box::new(e),
        })?;
        let path = tempfile
            .path()
            .to_str()
            .ok_or_else(|| DatabaseError::PoolCreation {
                backend: "SQLite",
                source: Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "tempfile path is not valid UTF-8",
                )),
            })?
            .to_string();
        info!(
            "SQLite `:memory:` substituted with tempfile path '{}' (per-Database, cleaned on drop)",
            path
        );
        Ok((path, Some(Arc::new(tempfile))))
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
                .map_err(|e| format!("Failed to run migrations: {}", e))??;
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
                .map_err(|e| format!("Failed to run migrations: {}", e))??;
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
        .map_err(|e| format!("Failed to run migrations in schema: {}", e))??;

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
            // Validate schema name to prevent SQL injection.
            // Already validated at construction; re-validate as defense in depth.
            let validated_schema = match validate_schema_name(schema) {
                Ok(v) => v.to_string(),
                Err(e) => {
                    // CLOACI-T-0582: a re-validation failure means the
                    // tenant schema name was tampered with after pool
                    // construction. Fail closed, do not return the conn.
                    drop(conn);
                    return Err(search_path_pool_error(schema, &format!("{}", e)));
                }
            };

            // CLOACI-T-0582: previously this was `let _ = conn.interact(...)`
            // which silently masked SET failures and routed subsequent
            // queries to the default search_path (typically `public` +
            // admin schema). Fail closed instead: propagate the error and
            // discard the connection so it isn't reused with an unknown
            // search_path state.
            let schema_name = validated_schema.clone();
            let set_result: Result<Result<usize, diesel::result::Error>, _> = conn
                .interact(move |conn| {
                    let set_search_path_sql = format!("SET search_path TO {}, public", schema_name);
                    diesel::sql_query(&set_search_path_sql).execute(conn)
                })
                .await;
            match set_result {
                Ok(Ok(_)) => { /* SET succeeded */ }
                Ok(Err(diesel_err)) => {
                    tracing::error!(
                        tenant_schema = %validated_schema,
                        error = %diesel_err,
                        "SET search_path failed; rejecting tenant-scoped connection (CLOACI-T-0582)"
                    );
                    drop(conn);
                    return Err(search_path_pool_error(
                        &validated_schema,
                        &format!("{}", diesel_err),
                    ));
                }
                Err(interact_err) => {
                    tracing::error!(
                        tenant_schema = %validated_schema,
                        error = %interact_err,
                        "SET search_path interact failed; rejecting connection (CLOACI-T-0582)"
                    );
                    drop(conn);
                    return Err(search_path_pool_error(
                        &validated_schema,
                        &format!("{}", interact_err),
                    ));
                }
            }

            // CLOACI-T-0582: defense-in-depth check, gated by the
            // process-wide strict-search-path flag. Verifies the SET
            // actually landed in the expected schema by reading
            // `current_schema()` (the first schema in the path).
            if is_strict_search_path() {
                let expected_schema = validated_schema.clone();
                let probe: Result<Result<CurrentSchemaRow, diesel::result::Error>, _> = conn
                    .interact(move |conn| {
                        diesel::sql_query("SELECT current_schema() AS s").get_result(conn)
                    })
                    .await;
                match probe {
                    Ok(Ok(row)) if row.s.as_deref() == Some(expected_schema.as_str()) => {
                        // Match — connection is good.
                    }
                    Ok(Ok(row)) => {
                        tracing::error!(
                            tenant_schema = %expected_schema,
                            actual = ?row.s,
                            "current_schema() mismatch — connection search_path is not the expected tenant schema (CLOACI-T-0582)"
                        );
                        drop(conn);
                        return Err(search_path_pool_error(
                            &expected_schema,
                            &format!(
                                "search_path mismatch: expected '{}', got {:?}",
                                expected_schema, row.s
                            ),
                        ));
                    }
                    Ok(Err(diesel_err)) => {
                        tracing::error!(
                            tenant_schema = %expected_schema,
                            error = %diesel_err,
                            "current_schema() probe failed; rejecting connection (CLOACI-T-0582)"
                        );
                        drop(conn);
                        return Err(search_path_pool_error(
                            &expected_schema,
                            &format!("{}", diesel_err),
                        ));
                    }
                    Err(interact_err) => {
                        tracing::error!(
                            tenant_schema = %expected_schema,
                            error = %interact_err,
                            "current_schema() interact failed; rejecting connection (CLOACI-T-0582)"
                        );
                        drop(conn);
                        return Err(search_path_pool_error(
                            &expected_schema,
                            &format!("{}", interact_err),
                        ));
                    }
                }
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

        let conn = pool.get().await?;
        // Ensure SQLite pragmas are set on every checkout — pragmas are per-connection
        // and may be lost if the pool recycles the connection.
        conn.interact(|conn| {
            use diesel::prelude::*;
            let _ = diesel::sql_query("PRAGMA journal_mode=WAL;").execute(conn);
            let _ = diesel::sql_query("PRAGMA busy_timeout=30000;").execute(conn);
        })
        .await
        .ok();
        Ok(conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // CLOACI-T-0649: build_postgres_url must respect an explicit database name
    // in the URL and only fall back to the parameter when the URL has none.
    #[test]
    fn build_postgres_url_respects_explicit_dbname() {
        // Explicit dbname in the URL is preserved (NOT overridden by the param).
        let url =
            Database::build_postgres_url("postgres://u:p@host:5432/mydb", "cloacina").unwrap();
        assert!(
            url.contains("/mydb") && !url.contains("/cloacina"),
            "explicit dbname must win: {url}"
        );
    }

    #[test]
    fn build_postgres_url_falls_back_when_no_dbname() {
        // No path → fall back to the parameter.
        let url = Database::build_postgres_url("postgres://u:p@host:5432", "cloacina").unwrap();
        assert!(
            url.contains("/cloacina"),
            "should fall back to param: {url}"
        );
        // Bare "/" path also counts as "no database".
        let url2 = Database::build_postgres_url("postgres://u:p@host:5432/", "cloacina").unwrap();
        assert!(
            url2.contains("/cloacina"),
            "bare slash should fall back: {url2}"
        );
    }

    // -----------------------------------------------------------------------
    // CLOACI-T-0582: strict-search-path flag toggle
    // -----------------------------------------------------------------------
    //
    // The flag is a process-wide AtomicBool; tests for it must not run in
    // parallel with anything else that reads `is_strict_search_path()`.
    // We don't have a serial-test gate here since this is the only writer
    // surface, but document the constraint for future maintainers.

    #[test]
    fn strict_search_path_default_off() {
        // Snapshot then restore so we don't perturb other tests running
        // serially after this one.
        let prev = is_strict_search_path();
        set_strict_search_path(false);
        assert!(!is_strict_search_path());
        set_strict_search_path(prev);
    }

    #[test]
    fn strict_search_path_set_round_trip() {
        let prev = is_strict_search_path();
        set_strict_search_path(true);
        assert!(is_strict_search_path());
        set_strict_search_path(false);
        assert!(!is_strict_search_path());
        set_strict_search_path(prev);
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn search_path_pool_error_carries_tenant_and_cause() {
        let err = search_path_pool_error("tenant_acme", "SET failed: permission denied");
        // PoolError doesn't impl PartialEq; assert via the Display impl.
        let s = format!("{}", err);
        assert!(
            s.contains("tenant_acme"),
            "error should name the tenant: {s}"
        );
        assert!(
            s.contains("CLOACI-T-0582"),
            "error should be marked with the ticket id: {s}"
        );
        assert!(
            s.contains("permission denied"),
            "error should carry the underlying cause: {s}"
        );
    }

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

    #[cfg(feature = "sqlite")]
    #[test]
    fn test_sqlite_connection_strings_passthrough() {
        // Plain file paths pass through unchanged + no tempfile owner.
        let (url, owner) = Database::materialize_sqlite_connection("/path/to/database.db").unwrap();
        assert_eq!(url, "/path/to/database.db");
        assert!(owner.is_none());

        let (url, owner) = Database::materialize_sqlite_connection("./database.db").unwrap();
        assert_eq!(url, "./database.db");
        assert!(owner.is_none());

        // sqlite:// prefix is stripped.
        let (url, owner) =
            Database::materialize_sqlite_connection("sqlite:///path/to/db.sqlite").unwrap();
        assert_eq!(url, "/path/to/db.sqlite");
        assert!(owner.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn test_sqlite_memory_substitutes_tempfile() {
        // `:memory:` (with or without prefix) becomes a real tempfile path,
        // and the NamedTempFile owner is returned so the caller can keep
        // the file alive for the Database's lifetime.
        for input in [":memory:", "sqlite://:memory:"] {
            let (url, owner) = Database::materialize_sqlite_connection(input).unwrap();
            assert_ne!(url, ":memory:", "input '{}' was not substituted", input);
            let owner =
                owner.unwrap_or_else(|| panic!("input '{}' returned no tempfile owner", input));
            assert!(
                std::path::Path::new(&url).exists(),
                "substituted path '{}' for input '{}' does not exist on disk",
                url,
                input
            );
            // Drop the owner — file should disappear.
            drop(owner);
            assert!(
                !std::path::Path::new(&url).exists(),
                "tempfile '{}' for input '{}' was not cleaned on owner drop",
                url,
                input
            );
        }
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
