---
title: "package! Macro Reference"
description: "Reference for the cloacina::package!() unified plugin shell macro emitted by I-0102."
weight: 30
---

# `cloacina::package!()` Macro

`cloacina::package!()` is the single-line macro that turns a Rust crate
into a fully-formed Cloacina plugin (`.cloacina` package). It replaces
the per-macro `_ffi` emission path that pre-I-0102 packages used —
authors no longer hand-stitch the FFI vtable, the `inventory`
walk-and-project step, or the trait-impl boilerplate.

## Where to put it

At the **crate root** of a packaged cdylib, gated on `feature = "packaged"`:

```rust
// src/lib.rs

mod tasks;
mod triggers;
mod graphs;

#[cfg(feature = "packaged")]
cloacina::package!();
```

The crate must:

1. Declare `crate-type = ["cdylib", "rlib"]` in `Cargo.toml`.
2. Declare a `packaged` Cargo feature.
3. Depend on `cloacina-workflow`, `cloacina-macros`, and
   `cloacina-workflow-plugin` (as `default-features = false` is
   appropriate for slim cdylib builds).
4. Carry a `cloacina_build::configure()` call in `build.rs` so the
   linker emits the right symbols for fidius-host to discover.

See the [migration guide]({{< ref "/workflows/how-to-guides/migrating-to-service-mode" >}}) for the full Cargo.toml wiring.

## What it emits

The macro emits, gated on `#[cfg(feature = "packaged")]`:

- A `CloacinaPackagePlugin` struct.
- An `impl cloacina_workflow_plugin::CloacinaPlugin for
  CloacinaPackagePlugin` block — all nine FFI vtable methods (indices
  0–8). Each method walks the cdylib's local
  `inventory::iter::<TaskEntry>` /
  `<TriggerEntry>` / `<ReactorEntry>` /
  `<ComputationGraphEntry>` / `<TriggerlessGraphEntry>` section and
  projects matching entries into the corresponding wire types.
- A `fidius_plugin_registry!` registration so fidius-host can discover
  the plugin at load time.

The host (`cloacina::registry::reconciler`) calls each method by
**positional index** (see the [FFI Vtable Reference]({{< ref "/platform/reference/ffi-vtable" >}})). The
macro never inserts new abstraction layers between the host and the
inventory entries — it is pure projection across the FFI boundary.

## Duplicate-invocation guard

Calling `cloacina::package!()` twice in the same crate is a compile
error. The macro emits a sentinel that collides on a duplicate call,
preventing two `CloacinaPackagePlugin` impls from coexisting.

## Why a single shell macro

Pre-I-0102, every per-symbol macro (`#[task]`, `#[trigger]`,
`#[reactor]`, `#[computation_graph]`) emitted its own `_ffi` block.
This produced O(N) FFI surface per crate, made it hard to add new
methods to the trait without touching every macro, and let the host
and plugin disagree about the vtable shape if a developer forgot to
update one of the macros. The unified shell collapses everything into
one expansion site that always matches the canonical
`CloacinaPlugin` trait declaration in
`cloacina-workflow-plugin/src/lib.rs`. Adding a method now means
adding it to the trait, the canonical method-index constant, and the
shell macro — three sites in two files, all reviewed together.

## Inventory boundary

The shell macro walks the **cdylib's own** inventory section, not the
host's. This is deliberate: `inventory` entries do not span shared-
library boundaries, so a host-side `Runtime::seed_from_inventory()` will
*not* discover plugin entries. The FFI vtable is the bridge that lets
the host enumerate plugin entries indirectly. See [Inventory and
Runtime Seeding]({{< ref "/platform/explanation/inventory-and-runtime-seeding" >}})
for the rationale.

## Versioning

The `CloacinaPlugin` trait is versioned. Methods 4–8 are marked
`#[optional(since = 2)]`, so plugins built against an older version of
`cloacina-workflow-plugin` compile and load against newer hosts; the
unsupported methods return `CallError::NotImplemented`, which the
reconciler treats as "package declares no reactors / triggers /
trigger-less graphs."

The trait's `INTERFACE_HASH` is checked at load time. If it doesn't
match, fidius-host refuses to load the plugin — preventing the
positional-dispatch ABI from drifting silently between host and plugin
builds.

## Related

- [FFI Vtable Reference]({{< ref "/platform/reference/ffi-vtable" >}}) — full method-by-method spec.
- [Inventory and Runtime Seeding]({{< ref "/platform/explanation/inventory-and-runtime-seeding" >}}) — why the cdylib boundary matters.
- [Reconciler Pipeline]({{< ref "/platform/explanation/reconciler-pipeline" >}}) — how the host consumes what `package!()` emits.
- [Migrating to Service Mode]({{< ref "/workflows/how-to-guides/migrating-to-service-mode" >}}) — full Cargo.toml setup.
