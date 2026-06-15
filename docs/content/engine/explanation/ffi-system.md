---
title: "FFI System"
description: "C FFI interface for dynamic plugin loading"
weight: 21
reviewer: "dstorey"
review_date: "2025-01-17"
---

This article describes the plugin system Cloacina uses to dynamically load and execute workflow packages. Cloacina uses [fidius](https://github.com/fidius-io/fidius), a framework that transforms a Rust trait into a stable C ABI plugin, eliminating the need for hand-written `extern "C"` functions and `#[repr(C)]` structs.

## Overview

Workflow packages are compiled as `cdylib` shared libraries. At runtime, Cloacina's host loader opens each library and dispatches calls through a single well-known entry point. The fidius framework sits between the host and the plugin, handling:

- **Serialization and deserialization** of method arguments and return values
- **Panic catching** so a panicking plugin cannot crash the host process
- **Buffer management** with automatic allocation on both sides of the boundary
- **ABI validation** to detect version drift before any calls are made

## Plugin Interface

The interface contract is defined in `cloacina-workflow-plugin`, a small crate shared by both the plugin author and the host. It declares the `CloacinaPlugin` trait using the `#[plugin_interface]` attribute from fidius. Post-CLOACI-I-0102 (the unified `cloacina::package!();` shell), the trait exposes **nine methods** (indices 0–8) covering tasks, triggers, reactors, accumulators, and trigger-less computation graphs:

```rust
#[plugin_interface]
pub trait CloacinaPlugin {
    fn get_task_metadata(&self) -> PackageTasksMetadata;        // index 0
    fn execute_task(&self, request: TaskExecutionRequest)
        -> TaskExecutionResult;                                  // index 1
    fn get_trigger_metadata(&self) -> Vec<TriggerMetadata>;     // index 2
    fn invoke_trigger(&self, request: TriggerInvokeRequest)
        -> TriggerInvokeResult;                                  // index 3
    fn get_reactor_metadata(&self) -> Vec<ReactorPackageMetadata>; // index 4 (optional, since v2)
    fn get_accumulator_metadata(&self) -> Vec<AccumulatorPackageMetadata>; // index 5 (optional, since v2)
    fn instantiate_accumulator(&self, request: AccumulatorInstantiateRequest)
        -> AccumulatorInstantiateResult;                         // index 6 (optional, since v2)
    fn get_triggerless_graph_metadata(&self) -> Vec<TriggerlessGraphMetadata>; // index 7 (optional, since v2)
    fn invoke_triggerless_graph(&self, request: TriggerlessGraphInvokeRequest)
        -> TriggerlessGraphInvokeResult;                         // index 8 (optional, since v2)
}
```

Methods 4–8 are marked `optional(since = 2)` — older packages that pre-date CLOACI-I-0102 still load, they just don't expose reactors, accumulators, or trigger-less graphs (the host treats missing methods as "no items of that kind"). New packages built with the unified `cloacina::package!();` shell expose all nine.

This crate is the single source of truth for the interface. Both the plugin and the host depend on exactly this crate, which ensures they agree on method signatures, type layouts, and the ABI hash fidius derives from the trait definition. See [FFI vtable reference]({{< ref "/reference/ffi-vtable" >}}) for the per-method wire types and [package!() macro reference]({{< ref "/reference/package-shell-macro" >}}) for the unified shell that emits all nine methods.

### Shared Types

The types that cross the FFI boundary are plain Rust structs that derive `serde::Serialize` and `serde::Deserialize`:

- **`PackageTasksMetadata`** — package name, task list, dependency graph; returned by `get_task_metadata`
- **`TaskExecutionRequest`** — task name and serialized context; passed to `execute_task`
- **`TaskExecutionResult`** — success/error status and updated context; returned from `execute_task`

Because fidius serializes these types rather than passing raw pointers, there are no `*const c_char` fields or manual `CStr` conversions.

## How Plugins Are Built

Post-CLOACI-I-0102, the `cloacina::package!();` shell macro (invoked once at the crate root of a packaged-workflow cdylib) generates the entire FFI surface in one place. It collects every `#[task]`, `#[trigger]`, `#[reactor]`, `#[accumulator]`, and `#[computation_graph]` declaration from the local crate's `inventory` section and emits:

1. An `impl CloacinaPlugin` block that dispatches all nine vtable methods to the workflow's actual declarations.
2. The fidius registration boilerplate — `#[plugin_impl(CloacinaPlugin)]` on the impl and a `fidius_plugin_registry!()` call that exports the `fidius_get_registry` symbol.

The `package!()` invocation is the *single* FFI entry point per cdylib (replaces the pre-I-0102 per-macro `_ffi` emission paths). See [package!() macro reference]({{< ref "/reference/package-shell-macro" >}}) for the duplicate-invocation guard and the inventory walk it performs.

Plugin authors do not write any of this by hand. The macro output is equivalent to:

```rust
#[plugin_impl(CloacinaPlugin)]
impl CloacinaPlugin for DataProcessingPlugin {
    fn get_task_metadata(&self) -> PackageTasksMetadata {
        // returns statically-known metadata for the workflow
    }

    fn execute_task(&self, request: TaskExecutionRequest) -> TaskExecutionResult {
        // dispatches to the requested task function
    }
}

fidius_plugin_registry!(DataProcessingPlugin);
```

The `fidius_plugin_registry!()` macro exports the single C symbol `fidius_get_registry`, which is the only symbol the host needs to locate.

## Host Loading

The host (cloacinactl and the runtime) loads plugins using `fidius_host::load_library()`:

```rust
let handle = fidius_host::load_library::<dyn CloacinaPlugin>(path)?;
```

Before returning the handle, fidius performs a sequence of validations:

1. **Magic bytes** — confirms the library was built with fidius
2. **ABI version** — checks the fidius framework version matches
3. **Interface hash** — a hash derived from the `CloacinaPlugin` trait definition; if the plugin was compiled against a different version of `cloacina-workflow-plugin`, this check fails immediately
4. **Wire format** — confirms both sides agree on the serialization format

Once loaded, method calls go through `PluginHandle::call_method()`, which serializes arguments, calls across the boundary, deserializes the result, and surfaces any plugin panic as a `Result::Err` rather than unwinding into the host.

## Wire Format

fidius uses different serialization formats depending on the build profile:

- **Debug builds**: JSON — human-readable, easy to inspect in logs
- **Release builds**: bincode — compact and fast

This is automatic and requires no configuration. Both the plugin and host switch format together because they share the same `cloacina-workflow-plugin` crate.

## Safety Guarantees

The fidius approach provides several safety properties that the previous hand-written FFI did not:

- **No raw pointer fields**: all data crosses the boundary as serialized bytes; there are no `*const c_char` pointers for the caller to misuse or fail to free
- **ABI hash drift detection**: a plugin compiled against an older interface crate is rejected at load time rather than silently calling the wrong method
- **Panic isolation**: plugin panics are caught at the boundary and returned as errors; the host process is never unwound by a plugin
- **Automatic buffer sizing**: fidius allocates exactly the right buffer for each call; there is no fixed-size buffer that could truncate large results

## Related Resources

- [Tutorial: Creating Your First Workflow Package]({{< ref "/service/tutorials/07-packaged-workflows" >}})
- [Explanation: Package Format]({{< ref "package-format" >}})
- [Explanation: Packaged Workflow Architecture]({{< ref "packaged-workflow-architecture" >}})
- [Explanation: Inventory and Runtime Seeding]({{< ref "inventory-and-runtime-seeding" >}}) — how the post-I-0096 inventory feeds the `package!()` macro.
- [Reference: FFI vtable]({{< ref "/reference/ffi-vtable" >}}) — per-method wire types and optional-since-v2 semantics.
- [Reference: `package!()` macro]({{< ref "/reference/package-shell-macro" >}}) — the unified shell that emits all nine methods.
