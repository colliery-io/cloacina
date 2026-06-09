# cloacina::inventory_entries <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Inventory entry types for linker-collected registry seeding.

The macros (`#[task]`, `#[workflow]`, `#[trigger]`, `#[computation_graph]`,
and the stream-backend registration helper) emit
`inventory::submit!` statements of these types instead of `#[ctor]`
constructors. The runtime reads them post-`main()` via `inventory::iter`,
eliminating the initialization-ordering bug that sank I-0095.
Function pointers — not `Box<dyn Fn>` — are used because `inventory` stores
entries in a linker section with `'static` + `Sized` bounds. Zero-capture
closures at the macro call site coerce to `fn` pointers automatically, so
the ergonomics stay identical.
Nothing in this file reads inventory yet. That wiring lands in T-0506
together with the removal of the global static registries.

## Structs

### `cloacina::inventory_entries::WorkflowEntry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Workflow entry emitted by `#[workflow]`. Stays in cloacina because `Workflow` is an engine-only runtime type.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `& 'static str` |  |
| `constructor` | `fn () -> Workflow` |  |



### `cloacina::inventory_entries::StreamBackendEntry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


#### Fields

| Name | Type | Description |
|------|------|-------------|
| `type_name` | `& 'static str` |  |
| `factory` | `StreamBackendFactoryFn` |  |
