# cloacina::database <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>




## Functions

### `cloacina::database::run_migrations`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn run_migrations (conn : & mut DbConnection) -> Result < () >
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
| `conn` | `-` | Mutable reference to a database connection (PostgreSQL or SQLite) |


**Returns:**

* `Ok(())` - If migrations complete successfully * `Err(_)` - If migration fails

**Examples:**

```rust,ignore
use cloacina::database::{run_migrations, DbConnection};

# fn example(mut conn: DbConnection) -> Result<(), diesel::result::Error> {
run_migrations(&mut conn)?;
# Ok(())
# }
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



### `cloacina::database::run_migrations_postgres`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn run_migrations_postgres (conn : & mut diesel :: pg :: PgConnection) -> Result < () >
```

Runs pending PostgreSQL database migrations.

This function applies any pending migrations to bring the PostgreSQL database
schema up to date with the current version. Use this function in dual-backend
builds where `run_migrations` is not available.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `conn` | `-` | Mutable reference to a PostgreSQL database connection |


**Returns:**

* `Ok(())` - If migrations complete successfully * `Err(_)` - If migration fails

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



### `cloacina::database::run_migrations_sqlite`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn run_migrations_sqlite (conn : & mut diesel :: sqlite :: SqliteConnection) -> Result < () >
```

Runs pending SQLite database migrations.

This function applies any pending migrations to bring the SQLite database
schema up to date with the current version. Use this function in dual-backend
builds where `run_migrations` is not available.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `conn` | `-` | Mutable reference to a SQLite database connection |


**Returns:**

* `Ok(())` - If migrations complete successfully * `Err(_)` - If migration fails

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
