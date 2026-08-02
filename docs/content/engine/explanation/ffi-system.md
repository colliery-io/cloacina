---
title: "FFI System"
description: "C FFI interface for dynamic plugin loading"
weight: 21
reviewer: "dstorey"
review_date: "2025-01-17"
aliases:
  - "/platform/explanation/ffi-system/"

---

This article describes the plugin system Cloacina uses to dynamically load and execute workflow packages. Cloacina uses [fidius](https://crates.io/crates/fidius), a framework that transforms a Rust trait into a stable C ABI plugin, eliminating the need for hand-written `extern "C"` functions and `#[repr(C)]` structs.

## Overview

Workflow packages are compiled as `cdylib` shared libraries. At runtime, Cloacina's host loader opens each library and dispatches calls through a single well-known entry point. The fidius framework sits between the host and the plugin, handling:

- **Serialization and deserialization** of method arguments and return values
- **Panic catching** so a panicking plugin cannot crash the host process
- **Buffer management** with automatic allocation on both sides of the boundary
- **ABI validation** to detect version drift before any calls are made

## Plugin Interface

The interface contract is defined in `cloacina-workflow-plugin`, a small crate shared by both the plugin author and the host. It declares the `CloacinaPlugin` trait using the `#[fidius::plugin_interface]` attribute. The interface is currently **version 5** and exposes **eleven indexed methods** (indices 0–10) covering tasks, computation graphs, reactors, triggers, trigger-less graphs, input interfaces, and constructor nodes (`crates/cloacina-workflow-plugin/src/lib.rs`, `#[fidius::plugin_interface(version = 5, buffer = PluginAllocated)]`):

```rust
#[fidius::plugin_interface(version = 5, buffer = PluginAllocated)]
pub trait CloacinaPlugin: Send + Sync {
    fn get_task_metadata(&self) -> Result<PackageTasksMetadata, PluginError>;          // index 0
    fn execute_task(&self, request: TaskExecutionRequest)
        -> Result<TaskExecutionResult, PluginError>;                                    // index 1
    fn get_graph_metadata(&self) -> Result<GraphPackageMetadata, PluginError>;         // index 2
    fn execute_graph(&self, request: GraphExecutionRequest)
        -> Result<GraphExecutionResult, PluginError>;                                   // index 3
    #[optional(since = 2)]
    fn get_reactor_metadata(&self) -> Result<Vec<ReactorPackageMetadata>, PluginError>; // index 4
    #[optional(since = 2)]
    fn get_trigger_metadata(&self) -> Result<Vec<TriggerPackageMetadata>, PluginError>; // index 5
    #[optional(since = 2)]
    fn invoke_trigger_poll(&self, request: TriggerInvokeRequest)
        -> Result<TriggerInvokeResult, PluginError>;                                    // index 6
    #[optional(since = 2)]
    fn get_triggerless_graph_metadata(&self)
        -> Result<Vec<TriggerlessGraphMetadataEntry>, PluginError>;                     // index 7
    #[optional(since = 2)]
    fn invoke_triggerless_graph(&self, request: TriggerlessGraphInvokeRequest)
        -> Result<TriggerlessGraphInvokeResult, PluginError>;                           // index 8
    #[optional(since = 3)]
    fn get_input_interface(&self) -> Result<InputInterfaceDescriptor, PluginError>;    // index 9
    #[optional(since = 4)]
    fn get_constructor_metadata(&self)
        -> Result<Vec<ConstructorPackageMetadata>, PluginError>;                        // index 10
}
```

Methods 4–8 are `optional(since = 2)`, method 9 is `optional(since = 3)` (declared workflow params, CLOACI-I-0128), and method 10 is `optional(since = 4)` (packaged `constructor!(...)` node declarations, CLOACI-T-0832). "Optional" means the host tolerates `CallError::NotImplemented` from older plugins and treats it as "no items of that kind". The version 4 → 5 bump exists because `TaskExecutionRequest` gained the `resolved_secrets` wire field (CLOACI-T-0895) — a bincode layout change, so stale artifacts fail the version gate at load rather than mis-decoding. New packages built with the unified `cloacina::package!();` shell implement all eleven methods.

This crate is the single source of truth for the interface. Both the plugin and the host depend on exactly this crate, which ensures they agree on method signatures, type layouts, and the ABI hash fidius derives from the trait definition. See [FFI vtable reference]({{< ref "/reference/ffi-vtable" >}}) for the per-method wire types and [package!() macro reference]({{< ref "/reference/package-shell-macro" >}}) for the unified shell that emits all eleven methods.

### Shared Types

The types that cross the FFI boundary are plain Rust structs that derive `serde::Serialize` and `serde::Deserialize`:

- **`PackageTasksMetadata`** — package name, task list, dependency graph; returned by `get_task_metadata`
- **`TaskExecutionRequest`** — task name and serialized context; passed to `execute_task`
- **`TaskExecutionResult`** — success/error status and updated context; returned from `execute_task`

Because fidius serializes these types rather than passing raw pointers, there are no `*const c_char` fields or manual `CStr` conversions.

## How Plugins Are Built

Post-CLOACI-I-0102, the `cloacina::package!();` shell macro (invoked once at the crate root of a packaged-workflow cdylib) generates the entire FFI surface in one place. It collects every `#[task]`, `#[trigger]`, `#[reactor]`, accumulator-macro, and `#[computation_graph]` declaration from the local crate's `inventory` section and emits:

1. An `impl CloacinaPlugin` block that dispatches all eleven vtable methods to the workflow's actual declarations.
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

fidius serializes every call with **bincode**, in both debug and release builds. (An earlier fidius release used JSON in debug builds and bincode in release; that split is gone — the wire is always bincode now.) This is automatic and requires no configuration; the host's load-time validation confirms both sides agree on the wire format before any call is made.

## Safety Guarantees

The fidius approach provides several safety properties that the previous hand-written FFI did not:

- **No raw pointer fields**: all data crosses the boundary as serialized bytes; there are no `*const c_char` pointers for the caller to misuse or fail to free
- **ABI hash drift detection**: a plugin compiled against an older interface crate is rejected at load time rather than silently calling the wrong method
- **Panic isolation**: plugin panics are caught at the boundary and returned as errors; the host process is never unwound by a plugin
- **Automatic buffer sizing**: fidius allocates exactly the right buffer for each call; there is no fixed-size buffer that could truncate large results

## Related Resources

- [Tutorial: Creating Your First Workflow Package]({{< ref "/service/tutorials/03-packaged-workflows" >}})
- [Explanation: Package Format]({{< ref "package-format" >}})
- [Explanation: Packaged Workflow Architecture]({{< ref "packaged-workflow-architecture" >}})
- [Explanation: Inventory and Runtime Seeding]({{< ref "inventory-and-runtime-seeding" >}}) — how the post-I-0096 inventory feeds the `package!()` macro.
- [Reference: FFI vtable]({{< ref "/reference/ffi-vtable" >}}) — per-method wire types and optional-method semantics.
- [Reference: `package!()` macro]({{< ref "/reference/package-shell-macro" >}}) — the unified shell that emits all eleven methods.
