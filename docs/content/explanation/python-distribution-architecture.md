---
title: "Python Distribution Architecture"
weight: 50
---

# Python Distribution Architecture

Cloacina's Python bindings use a sophisticated distribution architecture that solves the fundamental challenge of providing compile-time database backend selection through Python's runtime package management system.

## The Challenge

Python packages typically handle feature selection at runtime (installing additional dependencies), while Rust crates handle it at compile time (through cargo features). This creates a mismatch when distributing Rust-based Python extensions:

- **Rust Reality**: `postgres` and `sqlite` features are mutually exclusive and must be selected at compile time
- **Python Expectation**: Users want `pip install cloacina[postgres]` to "just work" without build toolchains
- **PyPI Limitation**: Cannot upload multiple wheels with different compile-time features under the same package name

## Architecture Overview

Our solution uses a **dispatcher pattern** with three separate PyPI packages:

```
┌─────────────────┐    ┌─────────────────────┐
│   cloacina      │    │  cloacina[postgres] │
│   (dispatcher)  │    │  cloacina[sqlite]   │
└─────────────────┘    └─────────────────────┘
         │                       │
         │              ┌────────┴────────┐
         │              │                 │
         ▼              ▼                 ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│ cloacina-postgres│ │ cloacina-postgres│ │ cloacina-sqlite │
│    (wheel)      │ │    (wheel)      │ │    (wheel)      │
└─────────────────┘ └─────────────────┘ └─────────────────┘
```

## Package Structure

### 1. Backend Packages (`cloacina-postgres`, `cloacina-sqlite`)

These are **maturin-built wheels** containing the actual Rust extensions:

```
cloacina-postgres/
├── Cargo.toml              # Rust crate with postgres features
├── pyproject.toml          # Maturin build configuration  
├── src/                    # Rust source code
└── python/cloacina_postgres/
    └── __init__.py         # Python module exports
```

**Key Characteristics:**
- Built with specific database backend (`--features postgres` or `--features sqlite`)
- Contains the actual PyO3 bindings and compiled Rust code
- Published as separate wheels to PyPI
- Not intended for direct user installation

### 2. Dispatcher Package (`cloacina`)

This is a **pure Python package** that provides the user-facing API:

```python
# cloacina/__init__.py
try:
    from cloacina_postgres import *
    __backend__ = "postgres"
except ImportError:
    try:
        from cloacina_sqlite import *
        __backend__ = "sqlite"  
    except ImportError:
        raise ImportError("No Cloacina backend found...")
```

**Key Characteristics:**
- Pure Python (no compilation required)
- Detects and imports from available backend packages
- Provides unified API regardless of backend
- Defines optional dependencies for backend selection

## User Experience

### Installation

Users interact only with the main `cloacina` package:

```bash
# Install with PostgreSQL backend
pip install cloacina[postgres]

# Install with SQLite backend  
pip install cloacina[sqlite]
```

### Usage

The API is identical regardless of backend:

```python
from cloacina import task, Workflow, UnifiedExecutor

@task(id="my_task", dependencies=[])
def my_task(context):
    return context

# Works with any backend
workflow = Workflow("my_workflow")
```

### Backend Detection

Users can check which backend is active:

```python
import cloacina
print(f"Using backend: {cloacina.__backend__}")  # "postgres" or "sqlite"
```

## Distribution Workflow

### For Maintainers

Building and publishing requires three steps:

```bash
# 1. Build PostgreSQL backend
cd cloacina-postgres
maturin build --release
twine upload target/wheels/*

# 2. Build SQLite backend
cd cloacina-sqlite  
maturin build --release
twine upload target/wheels/*

# 3. Build dispatcher package
cd cloacina-dispatcher
python -m build
twine upload dist/*
```

### Version Management

All packages use **workspace versioning** from the root `Cargo.toml`:

```toml
[workspace.package]
version = "0.1.0"  # Single source of truth
```

This ensures all three packages stay synchronized across releases.

## Technical Implementation Details

### Package Dependencies

The dispatcher package's `pyproject.toml` defines optional dependencies:

```toml
[project.optional-dependencies]
postgres = ["cloacina-postgres==0.1.0"]
sqlite = ["cloacina-sqlite==0.1.0"] 
```

When users run `pip install cloacina[postgres]`, pip automatically installs both `cloacina` and `cloacina-postgres`.

### Import Resolution

The dispatcher uses a **fallback import strategy**:

1. Try importing `cloacina_postgres`
2. If that fails, try importing `cloacina_sqlite`
3. If both fail, raise a helpful error message

This gracefully handles cases where users install multiple backends or none at all.

### Module Structure

Each backend package exposes the same Python API:

```python
# Both cloacina_postgres and cloacina_sqlite export:
__all__ = [
    "__version__",
    "task", 
    "Workflow",
    "UnifiedExecutor",
    "TaskDecorator",
]
```

The dispatcher re-exports everything using `from backend import *`, maintaining API compatibility.

## Advantages

### ✅ User Experience
- Familiar `pip install package[extra]` syntax
- No build toolchains required
- Works on all platforms where wheels are available
- Clear error messages for missing backends

### ✅ Maintainer Simplicity  
- One-time setup, then just build commands
- Automatic version synchronization
- Standard Python packaging tools
- CI/CD friendly

### ✅ Technical Soundness
- Respects Rust's compile-time feature system
- Works within PyPI's constraints
- Follows Python packaging conventions
- Graceful error handling

## Trade-offs

### ⚠️ PyPI Namespace
- Uses three package names instead of one
- Could confuse users who discover backend packages directly

### ⚠️ Build Complexity
- Maintainers must build and upload three packages
- Risk of version mismatches if process is manual

### ⚠️ Import Overhead
- Slight runtime cost for import resolution
- Potential for confusion if multiple backends installed

## Alternative Approaches Considered

### Environment Variable Build
```bash
CLOACINA_BACKEND=postgres pip install cloacina --no-binary cloacina
```
**Rejected**: Requires users to have Rust toolchains and understand build processes.

### Runtime Backend Selection
Build with all features enabled, select at runtime.
**Rejected**: Results in larger binaries and violates Rust's mutual exclusion of postgres/sqlite features.

### Separate Package Names
```bash
pip install cloacina-postgres  # Different package entirely
```
**Rejected**: Doesn't provide the familiar `[extras]` syntax Python users expect.

## Conclusion

The dispatcher architecture successfully bridges the gap between Rust's compile-time feature system and Python's runtime package management. While it adds some complexity for maintainers, it provides an excellent user experience that follows Python packaging conventions and "just works" without requiring build toolchains.