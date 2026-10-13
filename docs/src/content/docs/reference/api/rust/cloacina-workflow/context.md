---
title: "cloacina-workflow::context"
description: "API reference of the Rust module cloacina-workflow::context."
editUrl: false
sidebar:
  order: 30
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

**Minimal Context for Workflow Authoring**

This module provides a minimal `Context` type for sharing data between tasks.
It contains only the core data operations without runtime-specific features
like database persistence or dependency loading.

## Structs

### <span id="struct.Context"></span><code style="font-size: 0.85em">struct Context&lt;T = serde_​json::Value&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

A context that holds data for pipeline execution.

The context is a type-safe, serializable container that flows through your pipeline,
allowing tasks to share data. It supports JSON serialization and provides key-value
access patterns with comprehensive error handling.

**Type Parameter**

- `T`: The type of values stored in the context. Must implement `Serialize`, `Deserialize`, and `Debug`.

**Examples:**

```rust
use cloacina_workflow::Context;
use serde_json::Value;

// Create a context for JSON values
let mut context = Context::<Value>::new();

// Insert and retrieve data
context.insert("user_id", serde_json::json!(123)).unwrap();
let user_id = context.get("user_id").unwrap();
```

#### Methods

##### <span id="Context.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new() -> Self
```

Creates a new empty context.

**Examples:**

```rust
use cloacina_workflow::Context;

let context = Context::<i32>::new();
assert!(context.get("any_key").is_none());
```

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Self {
        debug!("Creating new empty context");
        Self {
            data: HashMap::new(),
            secrets: None,
        }
    }
```

</details>

##### <span id="Context.clone_data"></span><code>clone_​data</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn clone_data(&self) -> Self
where
    T: Clone,
```

Creates a clone of this context's data.

**Performance**

- Time complexity: O(n) where n is the number of key-value pairs
- Space complexity: O(n) for the cloned data

<details>
<summary>Source</summary>

```rust
    pub fn clone_data(&self) -> Self
    where
        T: Clone,
    {
        debug!("Cloning context data");
        Self {
            data: self.data.clone(),
            // Carry the resolver handle (cheap Arc clone) so a cloned execution
            // scope can still resolve secrets.
            secrets: self.secrets.clone(),
        }
    }
```

</details>

##### <span id="Context.insert"></span><code>insert</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn insert(
    &mut self) -> Result<(,
    key: impl Into<String>) -> Result<(,
    value: T) -> Result<(,
) -> Result<(), ContextError>
```

Inserts a value into the context.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `impl Into<String>` | The key to insert (can be any type that converts to String) |
| `value` | `T` | The value to store |

**Returns:**

* `Ok(())` - If the insertion was successful
* `Err(ContextError::KeyExists)` - If the key already exists

**Examples:**

```rust
use cloacina_workflow::{Context, ContextError};

let mut context = Context::<i32>::new();

// First insertion succeeds
assert!(context.insert("count", 42).is_ok());

// Duplicate insertion fails
assert!(matches!(context.insert("count", 43), Err(ContextError::KeyExists(_))));
```

<details>
<summary>Source</summary>

```rust
    pub fn insert(&mut self, key: impl Into<String>, value: T) -> Result<(), ContextError> {
        let key = key.into();
        if self.data.contains_key(&key) {
            warn!("Attempted to insert duplicate key: {}", key);
            return Err(ContextError::KeyExists(key));
        }
        debug!("Inserting value for key: {}", key);
        self.data.insert(key, value);
        Ok(())
    }
```

</details>

##### <span id="Context.update"></span><code>update</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn update(
    &mut self) -> Result<(,
    key: impl Into<String>) -> Result<(,
    value: T) -> Result<(,
) -> Result<(), ContextError>
```

Updates an existing value in the context.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `impl Into<String>` | The key to update |
| `value` | `T` | The new value |

**Returns:**

* `Ok(())` - If the update was successful
* `Err(ContextError::KeyNotFound)` - If the key doesn't exist

**Examples:**

```rust
use cloacina_workflow::{Context, ContextError};

let mut context = Context::<i32>::new();
context.insert("count", 42).unwrap();

// Update existing key
assert!(context.update("count", 100).is_ok());
assert_eq!(context.get("count"), Some(&100));

