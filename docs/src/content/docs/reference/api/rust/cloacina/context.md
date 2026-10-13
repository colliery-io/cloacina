---
title: "cloacina::context"
description: "API reference of the Rust module cloacina::context."
editUrl: false
sidebar:
  order: 54
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

**Context Management**

The context module provides a type-safe, serializable container for sharing data between tasks
in a workflow. Contexts can be persisted to and restored from a database, enabling robust
checkpoint and recovery capabilities.

**Overview**

The [`Context`](../../../rust/cloacina-workflow/context/#struct.Context) struct is the core data container that flows through your workflow. It provides:
- Type-safe data storage with compile-time guarantees
- JSON serialization for database persistence
- Key-value access patterns with error handling
- Integration with the database layer via [`ContextDbExt`](../../../rust/cloacina/context/#trait.ContextDbExt)

**Key Features**

**Type Safety**

- Generic type parameter `T` must implement `Serialize`, `Deserialize`, and `Debug`
- Compile-time type checking prevents type mismatches
- Type information is preserved during serialization/deserialization

**Error Handling**

- Comprehensive error types via [`ContextError`](../../../rust/cloacina/error/#enum.ContextError)
- Clear error messages for common failure cases
- Proper error propagation for database operations

**Database Integration**

- Seamless conversion between Context and database records via [`ContextDbExt`](../../../rust/cloacina/context/#trait.ContextDbExt)
- Automatic timestamp management
- UUID-based record identification

**Best Practices**

1. **Type Selection**
   - Choose types that implement `Serialize` and `Deserialize`
   - Consider using `serde_json::Value` for maximum flexibility
   - Use concrete types when type safety is critical

2. **Error Handling**
   - Always handle potential errors from context operations
   - Use `?` operator for error propagation
   - Consider using `Result` types in your task implementations

3. **Performance**
   - Cache frequently accessed values
   - Use `clone_data()` for creating lightweight copies

4. **Database Usage**
   - Use `to_new_db_record()` for new records
   - Use `to_db_record()` when you need to specify an ID
   - Consider batching database operations

**Usage Patterns**

**Basic Operations**

```rust
use cloacina::Context;

let mut context = Context::<i32>::new();

// Insert values
context.insert("count", 42)?;

// Retrieve values
let count = context.get("count").unwrap();
assert_eq!(*count, 42);

// Update existing values
context.update("count", 100)?;
```

**Database Integration**

```rust
use cloacina::{Context, ContextDbExt};
use uuid::Uuid;

let mut context = Context::<String>::new();
context.insert("message", "Hello, World!".to_string())?;

// Convert to database record
let db_record = context.to_new_db_record()?;

// Restore from database record
let restored = Context::<String>::from_json(db_record.value)?;
```

## Re-exports

- [`Context`](../../../rust/cloacina-workflow/context/#struct.Context)

## Traits

### <span id="trait.ContextDbExt"></span><code style="font-size: 0.85em">trait ContextDbExt&lt;T&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Extension trait providing database operations for Context.

This trait adds methods for converting contexts to and from database records,
enabling persistent storage of execution state.

**Examples:**

```rust
use cloacina::{Context, ContextDbExt};
use uuid::Uuid;

let mut context = Context::<String>::new();
context.insert("key", "value".to_string()).unwrap();

// Convert to new database record
let new_record = context.to_new_db_record().unwrap();

// Convert to full database record with ID
let full_record = context.to_db_record(Uuid::new_v4()).unwrap();
```

#### Methods

##### <span id="ContextDbExt.from_db_record"></span><code>from_​db_​record</code>

```rust
fn from_db_record(
    db_context: &DbContext,
) -> Result<Context<T>, ContextError>
```

Creates a Context from a database record.

This is a convenience method that combines database record retrieval
with context deserialization.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `db_context` | `&DbContext` | The database context record |

**Returns:**

* `Ok(Context<T>)` - The deserialized context
* `Err(ContextError)` - If deserialization fails

<details>
<summary>Source</summary>

```rust
    fn from_db_record(db_context: &DbContext) -> Result<Context<T>, ContextError>;
```

</details>

##### <span id="ContextDbExt.to_new_db_record"></span><code>to_​new_​db_​record</code>

```rust
fn to_new_db_record(&self) -> Result<NewDbContext, ContextError>
```

Converts this context into a new database record for insertion.

Creates a [`NewDbContext`](../../../rust/cloacina/models/context/#struct.NewDbContext) that can be inserted into the database.
The ID and timestamps will be generated by the database.

**Returns:**

* `Ok(NewDbContext)` - The database record ready for insertion
* `Err(ContextError)` - If serialization fails

<details>
<summary>Source</summary>

```rust
    fn to_new_db_record(&self) -> Result<NewDbContext, ContextError>;
```

</details>

##### <span id="ContextDbExt.to_db_record"></span><code>to_​db_​record</code>

```rust
fn to_db_record(&self, id: Uuid) -> Result<DbContext, ContextError>
```

Converts this context into a complete database record.

Creates a [`DbContext`](../../../rust/cloacina/models/context/#struct.DbContext) with the specified ID and current timestamp.
This is useful when you need to create a complete record with known ID.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `id` | `Uuid` | The UUID for this context record |

**Returns:**

* `Ok(DbContext)` - The complete database record
* `Err(ContextError)` - If serialization fails

<details>
<summary>Source</summary>

```rust
    fn to_db_record(&self, id: Uuid) -> Result<DbContext, ContextError>;
```

</details>
