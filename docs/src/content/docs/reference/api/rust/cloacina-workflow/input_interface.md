---
title: "cloacina-workflow::input_interface"
description: "Injectable input interface — JSON Schema generation (CLOACI-I-0128)."
editUrl: false
sidebar:
  order: 33
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Injectable input interface — JSON Schema generation (CLOACI-I-0128).

Canonical home for turning a Rust type into a JSON Schema descriptor and for
the `InputSlot` contract. Lives in `cloacina-workflow` (not core `cloacina`)
because the `#[workflow(params(...))]` macro emits calls to these helpers into
**packaged cdylibs**, which depend on `cloacina-workflow`, not core. Core
`cloacina` re-exports this module.

Spec: [CLOACI-S-0013]; descriptor decision: [CLOACI-A-0007] (JSON Schema via
`schemars`).

## Re-exports

- `InputSlot` (`cloacina_api_types::InputSlot`)

## Structs

### <span id="struct.SchemaProbe"></span><code style="font-size: 0.85em">struct SchemaProbe&lt;T&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Opt-in schema derivation for accumulator/reactor boundary types (CLOACI-I-0128 Task D).

Unlike workflow params (whose types we control via `#[workflow(params(...))]`),
computation-graph **boundary types** are defined in the author's crate and may
or may not derive `schemars::JsonSchema`. We do NOT want to force the derive
— so the `#[computation_graph]` macro can't unconditionally call
[`schema_for`](../../../rust/cloacina-workflow/input_interface/#fn.schema_for) (that needs the bound at compile time).

Instead the macro emits a probe over [`SchemaProbe<T>`] that, via **autoref
specialization** (the stable-Rust dtolnay pattern), resolves to the real
schema when `T: JsonSchema` and to a permissive `{}` ("any") schema otherwise.
Authors opt a boundary type into rich typing simply by adding
`#[derive(schemars::JsonSchema)]`; types without it degrade to name-only slots
rather than failing to compile.

**Usage (what the macro emits)**

```rust
{
    use ::cloacina_workflow::input_interface::{ProbeTyped as _, ProbeFallback as _};
    (&::cloacina_workflow::input_interface::SchemaProbe::<MyType>::new())
        .probe_input_schema()
}
```

#### Methods

##### <span id="SchemaProbe.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new() -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Self {
        SchemaProbe(core::marker::PhantomData)
    }
```

</details>

## Traits

### <span id="trait.ProbeTyped"></span><code style="font-size: 0.85em">trait ProbeTyped</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Specific arm of the probe: selected when `T: JsonSchema` (zero autorefs on a `&SchemaProbe<T>` receiver), yielding the real derived schema.

#### Methods

##### <span id="ProbeTyped.probe_input_schema"></span><code>probe_​input_​schema</code>

```rust
fn probe_input_schema(&self) -> serde_json::Value
```

<details>
<summary>Source</summary>

```rust
    fn probe_input_schema(&self) -> serde_json::Value;
```

</details>

### <span id="trait.ProbeFallback"></span><code style="font-size: 0.85em">trait ProbeFallback</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Fallback arm of the probe: selected for any `T` (one autoref on a `&SchemaProbe<T>` receiver), yielding a permissive "any" schema. Method resolution prefers [`ProbeTyped`](../../../rust/cloacina-workflow/input_interface/#trait.ProbeTyped) whenever its `T: JsonSchema` bound holds.

#### Methods

##### <span id="ProbeFallback.probe_input_schema"></span><code>probe_​input_​schema</code>

```rust
fn probe_input_schema(&self) -> serde_json::Value
```

<details>
<summary>Source</summary>

```rust
    fn probe_input_schema(&self) -> serde_json::Value;
```

</details>

## Functions

### <span id="fn.schema_for"></span><code style="font-size: 0.85em">fn schema_​for</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn schema_for<T: schemars::JsonSchema>() -> serde_json::Value
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
    let root = schemars::SchemaGenerator::default().into_root_schema_for::<T>();
    serde_json::to_value(root).unwrap_or(serde_json::Value::Null)
}
```

</details>

### <span id="fn.default_json"></span><code style="font-size: 0.85em">fn default_​json</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn default_json<T: serde::Serialize>(
    value: T,
) -> Option<serde_json::Value>
```

Serialize a default value to `serde_json::Value` for an `InputSlot::default`. Returns `None` if the value can't serialize.

<details>
<summary>Source</summary>

```rust
pub fn default_json<T: serde::Serialize>(value: T) -> Option<serde_json::Value> {
    serde_json::to_value(value).ok()
}
```

</details>

### <span id="fn.slots_to_json"></span><code style="font-size: 0.85em">fn slots_​to_​json</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn slots_to_json(slots: &[InputSlot]) -> String
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
