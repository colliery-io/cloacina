---
title: "Variable Registry (var/var_or)"
description: "How to use the CLOACINA_VAR_ environment variable system for external configuration"
weight: 60
---

# Variable Registry

The variable registry resolves configuration values from `CLOACINA_VAR_` environment variables at runtime, keeping secrets and environment-specific settings out of your workflow code.

## Prerequisites

- A working workflow (Rust or Python)

## Convention

All variables use the `CLOACINA_VAR_` prefix:

```bash
export CLOACINA_VAR_KAFKA_BROKER=localhost:9092
export CLOACINA_VAR_ANALYTICS_DB=postgres://user:pass@host/db
export CLOACINA_VAR_API_KEY=abc123
export CLOACINA_VAR_MODEL_THRESHOLD=0.85
```

In your code, you reference the name **without** the prefix.

## Rust API

### Required Variable

```rust
use cloacina::var;

// Reads CLOACINA_VAR_KAFKA_BROKER
let broker = var("KAFKA_BROKER")?;
// Returns Err(VarNotFound) if not set
```

### Optional Variable with Default

```rust
use cloacina::var_or;

// Reads CLOACINA_VAR_MODEL_THRESHOLD, falls back to "0.5"
let threshold: f64 = var_or("MODEL_THRESHOLD", "0.5").parse().unwrap();
```

### Template Resolution

Expand `{{ VAR_NAME }}` placeholders in configuration strings:

```rust
use cloacina::var::resolve_template;

// With CLOACINA_VAR_HOST=db.example.com and CLOACINA_VAR_PORT=5432
let url = resolve_template("postgres://user@{{ HOST }}:{{ PORT }}/mydb")?;
// Result: "postgres://user@db.example.com:5432/mydb"
```

If any referenced variables are missing, `resolve_template` returns `Err(Vec<VarNotFound>)` listing all unresolved names. Unresolved placeholders are preserved literally in the output. Unclosed `{{` without a matching `}}` is copied through unchanged (no error).

## Python API

### Required Variable

```python
import cloaca

# Reads CLOACINA_VAR_KAFKA_BROKER
broker = cloaca.var("KAFKA_BROKER")
# Raises KeyError if not set
```

### Optional Variable with Default

```python
# Reads CLOACINA_VAR_MODEL_THRESHOLD, falls back to "0.5"
threshold = float(cloaca.var_or("MODEL_THRESHOLD", "0.5"))
```

## Use Cases

### Per-Environment Database Connections

```python
@cloaca.task(id="load_data")
def load_data(context):
    db_url = cloaca.var("WAREHOUSE_URL")
    # dev:  CLOACINA_VAR_WAREHOUSE_URL=sqlite:///tmp/dev.db
    # prod: CLOACINA_VAR_WAREHOUSE_URL=postgres://prod-host/warehouse
    conn = connect(db_url)
    conn.execute(...)
    return context
```

### Per-Tenant Credentials

```python
@cloaca.task(id="call_api")
def call_api(context):
    api_key = cloaca.var("PARTNER_API_KEY")
    # Each tenant's daemon sets a different key
    response = requests.get(url, headers={"X-API-Key": api_key})
    context.set("response", response.json())
    return context
```

### Configuration Thresholds

```rust
#[task(id = "evaluate_model")]
async fn evaluate(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let threshold: f64 = var_or("CONFIDENCE_THRESHOLD", "0.8")
        .parse()
        .map_err(|e| TaskError::execution(format!("bad threshold: {e}")))?;

    let score = context.get("score").unwrap().as_f64().unwrap();
    context.insert("passed", serde_json::json!(score >= threshold))?;
    Ok(())
}
```

### Template Strings in Configuration

Configuration strings can reference variables with `{{ VAR_NAME }}` syntax. This is expanded at load time using `resolve_template()` against the host's environment variables:

```rust
let config = resolve_template("broker={{ KAFKA_BROKER }}:{{ KAFKA_PORT }}")?;
// With CLOACINA_VAR_KAFKA_BROKER=localhost and CLOACINA_VAR_KAFKA_PORT=9092
// Result: "broker=localhost:9092"
```

Whitespace inside `{{ }}` is trimmed — `{{  HOST  }}` resolves the same as `{{ HOST }}`. There is no escape syntax; `{{` is always treated as a variable reference.

## Error Handling

### Rust

`var()` returns `Result<String, VarNotFound>`. The error message includes the expected environment variable name:

```
required variable 'KAFKA_BROKER' not set (expected env var CLOACINA_VAR_KAFKA_BROKER)
```

### Python

`cloaca.var()` raises `KeyError` with a message like:

```
KeyError: "CLOACINA_VAR_KAFKA_BROKER not set"
```

Use `cloaca.var_or()` if the value is truly optional.

## See Also

- [Environment Variables Reference]({{< ref "/platform/reference/environment-variables" >}}) — all Cloacina environment variables
- [Per-Tenant Credentials Tutorial]({{< ref "/workflows/tutorials/service/06-multi-tenancy" >}}) — tenant-scoped configuration
- [Packaging Python Workflows]({{< ref "/python/how-to-guides/packaging-python-workflows" >}}) — using variables in packaged workflows
