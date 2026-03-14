---
title: "Package Format"
description: ".cloacina archive structure and creation process"
weight: 20
reviewer: "dstorey"
review_date: "2025-01-17"
---

This article explains the structure and format of `.cloacina` packages, which are the distributable units for Cloacina workflows. Understanding the package format is essential for creating custom tooling, debugging package issues, and working with the packaging system.

## Overview

A `.cloacina` package is a **compressed tar.gz archive** containing exactly two components:
1. A JSON manifest with package metadata (`manifest.json`)
2. A platform-specific dynamic library containing the compiled workflow

This simple but standardized format enables cross-platform distribution while maintaining compatibility with standard archiving tools.

## Archive Structure

### Basic Package Layout

```
example-workflow.cloacina (tar.gz archive)
├── manifest.json          # Package metadata and library information
└── libexample_workflow.so  # Platform-specific dynamic library
```

### Platform-Specific Library Names

The dynamic library name varies by target platform following standard conventions:

| Platform | Extension | Example Filename | Format |
|----------|-----------|------------------|---------|
| **Linux** | `.so` | `libworkflow.so` | Shared Object |
| **macOS** | `.dylib` | `libworkflow.dylib` | Dynamic Library |
| **Windows** | `.dll` | `workflow.dll` | Dynamic Link Library |

{{< hint type=info title="Naming Consistency" >}}
The library filename in the archive must exactly match the `filename` field in the manifest. This consistency check ensures package integrity during validation.
{{< /hint >}}

## Manifest Specification

The `manifest.json` file contains all metadata required for package validation, loading, and execution. It follows a structured JSON schema defined in `cloacina/src/packaging/types.rs`.

### Complete Manifest Example

```json
{
  "package": {
    "name": "data_processing_workflow",
    "version": "1.2.0",
    "description": "Advanced data processing and validation workflow",
    "author": "ACME Corp",
    "cloacina_version": "0.3.0",
    "workflow_fingerprint": "sha256:abc123..."
  },
  "library": {
    "filename": "libdata_processing_workflow.so",
    "symbols": [
      "cloacina_execute_task",
      "cloacina_get_task_metadata"
    ],
    "architecture": "x86_64-unknown-linux-gnu"
  },
  "tasks": [
    {
      "index": 0,
      "id": "fetch_data",
      "dependencies": [],
      "description": "Fetch raw data from external APIs",
      "source_location": "src/lib.rs:45:1"
    },
    {
      "index": 1,
      "id": "validate_data",
      "dependencies": ["fetch_data"],
      "description": "Validate and clean fetched data",
      "source_location": "src/lib.rs:67:1"
    },
    {
      "index": 2,
      "id": "process_data",
      "dependencies": ["validate_data"],
      "description": "Transform data for downstream systems",
      "source_location": "src/lib.rs:89:1"
    }
  ],
  "graph": {
    "tasks": {
      "fetch_data": {
        "id": "fetch_data",
        "dependencies": []
      },
      "validate_data": {
        "id": "validate_data",
        "dependencies": ["fetch_data"]
      },
      "process_data": {
        "id": "process_data",
        "dependencies": ["validate_data"]
      }
    },
    "execution_order": ["fetch_data", "validate_data", "process_data"]
  }
}
```

### Manifest Field Reference

#### Package Section

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | ✅ | Package identifier (must match Cargo.toml name) |
| `version` | string | ✅ | Package version (semantic versioning) |
| `description` | string | ✅ | Human-readable package description |
| `author` | string | ❌ | Package author |
| `cloacina_version` | string | ✅ | Minimum compatible Cloacina runtime version |
| `workflow_fingerprint` | string | ❌ | Content-based fingerprint for integrity verification |

#### Library Section

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `filename` | string | ✅ | Exact filename of library in archive |
| `symbols` | array | ✅ | Required FFI symbols present in library |
| `architecture` | string | ✅ | Target triple (e.g., `x86_64-unknown-linux-gnu`) |

