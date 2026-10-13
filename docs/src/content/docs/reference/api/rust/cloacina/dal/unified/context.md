---
title: "cloacina::dal::unified::context"
description: "Unified Context DAL with runtime backend selection"
editUrl: false
sidebar:
  order: 71
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Unified Context DAL with runtime backend selection

This module provides CRUD operations for Context entities that work with
both PostgreSQL and SQLite backends, selecting the appropriate implementation
at runtime based on the database connection type.

## Structs

### <span id="struct.ContextDAL"></span><code style="font-size: 0.85em">struct ContextDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Data access layer for context operations with runtime backend selection.

#### Methods

##### <span id="ContextDAL.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: &'a DAL) -> Self
```

Creates a new ContextDAL instance.

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }
```

</details>

##### <span id="ContextDAL.create"></span><code>create</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn create<T>(
    &self,
    context: &Context<T>,
) -> Result<Option<UniversalUuid>, ContextError>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send
        + 'static,
```

Create a new context in the database.

This method serializes the provided context data to JSON and stores it in the database.
Empty contexts (containing only whitespace or empty objects) are skipped.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `context` | `&Context<T>` | The context to be stored in the database |

**Returns:**

* `Result<Option<UniversalUuid>, ContextError>` - The UUID of the created context if successful,
  or None if the context was empty and skipped

<details>
<summary>Source</summary>

```rust
    pub async fn create<T>(
        &self,
        context: &Context<T>,
    ) -> Result<Option<UniversalUuid>, ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send + 'static,
    {
        use super::models::NewUnifiedDbContext;
        use crate::database::schema::unified::contexts;
        use crate::database::universal_types::UniversalTimestamp;

        // Serialize the context data
        let value = context.to_json()?;

        // Skip insertion if context is empty or whitespace-only
        let trimmed_value = value
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>();
        if trimmed_value == "{}" {
            warn!("Skipping insertion of empty context");
            return Ok(None);
        }

        // Generate ID and timestamps
        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_context = NewUnifiedDbContext {
            id,
            value,
            created_at: now,
            updated_at: now,
        };

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(contexts::table)
                .values(&new_context)
                .execute(conn)
        })?;

        Ok(Some(id))
    }
```

</details>

##### <span id="ContextDAL.read"></span><code>read</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn read<T>(&self, id: UniversalUuid) -> Result<Context<T>, ContextError>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send
        + 'static,
```

Read a context from the database.

Retrieves a context by its UUID and deserializes it into the specified type.

<details>
<summary>Source</summary>

```rust
    pub async fn read<T>(&self, id: UniversalUuid) -> Result<Context<T>, ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send + 'static,
    {
        use super::models::UnifiedDbContext;
        use crate::database::schema::unified::contexts;

        let db_context: UnifiedDbContext =
            crate::interact_on_backend!(self.dal, |conn| contexts::table.find(id).first(conn))?;

        Ok(Context::<T>::from_json(db_context.value)?)
    }
```

</details>

##### <span id="ContextDAL.update"></span><code>update</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn update<T>(
    &self,
    id: UniversalUuid,
    context: &Context<T>,
) -> Result<(), ContextError>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send
        + 'static,
```

Update an existing context in the database.

<details>
<summary>Source</summary>

```rust
    pub async fn update<T>(
        &self,
        id: UniversalUuid,
        context: &Context<T>,
    ) -> Result<(), ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send + 'static,
    {
        use crate::database::schema::unified::contexts;
        use crate::database::universal_types::UniversalTimestamp;

        let value = context.to_json()?;
        let now = UniversalTimestamp::now();

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(contexts::table.find(id))
                .set((contexts::value.eq(value), contexts::updated_at.eq(now)))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="ContextDAL.delete"></span><code>delete</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn delete(&self, id: UniversalUuid) -> Result<(), ContextError>
```

Delete a context from the database.

<details>
<summary>Source</summary>

```rust
    pub async fn delete(&self, id: UniversalUuid) -> Result<(), ContextError> {
        use crate::database::schema::unified::contexts;

        crate::interact_on_backend!(self.dal, |conn| diesel::delete(contexts::table.find(id))
            .execute(conn))?;

        Ok(())
    }
```

</details>

##### <span id="ContextDAL.list"></span><code>list</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list<T>(&self, limit: i64, offset: i64) -> Result<Vec<Context<T>>, ContextError>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send
        + 'static,
```

List contexts with pagination.

Retrieves a paginated list of contexts from the database, ordered by creation date
in descending order.

<details>
<summary>Source</summary>

```rust
    pub async fn list<T>(&self, limit: i64, offset: i64) -> Result<Vec<Context<T>>, ContextError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send + 'static,
    {
        use super::models::UnifiedDbContext;
        use crate::database::schema::unified::contexts;

        let db_contexts: Vec<UnifiedDbContext> = crate::interact_on_backend!(self.dal, |conn| {
            contexts::table
                .limit(limit)
                .offset(offset)
                .order(contexts::created_at.desc())
                .load(conn)
        })?;

        let mut results = Vec::new();
        for db_context in db_contexts {
            let context = Context::<T>::from_json(db_context.value)?;
            results.push(context);
        }

        Ok(results)
    }
```

</details>
