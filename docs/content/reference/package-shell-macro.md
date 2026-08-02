---
title: "package! Macro Reference"
description: "Reference for the cloacina_workflow_plugin::package!() unified plugin shell macro: where to put it, what it emits, ABI versioning."
weight: 30
aliases:
  - "/platform/reference/package-shell-macro/"

---

# `cloacina_workflow_plugin::package!()` Macro

`cloacina_workflow_plugin::package!()` is the single-line macro that
turns a Rust crate into a fully-formed Cloacina plugin (`.cloacina`
package). It replaces the per-macro `_ffi` emission path used by older
packages — authors no longer hand-stitch the FFI vtable, the
`inventory` walk-and-project step, or the trait-impl boilerplate. The
macro is `#[macro_export]`ed from `cloacina-workflow-plugin`, so invoke
it by that path — `cloacina::package!()` does not resolve.

## Where to put it

At the **crate root**, un-gated:

```rust
// src/lib.rs

mod tasks;
mod triggers;
mod graphs;

cloacina_workflow_plugin::package!();
```

You do **not** add any Cargo wiring for it: there is no
`[lib] crate-type`, no `packaged` Cargo feature to declare, no
`cloacina-macros` direct dependency, and no `build.rs`. The compiler
injects the `cdylib` crate-type and the `packaged` feature (which the
macro's expansion is gated under) when it builds the package, and the
shell routes its runtime companions (async-trait, chrono,
computation-graph) so you hand-add none of them.

See the [migration guide]({{< ref "/service/how-to/migrating-to-service-mode" >}}) for the minimal package shell in context.

## What it emits

The macro emits, gated on `#[cfg(feature = "packaged")]`:

- A `CloacinaPackagePlugin` struct.
- An `impl cloacina_workflow_plugin::CloacinaPlugin for
  CloacinaPackagePlugin` block — all eleven FFI vtable methods (indices
  0–10). Each method walks the cdylib's local
  `inventory::iter::<TaskEntry>` /
  `<TriggerEntry>` / `<ReactorEntry>` /
  `<ComputationGraphEntry>` / `<TriggerlessGraphEntry>` /
  `<WorkflowDescriptorEntry>` / `<ConstructorEntry>` section and
  projects matching entries into the corresponding wire types.
- A `fidius_plugin_registry!` registration so fidius-host can discover
  the plugin at load time.

The host (`cloacina::registry::reconciler`) calls each method by
**positional index** (see the [FFI Vtable Reference]({{< ref "/reference/ffi-vtable" >}})). The
macro never inserts new abstraction layers between the host and the
inventory entries — it is pure projection across the FFI boundary.

## Duplicate-invocation guard

Calling `cloacina_workflow_plugin::package!()` twice in the same crate is a compile
error. The macro emits a sentinel that collides on a duplicate call,
preventing two `CloacinaPackagePlugin` impls from coexisting.

## Why a single shell macro

The unified shell collapses every per-symbol FFI emission into one
expansion site that always matches the canonical `CloacinaPlugin`
trait declaration. For the design rationale, the predecessor model
this replaced, and the trade-offs, see [Inventory and Runtime
Seeding]({{< ref "/engine/explanation/inventory-and-runtime-seeding" >}}).

## Inventory boundary

The shell macro walks the cdylib's own inventory section, not the
host's. The FFI vtable bridges across the shared-library boundary.
See [Inventory and Runtime Seeding]({{< ref "/engine/explanation/inventory-and-runtime-seeding" >}})
for the full mechanism.

## Versioning

The `CloacinaPlugin` trait is versioned — the current interface
version is **5** (`#[fidius::plugin_interface(version = 5)]`).
Methods 4–8 are marked `#[optional(since = 2)]`, method 9
(`get_input_interface`) is `#[optional(since = 3)]`, and method 10
(`get_constructor_metadata`) is `#[optional(since = 4)]`, so plugins
built against an older version of `cloacina-workflow-plugin` load
against newer hosts for those additive bumps; the unsupported methods
return `CallError::NotImplemented`, which the reconciler treats as
"package declares no reactors / triggers / trigger-less graphs /
declared interfaces / constructor nodes."

The **4 → 5** bump is different: `TaskExecutionRequest` gained the
`resolved_secrets` field (CLOACI-T-0895) — a bincode wire-layout
change, so pre-v5 artifacts fail the version gate at load rather than
mis-decode.

The trait's `INTERFACE_HASH` is checked at load time. If it doesn't
match, fidius-host refuses to load the plugin — preventing the
positional-dispatch ABI from drifting silently between host and plugin
builds.

## Related

- [FFI Vtable Reference]({{< ref "/reference/ffi-vtable" >}}) — full method-by-method spec.
- [Inventory and Runtime Seeding]({{< ref "/engine/explanation/inventory-and-runtime-seeding" >}}) — why the cdylib boundary matters.
- [Reconciler Pipeline]({{< ref "/service/explanation/reconciler-pipeline" >}}) — how the host consumes what `package!()` emits.
- [Migrating to Service Mode]({{< ref "/service/how-to/migrating-to-service-mode" >}}) — the minimal package shell in context.