#### Tasks Array

Each task object contains:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `index` | number | ✅ | Unique task index within package |
| `id` | string | ✅ | Task identifier for execution and dependencies |
| `dependencies` | array | ✅ | Array of task IDs this task depends on |
| `description` | string | ✅ | Human-readable task description |
| `source_location` | string | ✅ | Source file and line number for debugging |

#### Graph Section

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `graph` | object | ❌ | Complete workflow graph data for visualization |

## Package Creation Process

### 1. Source Code Requirements

For a Rust project to be packaged, it must meet specific requirements:

**Cargo.toml Configuration:**
```toml
[package]
name = "my_workflow"
version = "1.0.0"
edition = "2021"

# Required: Generate shared library
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
cloacina-workflow = "0.2"  # Includes macros by default
serde_json = "1.0"
async-trait = "0.1"
# Other dependencies...
```

{{< hint type=info title="cloacina-workflow" >}}
Packaged workflows use `cloacina-workflow`, which contains only the types needed for workflow compilation (`Context`, `Task`, `TaskError`, `RetryPolicy`). This enables fast compilation without database drivers or runtime dependencies.
{{< /hint >}}

**Source Structure:**
```
my_workflow/
├── Cargo.toml
├── src/
│   └── lib.rs        # Note: lib.rs, not main.rs
└── target/
    └── release/
        └── libmy_workflow.so  # Generated by cargo build
```

### 2. Package Creation with cloacinactl

The `cloacinactl package build` command handles the packaging process:

```bash
# Basic package creation
cloacinactl package build -o my-workflow.cloacina

# With specific target
cloacinactl package build -o my-workflow.cloacina --target x86_64-unknown-linux-gnu

# Dry run to see what would be built
cloacinactl package build -o my-workflow.cloacina --dry-run
```

### 3. Internal Package Creation Steps

The Rust packaging process (from `cloacina/src/packaging/`) involves two steps:

1. **Compilation**: Build the workflow as a shared library and extract metadata using `compile_workflow`
2. **Archive Creation**: Create a tar.gz archive with manifest and library using `create_package_archive`

```rust
// Step 1: Compile workflow to shared library (from compile.rs)
let compile_result = compile_workflow(
    project_path,
    output_path,
    CompileOptions { target, .. },
)?;

// Step 2: Create package archive (from archive.rs)
create_package_archive(&compile_result, &output)?;
```

{{< hint type=info title="cloacinactl package build" >}}
Note that `cloacinactl package build` delegates to the Python `cloaca build` pipeline via PyO3. The Rust `compile_workflow` + `create_package_archive` path is used internally for Rust-only packaging.
{{< /hint >}}

### 4. Archive Creation Implementation

The archive creation process (from `cloacina/src/packaging/archive.rs`):

```rust
pub fn create_package_archive(
    compile_result: &CompileResult,
    output: &PathBuf,
) -> Result<()> {
    // Create compressed tar.gz file
    let output_file = fs::File::create(output)?;
    let gz_encoder = GzEncoder::new(output_file, Compression::default());
    let mut tar_builder = Builder::new(gz_encoder);

    // Add manifest.json to archive
    let manifest_json = serde_json::to_string_pretty(&compile_result.manifest)?;
    let manifest_bytes = manifest_json.as_bytes();
    let mut header = tar::Header::new_gnu();
    header.set_size(manifest_bytes.len() as u64);
    header.set_cksum();

    tar_builder.append_data(&mut header, "manifest.json", manifest_bytes)?;

    // Add shared library file
    tar_builder.append_file(
        &compile_result.manifest.library.filename,
        &mut fs::File::open(&compile_result.so_path)?
    )?;

    // Finalize archive
    tar_builder.finish()?;
    Ok(())
}
```

## Package Inspection

### Using cloacinactl package inspect

The `cloacinactl package inspect` command inspects detached signature files:

