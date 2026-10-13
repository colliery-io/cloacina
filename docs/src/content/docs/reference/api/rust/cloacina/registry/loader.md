---
title: "cloacina::registry::loader"
description: "Package loader module for workflow registry."
editUrl: false
sidebar:
  order: 157
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Package loader module for workflow registry.

This module provides functionality to load workflow packages (.so files),
extract metadata, validate package integrity, and register tasks with the
global task registry.

## Re-exports

- [`PackageLoader`](../../../../rust/cloacina/registry/loader/package_loader/#struct.PackageLoader)
- [`TaskRegistrar`](../../../../rust/cloacina/registry/loader/task_registrar/#struct.TaskRegistrar)
- [`ConstructorBinding`](../../../../rust/cloacina/registry/loader/constructor_loader/#enum.ConstructorBinding)
- [`ConstructorNode`](../../../../rust/cloacina/registry/loader/constructor_loader/#struct.ConstructorNode)
- [`DEFAULT_PROVIDER_DIR`](../../../../rust/cloacina/registry/loader/constructor_loader/#const.DEFAULT_PROVIDER_DIR)
- [`PROVIDER_PATH_ENV`](../../../../rust/cloacina/registry/loader/constructor_loader/#const.PROVIDER_PATH_ENV)
- [`ProviderScope`](../../../../rust/cloacina/registry/loader/constructor_loader/#enum.ProviderScope)
- [`ScopedProviderSearch`](../../../../rust/cloacina/registry/loader/constructor_loader/#struct.ScopedProviderSearch)
- [`TriggerBinding`](../../../../rust/cloacina/registry/loader/constructor_loader/#struct.TriggerBinding)
- [`WasmTaskConstructor`](../../../../rust/cloacina/registry/loader/constructor_loader/#struct.WasmTaskConstructor)
- [`WasmTriggerConstructor`](../../../../rust/cloacina/registry/loader/constructor_loader/#struct.WasmTriggerConstructor)
- [`clear_provider_search_path`](../../../../rust/cloacina/registry/loader/constructor_loader/#fn.clear_provider_search_path)
- [`current_provider_scope`](../../../../rust/cloacina/registry/loader/constructor_loader/#fn.current_provider_scope)
- [`load_constructor`](../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_constructor)
- [`load_constructor_node`](../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_constructor_node)
- [`load_constructor_node_in`](../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_constructor_node_in)
- [`load_constructor_node_pinned`](../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_constructor_node_pinned)
- [`load_constructor_node_pinned_in`](../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_constructor_node_pinned_in)
- [`load_reactor_constructor_node`](../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_reactor_constructor_node)
- [`load_reactor_constructor_node_pinned`](../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_reactor_constructor_node_pinned)
- [`load_reactor_constructor_node_pinned_in`](../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_reactor_constructor_node_pinned_in)
- [`load_task_constructor`](../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_task_constructor)
- [`load_task_constructor_from_package`](../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_task_constructor_from_package)
- [`load_trigger_constructor`](../../../../rust/cloacina/registry/loader/constructor_loader/#fn.load_trigger_constructor)
- [`parse_runtime_pin`](../../../../rust/cloacina/registry/loader/constructor_loader/#fn.parse_runtime_pin)
- [`provider_search_path`](../../../../rust/cloacina/registry/loader/constructor_loader/#fn.provider_search_path)
- [`set_provider_search_path`](../../../../rust/cloacina/registry/loader/constructor_loader/#fn.set_provider_search_path)
- [`unpack_provider_archive`](../../../../rust/cloacina/registry/loader/constructor_loader/#fn.unpack_provider_archive)
- `ProviderRuntime` (`cloacina_constructor_contract::ProviderRuntime`)
