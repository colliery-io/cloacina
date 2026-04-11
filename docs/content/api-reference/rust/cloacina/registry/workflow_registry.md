# cloacina::registry::workflow_registry <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Complete implementation of the workflow registry.

This module provides the `WorkflowRegistryImpl` that combines all registry
components - storage, loading, validation, and task registration - into a
cohesive system for managing packaged workflows.

## Structs

### `cloacina::registry::workflow_registry::WorkflowRegistryImpl`<S: RegistryStorage>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Complete implementation of the workflow registry.

This registry implementation combines storage backends, package loading,
validation, and task registration to provide a full-featured system for
managing packaged workflows with proper lifecycle management.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `storage` | `S` | Storage backend for binary data |
| `database` | `Database` | Database for metadata storage |
| `loader` | `PackageLoader` | Package loader for metadata extraction |
| `registrar` | `TaskRegistrar` | Task registrar for global registry integration |
| `validator` | `PackageValidator` | Package validator for safety checks |
| `loaded_packages` | `HashMap < Uuid , Vec < TaskNamespace > >` | Map of package IDs to registered task namespaces for cleanup tracking |
