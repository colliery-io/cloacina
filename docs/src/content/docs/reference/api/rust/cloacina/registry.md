---
title: "cloacina::registry"
description: "API reference of the Rust module cloacina::registry."
editUrl: false
sidebar:
  order: 155
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

**Workflow Registry**

This module provides a registry system for dynamically loading and managing
packaged workflows (.so files) created by cloacina-ctl. The registry enables
runtime distribution of workflows with proper isolation and versioning.

**Architecture**

The registry uses a two-table design:
- `workflow_registry`: Simple key-value storage for binary .so data
- `workflow_packages`: Rich metadata with foreign key to registry

This separation allows efficient metadata queries without loading binaries
and enables future migration to object storage for the binary data.

**Key Components**

- [`traits`](../../../rust/cloacina/registry/traits/): Core trait definitions for registry and storage
- [`types`](../../../rust/cloacina/registry/types/): Data types for workflows, metadata, and errors
- [`error`](../../../rust/cloacina/registry/error/): Error types for registry operations
- [`storage`](../../../rust/cloacina/registry/storage/): Storage backend implementations (PostgreSQL, filesystem)

**Usage Example**

```rust
use cloacina::registry::{WorkflowRegistry, WorkflowPackage};
use cloacina::dal::{PostgresWorkflowRegistryDAL, FilesystemWorkflowRegistryDAL};

// Create storage backend (PostgreSQL or filesystem)
let storage = PostgresWorkflowRegistryDAL::new(database);
// or: let storage = FilesystemWorkflowRegistryDAL::new("/var/lib/cloacina/registry")?;

// Create registry with chosen storage
let mut registry = WorkflowRegistryImpl::new(database, storage)?;

// Register a packaged workflow
let package = WorkflowPackage::from_file("analytics.cloacina")?;
let package_id = registry.register_workflow(package).await?;

// List registered workflows
let workflows = registry.list_workflows().await?;

// Load and execute
let loaded = registry.get_workflow("analytics", "1.0.0").await?;
```

## Re-exports

- [`LoaderError`](../../../rust/cloacina/registry/error/#enum.LoaderError)
- [`RegistryError`](../../../rust/cloacina/registry/error/#enum.RegistryError)
- [`StorageError`](../../../rust/cloacina/registry/error/#enum.StorageError)
- [`PackageLoader`](../../../rust/cloacina/registry/loader/package_loader/#struct.PackageLoader)
- [`TaskRegistrar`](../../../rust/cloacina/registry/loader/task_registrar/#struct.TaskRegistrar)
- [`CronWorkflowRegistrar`](../../../rust/cloacina/registry/reconciler/#trait.CronWorkflowRegistrar)
- [`PackageStatusDetail`](../../../rust/cloacina/registry/reconciler/#struct.PackageStatusDetail)
- [`ReconcileResult`](../../../rust/cloacina/registry/reconciler/#struct.ReconcileResult)
- [`ReconcilerConfig`](../../../rust/cloacina/registry/reconciler/#struct.ReconcilerConfig)
- [`ReconcilerStatus`](../../../rust/cloacina/registry/reconciler/#struct.ReconcilerStatus)
- [`RegistryReconciler`](../../../rust/cloacina/registry/reconciler/#struct.RegistryReconciler)
- [`RegistryStorage`](../../../rust/cloacina/registry/traits/#trait.RegistryStorage)
- [`WorkflowRegistry`](../../../rust/cloacina/registry/traits/#trait.WorkflowRegistry)
- [`LoadedWorkflow`](../../../rust/cloacina/registry/types/#struct.LoadedWorkflow)
- [`WorkflowMetadata`](../../../rust/cloacina/registry/types/#struct.WorkflowMetadata)
- [`WorkflowPackage`](../../../rust/cloacina/registry/types/#struct.WorkflowPackage)
- [`WorkflowPackageId`](../../../rust/cloacina/registry/types/#type.WorkflowPackageId)
- [`WorkflowSourceFile`](../../../rust/cloacina/registry/types/#struct.WorkflowSourceFile)
- [`WorkflowRegistryImpl`](../../../rust/cloacina/registry/workflow_registry/#struct.WorkflowRegistryImpl)
- [`FilesystemWorkflowRegistry`](../../../rust/cloacina/registry/workflow_registry/filesystem/#struct.FilesystemWorkflowRegistry)
