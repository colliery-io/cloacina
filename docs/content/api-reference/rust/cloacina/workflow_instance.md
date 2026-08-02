# cloacina::workflow_instance <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Parameterized workflow instances (CLOACI-I-0116).

A [`WorkflowInstance`] is a *partial*: a clonable, serializable value of
`(workflow name + fully-resolved bound parameters)`. It is NOT a captured
closure — the same workflow may run in-process, inside a dlopen'd package,
or on a remote fleet agent, so what an instance binds must be data that
travels with the run (via `Context`). Defaults are snapshotted at
[`WorkflowInstanceBuilder::build`]; a registered instance is an immutable
snapshot (re-register to adopt new defaults).
Params are delivered as FLAT top-level context keys — the same mapping the
server's validated execute path uses — with the scheduler's reserved keys
(`scheduled_time`, `schedule_id`, `schedule_timezone`,
`schedule_expression`, `trigger_name`, `triggered_at`) always winning on
conflict so a binding can never spoof them.

## Structs

### `cloacina::workflow_instance::WorkflowInstance`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

A fully-resolved, immutable, serializable workflow "partial" (CLOACI-I-0116 decision #1). Build via [`WorkflowInstanceBuilder`].

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `workflow_name` | `String` |  |
| `params` | `serde_json :: Map < String , serde_json :: Value >` | Fully-resolved params: every declared param present (defaults
snapshotted at build), validated against the declared slots. |

#### Methods

##### `builder` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn builder (workflow_name : impl Into < String >) -> WorkflowInstanceBuilder
```

Start building an instance of `workflow_name`.

<details>
<summary>Source</summary>

```rust
    pub fn builder(workflow_name: impl Into<String>) -> WorkflowInstanceBuilder {
        WorkflowInstanceBuilder {
            workflow_name: workflow_name.into(),
            supplied: serde_json::Map::new(),
        }
    }
```

</details>



##### `from_resolved` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_resolved (workflow_name : impl Into < String > , params : serde_json :: Map < String , serde_json :: Value > ,) -> Self
```

Construct from an ALREADY-RESOLVED param map, trusting the caller (dynamic surfaces — e.g. the Python bindings — where the declared slots aren't at hand; the validated path is [`Self::builder`]).

<details>
<summary>Source</summary>

```rust
    pub fn from_resolved(
        workflow_name: impl Into<String>,
        params: serde_json::Map<String, serde_json::Value>,
    ) -> Self {
        Self {
            workflow_name: workflow_name.into(),
            params,
        }
    }
```

</details>



##### `params_json` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn params_json (& self) -> Result < String , WorkflowInstanceError >
```

The instance's params as a `Context`-ready JSON object string (persisted on the schedule row at register time).

<details>
<summary>Source</summary>

```rust
    pub fn params_json(&self) -> Result<String, WorkflowInstanceError> {
        serde_json::to_string(&self.params)
            .map_err(|e| WorkflowInstanceError::Serialization(e.to_string()))
    }
```

</details>





### `cloacina::workflow_instance::WorkflowInstanceBuilder`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Builder: `.param(k, v)` binds values; [`build`](Self::build) validates against the workflow's declared input slots and snapshots defaults.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `workflow_name` | `String` |  |
| `supplied` | `serde_json :: Map < String , serde_json :: Value >` |  |

#### Methods

##### `param` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn param (mut self , name : impl Into < String > , value : impl Serialize ,) -> Result < Self , WorkflowInstanceError >
```

Bind a param value. Values must be serde-serializable data — you can bind a path, a mode, an ID; not a live handle (decision #1).

<details>
<summary>Source</summary>

```rust
    pub fn param(
        mut self,
        name: impl Into<String>,
        value: impl Serialize,
    ) -> Result<Self, WorkflowInstanceError> {
        let name = name.into();
        let value = serde_json::to_value(value)
            .map_err(|e| WorkflowInstanceError::Serialization(e.to_string()))?;
        self.supplied.insert(name, value);
        Ok(self)
    }
```

</details>



##### `build` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn build (self , declared : & [InputSlot]) -> Result < WorkflowInstance , WorkflowInstanceError >
```

Validate against the workflow's declared input slots and produce the fully-resolved instance: - unknown supplied param → error - required (no default) and omitted → error - omitted with default → default snapshotted NOW (decision #3) - param named like a reserved scheduler key → error

<details>
<summary>Source</summary>

```rust
    pub fn build(self, declared: &[InputSlot]) -> Result<WorkflowInstance, WorkflowInstanceError> {
        for name in self.supplied.keys() {
            if !declared.iter().any(|s| &s.name == name) {
                return Err(WorkflowInstanceError::UnknownParam(name.clone()));
            }
            if RESERVED_FIRE_KEYS.contains(&name.as_str()) {
                return Err(WorkflowInstanceError::ReservedParam(name.clone()));
            }
        }

        let mut resolved = serde_json::Map::new();
        for slot in declared {
            if RESERVED_FIRE_KEYS.contains(&slot.name.as_str()) {
                return Err(WorkflowInstanceError::ReservedParam(slot.name.clone()));
            }
            match self.supplied.get(&slot.name) {
                Some(v) => {
                    // CLOACI-T-0859: an encrypted slot must be bound with a
                    // `{"$secret": name}` reference (never a literal value, which
                    // would leak into the plaintext context); a plaintext slot
                    // must NOT be bound with a secret reference.
                    let is_ref = secret_ref_target(v)
                        .map_err(|message| WorkflowInstanceError::MalformedSecretRef {
                            name: slot.name.clone(),
                            message,
                        })?
                        .is_some();
                    if slot.encrypted && !is_ref {
                        return Err(WorkflowInstanceError::SecretRequiresRef(slot.name.clone()));
                    }
                    if !slot.encrypted && is_ref {
                        return Err(WorkflowInstanceError::UnexpectedSecretRef(
                            slot.name.clone(),
                        ));
                    }
                    resolved.insert(slot.name.clone(), v.clone());
                }
                None => match &slot.default {
                    Some(d) => {
                        resolved.insert(slot.name.clone(), d.clone());
                    }
                    None if slot.required => {
                        return Err(WorkflowInstanceError::MissingParam(slot.name.clone()));
                    }
                    None => {}
                },
            }
        }

        Ok(WorkflowInstance {
            workflow_name: self.workflow_name,
            params: resolved,
        })
    }
```

</details>





## Enums

### `cloacina::workflow_instance::WorkflowInstanceError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors from building or using a workflow instance.

#### Variants

- **`UnknownParam`**
- **`MissingParam`**
- **`ReservedParam`**
- **`InvalidParam`**
- **`SecretRequiresRef`**
- **`UnexpectedSecretRef`**
- **`MalformedSecretRef`**
- **`Serialization`**



## Functions

### `cloacina::workflow_instance::secret_ref_target`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn secret_ref_target (value : & serde_json :: Value) -> Result < Option < String > , String >
```

Classify an instance-param value as a secret reference.

Returns:
- `Ok(Some(secret_name))` when `value` is exactly `{"$secret": "<name>"}`
(a single `$secret` key mapping to a non-empty string);
- `Ok(None)` for any value that is not a `$secret` marker object (a plain
param);
- `Err(message)` when the value *looks like* a secret reference but is
malformed (`$secret` present but not a lone non-empty string key) — a clear
error rather than a silent mis-route.

<details>
<summary>Source</summary>

```rust
pub fn secret_ref_target(value: &serde_json::Value) -> Result<Option<String>, String> {
    let serde_json::Value::Object(map) = value else {
        return Ok(None);
    };
    if !map.contains_key(SECRET_REF_MARKER) {
        return Ok(None);
    }
    if map.len() != 1 {
        return Err(format!(
            "a '{}' reference object must contain only the '{}' key",
            SECRET_REF_MARKER, SECRET_REF_MARKER
        ));
    }
    match map.get(SECRET_REF_MARKER) {
        Some(serde_json::Value::String(name)) if !name.is_empty() => Ok(Some(name.clone())),
        _ => Err(format!(
            "'{}' must reference a non-empty secret name string",
            SECRET_REF_MARKER
        )),
    }
}
```

</details>



### `cloacina::workflow_instance::merge_instance_params`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn merge_instance_params (context : & mut crate :: Context < serde_json :: Value > , params_json : & str ,) -> Result < () , String >
```

Merge a schedule row's stored instance params into a fire context as flat top-level keys, SKIPPING the reserved scheduler keys (reserved always wins) and — via `Context::update` semantics on the caller side — letting bound params override a trigger-produced payload (OQ-3). Shared by the cron and trigger fire paths.

<details>
<summary>Source</summary>

```rust
pub fn merge_instance_params(
    context: &mut crate::Context<serde_json::Value>,
    params_json: &str,
) -> Result<(), String> {
    let params: serde_json::Map<String, serde_json::Value> = serde_json::from_str(params_json)
        .map_err(|e| format!("instance params JSON parse: {}", e))?;

    // CLOACI-T-0859: `{"$secret": name}` bindings are routed AWAY from the
    // plaintext context. We accumulate only the non-sensitive
    // `local_binding_name -> secret_name` alias here (NAMES only, never values);
    // the resolved secret value never touches the context (NFR-001). The alias
    // map is stored under the reserved `SECRET_REFS_KEY` so the T-0858 accessor
    // can map a task's declared local name to the concrete secret at fire time.
    let mut secret_refs = serde_json::Map::new();

    for (k, v) in params {
        if RESERVED_FIRE_KEYS.contains(&k.as_str()) {
            continue;
        }
        if k == cloacina_workflow::secret::SECRET_REFS_KEY {
            return Err(format!(
                "instance param '{}' collides with the reserved secret-reference key",
                k
            ));
        }

        match secret_ref_target(&v).map_err(|m| {
            format!(
                "instance param '{}' has a malformed secret reference: {}",
                k, m
            )
        })? {
            // A `$secret` reference: record the alias, keep the value out of the
            // plaintext context entirely.
            Some(secret_name) => {
                secret_refs.insert(k, serde_json::Value::String(secret_name));
            }
            // A plain param: merged as before. Bound params override any
            // same-named key already in the context (e.g. a trigger-produced
            // payload key) — update-or-insert.
            None => {
                if context.update(&k, v.clone()).is_err() {
                    context
                        .insert(k.as_str(), v)
                        .map_err(|e| format!("instance param '{}' insert: {}", k, e))?;
                }
            }
        }
    }

    if !secret_refs.is_empty() {
        let alias_map = serde_json::Value::Object(secret_refs);
        let key = cloacina_workflow::secret::SECRET_REFS_KEY;
        if context.update(key, alias_map.clone()).is_err() {
            context
                .insert(key, alias_map)
                .map_err(|e| format!("secret-reference map insert: {}", e))?;
        }
    }

    Ok(())
}
```

</details>
