---
title: "Inventory and Runtime Seeding"
description: "Why Cloacina uses the inventory crate to seed its Runtime registries, and why inventory does not cross shared-library boundaries."
weight: 20
aliases:
  - "/platform/explanation/inventory-and-runtime-seeding/"

---

# Inventory and Runtime Seeding

Cloacina maintains five registries on every host process:

- Tasks, indexed by `TaskNamespace`
- Workflows, indexed by name
- Triggers, indexed by name
- Computation graphs (bundled-form + trigger-less), indexed by name
- Reactors, indexed by name

Each registry holds **constructors**, not instances — closures that
the runtime calls to mint a fresh value when work needs to run. This
document explains how those constructors get into the registries.

## The mechanism: `inventory::submit!`

When you write `#[task(...)]`, `#[workflow(...)]`, `#[trigger(...)]`,
`#[reactor(...)]`, or `#[computation_graph(...)]` in embedded mode, the
macro expands to two pieces: (1) the user-facing task/workflow/etc.
implementation, and (2) an `inventory::submit!` block that registers a
**constructor entry** into a per-binary linker section.

For example, `#[task]` expands roughly to:

```rust
cloacina_workflow_plugin::inventory::submit! {
    cloacina_workflow_plugin::TaskEntry {
        namespace: || cloacina::TaskNamespace::new(
            "public", "embedded", "my_workflow", "my_task",
        ),
        constructor: || std::sync::Arc::new(MyTask::new()),
    }
}
```

The [`inventory`](https://docs.rs/inventory) crate uses linker-section
collection: every `submit!` call adds a node to a linked list rooted in
a known section. At runtime, `inventory::iter::<TaskEntry>()` walks the
list and yields every registered entry.

When `cloacina::Runtime::seed_from_inventory()` is called at startup,
it walks the inventory iterators for all five entry types and registers
constructors in the corresponding registries. After seeding, the
runtime knows about every embedded task/workflow/etc. without any
explicit per-symbol registration call.

## Why this replaced `#[ctor]`

In an earlier model, Cloacina used the `ctor` crate to register
symbols. Each macro emitted a `#[ctor::ctor]` function that ran
before `main()` and mutated process-global registries. This worked
but had three sharp problems:

1. **Pre-`main` execution forbids many things.** `#[ctor]` runs
   before tokio is initialized, before logging, before allocator setup.
   Any registration code that needed those subsystems crashed silently
   or deadlocked. One incident in particular — a `#[ctor]` constructor
   blocking on a database init that depended on tokio, which wasn't
   running yet — motivated the eventual flip.

2. **Process-global registries collide across cdylibs.** When two
   independently-built cdylibs both register tasks under the same name
   into a process-global registry, the loser silently overwrites the
   winner. Debugging this requires staring at link order. The
   redesign moved registries onto `Runtime` instances, so each
   `Runtime::new()` produces an isolated set of registries seeded fresh
   from inventory.

3. **Test isolation was impossible.** Tests couldn't run with a
   blank-slate registry; whatever pre-`main` registration ran during
   the test binary's startup leaked into every test. Today,
   `Runtime::empty()` mints a registry with no constructors, and
   `seed_from_inventory()` is opt-in — tests register exactly what
   they need.

The flip kept the macro surface identical from a user's perspective
(the macros still emit registration; users still do nothing manual),
but the mechanism underneath is now `inventory` + per-`Runtime`
seeding instead of `ctor` + process-global static state.

## The cdylib boundary

`inventory` works by writing entries to a known linker section. The
critical property: **each shared library has its own linker section,
and `inventory::iter::<T>()` only walks the section of the binary
that's iterating.**

For embedded code, this is fine: every macro and every iterator are
in the same binary. The binary's inventory section is the universe.

For packaged plugins (`.cloacina` cdylibs), this is a hard boundary.
The plugin's `inventory::submit!` calls populate the **plugin's**
inventory section. The host's `Runtime::seed_from_inventory()` walks
the **host's** inventory section. These are two distinct sections.
A naïve attempt to load a plugin and walk its inventory from the host
finds *nothing*, because the host iterator never sees the plugin's
section.

This is not a bug — it's the C linker model. The Rust `inventory`
crate is faithful to it.

## How packaged plugins work

The FFI vtable is the bridge across the boundary. Each plugin
implements `CloacinaPlugin` (the [`cloacina::package!()`]({{< ref "/reference/package-shell-macro" >}})
macro emits this) and exposes nine methods. The plugin's
implementation **does** see its own inventory section — it walks
`inventory::iter::<TaskEntry>` etc. inside the cdylib and projects
each entry into a wire-format type that fidius-host can serialize.

The host calls those methods at load time:

- Method 0 (`get_task_metadata`) → host registers `DynamicLibraryTask`
  constructors per task.
- Method 4 (`get_reactor_metadata`) → host registers reactor
  constructors that, when called, mint a `ReactorRegistration`.
- ...and so on for triggers, CGs, and trigger-less CGs.

The host's `Runtime` ends up with the same shape of constructors
either way — embedded constructors come from the host's own inventory,
plugin constructors come from the FFI projection. Lookup paths are
unified.

## When this matters in user code

In day-to-day workflow authoring, it doesn't. The macros hide the
mechanism. You should care about it only when:

- **Writing a packaged plugin.** Make sure
  [`cloacina::package!()`]({{< ref "/reference/package-shell-macro" >}})
  is at the crate root and `feature = "packaged"` is set; otherwise
  the cdylib has no FFI vtable and the host has no way to discover
  your tasks.
- **Debugging "where did my task go?".** If embedded-mode registration
  is failing, check that `seed_from_inventory()` runs (it's automatic
  in `Runtime::new()`, but `Runtime::empty()` skips it). If
  packaged-mode registration is failing, check that the plugin's
  `get_task_metadata()` returns the expected entries — that's the FFI
  bridge equivalent.
- **Writing tests that exercise registration.** Use
  `Runtime::empty()` + explicit `register_*` calls for full isolation;
  use `Runtime::new()` if you want the inventory baseline.

## Related

- [`package!()` Macro Reference]({{< ref "/reference/package-shell-macro" >}})
- [FFI Vtable Reference]({{< ref "/reference/ffi-vtable" >}})
- [Reconciler Pipeline]({{< ref "/service/explanation/reconciler-pipeline" >}}) — what the host does after receiving FFI metadata.