// Update non-existent key fails
assert!(matches!(context.update("missing", 1), Err(ContextError::KeyNotFound(_))));
```

<details>
<summary>Source</summary>

```rust
    pub fn update(&mut self, key: impl Into<String>, value: T) -> Result<(), ContextError> {
        let key = key.into();
        if !self.data.contains_key(&key) {
            warn!("Attempted to update non-existent key: {}", key);
            return Err(ContextError::KeyNotFound(key));
        }
        debug!("Updating value for key: {}", key);
        self.data.insert(key, value);
        Ok(())
    }
```

</details>

##### <span id="Context.get"></span><code>get</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get(&self, key: &str) -> Option<&T>
```

Gets a reference to a value from the context.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `&str` | The key to look up |

**Returns:**

* `Some(&T)` - If the key exists
* `None` - If the key doesn't exist

**Examples:**

```rust
use cloacina_workflow::Context;

let mut context = Context::<String>::new();
context.insert("message", "Hello".to_string()).unwrap();

assert_eq!(context.get("message"), Some(&"Hello".to_string()));
assert_eq!(context.get("missing"), None);
```

<details>
<summary>Source</summary>

```rust
    pub fn get(&self, key: &str) -> Option<&T> {
        debug!("Getting value for key: {}", key);
        self.data.get(key)
    }
```

</details>

##### <span id="Context.remove"></span><code>remove</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn remove(&mut self, key: &str) -> Option<T>
```

Removes and returns a value from the context.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `&str` | The key to remove |

**Returns:**

* `Some(T)` - If the key existed and was removed
* `None` - If the key didn't exist

**Examples:**

```rust
use cloacina_workflow::Context;

let mut context = Context::<i32>::new();
context.insert("temp", 42).unwrap();

assert_eq!(context.remove("temp"), Some(42));
assert_eq!(context.get("temp"), None);
assert_eq!(context.remove("missing"), None);
```

<details>
<summary>Source</summary>

```rust
    pub fn remove(&mut self, key: &str) -> Option<T> {
        debug!("Removing value for key: {}", key);
        self.data.remove(key)
    }
```

</details>

##### <span id="Context.data"></span><code>data</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn data(&self) -> &HashMap<String, T>
```

Gets a reference to the underlying data HashMap.

This method provides direct access to the internal data structure
for advanced use cases that need to iterate over all key-value pairs.

**Returns:**

A reference to the HashMap containing all context data

**Examples:**

```rust
use cloacina_workflow::Context;

let mut context = Context::<i32>::new();
context.insert("a", 1).unwrap();
context.insert("b", 2).unwrap();

for (key, value) in context.data() {
    println!("{}: {}", key, value);
}
```

<details>
<summary>Source</summary>

```rust
    pub fn data(&self) -> &HashMap<String, T> {
        &self.data
    }
```

</details>

##### <span id="Context.into_data"></span><code>into_​data</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn into_data(self) -> HashMap<String, T>
```

Consumes the context and returns the underlying data HashMap.

**Returns:**

The HashMap containing all context data

<details>
<summary>Source</summary>

```rust
    pub fn into_data(self) -> HashMap<String, T> {
        self.data
    }
```

</details>

##### <span id="Context.from_data"></span><code>from_​data</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_data(data: HashMap<String, T>) -> Self
```

Creates a Context from a HashMap.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `data` | `HashMap<String, T>` | The HashMap to use as context data |

**Returns:**

A new Context with the provided data

<details>
<summary>Source</summary>

```rust
    pub fn from_data(data: HashMap<String, T>) -> Self {
        Self {
            data,
            secrets: None,
        }
    }
```

</details>

##### <span id="Context.to_json"></span><code>to_​json</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn to_json(&self) -> Result<String, ContextError>
```

Serializes the context to a JSON string.

**Returns:**

* `Ok(String)` - The JSON representation of the context
* `Err(ContextError)` - If serialization fails

<details>
<summary>Source</summary>

```rust
    pub fn to_json(&self) -> Result<String, ContextError> {
        debug!("Serializing context to JSON");
        let json = serde_json::to_string(&self.data)?;
        debug!("Context serialized successfully");
        Ok(json)
    }
```

</details>

##### <span id="Context.from_json"></span><code>from_​json</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_json(json: String) -> Result<Self, ContextError>
```

Deserializes a context from a JSON string.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `json` | `String` | The JSON string to deserialize |

**Returns:**

* `Ok(Context<T>)` - The deserialized context
* `Err(ContextError)` - If deserialization fails

<details>
<summary>Source</summary>

