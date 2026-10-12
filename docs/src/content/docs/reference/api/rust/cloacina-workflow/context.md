---
title: "cloacina-workflow::context"
description: "API reference of the Rust module cloacina-workflow::context."
sidebar:
  order: 46
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>




## Structs

### <span id="struct.Context"></span>`struct Context<T>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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
| `secrets` | `Option < Arc < dyn SecretResolver > >` | Secret resolution side channel (CLOACI-I-0133 / T-0858, design D-1).

A runtime-only handle used by [`Context::secret`]. It is **never**
serialized: [`Context::to_json`] writes only `data`, and this field has
no `Serialize`/`Deserialize` — the moral equivalent of `#[serde(skip)]`.
That structural exclusion is exactly what keeps a resolved secret out of
the durable context / `schedules.params` / fires log (NFR-001). It is
likewise redacted from [`Debug`] below so it cannot leak through logs. |