```bash
# Inspect a signature file
cloacinactl package inspect my-workflow.cloacina.sig
```

**Example output:**
```
Signature File: my-workflow.cloacina.sig
  Format version: 1
  Algorithm:      Ed25519
  Package hash:   sha256:...
  Key fingerprint: ...
  Signed at:      2025-03-13T...
```

### Manual Package Inspection

Since packages are standard tar.gz files, you can inspect them with standard tools:

```bash
# List archive contents
tar -tzf my-workflow.cloacina

# Extract to directory
tar -xzf my-workflow.cloacina -C extracted/

# View manifest
tar -xzOf my-workflow.cloacina manifest.json | jq .

# Check library symbols (Linux)
tar -xzOf my-workflow.cloacina libmy_workflow.so | nm -D
```

## Package Validation

### Basic Validation

The packaging system includes validation (from `cloacina/src/registry/loader/validator/`):

- **Size validation**: Empty packages and oversized packages are rejected
- **Format validation**: Files must be valid dynamic libraries (ELF, Mach-O, PE)
- **Symbol validation**: Required FFI symbols must be present
- **Metadata validation**: Package names, task IDs, and dependencies are checked

### Library Format Validation

The validator checks for proper dynamic library formats:

```rust
// ELF format (Linux) - starts with 0x7f followed by "ELF"
if &data[0..4] == b"\x7fELF" {
    // Valid ELF format
}

// Mach-O format (macOS) - magic numbers
else if &data[0..4] == b"\xcf\xfa\xed\xfe" {
    // 64-bit Mach-O format
}

// PE format (Windows) - starts with "MZ"
else if &data[0..2] == b"MZ" {
    // Valid PE format
}
```

### Required Symbols

Every valid package must export these FFI symbols:
- `cloacina_execute_task` - Main task execution entry point
- `cloacina_get_task_metadata` - Metadata extraction function

## Best Practices

### Package Naming

- Use descriptive, lowercase names with underscores: `data_processor`
- Include organization prefix for uniqueness: `acme_data_processor`
- Keep names concise but meaningful

### Version Management

- Follow semantic versioning strictly
- Increment major version for breaking changes to task interfaces
- Use pre-release versions for testing: `1.0.0-beta.1`

### Documentation

- Provide comprehensive task descriptions in the manifest
- Include source locations for debugging
- Document dependencies clearly

### Size Optimization

- Use release builds to minimize library size
- Remove unnecessary dependencies from Cargo.toml
- Consider target-specific builds for production

### Testing

- Test package creation and loading on all target platforms
- Validate metadata extraction works correctly
- Verify all tasks execute successfully after packaging

## File Format Constants

Key constants defined in the codebase:

```rust
// From cloacina/src/packaging/types.rs
pub const CLOACINA_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MANIFEST_FILENAME: &str = "manifest.json";
pub const EXECUTE_TASK_SYMBOL: &str = "cloacina_execute_task";

// From cloacina/src/registry/loader/package_loader.rs
pub const GET_METADATA_SYMBOL: &str = "cloacina_get_task_metadata";
```

## Python Packages (Manifest V2)

Starting with format version 2, `.cloacina` packages support Python workflow packages in addition to Rust. The manifest uses a `language` discriminator to determine the runtime configuration.

### Python Package Layout

```
my-workflow.cloacina (tar.gz archive)
├── manifest.json          # V2 manifest with language: "python"
├── requirements.lock      # Pinned versions with hashes
├── workflow/              # Python source modules
│   ├── __init__.py
│   └── tasks.py           # @task decorated functions
└── vendor/                # Vendored dependencies (extracted wheels)
    ├── requests/
    ├── urllib3/
    └── VENDORED.txt        # List of vendored packages
```

### V2 Manifest Example (Python)

