---
title: "cloacina::database"
description: "API reference of the Rust module cloacina::database."
editUrl: false
sidebar:
  order: 97
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

**Database Layer**

This module provides database connectivity, schema management, and migration support
for persisting Cloacina execution context and metadata.

**Components**

- [`connection`](../../../rust/cloacina/database/connection/): Database connection management and pooling
- `schema`: Diesel schema definitions

**Database Support**

Supports both PostgreSQL and SQLite through compile-time feature flags:
- PostgreSQL: Full-featured with native UUID and timestamp types
- SQLite: File-based or in-memory with type conversions

The database layer handles:
- Context persistence and retrieval
- Task execution state tracking
- Automatic schema migrations

**Connection Pooling**

The module uses r2d2 for connection pooling with the following default settings:
- Maximum pool size: 10 connections
- Connection timeout: 30 seconds
- Idle timeout: 10 minutes

These settings can be customized when creating a new pool.

**Error Handling**

The module provides a custom `Result` type alias that standardizes error handling
across database operations. All database operations return this type, which wraps
`diesel::result::Error` for consistent error handling.

**Migration System**

The migration system is built on top of Diesel's migration framework and supports:
- Automatic migration detection and application
- Version tracking in the database
- Rollback support
- Transaction-safe migrations

Migrations are stored in the `src/database/migrations` directory and are embedded
into the binary at compile time.

**Usage**

```rust
use cloacina::runner::DefaultRunner;
use cloacina::database::{DbPool, Result};

// Initialize executor (automatically runs migrations)
let executor = DefaultRunner::new("postgresql://user:pass@localhost/cloacina").await?;

// Manual database access example
let pool = DbPool::builder()
    .max_size(15)
    .build(ConnectionManager::new("postgresql://user:pass@localhost/cloacina"))?;

// Get a connection from the pool
let conn = pool.get()?;

// Run migrations manually if needed
run_migrations(&mut conn)?;
```

**Public Types**

