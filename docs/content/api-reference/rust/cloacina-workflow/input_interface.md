# cloacina-workflow::input_interface <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Injectable input interface — JSON Schema generation (CLOACI-I-0128).

Canonical home for turning a Rust type into a JSON Schema descriptor and for
the [`InputSlot`] contract. Lives in `cloacina-workflow` (not core `cloacina`)
because the `#[workflow(params(...))]` macro emits calls to these helpers into
**packaged cdylibs**, which depend on `cloacina-workflow`, not core. Core
`cloacina` re-exports this module.
Spec: [CLOACI-S-0013]; descriptor decision: [CLOACI-A-0007] (JSON Schema via
`schemars`).

## Structs

### `cloacina-workflow::input_interface::SchemaProbe`<T>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Opt-in schema derivation for accumulator/reactor boundary types (CLOACI-I-0128 Task D).

Unlike workflow params (whose types we control via `#[workflow(params(...))]`),
computation-graph **boundary types** are defined in the author's crate and may
or may not derive [`schemars::JsonSchema`]. We do NOT want to force the derive
— so the `#[computation_graph]` macro can't unconditionally call
[`schema_for`] (that needs the bound at compile time).
Instead the macro emits a probe over [`SchemaProbe<T>`] that, via **autoref
specialization** (the stable-Rust dtolnay pattern), resolves to the real
schema when `T: JsonSchema` and to a permissive `{}` ("any") schema otherwise.
Authors opt a boundary type into rich typing simply by adding
`#[derive(schemars::JsonSchema)]`; types without it degrade to name-only slots
rather than failing to compile.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `0` | `core :: marker :: PhantomData < T >` |  |



## Functions

### `cloacina-workflow::input_interface::schema_for`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn schema_for < T : schemars :: JsonSchema > () -> serde_json :: Value
```

Generate a JSON Schema for `T` as a `serde_json::Value`.

The macro-generated `#[workflow(params(...))]` descriptor calls this per
declared param type; accumulator/reactor boundary derivation (Task D) calls
it per boundary type. Returns `Value::Null` only if the generated schema
fails to serialize (not expected for a well-formed `JsonSchema` impl).

<details>
<summary>Source</summary>

```rust
pub fn schema_for<T: schemars::JsonSchema>() -> serde_json::Value {
    let root = schemars::gen::SchemaGenerator::default().into_root_schema_for::<T>();
    serde_json::to_value(root).unwrap_or(serde_json::Value::Null)
}
```

</details>



### `cloacina-workflow::input_interface::default_json`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn default_json < T : serde :: Serialize > (value : T) -> Option < serde_json :: Value >
```

Serialize a default value to `serde_json::Value` for an [`InputSlot::default`]. Returns `None` if the value can't serialize.

<details>
<summary>Source</summary>

```rust
pub fn default_json<T: serde::Serialize>(value: T) -> Option<serde_json::Value> {
    serde_json::to_value(value).ok()
}
```

</details>



### `cloacina-workflow::input_interface::slots_to_json`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn slots_to_json (slots : & [InputSlot]) -> String
```

Serialize a slot list to the JSON array string carried across the FFI descriptor entrypoint (`InputInterfaceEntry::slots_json`). Falls back to an empty array on serialization failure.

<details>
<summary>Source</summary>

```rust
pub fn slots_to_json(slots: &[InputSlot]) -> String {
    serde_json::to_string(slots).unwrap_or_else(|_| "[]".to_string())
}
```

</details>
