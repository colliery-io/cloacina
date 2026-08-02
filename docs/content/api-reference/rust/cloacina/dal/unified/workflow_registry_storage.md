# cloacina::dal::unified::workflow_registry_storage <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Unified workflow registry storage with runtime backend selection

This module provides binary storage operations that work with both
PostgreSQL and SQLite backends, selecting the appropriate implementation
at runtime based on the database connection type.

## Structs

### `cloacina::dal::unified::workflow_registry_storage::UnifiedRegistryStorage`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Unified registry storage that works with both PostgreSQL and SQLite.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `database` | `Database` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (database : Database) -> Self
```

Creates a new UnifiedRegistryStorage instance.

<details>
<summary>Source</summary>

```rust
    pub fn new(database: Database) -> Self {
        Self { database }
    }
```

</details>



##### `database` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn database (& self) -> & Database
```

Returns a reference to the underlying database.

<details>
<summary>Source</summary>

```rust
    pub fn database(&self) -> &Database {
        &self.database
    }
```

</details>
