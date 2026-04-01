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

The interface contract is defined in `cloacina-workflow-plugin`, a small crate shared by both the plugin author and the host. It declares the `CloacinaPlugin` trait using the `#[plugin_interface]` attribute from fidius:

```rust
#[plugin_interface]
pub trait CloacinaPlugin {
    fn get_task_metadata(&self) -> PackageTasksMetadata;
    fn execute_task(&self, request: TaskExecutionRequest) -> TaskExecutionResult;
}
```

This crate is the single source of truth for the interface. Both the plugin and the host depend on exactly this crate, which ensures they agree on method signatures, type layouts, and the ABI hash fidius derives from the trait definition.

### Shared Types

The types that cross the FFI boundary are plain Rust structs that derive `serde::Serialize` and `serde::Deserialize`:

- **`PackageTasksMetadata`** — package name, task list, dependency graph; returned by `get_task_metadata`
- **`TaskExecutionRequest`** — task name and serialized context; passed to `execute_task`
- **`TaskExecutionResult`** — success/error status and updated context; returned from `execute_task`

Because fidius serializes these types rather than passing raw pointers, there are no `*const c_char` fields or manual `CStr` conversions.

## How Plugins Are Built

The `#[workflow]` macro, when building for a `cdylib` target, generates two things:

1. An `impl CloacinaPlugin` block that dispatches `get_task_metadata` and `execute_task` to the workflow's actual task functions.
2. The fidius registration boilerplate — `#[plugin_impl(CloacinaPlugin)]` on the impl and a `fidius_plugin_registry!()` call that exports the `fidius_get_registry` symbol.

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

The host (cloacina-ctl and the runtime) loads plugins using `fidius_host::load_library()`:

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

- [Tutorial: Creating Your First Workflow Package]({{< ref "/tutorials/07-packaged-workflows/" >}})
- [Explanation: Package Format]({{< ref "/explanation/package-format/" >}})
- [Explanation: Packaged Workflow Architecture]({{< ref "/explanation/packaged-workflow-architecture/" >}})