- [`DbPool`](../../../rust/cloacina/database/connection/backend/#type.DbPool): The connection pool type for managing database connections
- [`Result<T>`]: Type alias for database operation results

**Public Functions**

- [`run_migrations`](../../../rust/cloacina/database/#fn.run_migrations): Manually runs pending database migrations

Migrations are automatically applied when using `DefaultRunner`. For lower-level
database access, migrations can be run manually using `run_migrations()`.

## Re-exports

- [`AnyConnection`](../../../rust/cloacina/database/connection/backend/#enum.AnyConnection)
- [`AnyPool`](../../../rust/cloacina/database/connection/backend/#enum.AnyPool)
- [`BackendType`](../../../rust/cloacina/database/connection/backend/#enum.BackendType)
- [`Database`](../../../rust/cloacina/database/connection/#struct.Database)
- [`DbConnection`](../../../rust/cloacina/database/connection/backend/#type.DbConnection)
- [`DbConnectionManager`](../../../rust/cloacina/database/connection/backend/#type.DbConnectionManager)
- [`DbPool`](../../../rust/cloacina/database/connection/backend/#type.DbPool)
- [`DbConnection`](../../../rust/cloacina/database/connection/backend/#type.DbConnection)
- [`DbPool`](../../../rust/cloacina/database/connection/backend/#type.DbPool)
- `AdminError` (`admin::AdminError`)
- `DatabaseAdmin` (`admin::DatabaseAdmin`)
- `TenantConfig` (`admin::TenantConfig`)
- `TenantCredentials` (`admin::TenantCredentials`)
- [`DbBinary`](../../../rust/cloacina/database/universal_types/#struct.DbBinary)
- [`DbBool`](../../../rust/cloacina/database/universal_types/#struct.DbBool)
- [`DbTimestamp`](../../../rust/cloacina/database/universal_types/#struct.DbTimestamp)
- [`DbUuid`](../../../rust/cloacina/database/universal_types/#struct.DbUuid)
- [`UniversalBinary`](../../../rust/cloacina/database/universal_types/#struct.UniversalBinary)
- [`UniversalBool`](../../../rust/cloacina/database/universal_types/#struct.UniversalBool)
- [`UniversalTimestamp`](../../../rust/cloacina/database/universal_types/#struct.UniversalTimestamp)
- [`UniversalUuid`](../../../rust/cloacina/database/universal_types/#struct.UniversalUuid)

## Functions

### <span id="fn.run_migrations"></span><code style="font-size: 0.85em">fn run_​migrations</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn run_migrations(conn: &mut DbConnection) -> Result<()>
```

Runs pending database migrations.

This function applies any pending migrations to bring the database
schema up to date with the current version.

Note: This function is only available when exactly one database backend
is enabled (either postgres or sqlite, but not both). For dual-backend
builds, use `run_migrations_postgres` or `run_migrations_sqlite` instead.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `conn` | `&mut DbConnection` | Mutable reference to a database connection (PostgreSQL or SQLite) |

**Returns:**

* `Ok(())` - If migrations complete successfully
* `Err(_)` - If migration fails

**Examples:**

```rust
use cloacina::database::{run_migrations, DbConnection};

run_migrations(&mut conn)?;
```

<details>
<summary>Source</summary>

```rust
pub fn run_migrations(conn: &mut DbConnection) -> Result<()> {
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run migrations");
    Ok(())
}
```

</details>

### <span id="fn.run_migrations_postgres"></span><code style="font-size: 0.85em">fn run_​migrations_​postgres</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn run_migrations_postgres(
    conn: &mut diesel::pg::PgConnection) -> Result<(,
) -> Result<()>
```

Runs pending PostgreSQL database migrations.

This function applies any pending migrations to bring the PostgreSQL database
schema up to date with the current version. Use this function in dual-backend
builds where `run_migrations` is not available.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `conn` | `&mut diesel::pg::PgConnection` | Mutable reference to a PostgreSQL database connection |

**Returns:**

* `Ok(())` - If migrations complete successfully
* `Err(_)` - If migration fails

<details>
<summary>Source</summary>

```rust
pub fn run_migrations_postgres(conn: &mut diesel::pg::PgConnection) -> Result<()> {
    conn.run_pending_migrations(POSTGRES_MIGRATIONS)
        .expect("Failed to run PostgreSQL migrations");
    Ok(())
}
```

</details>

### <span id="fn.run_migrations_sqlite"></span><code style="font-size: 0.85em">fn run_​migrations_​sqlite</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn run_migrations_sqlite(
    conn: &mut diesel::sqlite::SqliteConnection) -> Result<(,
) -> Result<()>
```

Runs pending SQLite database migrations.

This function applies any pending migrations to bring the SQLite database
schema up to date with the current version. Use this function in dual-backend
builds where `run_migrations` is not available.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `conn` | `&mut diesel::sqlite::SqliteConnection` | Mutable reference to a SQLite database connection |

**Returns:**

* `Ok(())` - If migrations complete successfully
* `Err(_)` - If migration fails

<details>
<summary>Source</summary>

```rust
pub fn run_migrations_sqlite(conn: &mut diesel::sqlite::SqliteConnection) -> Result<()> {
    conn.run_pending_migrations(SQLITE_MIGRATIONS)
        .expect("Failed to run SQLite migrations");
    Ok(())
}
```

</details>

## Type aliases

| Name | Type | Description |
|------|------|-------------|
| <span id="type.Result"></span>`Result<T>` | `std::result::Result<T, diesel::result::Error>` | Type alias for database operation results.<br>Standardizes error handling across database operations. |

## Constants

| Name | Type | Value | Description |
|------|------|-------|-------------|
| <span id="const.POSTGRES_MIGRATIONS"></span>`POSTGRES_MIGRATIONS` | `EmbeddedMigrations` | `embed_migrations!("src/database/migrations/postgres")` | Embedded migrations for PostgreSQL. |
| <span id="const.SQLITE_MIGRATIONS"></span>`SQLITE_MIGRATIONS` | `EmbeddedMigrations` | `embed_migrations!("src/database/migrations/sqlite")` | Embedded migrations for SQLite. |
| <span id="const.MIGRATIONS"></span>`MIGRATIONS` | `EmbeddedMigrations` | `POSTGRES_MIGRATIONS` | Embedded migrations for automatic schema management.<br>Contains all SQL migration files for setting up the database schema. Note: With dual-backend support, use POSTGRES_MIGRATIONS or SQLITE_MIGRATIONS directly based on your backend. This alias defaults to PostgreSQL. |
| <span id="const.MIGRATIONS"></span>`MIGRATIONS` | `EmbeddedMigrations` | `SQLITE_MIGRATIONS` | Embedded migrations alias (defaults to SQLite when postgres not enabled) |
