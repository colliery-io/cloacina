---
title: "cloacina::database::connection"
description: "Database connection management module supporting both PostgreSQL and SQLite."
sidebar:
  order: 115
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Database connection management module supporting both PostgreSQL and SQLite.

This module provides an async connection pool implementation using `deadpool-diesel` for managing
database connections efficiently. It handles async connection pooling, connection lifecycle,
and provides a thread-safe way to access database connections.

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

## Structs

### <span id="struct.CurrentSchemaRow"></span>`struct CurrentSchemaRow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


**Derives:** `diesel :: QueryableByName`, `Debug`

Row shape for the `SELECT current_schema()` defense-in-depth probe. `current_schema()` returns NULL when no schema is set; we model that as `Option<String>`. CLOACI-T-0582.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `s` | `Option < String >` |  |



### <span id="struct.Database"></span>`struct Database`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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
| `endpoint` | `Option < String >` | The resolved PostgreSQL endpoint this pool dials, with ANY userinfo
stripped — `postgresql://host:port/dbname?params` (CLOACI-T-0888).

Kept so tenant provisioning can hand out a credential string that points
at the SAME deployment the admin connection uses, instead of guessing
`localhost:5432`. Credentials are deliberately NOT retained here: the
pool already holds the admin secret, and this field exists purely to
carry the non-secret coordinates. `None` for SQLite, which has no
per-user credential model to hand out. |
| `_memory_tempfile` | `Option < Arc < NamedTempFile > >` | Backing tempfile when the user requested `:memory:` (or
`sqlite://:memory:`). Held via Arc so every Database clone keeps the
file alive; when the last clone drops, NamedTempFile::Drop deletes
the file. See `materialize_sqlite_connection` for why we substitute
a real tempfile for in-memory requests. |

#### Methods

##### <span id="Database.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="Database.new_with_schema"></span>`new_with_schema`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="Database.try_new_with_schema"></span>`try_new_with_schema`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="Database.backend"></span>`backend`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="Database.schema"></span>`schema`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="Database.tenant_connection_string"></span>`tenant_connection_string`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn tenant_connection_string (& self , username : & str , password : & str) -> Option < String >
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



##### <span id="Database.pool"></span>`pool`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="Database.get_connection"></span>`get_connection`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="Database.close"></span>`close`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn close (& self)
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



##### <span id="Database.build_postgres_url"></span>`build_postgres_url`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn build_postgres_url (base_url : & str , database_name : & str) -> Result < String , url :: ParseError >
```

Builds a PostgreSQL connection URL.

Respects an explicit database name already present in `base_url` (e.g.
`postgres://host/mydb`); only falls back to the `database_name`
parameter when the URL carries no database (empty path or just `/`).
Previously this unconditionally called `set_path(database_name)`, so a
`--database-url postgres://…/mydb` silently connected to the
caller-supplied `database_name` (the server hardcodes `"cloacina"`)
while logging `mydb` — data landed in the wrong database (CLOACI-T-0649).

<details>
<summary>Source</summary>

```rust
    fn build_postgres_url(base_url: &str, database_name: &str) -> Result<String, url::ParseError> {
        let mut url = Url::parse(base_url)?;
        let has_explicit_db = !url.path().trim_start_matches('/').is_empty();
        if !has_explicit_db {
            url.set_path(database_name);
        }
        // CLOACI-T-0910: default `gssencmode=disable` (caller's explicit value
        // wins). libpq's GSSAPI negotiation walks libkrb5 init on every new
        // connection, which is getenv-heavy — and glibc's getenv is unlocked,
        // so it segfaults if ANY other thread setenv/putenv's concurrently
        // (e.g. Python `os.environ[...] = ...` while a pool thread opens a
        // connection). This was the rotating CI segfault class: symbolized
        // core showed SIGSEGV in __GI_getenv("KRB5_TRACE") under
        // PQconnectdb ← deadpool Manager::create on a tokio blocking thread.
        // Disabling GSS encryption skips the krb5 path entirely; anyone who
        // actually needs GSSAPI sets gssencmode explicitly in their URL.
        let has_gssencmode = url.query_pairs().any(|(k, _)| k == "gssencmode");
        if !has_gssencmode {
            url.query_pairs_mut().append_pair("gssencmode", "disable");
        }
        Ok(url.to_string())
    }
```

</details>



