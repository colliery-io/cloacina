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



##### `store_binary_postgres` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn store_binary_postgres (& self , data : Vec < u8 >) -> Result < String , StorageError >
```

<details>
<summary>Source</summary>

```rust
    async fn store_binary_postgres(&self, data: Vec<u8>) -> Result<String, StorageError> {
        let conn = self.database.get_postgres_connection().await.map_err(|e| {
            StorageError::Backend(format!("Failed to get database connection: {}", e))
        })?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_entry = NewUnifiedWorkflowRegistryEntry {
            id,
            created_at: now,
            data: UniversalBinary::from(data),
        };

        conn.interact(move |conn| {
            diesel::insert_into(workflow_registry::table)
                .values(&new_entry)
                .execute(conn)
        })
        .await
        .map_err(|e| StorageError::Backend(format!("Database interaction error: {}", e)))?
        .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        Ok(id.0.to_string())
    }
```

</details>



##### `store_binary_sqlite` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn store_binary_sqlite (& self , data : Vec < u8 >) -> Result < String , StorageError >
```

<details>
<summary>Source</summary>

```rust
    async fn store_binary_sqlite(&self, data: Vec<u8>) -> Result<String, StorageError> {
        let conn = self
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| StorageError::Backend(format!("Failed to get connection: {}", e)))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_entry = NewUnifiedWorkflowRegistryEntry {
            id,
            created_at: now,
            data: UniversalBinary::from(data),
        };

        conn.interact(move |conn| {
            diesel::insert_into(workflow_registry::table)
                .values(&new_entry)
                .execute(conn)
        })
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?
        .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        Ok(id.0.to_string())
    }
```

</details>



##### `retrieve_binary_postgres` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn retrieve_binary_postgres (& self , id : & str) -> Result < Option < Vec < u8 > > , StorageError >
```

<details>
<summary>Source</summary>

```rust
    async fn retrieve_binary_postgres(&self, id: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let registry_uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;

        let conn = self.database.get_postgres_connection().await.map_err(|e| {
            StorageError::Backend(format!("Failed to get database connection: {}", e))
        })?;

        let registry_id = UniversalUuid(registry_uuid);
        let entry: Option<UnifiedWorkflowRegistryEntry> = conn
            .interact(move |conn| {
                workflow_registry::table
                    .filter(workflow_registry::id.eq(registry_id))
                    .first::<UnifiedWorkflowRegistryEntry>(conn)
                    .optional()
            })
            .await
            .map_err(|e| StorageError::Backend(format!("Database interaction error: {}", e)))?
            .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        Ok(entry.map(|e| e.data.into_inner()))
    }
```

</details>



##### `retrieve_binary_sqlite` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn retrieve_binary_sqlite (& self , id : & str) -> Result < Option < Vec < u8 > > , StorageError >
```

<details>
<summary>Source</summary>

```rust
    async fn retrieve_binary_sqlite(&self, id: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;

        let conn = self
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| StorageError::Backend(format!("Failed to get connection: {}", e)))?;

        let registry_id = UniversalUuid(uuid);
        let result: Result<Option<UnifiedWorkflowRegistryEntry>, diesel::result::Error> = conn
            .interact(move |conn| {
                workflow_registry::table
                    .filter(workflow_registry::id.eq(registry_id))
                    .first::<UnifiedWorkflowRegistryEntry>(conn)
                    .optional()
            })
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        match result {
            Ok(Some(entry)) => Ok(Some(entry.data.into_inner())),
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Backend(format!("Database error: {}", e))),
        }
    }
```

</details>



##### `delete_binary_postgres` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn delete_binary_postgres (& self , id : & str) -> Result < () , StorageError >
```

<details>
<summary>Source</summary>

```rust
    async fn delete_binary_postgres(&self, id: &str) -> Result<(), StorageError> {
        let registry_uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;

        let conn = self.database.get_postgres_connection().await.map_err(|e| {
            StorageError::Backend(format!("Failed to get database connection: {}", e))
        })?;

        let registry_id = UniversalUuid(registry_uuid);
        conn.interact(move |conn| {
            diesel::delete(workflow_registry::table.filter(workflow_registry::id.eq(registry_id)))
                .execute(conn)
        })
        .await
        .map_err(|e| StorageError::Backend(format!("Database interaction error: {}", e)))?
        .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        Ok(())
    }
```

</details>



##### `delete_binary_sqlite` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn delete_binary_sqlite (& self , id : & str) -> Result < () , StorageError >
```

<details>
<summary>Source</summary>

```rust
    async fn delete_binary_sqlite(&self, id: &str) -> Result<(), StorageError> {
        let uuid =
            Uuid::parse_str(id).map_err(|_| StorageError::InvalidId { id: id.to_string() })?;

        let conn = self
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| StorageError::Backend(format!("Failed to get connection: {}", e)))?;

        let registry_id = UniversalUuid(uuid);
        conn.interact(move |conn| {
            diesel::delete(workflow_registry::table.filter(workflow_registry::id.eq(registry_id)))
                .execute(conn)
        })
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?
        .map_err(|e| StorageError::Backend(format!("Database error: {}", e)))?;

        // Idempotent - success even if no rows deleted
        Ok(())
    }
```

</details>
