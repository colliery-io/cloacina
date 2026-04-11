# cloacina::database::connection <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Database connection management module supporting both PostgreSQL and SQLite.

This module provides an async connection pool implementation using `deadpool-diesel` for managing
database connections efficiently. It handles async connection pooling, connection lifecycle,
and provides a thread-safe way to access database connections.

**Examples:**

```rust,ignore
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

## Structs

### `cloacina::database::connection::Database`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Represents a pool of database connections.

This struct provides a thread-safe wrapper around a connection pool,
allowing multiple parts of the application to share database connections
efficiently. Supports runtime backend selection between PostgreSQL and SQLite.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `pool` | `AnyPool` | The connection pool (PostgreSQL or SQLite) |
| `backend` | `BackendType` | The detected backend type |
| `schema` | `Option < String >` | The PostgreSQL schema name for multi-tenant isolation (ignored for SQLite) |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (connection_string : & str , database_name : & str , max_size : u32) -> Self
```

Creates a new database connection pool with automatic backend detection.

The backend is detected from the connection string:
- `postgres://` or `postgresql://` -> PostgreSQL
- `sqlite://`, file paths, or `:memory:` -> SQLite

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `connection_string` | `-` | The database connection URL or path |
| `database_name` | `-` | The database name (used for PostgreSQL, ignored for SQLite) |
| `max_size` | `-` | Maximum number of connections in the pool |


**Raises:**

| Exception | Description |
|-----------|-------------|
| `Panic` | Panics if the connection pool cannot be created. |


<details>
<summary>Source</summary>

```rust
    pub fn new(connection_string: &str, database_name: &str, max_size: u32) -> Self {
        Self::new_with_schema(connection_string, database_name, max_size, None)
    }
```

</details>



##### `new_with_schema` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new_with_schema (connection_string : & str , database_name : & str , max_size : u32 , schema : Option < & str > ,) -> Self
```

Creates a new database connection pool with optional schema support.

The backend is detected from the connection string. Schema support is only
effective for PostgreSQL; the schema parameter is stored but ignored for SQLite.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `connection_string` | `-` | The database connection URL or path |
| `database_name` | `-` | The database name (used for PostgreSQL, ignored for SQLite) |
| `max_size` | `-` | Maximum number of connections in the pool |
| `schema` | `-` | Optional schema name for PostgreSQL multi-tenant isolation |


**Raises:**

| Exception | Description |
|-----------|-------------|
| `Panic` | Panics if connection pool creation fails or if the schema name is invalid. Use [`try_new_with_schema`](Self::try_new_with_schema) for fallible construction. |


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



##### `try_new_with_schema` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn try_new_with_schema (connection_string : & str , _database_name : & str , max_size : u32 , schema : Option < & str > ,) -> Result < Self , DatabaseError >
```

Creates a new database connection pool with optional schema support.

This is the fallible version of [`new_with_schema`](Self::new_with_schema).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `connection_string` | `-` | The database connection URL or path |
| `database_name` | `-` | The database name (used for PostgreSQL, ignored for SQLite) |
| `max_size` | `-` | Maximum number of connections in the pool |
| `schema` | `-` | Optional schema name for PostgreSQL multi-tenant isolation |


**Raises:**

| Exception | Description |
|-----------|-------------|
| `Error` | Returns an error if: |
| `Error` | The schema name is invalid (SQL injection prevention) |
| `Error` | The connection pool cannot be created |


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

                Ok(Self {
                    pool: AnyPool::Postgres(pool),
                    backend,
                    schema: validated_schema,
                })
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

                Ok(Self {
                    pool: AnyPool::Sqlite(pool),
                    backend,
                    schema: validated_schema,
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
```

</details>



##### `backend` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn backend (& self) -> BackendType
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



##### `schema` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn schema (& self) -> Option < & str >
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



##### `pool` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn pool (& self) -> AnyPool
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



##### `get_connection` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_connection (& self) -> AnyPool
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



##### `close` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn close (& self)
```

Closes the connection pool, releasing all database connections.

After calling this method, all current and future attempts to get
connections from the pool will fail immediately. This should be called
when shutting down to ensure connections are properly released back to
the database server.

**Examples:**

```rust,ignore
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



##### `build_postgres_url` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn build_postgres_url (base_url : & str , database_name : & str) -> Result < String , url :: ParseError >
```

Builds a PostgreSQL connection URL.

<details>
<summary>Source</summary>

```rust
    fn build_postgres_url(base_url: &str, database_name: &str) -> Result<String, url::ParseError> {
        let mut url = Url::parse(base_url)?;
        url.set_path(database_name);
        Ok(url.to_string())
    }
```

</details>



##### `build_sqlite_url` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn build_sqlite_url (connection_string : & str) -> String
```

Builds a SQLite connection URL.

<details>
<summary>Source</summary>

```rust
    fn build_sqlite_url(connection_string: &str) -> String {
        // Strip sqlite:// prefix if present
        if let Some(path) = connection_string.strip_prefix("sqlite://") {
            path.to_string()
        } else {
            connection_string.to_string()
        }
    }
```

</details>



##### `run_migrations` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn run_migrations (& self) -> Result < () , String >
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



##### `setup_schema` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn setup_schema (& self , schema : & str) -> Result < () , String >
```

Sets up the PostgreSQL schema for multi-tenant isolation.

Creates the schema if it doesn't exist and runs migrations within it.
Returns an error if called on a SQLite backend or if the schema name
is invalid (to prevent SQL injection).

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



##### `get_connection_with_schema` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn get_connection_with_schema (& self ,) -> Result < deadpool :: managed :: Object < PgManager > , deadpool :: managed :: PoolError < deadpool_diesel :: Error > , >
```

Gets a PostgreSQL connection with the schema search path set.

For PostgreSQL, this sets the search path to the configured schema.
For SQLite, this is a no-op and returns an error.

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
```

</details>



##### `get_postgres_connection` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn get_postgres_connection (& self ,) -> Result < deadpool :: managed :: Object < PgManager > , deadpool :: managed :: PoolError < deadpool_diesel :: Error > , >
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



##### `get_sqlite_connection` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn get_sqlite_connection (& self ,) -> Result < deadpool :: managed :: Object < SqliteManager > , deadpool :: managed :: PoolError < deadpool_diesel :: Error > , >
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

        pool.get().await
    }
```

</details>





## Enums

### `cloacina::database::connection::DatabaseError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur during database operations.

This error type covers connection pool creation, URL parsing,
migration execution, and schema validation failures.

#### Variants

- **`PoolCreation`** - Failed to create connection pool
- **`InvalidUrl`** - Failed to parse database URL
- **`Schema`** - Schema validation failed
- **`Migration`** - Migration execution failed
