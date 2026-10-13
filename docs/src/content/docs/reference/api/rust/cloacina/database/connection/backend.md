---
title: "cloacina::database::connection::backend"
description: "Database backend types and runtime backend selection."
editUrl: false
sidebar:
  order: 100
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Database backend types and runtime backend selection.

## Enums

### <span id="enum.BackendType"></span><code style="font-size: 0.85em">enum BackendType</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Represents the database backend type, detected at runtime from the connection URL.

#### Variants

- **`Postgres`** - PostgreSQL backend
- **`Sqlite`** - SQLite backend

#### Methods

##### <span id="BackendType.from_url"></span><code>from_​url</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_url(url: &str) -> Self
```

Detect the backend type from a connection URL.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `url` | `&str` | The database connection URL |

**Returns:**

The detected `BackendType`

**Panics:**

Panics if the URL scheme doesn't match any enabled backend.

<details>
<summary>Source</summary>

```rust
    pub fn from_url(url: &str) -> Self {
        #[cfg(feature = "postgres")]
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            return BackendType::Postgres;
        }

        // SQLite URLs can be:
        // - sqlite:// prefix
        // - file: URI format (e.g., file:test?mode=memory&cache=shared)
        // - file paths (relative or absolute)
        // - :memory: for in-memory databases
        #[cfg(feature = "sqlite")]
        if url.starts_with("sqlite://")
            || url.starts_with("file:")
            || url.starts_with("/")
            || url.starts_with("./")
            || url.starts_with("../")
            || url == ":memory:"
            || url.ends_with(".db")
            || url.ends_with(".sqlite")
            || url.ends_with(".sqlite3")
        {
            return BackendType::Sqlite;
        }

        #[cfg(all(feature = "postgres", feature = "sqlite"))]
        panic!(
            "Unable to detect database backend from URL '{}'. \
             Expected postgres://, postgresql://, sqlite://, or a file path.",
            url
        );

        #[cfg(all(feature = "postgres", not(feature = "sqlite")))]
        panic!(
            "Unable to detect database backend from URL '{}'. \
             Expected postgres:// or postgresql:// (sqlite feature not enabled).",
            url
        );

        #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
        panic!(
            "Unable to detect database backend from URL '{}'. \
             Expected sqlite://, file path, or :memory: (postgres feature not enabled).",
            url
        );

        #[cfg(not(any(feature = "postgres", feature = "sqlite")))]
        panic!("No database backend enabled. Enable either 'postgres' or 'sqlite' feature.");
    }
```

</details>

### <span id="enum.AnyConnection"></span><code style="font-size: 0.85em">enum AnyConnection</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Multi-connection enum that wraps both PostgreSQL and SQLite connections.

This enum enables runtime database backend selection using Diesel's
`MultiConnection` derive macro. The actual connection type is determined
at runtime based on the connection URL.

#### Variants

- **`Postgres`** - PostgreSQL connection variant
- **`Sqlite`** - SQLite connection variant

### <span id="enum.AnyPool"></span><code style="font-size: 0.85em">enum AnyPool</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Pool enum that wraps both PostgreSQL and SQLite connection pools.

This enum enables runtime pool selection based on the detected backend.

#### Variants

- **`Postgres`** - PostgreSQL connection pool
- **`Sqlite`** - SQLite connection pool

#### Methods

##### <span id="AnyPool.as_postgres"></span><code>as_​postgres</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn as_postgres(&self) -> Option<&PgPool>
```

Returns a reference to the PostgreSQL pool if this is a PostgreSQL backend.

<details>
<summary>Source</summary>

```rust
    pub fn as_postgres(&self) -> Option<&PgPool> {
        match self {
            AnyPool::Postgres(pool) => Some(pool),
            _ => None,
        }
    }
```

</details>

##### <span id="AnyPool.as_sqlite"></span><code>as_​sqlite</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn as_sqlite(&self) -> Option<&SqlitePool>
```

Returns a reference to the SQLite pool if this is a SQLite backend.

<details>
<summary>Source</summary>

```rust
    pub fn as_sqlite(&self) -> Option<&SqlitePool> {
        match self {
            AnyPool::Sqlite(pool) => Some(pool),
            _ => None,
        }
    }
```

</details>

##### <span id="AnyPool.expect_postgres"></span><code>expect_​postgres</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn expect_postgres(&self) -> &PgPool
```

Returns the PostgreSQL pool, panicking if this is not a PostgreSQL backend.

<details>
<summary>Source</summary>

```rust
    pub fn expect_postgres(&self) -> &PgPool {
        match self {
            AnyPool::Postgres(pool) => pool,
            _ => panic!("Expected PostgreSQL pool but got SQLite"),
        }
    }
```

</details>

##### <span id="AnyPool.expect_sqlite"></span><code>expect_​sqlite</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn expect_sqlite(&self) -> &SqlitePool
```

Returns the SQLite pool, panicking if this is not a SQLite backend.

<details>
<summary>Source</summary>

```rust
    pub fn expect_sqlite(&self) -> &SqlitePool {
        match self {
            AnyPool::Sqlite(pool) => pool,
            _ => panic!("Expected SQLite pool but got PostgreSQL"),
        }
    }
```

</details>

##### <span id="AnyPool.close"></span><code>close</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn close(&self)
```

Closes the connection pool, releasing all connections.

After calling this method, all current and future attempts to get
connections from the pool will fail immediately.

<details>
<summary>Source</summary>

```rust
    pub fn close(&self) {
        match self {
            AnyPool::Postgres(pool) => pool.close(),
            AnyPool::Sqlite(pool) => pool.close(),
        }
    }
```

</details>

## Type aliases

| Name | Type | Description |
|------|------|-------------|
| <span id="type.AnyConnection"></span>`AnyConnection` | `PgConnection` | When only PostgreSQL is enabled, AnyConnection is just a PgConnection. |
| <span id="type.AnyConnection"></span>`AnyConnection` | `SqliteConnection` | When only SQLite is enabled, AnyConnection is just a SqliteConnection. |
| <span id="type.AnyPool"></span>`AnyPool` | `PgPool` | When only PostgreSQL is enabled, AnyPool is just a PgPool. |
| <span id="type.AnyPool"></span>`AnyPool` | `SqlitePool` | When only SQLite is enabled, AnyPool is just a SqlitePool. |
| <span id="type.DbConnection"></span>`DbConnection` | `PgConnection` | Type alias for the connection type (defaults to PostgreSQL) |
| <span id="type.DbConnection"></span>`DbConnection` | `SqliteConnection` | Type alias for the connection type (SQLite when postgres not enabled) |
| <span id="type.DbConnectionManager"></span>`DbConnectionManager` | `PgManager` | Type alias for the connection manager (defaults to PostgreSQL) |
| <span id="type.DbPool"></span>`DbPool` | `PgPool` | Type alias for the connection pool (defaults to PostgreSQL) |
| <span id="type.DbPool"></span>`DbPool` | `SqlitePool` | Type alias for the connection pool (SQLite when postgres not enabled) |
