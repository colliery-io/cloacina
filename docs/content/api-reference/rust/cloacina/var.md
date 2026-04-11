# cloacina::var <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Variable registry for external connections and configuration.

Provides a type-agnostic variable system using the `CLOACINA_VAR_{NAME}`
environment variable convention. Similar to Airflow's connection/variable
system — external connections, secrets, and config values are referenced
by name and resolved from env vars at runtime.

## Structs

### `cloacina::var::VarNotFound`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Error returned when a required variable is not found.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | The variable name (without the `CLOACINA_VAR_` prefix). |



## Functions

### `cloacina::var::var`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn var (name : & str) -> Result < String , VarNotFound >
```

Resolve a variable by name from `CLOACINA_VAR_{NAME}`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `-` | Variable name without the prefix (e.g., `"KAFKA_BROKER"`) |


**Returns:**

The variable value, or `VarNotFound` if not set.

**Examples:**

```rust,ignore
// With CLOACINA_VAR_KAFKA_BROKER=localhost:9092
let broker = cloacina::var("KAFKA_BROKER")?;
assert_eq!(broker, "localhost:9092");
```

<details>
<summary>Source</summary>

```rust
pub fn var(name: &str) -> Result<String, VarNotFound> {
    let env_key = format!("{}{}", PREFIX, name);
    std::env::var(&env_key).map_err(|_| VarNotFound {
        name: name.to_string(),
    })
}
```

</details>



### `cloacina::var::var_or`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn var_or (name : & str , default : & str) -> String
```

Resolve a variable by name, returning a default if not set.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `-` | Variable name without the prefix (e.g., `"MODEL_THRESHOLD"`) |
| `default` | `-` | Default value if the variable is not set |


**Examples:**

```rust,ignore
let threshold = cloacina::var_or("MODEL_THRESHOLD", "0.5");
```

<details>
<summary>Source</summary>

```rust
pub fn var_or(name: &str, default: &str) -> String {
    var(name).unwrap_or_else(|_| default.to_string())
}
```

</details>



### `cloacina::var::resolve_template`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn resolve_template (input : & str) -> Result < String , Vec < VarNotFound > >
```

Resolve template references in a string, replacing `{{ VAR_NAME }}` with the corresponding `CLOACINA_VAR_{VAR_NAME}` value.

Unresolved references (missing env vars) are returned as errors
with the list of missing variable names.

**Examples:**

```rust,ignore
// With CLOACINA_VAR_KAFKA_BROKER=localhost:9092
let resolved = cloacina::var::resolve_template("broker={{ KAFKA_BROKER }}")?;
assert_eq!(resolved, "broker=localhost:9092");
```

<details>
<summary>Source</summary>

```rust
pub fn resolve_template(input: &str) -> Result<String, Vec<VarNotFound>> {
    let mut result = String::with_capacity(input.len());
    let mut missing = Vec::new();
    let mut rest = input;

    while let Some(start) = rest.find("{{") {
        result.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];

        if let Some(end) = after_open.find("}}") {
            let var_name = after_open[..end].trim();
            match var(var_name) {
                Ok(value) => result.push_str(&value),
                Err(e) => {
                    // Keep the original placeholder so caller can see what failed
                    result.push_str(&rest[start..start + 2 + end + 2]);
                    missing.push(e);
                }
            }
            rest = &after_open[end + 2..];
        } else {
            // Unclosed {{ — copy literally
            result.push_str(&rest[start..]);
            rest = "";
        }
    }
    result.push_str(rest);

    if missing.is_empty() {
        Ok(result)
    } else {
        Err(missing)
    }
}
```

</details>
