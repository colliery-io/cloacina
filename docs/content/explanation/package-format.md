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

The `manifest.json` file contains all metadata required for package validation, loading, and execution. It follows a structured JSON schema defined in `cloacina-ctl/src/manifest/types.rs`.

### Complete Manifest Example

```json
{
  "package": {
    "name": "data_processing_workflow",
    "version": "1.2.0",
    "description": "Advanced data processing and validation workflow",
    "cloacina_version": "0.3.0"
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
  "execution_order": ["fetch_data", "validate_data", "process_data"],
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
| `cloacina_version` | string | ✅ | Minimum compatible Cloacina runtime version |

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

#### Optional Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `execution_order` | array | ❌ | Pre-computed topological sort of tasks |
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
cloacina = { version = "0.3", features = ["macros"] }
cloacina-macros = "0.3"
# Other dependencies...
```

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

### 2. Package Creation with cloacina-ctl

The `cloacina-ctl package` command handles the complete packaging process:

```bash
# Basic package creation
cloacina-ctl package . -o my-workflow.cloacina

# With specific target
cloacina-ctl package . -o my-workflow.cloacina --target x86_64-unknown-linux-gnu

# With release profile
cloacina-ctl package . -o my-workflow.cloacina --profile release
```

### 3. Internal Package Creation Steps

The packaging process (from `cloacina-ctl/src/commands/package.rs`) involves:

1. **Compilation**: Build the workflow as a shared library using `compile_workflow`
2. **Metadata Extraction**: Extract task metadata from the compiled library
3. **Archive Creation**: Create a tar.gz archive with manifest and library

```rust
// Simplified packaging process
pub fn package_workflow(
    project_path: PathBuf,
    output: PathBuf,
    target: Option<String>,
    profile: String,
    cargo_flags: Vec<String>,
    cli: &Cli,
) -> Result<()> {
    // Step 1: Compile workflow to shared library
    let compile_result = compile_workflow(
        project_path,
        temp_so_path,
        target,
        profile,
        cargo_flags,
        cli,
    )?;

    // Step 2: Create package archive
    create_package_archive(&compile_result, &output, cli)?;

    Ok(())
}
```

### 4. Archive Creation Implementation

The archive creation process (from `cloacina-ctl/src/archive/create.rs`):

```rust
pub fn create_package_archive(
    compile_result: &CompileResult,
    output: &PathBuf,
    cli: &Cli,
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

### Using cloacina-ctl inspect

The `cloacina-ctl inspect` command provides package inspection capabilities:

```bash
# Human-readable output
cloacina-ctl inspect my-workflow.cloacina

# JSON output
cloacina-ctl inspect my-workflow.cloacina --format json
```

**Example human-readable output:**
```
Package Information:
  File: my-workflow.cloacina
  Package: data_processing_workflow
  Version: 1.2.0
  Description: Advanced data processing and validation workflow
  Cloacina Version: 0.3.0 (compatible)

Library:
  File: libdata_processing_workflow.so
  Architecture: x86_64-unknown-linux-gnu
  Symbols: ["cloacina_execute_task", "cloacina_get_task_metadata"]

Tasks (3):
  0: fetch_data
     Dependencies: []
     Source: src/lib.rs:45:1

  1: validate_data
     Dependencies: ["fetch_data"]
     Source: src/lib.rs:67:1

  2: process_data
     Dependencies: ["validate_data"]
     Source: src/lib.rs:89:1

Execution Order: fetch_data → validate_data → process_data
```

### Manual Inspection

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

The packaging system includes validation (from `cloacina/src/registry/loader/validator.rs`):

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
// From cloacina-ctl/src/manifest/types.rs
pub const CLOACINA_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MANIFEST_FILENAME: &str = "manifest.json";
pub const EXECUTE_TASK_SYMBOL: &str = "cloacina_execute_task";
```

## Related Resources

- [Tutorial: Creating Your First Packaged Workflow]({{< ref "/tutorials/07-packaged-workflows/" >}})
- [Explanation: FFI System]({{< ref "/explanation/ffi-system/" >}})
- [Explanation: Packaged Workflow Architecture]({{< ref "/explanation/packaged-workflow-architecture/" >}})