```rust
    pub fn from_json(json: String) -> Result<Self, ContextError> {
        debug!("Deserializing context from JSON");
        let data = serde_json::from_str(&json)?;
        debug!("Context deserialized successfully");
        // A deserialized context is a durable snapshot: it never carries a
        // resolver. The runtime re-attaches one at fire time if needed.
        Ok(Self {
            data,
            secrets: None,
        })
    }
```

</details>

##### <span id="Context.with_secret_resolver"></span><code>with_​secret_​resolver</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_secret_resolver(
    mut self,
    resolver: Arc<dyn SecretResolver>,
) -> Self
```

Attach a secret resolver, builder-style.

The resolver is a runtime-only side channel: it is NEVER serialized (see
[`Context::to_json`](../../../rust/cloacina-workflow/context/#Context.to_json), which writes only `data`), which is what keeps a
resolved secret structurally out of the durable context.

<details>
<summary>Source</summary>

```rust
    pub fn with_secret_resolver(mut self, resolver: Arc<dyn SecretResolver>) -> Self {
        self.secrets = Some(resolver);
        self
    }
```

</details>

##### <span id="Context.set_secret_resolver"></span><code>set_​secret_​resolver</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn set_secret_resolver(&mut self, resolver: Arc<dyn SecretResolver>)
```

Attach (or replace) the secret resolver on this scope.

<details>
<summary>Source</summary>

```rust
    pub fn set_secret_resolver(&mut self, resolver: Arc<dyn SecretResolver>) {
        self.secrets = Some(resolver);
    }
```

</details>

##### <span id="Context.has_secret_resolver"></span><code>has_​secret_​resolver</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn has_secret_resolver(&self) -> bool
```

Whether a secret resolver is configured on this execution scope.

<details>
<summary>Source</summary>

```rust
    pub fn has_secret_resolver(&self) -> bool {
        self.secrets.is_some()
    }
```

</details>

##### <span id="Context.secret"></span><code>secret</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn secret(
    &self,
    name: &str,
) -> Result<BTreeMap<String, String>, SecretAccessError>
```

Resolve a named secret into its decrypted `{field: value}` map.