```json
{
  "format_version": "2",
  "package": {
    "name": "data-pipeline",
    "version": "1.0.0",
    "description": "Example data processing pipeline",
    "fingerprint": "sha256:abc123...",
    "targets": ["linux-x86_64", "macos-arm64"]
  },
  "language": "python",
  "python": {
    "requires_python": ">=3.11",
    "entry_module": "workflow.tasks"
  },
  "tasks": [
    {
      "id": "fetch-data",
      "function": "workflow.tasks:fetch_data",
      "dependencies": [],
      "description": "Fetch raw data from source",
      "retries": 3,
      "timeout_seconds": 300
    },
    {
      "id": "process-data",
      "function": "workflow.tasks:process_data",
      "dependencies": ["fetch-data"],
      "retries": 0
    }
  ],
  "created_at": "2025-03-13T12:00:00Z"
}
```

### V2 Manifest Fields

The V2 manifest extends the original format with these fields:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `format_version` | string | Yes | Always `"2"` for V2 packages |
| `language` | string | Yes | `"python"` or `"rust"` |
| `python` | object | If Python | Python runtime configuration |
| `python.requires_python` | string | Yes | PEP 440 version specifier (e.g., `">=3.10"`) |
| `python.entry_module` | string | Yes | Dotted module path for task discovery |
| `rust` | object | If Rust | Rust runtime configuration |
| `rust.library_path` | string | Yes | Relative path to compiled dynamic library |
| `package.fingerprint` | string | Yes | SHA-256 hash of package contents |
| `package.targets` | array | Yes | Supported platform strings |

#### Task Definition (V2)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | Task identifier (unique within package) |
| `function` | string | Yes | Python: `"module.path:function_name"`, Rust: symbol name |
| `dependencies` | array | No | IDs of tasks that must complete first |
| `description` | string | No | Human-readable description |
| `retries` | number | No | Automatic retry count (default 0) |
| `timeout_seconds` | number | No | Maximum execution time |

#### Supported Platforms

| Platform String | Description |
|----------------|-------------|
| `linux-x86_64` | Linux on Intel/AMD 64-bit |
| `linux-arm64` | Linux on ARM 64-bit (e.g., Graviton) |
| `macos-x86_64` | macOS on Intel |
| `macos-arm64` | macOS on Apple Silicon |

### V2 Validation

The V2 manifest includes additional validation beyond the original format:

- **Language-runtime match**: Python packages must have `python` config; Rust packages must have `rust` config
- **Function path format**: Python task functions must use `module.path:function_name` format (colon separator)
- **Target validation**: All targets must be in the supported platforms list
- **Task integrity**: No duplicate IDs, all dependencies must reference existing tasks

### Python Package Creation

Python packages are built using `cloaca build` (or `cloacinactl package build`):

```bash
# Build from a Python project with pyproject.toml
cloaca build -o my-workflow.cloacina --target linux-x86_64

# The pyproject.toml must include [tool.cloaca] section:
# [tool.cloaca]
# entry_module = "workflow.tasks"
```

The build process:
1. **Task discovery**: AST-based static analysis of the entry module (no code import)
2. **Dependency vendoring**: `uv` resolves and downloads platform-specific wheels
3. **Archive creation**: tar.gz containing manifest, workflow source, and vendored dependencies

{{< hint type=info title="Backward Compatibility" >}}
V2 manifests with `language: "rust"` are functionally equivalent to V1 manifests. The V2 format is a superset that adds Python support while maintaining compatibility with existing Rust workflows.
{{< /hint >}}

## Related Resources

- [Tutorial: Creating Your First Packaged Workflow]({{< ref "/tutorials/07-packaged-workflows/" >}})
- [Tutorial: Packaging Python Workflows]({{< ref "/python-bindings/tutorials/09-packaging-workflows/" >}})
- [API Reference: Packaging CLI]({{< ref "/python-bindings/api-reference/packaging/" >}})
- [Explanation: FFI System]({{< ref "/explanation/ffi-system/" >}})
- [Explanation: Packaged Workflow Architecture]({{< ref "/explanation/packaged-workflow-architecture/" >}})
