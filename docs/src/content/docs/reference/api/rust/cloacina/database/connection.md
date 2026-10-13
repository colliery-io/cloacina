---
title: "cloacina::database::connection"
description: "Database connection management module supporting both PostgreSQL and SQLite."
editUrl: false
sidebar:
  order: 99
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Database connection management module supporting both PostgreSQL and SQLite.

This module provides an async connection pool implementation using `deadpool-diesel` for managing
database connections efficiently. It handles async connection pooling, connection lifecycle,
and provides a thread-safe way to access database connections.

**Features**

- Connection pooling with configurable pool size
- Thread-safe connection management
- Automatic connection cleanup
- URL-based configuration for PostgreSQL
- File path or `:memory:` configuration for SQLite

**Examples:**

```rust
use cloacina::database::connection::Database;

// PostgreSQL
//! let db = Database::new(
    "postgres://username:password@localhost:5432",
    "my_database",
    10
);

// SQLite
//! let db = Database::new(
    "path/to/database.db",
    "", // Not used for SQLite
    10
);
```

## Re-exports

- [`AnyConnection`](../../../../rust/cloacina/database/connection/backend/#enum.AnyConnection)
- [`AnyPool`](../../../../rust/cloacina/database/connection/backend/#enum.AnyPool)
- [`BackendType`](../../../../rust/cloacina/database/connection/backend/#enum.BackendType)
- [`SchemaError`](../../../../rust/cloacina/database/connection/schema_validation/#enum.SchemaError)
- [`UsernameError`](../../../../rust/cloacina/database/connection/schema_validation/#enum.UsernameError)
- [`escape_password`](../../../../rust/cloacina/database/connection/schema_validation/#fn.escape_password)
- [`validate_schema_name`](../../../../rust/cloacina/database/connection/schema_validation/#fn.validate_schema_name)
- [`validate_username`](../../../../rust/cloacina/database/connection/schema_validation/#fn.validate_username)
- [`DbConnection`](../../../../rust/cloacina/database/connection/backend/#type.DbConnection)
- [`DbConnectionManager`](../../../../rust/cloacina/database/connection/backend/#type.DbConnectionManager)
- [`DbPool`](../../../../rust/cloacina/database/connection/backend/#type.DbPool)
- [`DbConnection`](../../../../rust/cloacina/database/connection/backend/#type.DbConnection)
- [`DbPool`](../../../../rust/cloacina/database/connection/backend/#type.DbPool)

## Structs

### <span id="struct.Database"></span><code style="font-size: 0.85em">struct Database</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Represents a pool of database connections.

This struct provides a thread-safe wrapper around a connection pool,
allowing multiple parts of the application to share database connections
efficiently. Supports runtime backend selection between PostgreSQL and SQLite.

**Thread Safety**

The `Database` struct is `Clone` and can be safely shared between threads.
Each clone references the same underlying connection pool.

#### Methods

##### <span id="Database.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(
    connection_string: &str,
    database_name: &str,
    max_size: u32,
) -> Self
```

Creates a new database connection pool with automatic backend detection.

The backend is detected from the connection string:
- `postgres://` or `postgresql://` -> PostgreSQL
- `sqlite://`, file paths, or `:memory:` -> SQLite

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `connection_string` | `&str` | The database connection URL or path |
| `database_name` | `&str` | The database name (used for PostgreSQL, ignored for SQLite) |
| `max_size` | `u32` | Maximum number of connections in the pool |

**Panics:**

Panics if the connection pool cannot be created.

<details>
<summary>Source</summary>

```rust
    pub fn new(connection_string: &str, database_name: &str, max_size: u32) -> Self {
        Self::new_with_schema(connection_string, database_name, max_size, None)
    }
```

</details>

##### <span id="Database.new_with_schema"></span><code>new_​with_​schema</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new_with_schema(
    connection_string: &str,
    database_name: &str,
    max_size: u32,
    schema: Option<&str>,
) -> Self
```

Creates a new database connection pool with optional schema support.

The backend is detected from the connection string. Schema support is only
effective for PostgreSQL; the schema parameter is stored but ignored for SQLite.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `connection_string` | `&str` | The database connection URL or path |
| `database_name` | `&str` | The database name (used for PostgreSQL, ignored for SQLite) |
| `max_size` | `u32` | Maximum number of connections in the pool |
| `schema` | `Option<&str>` | Optional schema name for PostgreSQL multi-tenant isolation |

**Panics:**

Panics if connection pool creation fails or if the schema name is invalid.
Use `try_new_with_schema` for fallible construction.

<details>
<summary>Source</summary>

```rust
    pub fn new_with_schema(
        connection_string: &str,
        database_name: &str,
        max_size: u32,
        schema: Option<&str>,
    ) -> Self {
        Self::try_new_with_schema(connection_string, database_name, max_size, schema)
            .expect("Failed to create database connection pool")
    }
```

</details>

##### <span id="Database.try_new_with_schema"></span><code>try_​new_​with_​schema</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn try_new_with_schema(
    connection_string: &str,
    _database_name: &str,
    max_size: u32,
    schema: Option<&str>,
) -> Result<Self, DatabaseError>
```

Creates a new database connection pool with optional schema support.

This is the fallible version of `new_with_schema`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `connection_string` | `&str` | The database connection URL or path |
| `database_name` | `-` | The database name (used for PostgreSQL, ignored for SQLite) |
| `max_size` | `u32` | Maximum number of connections in the pool |
| `schema` | `Option<&str>` | Optional schema name for PostgreSQL multi-tenant isolation |

**Errors:**

- Returns an error if:
- The schema name is invalid (SQL injection prevention)
- The connection pool cannot be created

<details>
<summary>Source</summary>

```rust
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
                let endpoint_source = connection_url.clone();
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
                    endpoint: endpoint_of(&endpoint_source),
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
                    endpoint: None,
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
            let endpoint_source = connection_url.clone();
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
                endpoint: endpoint_of(&endpoint_source),
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
                endpoint: None,
                _memory_tempfile: memory_tempfile,
            });
        }
    }
```

</details>

##### <span id="Database.backend"></span><code>backend</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn backend(&self) -> BackendType
```

Returns the detected backend type.

<details>
<summary>Source</summary>

```rust
    pub fn backend(&self) -> BackendType {
        self.backend
    }
```

</details>

##### <span id="Database.schema"></span><code>schema</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn schema(&self) -> Option<&str>
```

Returns the schema name if set.

<details>
<summary>Source</summary>

```rust
    pub fn schema(&self) -> Option<&str> {
        self.schema.as_deref()
    }
```

</details>

##### <span id="Database.tenant_connection_string"></span><code>tenant_​connection_​string</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn tenant_connection_string(
    &self,
    username: &str,
    password: &str,
) -> Option<String>
```

Build a connection string for `username`/`password` that points at the SAME deployment this pool dials (CLOACI-T-0888).

Host, port, database and query parameters are taken from the resolved
admin URL, so a tenant credential issued against a non-default
deployment — the dev stack on 15432, a remote host, a database not
named `cloacina` — is actually dialable. Only the credentials are
substituted.

Returns `None` for SQLite (no per-user credential model) and when the
endpoint could not be derived, so callers can decide what to do rather
than receive a plausible-looking but wrong string.

<details>
<summary>Source</summary>

```rust
    pub fn tenant_connection_string(&self, username: &str, password: &str) -> Option<String> {
        inject_credentials(self.endpoint.as_deref()?, username, password)
    }
```

</details>

##### <span id="Database.pool"></span><code>pool</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn pool(&self) -> AnyPool
```

Returns a clone of the connection pool.

<details>
<summary>Source</summary>

```rust
    pub fn pool(&self) -> AnyPool {
        self.pool.clone()
    }
```

</details>

##### <span id="Database.get_connection"></span><code>get_​connection</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_connection(&self) -> AnyPool
```

Alias for `pool()` for backward compatibility.

<details>
<summary>Source</summary>

```rust
    pub fn get_connection(&self) -> AnyPool {
        self.pool.clone()
    }
```

</details>

##### <span id="Database.close"></span><code>close</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn close(&self)
```

Closes the connection pool, releasing all database connections.

After calling this method, all current and future attempts to get
connections from the pool will fail immediately. This should be called
when shutting down to ensure connections are properly released back to
the database server.

**Examples:**

```rust
let db = Database::new("postgres://localhost/mydb", "mydb", 10)?;
// ... use database ...
db.close(); // Release all connections
```

<details>
<summary>Source</summary>

```rust
    pub fn close(&self) {
        tracing::info!("Closing database connection pool");
        self.pool.close();
    }
```

</details>

##### <span id="Database.run_migrations"></span><code>run_​migrations</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn run_migrations(&self) -> Result<(), String>
```

Runs pending database migrations for the appropriate backend.

This method detects the backend type and runs the corresponding migrations.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="Database.setup_schema"></span><code>setup_​schema</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn setup_schema(&self, schema: &str) -> Result<(), String>
```

Sets up the PostgreSQL schema for multi-tenant isolation.

Creates the schema if it doesn't exist and runs migrations within it.
Returns an error if called on a SQLite backend or if the schema name
is invalid (to prevent SQL injection).

**Security**

Schema names are validated to prevent SQL injection attacks.
Only alphanumeric characters and underscores are allowed.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="Database.get_connection_with_schema"></span><code>get_​connection_​with_​schema</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_connection_with_schema(
    &self,
) -> Result<
    deadpool::managed::Object<PgManager>,
    deadpool::managed::PoolError<deadpool_diesel::Error>,
>
```

Gets a PostgreSQL connection with the schema search path set.

For PostgreSQL, this sets the search path to the configured schema.
For SQLite, this is a no-op and returns an error.

**Security**

Schema names are validated before use in SQL to prevent injection attacks.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="Database.get_postgres_connection"></span><code>get_​postgres_​connection</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_postgres_connection(
    &self,
) -> Result<
    deadpool::managed::Object<PgManager>,
    deadpool::managed::PoolError<deadpool_diesel::Error>,
>
```

Gets a PostgreSQL connection.

Returns an error if this is a SQLite backend.

<details>
<summary>Source</summary>

```rust
    pub async fn get_postgres_connection(
        &self,
    ) -> Result<
        deadpool::managed::Object<PgManager>,
        deadpool::managed::PoolError<deadpool_diesel::Error>,
    > {
        self.get_connection_with_schema().await
    }
```

</details>

##### <span id="Database.get_sqlite_connection"></span><code>get_​sqlite_​connection</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_sqlite_connection(
    &self,
) -> Result<
    deadpool::managed::Object<SqliteManager>,
    deadpool::managed::PoolError<deadpool_diesel::Error>,
>
```

Gets a SQLite connection.

Returns an error if this is a PostgreSQL backend.

<details>
<summary>Source</summary>

```rust
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
```

</details>

## Enums

### <span id="enum.DatabaseError"></span><code style="font-size: 0.85em">enum DatabaseError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors that can occur during database operations.

This error type covers connection pool creation, URL parsing,
migration execution, and schema validation failures.

#### Variants

- **`PoolCreation`** - Failed to create connection pool
- **`InvalidUrl`** - Failed to parse database URL
- **`Schema`** - Schema validation failed
- **`Migration`** - Migration execution failed

## Functions

### <span id="fn.set_strict_search_path"></span><code style="font-size: 0.85em">fn set_​strict_​search_​path</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn set_strict_search_path(enabled: bool)
```

Set the process-wide strict-search-path flag. Idempotent; safe to call at any point during process startup. CLOACI-T-0582.

<details>
<summary>Source</summary>

```rust
pub fn set_strict_search_path(enabled: bool) {
    STRICT_SEARCH_PATH.store(enabled, std::sync::atomic::Ordering::Relaxed);
}
```

</details>

### <span id="fn.is_strict_search_path"></span><code style="font-size: 0.85em">fn is_​strict_​search_​path</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_strict_search_path() -> bool
```

Read the process-wide strict-search-path flag. CLOACI-T-0582.

<details>
<summary>Source</summary>

```rust
pub fn is_strict_search_path() -> bool {
    STRICT_SEARCH_PATH.load(std::sync::atomic::Ordering::Relaxed)
}
```

</details>