##### <span id="Database.materialize_sqlite_connection"></span>`materialize_sqlite_connection`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn materialize_sqlite_connection (connection_string : & str ,) -> Result < (String , Option < Arc < NamedTempFile > >) , DatabaseError >
```

Resolve a SQLite connection string into (url, optional tempfile owner).

`:memory:` (with or without the `sqlite://` prefix) is substituted
for a per-Database tempfile on disk. Diesel 2.3+ does set
`SQLITE_OPEN_URI` (and rewrites a leading `sqlite://` to `file:`),
but this crate strips the scheme before diesel sees the string, so
the path is always opened literally — the tempfile substitution
remains the reliable way to get multi-connection sharing here.
Query parameters after a `sqlite://` scheme are dropped with a
warning: they were never interpreted on this path (WAL and
busy_timeout are applied as pragmas on every connection), and
historically they leaked into the on-disk filename
(`app.db?mode=rwc` the file — CLOACI-T-0912). Bare paths pass
through byte-for-byte.
The returned `NamedTempFile` must be held for the lifetime of the
Database (we wrap it in `Arc` and stash it on `Self` so Clone'd
Databases share ownership and the file is deleted only when the
last clone drops).
Returns `(url, Some(handle))` for `:memory:` requests; otherwise
`(url, None)` and the file path passes through unchanged.

<details>
<summary>Source</summary>

```rust
    fn materialize_sqlite_connection(
        connection_string: &str,
    ) -> Result<(String, Option<Arc<NamedTempFile>>), DatabaseError> {
        let (stripped, had_scheme) = match connection_string.strip_prefix("sqlite://") {
            Some(rest) => (rest, true),
            None => (connection_string, false),
        };
        let stripped = if had_scheme {
            match stripped.split_once('?') {
                Some((path, params)) => {
                    warn!(
                        "ignoring query parameters in sqlite:// URL (they are not \
                         interpreted; WAL + busy_timeout are set automatically): ?{}",
                        params
                    );
                    path
                }
                None => stripped,
            }
        } else {
            stripped
        };
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
```

</details>



##### <span id="Database.run_migrations"></span>`run_migrations`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


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



##### <span id="Database.setup_schema"></span>`setup_schema`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


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



##### <span id="Database.get_connection_with_schema"></span>`get_connection_with_schema`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


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



##### <span id="Database.get_postgres_connection"></span>`get_postgres_connection`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


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



##### <span id="Database.get_sqlite_connection"></span>`get_sqlite_connection`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


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

### <span id="enum.DatabaseError"></span>`enum DatabaseError`

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

### <span id="fn.set_strict_search_path"></span>`fn set_strict_search_path`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn set_strict_search_path (enabled : bool)
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



### <span id="fn.is_strict_search_path"></span>`fn is_strict_search_path`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn is_strict_search_path () -> bool
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



### <span id="fn.search_path_pool_error"></span>`fn search_path_pool_error`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn search_path_pool_error (tenant_schema : & str , cause : & str ,) -> deadpool :: managed :: PoolError < deadpool_diesel :: Error >
```

Construct a `PoolError::Backend` carrying a CLOACI-T-0582 search_path failure so callers see a typed error rather than a silent fallback. The error message names the tenant schema and the underlying cause.

<details>
<summary>Source</summary>

```rust
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
```

</details>



### <span id="fn.endpoint_of"></span>`fn endpoint_of`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn endpoint_of (connection_url : & str) -> Option < String >
```

Reduce a resolved PostgreSQL URL to its non-secret coordinates (CLOACI-T-0888): scheme, host, port, database, and query params, with any `user:password@` removed.

Returns `None` when the URL will not parse — the caller then falls back
rather than emitting a half-built string.

<details>
<summary>Source</summary>

```rust
fn endpoint_of(connection_url: &str) -> Option<String> {
    let mut url = url::Url::parse(connection_url).ok()?;
    // Both setters only fail on a "cannot-be-a-base" URL, which a parsed
    // postgres:// URL never is.
    url.set_username("").ok()?;
    url.set_password(None).ok()?;
    Some(url.to_string())
}
```

</details>



### <span id="fn.inject_credentials"></span>`fn inject_credentials`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn inject_credentials (endpoint : & str , username : & str , password : & str) -> Option < String >
```

Substitute `username`/`password` into an endpoint produced by [`endpoint_of`], preserving host, port, database and query parameters (CLOACI-T-0888).

Split out from [`Database::tenant_connection_string`] so the string
behavior is unit-testable without standing up a connection pool.

<details>
<summary>Source</summary>

```rust
fn inject_credentials(endpoint: &str, username: &str, password: &str) -> Option<String> {
    let mut url = url::Url::parse(endpoint).ok()?;
    url.set_username(username).ok()?;
    // `set_password(Some(""))` would serialize a bare `:` — normalize an empty
    // password to "no password" instead.
    url.set_password(if password.is_empty() {
        None
    } else {
        Some(password)
    })
    .ok()?;
    Some(url.to_string())
}
```

</details>