`name` may be either the concrete secret name OR a **declared local
binding name** that an instance mapped to a secret via a
`{"$secret": "..."}` reference (CLOACI-I-0133 / T-0859). When the
context carries a `{"$secret"}` alias map (under
[`secret::SECRET_REFS_KEY`](../../../rust/cloacina-workflow/secret/#const.SECRET_REFS_KEY)) and `name`
appears as a local binding there, the mapped secret name is resolved
instead — so a task can read either the declared name it authored against
or the concrete secret the instance chose.

The returned map is handed to the task and is NEVER inserted into the
context's serialized `data`. Errors clearly when no resolver is
configured ([`SecretAccessError::NotConfigured`](../../../rust/cloacina-workflow/secret/#enum.SecretAccessError)) or the name is absent
([`SecretAccessError::NotFound`](../../../rust/cloacina-workflow/secret/#enum.SecretAccessError)).

<details>
<summary>Source</summary>

```rust
    pub async fn secret(&self, name: &str) -> Result<BTreeMap<String, String>, SecretAccessError> {
        let resolver = self
            .secrets
            .as_ref()
            .ok_or(SecretAccessError::NotConfigured)?;
        let effective = self.resolve_secret_alias(name);
        resolver.resolve(&effective).await.map_err(|e| match e {
            SecretResolverError::NotFound(n) => SecretAccessError::NotFound(n),
            SecretResolverError::NotGranted(n) => SecretAccessError::NotGranted(n),
            SecretResolverError::Backend(m) => SecretAccessError::Backend(m),
        })
    }
```

</details>

##### <span id="Context.secret_field"></span><code>secret_​field</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn secret_field(
    &self,
    name: &str,
    field: &str,
) -> Result<String, SecretAccessError>
```

Resolve one field of a named secret.

Convenience over [`Context::secret`](../../../rust/cloacina-workflow/context/#Context.secret); errors with
[`SecretAccessError::FieldNotFound`](../../../rust/cloacina-workflow/secret/#enum.SecretAccessError) when the secret exists but lacks the
requested field.

<details>
<summary>Source</summary>

```rust
    pub async fn secret_field(&self, name: &str, field: &str) -> Result<String, SecretAccessError> {
        let fields = self.secret(name).await?;
        fields
            .get(field)
            .cloned()
            .ok_or_else(|| SecretAccessError::FieldNotFound {
                secret: name.to_string(),
                field: field.to_string(),
            })
    }
```

</details>

##### <span id="Context.get_as"></span><code>get_​as</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_as<V>(&self, key: &str) -> Result<Option<V>, crate::error::TaskError>
where
    V: serde::de::DeserializeOwned,
```

Get a value by key and deserialize it into `V`.

Returns `Ok(None)` when the key is absent, `Ok(Some(value))` when it is
present and deserializes cleanly, and `Err(TaskError::ValidationFailed)`
when the stored JSON does not match `V` (the message names the key and
target type).

**Examples:**

```rust
use cloacina_workflow::Context;

let mut ctx = Context::new();
ctx.insert("count", serde_json::json!(7)).unwrap();
let n: Option<i64> = ctx.get_as("count").unwrap();
assert_eq!(n, Some(7));
assert_eq!(ctx.get_as::<i64>("missing").unwrap(), None);
```

<details>
<summary>Source</summary>

```rust
    pub fn get_as<V>(&self, key: &str) -> Result<Option<V>, crate::error::TaskError>
    where
        V: serde::de::DeserializeOwned,
    {
        match self.data.get(key) {
            None => Ok(None),
            Some(value) => serde_json::from_value(value.clone())
                .map(Some)
                .map_err(|e| crate::error::TaskError::ValidationFailed {
                    message: format!(
                        "context key '{}' could not be read as {}: {}",
                        key,
                        std::any::type_name::<V>(),
                        e
                    ),
                }),
        }
    }
```

</details>

##### <span id="Context.get_required"></span><code>get_​required</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_required<V>(&self, key: &str) -> Result<V, crate::error::TaskError>
where
    V: serde::de::DeserializeOwned,
```

Get a value by key, deserialize it into `V`, and error if the key is missing.

`Err(TaskError::ValidationFailed)` when the key is absent or the stored
JSON does not match `V`.

**Examples:**

```rust
use cloacina_workflow::Context;

let mut ctx = Context::new();
ctx.insert("name", serde_json::json!("ada")).unwrap();
let name: String = ctx.get_required("name").unwrap();
assert_eq!(name, "ada");
assert!(ctx.get_required::<String>("missing").is_err());
```

<details>
<summary>Source</summary>

```rust
    pub fn get_required<V>(&self, key: &str) -> Result<V, crate::error::TaskError>
    where
        V: serde::de::DeserializeOwned,
    {
        match self.get_as(key)? {
            Some(value) => Ok(value),
            None => Err(crate::error::TaskError::ValidationFailed {
                message: format!(
                    "required context key '{}' is missing (expected {})",
                    key,
                    std::any::type_name::<V>()
                ),
            }),
        }
    }
```

</details>

##### <span id="Context.insert_as"></span><code>insert_​as</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn insert_as<V>(
    &mut self,
    key: impl Into<String>,
    value: V,
) -> Result<(), crate::error::TaskError>
where
    V: serde::Serialize,
```

Serialize a value and write it under `key`, **upserting** (insert or overwrite).

Folds the `serde_json::json!(...)` / `to_value` wrapping — and the
"exists? update : insert" dance — that every context write otherwise
repeats. Upsert semantics mirror Python's `context.set(key, value)`
(unlike the lower-level [`Context::insert`](../../../rust/cloacina-workflow/context/#Context.insert), which errors on an existing
key). Errors with `TaskError::ValidationFailed` only if the value cannot
be serialized.

**Examples:**

```rust
use cloacina_workflow::Context;

let mut ctx = Context::new();
ctx.insert_as("total", 42u32).unwrap();
assert_eq!(ctx.get_as::<u32>("total").unwrap(), Some(42));
// Upserts — overwriting an existing key is fine.
ctx.insert_as("total", 100u32).unwrap();
assert_eq!(ctx.get_as::<u32>("total").unwrap(), Some(100));
```

<details>
<summary>Source</summary>

```rust
    pub fn insert_as<V>(
        &mut self,
        key: impl Into<String>,
        value: V,
    ) -> Result<(), crate::error::TaskError>
    where
        V: serde::Serialize,
    {
        let key = key.into();
        let json =
            serde_json::to_value(value).map_err(|e| crate::error::TaskError::ValidationFailed {
                message: format!("context key '{}' could not be serialized: {}", key, e),
            })?;
        // Upsert: overwrite if present, insert otherwise.
        self.data.insert(key, json);
        Ok(())
    }
```

</details>
