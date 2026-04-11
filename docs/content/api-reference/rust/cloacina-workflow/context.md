# cloacina-workflow::context <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>




## Structs

### `cloacina-workflow::context::Context`<T>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`

A context that holds data for pipeline execution.

The context is a type-safe, serializable container that flows through your pipeline,
allowing tasks to share data. It supports JSON serialization and provides key-value
access patterns with comprehensive error handling.

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

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `data` | `HashMap < String , T >` |  |
