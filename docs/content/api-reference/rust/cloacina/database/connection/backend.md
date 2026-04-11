# cloacina::database::connection::backend <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Database backend types and runtime backend selection.

## Enums

### `cloacina::database::connection::backend::BackendType` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Represents the database backend type, detected at runtime from the connection URL.

#### Variants

- **`Postgres`** - PostgreSQL backend
- **`Sqlite`** - SQLite backend



### `cloacina::database::connection::backend::AnyConnection` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Multi-connection enum that wraps both PostgreSQL and SQLite connections.

This enum enables runtime database backend selection using Diesel's
`MultiConnection` derive macro. The actual connection type is determined
at runtime based on the connection URL.

#### Variants

- **`Postgres`** - PostgreSQL connection variant
- **`Sqlite`** - SQLite connection variant



### `cloacina::database::connection::backend::AnyPool` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Pool enum that wraps both PostgreSQL and SQLite connection pools.

This enum enables runtime pool selection based on the detected backend.

#### Variants

- **`Postgres`** - PostgreSQL connection pool
- **`Sqlite`** - SQLite connection pool
