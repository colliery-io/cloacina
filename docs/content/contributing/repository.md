---
title: "Repository Organization"
description: "Understanding the Cloacina repository structure"
weight: 62
reviewer: "dstorey"
review_date: "2024-03-19"
---


This guide explains how the Cloacina repository is organized to support both Rust and Python development with a sophisticated multi-backend build system.

## Project Overview

Cloacina is organized as a multi-language workspace supporting both Rust and Python implementations. The repository includes:

### Core Components
- **`cloacina/`** - Core Rust workflow orchestration library
- **`cloacina-macros/`** - Procedural macros for workflow/task definition
- **`cloaca-backend/`** - Python bindings (PyO3/Maturin) with multi-backend support
- **`cloaca/`** - Python dispatcher package providing unified API

### Development Infrastructure  
- **`.angreal/`** - Complete development automation system with templated build pipeline
- **`python-tests/`** - Comprehensive Python binding test suite (28+ scenario files)
- **`examples/`** - Example projects showcasing various features and patterns
- **`docs/`** - Hugo-based documentation following Diátaxis framework

{{< hint info >}}
**Multi-Backend Architecture:** The Python bindings support both PostgreSQL and SQLite backends through feature-gated compilation and a sophisticated template-driven build system.
{{< /hint >}}

## Multi-Backend Python Bindings

The repository implements a sophisticated Python binding system supporting multiple database backends:

### Architecture Overview
```
cloaca (Python dispatcher)
    ├── Automatic backend detection
    ├── Unified API surface
    └── Optional dependencies per backend

cloaca-backend (PyO3/Maturin implementation)  
    ├── Feature-gated compilation (postgres/sqlite)
    ├── Template-driven configuration
    └── Shared Rust implementation with backend-specific features
```

### Backend Isolation
- **Complete separation**: PostgreSQL and SQLite backends are compiled as separate wheels
- **Feature gating**: Rust features control which database driver is included
- **Template system**: All configuration files generated from Jinja2 templates
- **Version synchronization**: Workspace version automatically propagated to all packages

## Testing Strategy

Cloacina employs a comprehensive testing approach:

1. **Unit Tests**
   - Located alongside the code they test in `cloacina/src/` and `cloacina-macros/src/`
   - Test individual components and functions in isolation
   - Run with `angreal tests unit`

2. **Integration Tests**
   - Located in `cloacina/tests/`
   - Test how different components work together
   - Include test utilities and helpers
   - Run with `angreal tests integration`

3. **End-to-End Tests**
   - Implemented as executable examples in the `examples/` directory
   - Tutorials that are executed via angreal
   - Demonstrate complete usage patterns and workflows
   - Run with `angreal examples all`

4. **Python Binding Tests**
   - Located in `python-tests/` with 28+ scenario files
   - File-level isolation with fresh virtual environments per test run
   - Supports both PostgreSQL and SQLite backends
   - Run with `angreal cloaca test`

{{< hint info >}}
**Python tests use complete isolation**: Each test file runs in a fresh virtual environment with clean database state to ensure no cross-contamination between test cycles.
{{< /hint >}}

## Development Workflow

The project uses [Angreal](https://angreal.io) for comprehensive development automation, providing commands for both Rust and Python development workflows.

### Angreal Command Groups

The repository provides two primary command groups:

- **Standard Angreal commands** - For Rust development and documentation
- **`cloaca` commands** - For Python binding development and testing

### Python Binding Commands

The Cloaca command group provides a complete development pipeline:

```bash
# Generate configuration files for a specific backend
angreal cloaca generate --backend postgres
angreal cloaca generate --backend sqlite

# Build backend wheels
angreal cloaca package --backend postgres
angreal cloaca package --backend sqlite

# Run comprehensive test suite
angreal cloaca test                    # Test both backends
angreal cloaca test --backend postgres # Test specific backend
angreal cloaca test --file test_scenario_03_function_based_dag_topology.py
angreal cloaca test -k "context"      # Filter tests

# Run smoke tests
angreal cloaca smoke                   # Quick verification for both backends
angreal cloaca smoke --backend sqlite # Test specific backend

# Build release artifacts
angreal cloaca release                 # Build both backend wheels
angreal cloaca release --backend postgres

# Clean up generated files
angreal cloaca scrub                   # Reset to clean state
```

### Template-Driven Build System

The Python bindings use a sophisticated template-driven build system that enables:

#### Dynamic Configuration Generation
- **Jinja2 templates** generate backend-specific configuration files
- **Version synchronization** automatically propagates workspace version
- **Feature gating** controls which database drivers are compiled

#### Template Files
```
.angreal/templates/
├── dispatcher_pyproject.toml.j2      # Cloaca dispatcher package config
├── backend_cargo.toml.j2             # PyO3/Maturin Cargo configuration  
└── backend_pyproject.toml.j2         # Backend wheel metadata
```

#### Generated Artifacts
- `cloaca/pyproject.toml` - Dispatcher package configuration
- `cloaca-backend/Cargo.toml` - Backend Rust compilation settings
- `cloaca-backend/pyproject.toml` - Backend wheel metadata
- `cloaca-backend/python/cloaca_{backend}/` - Backend-specific Python modules

{{< hint warning >}}
**Generated files are managed automatically**: All configuration files in the Python binding packages are generated from templates. Manual edits will be overwritten during the next `generate` command.
{{< /hint >}}

#### Build Process Flow
1. **Template Rendering** - Generate all configuration files with version and backend context
2. **Environment Setup** - Create isolated virtual environment with dependencies
3. **Compilation** - Use Maturin to build Rust extension with feature-gated backend
4. **Installation** - Install dispatcher and backend packages
5. **Testing** - Run tests with fresh database state per file
6. **Cleanup** - Remove environments and reset to clean template state

#### Backend Isolation Strategy
- **Separate wheels** for PostgreSQL (`cloaca_postgres`) and SQLite (`cloaca_sqlite`) backends
- **Unified dispatcher** (`cloaca`) detects available backends and provides single API
- **Feature-gated compilation** ensures only required database drivers are included
- **Template-driven configuration** maintains consistency across backend builds

### Development Infrastructure

The repository includes comprehensive development tooling:

#### Docker Services
- **PostgreSQL container** with health checks for testing
- **Managed via angreal** with `docker_up()` and `docker_down()` utilities
- **Volume persistence** for data between test runs
- **Automatic cleanup** with `remove_volumes=True` for fresh state

#### Virtual Environment Management  
- **Isolated test environments** created per test run
- **Automatic cleanup** to prevent disk space accumulation
- **Backend-specific environments** with appropriate dependencies
- **Build environments** separated from test environments

#### File Management
- **Atomic file operations** with error handling and backup support
- **Directory template rendering** for Python package structure
- **Artifact cleanup** for compiled extensions and Python cache
- **Safe placeholder replacement** during scrub operations

#### Test Isolation
- **Database state reset** between test files (PostgreSQL: smart reset vs. container restart)
- **File system cleanup** for SQLite database files
- **Process isolation** with separate virtual environments
- **Resource cleanup** with comprehensive error handling

## Repository Structure Updates

The repository structure has evolved to support the multi-language, multi-backend architecture:

### New Components Added
```
cloacina/                              # Core workflow engine (existing)
├── cloaca-backend/                    # NEW: PyO3/Maturin Python bindings
│   ├── python/cloaca_{{backend}}/     # Template directory for backend modules
│   ├── src/                           # Rust implementation for Python bindings
│   └── target/wheels/                 # Built wheel artifacts
├── cloaca/                           # NEW: Python dispatcher package
│   └── src/cloaca/                    # Backend detection and unified API
├── python-tests/                     # NEW: Comprehensive Python test suite
│   ├── test_scenario_*.py             # 28+ numbered scenario tests
│   └── conftest.py                    # Shared test configuration
├── .angreal/                         # NEW: Development automation
│   ├── task_cloaca.py                # Python binding development commands
│   ├── templates/                    # Jinja2 configuration templates
│   ├── utils.py                      # Docker and utility functions
│   ├── database_reset.py             # PostgreSQL state management
│   └── docker-compose.yaml           # PostgreSQL test container
├── debug-env-{backend}/              # Generated debug environments
├── test-env-{backend}/               # Generated test environments  
└── target/wheels/                    # Release wheel artifacts
```

### Integration Points
- **Workspace version** from `Cargo.toml` propagated to all Python packages
- **Shared examples** demonstrate both Rust and Python usage patterns
- **Unified documentation** covers both language implementations
- **Consistent testing** patterns across Rust and Python codebases

{{< hint info >}}
**Multi-language development**: The repository now supports full-stack development with Rust for the core engine and Python for high-level workflow authoring, each with appropriate tooling and testing infrastructure.
{{< /hint >}}

## Documentation Structure

Our documentation follows the [Diátaxis Framework](https://diataxis.fr/) and is organized into:
- `docs/content/tutorials/` - Step-by-step guides and examples that teach Cloacina features
- `docs/content/how-to/` - Task-oriented guides for specific operations
- `docs/content/reference/` - Technical reference and API documentation
- `docs/content/explanation/` - Conceptual documentation and deep dives

All documentation, including Rust documentation, is automatically bundled into a single site for easy reference. This ensures you have access to all project documentation in one place.

### Updated Documentation Sections
- **`docs/content/cloaca/`** - Complete Python binding documentation with tutorials, API reference, and quick start
- **Cross-references** between Rust and Python implementations
- **Unified navigation** supporting both language ecosystems

For detailed information about documentation standards and practices, please refer to the [Documentation Guide](./documentation.md).

## Development Best Practices

### Python Binding Development
1. **Always use `angreal cloaca generate`** before testing to ensure configuration files are current
2. **Run `angreal cloaca scrub`** after development sessions to clean up generated files  
3. **Test both backends** unless focusing on backend-specific features
4. **Use file-level test isolation** for reliable test results
5. **Inspect `debug-env-{backend}/`** environments for debugging issues

### Workspace Version Management
- All Python packages automatically inherit the workspace version from `Cargo.toml`
- Update version in **one place only**: `[workspace.package] version` in root `Cargo.toml`
- Template system propagates version changes to all generated configuration files

### Testing Strategy
- **Unit tests**: Test individual components in isolation
- **Integration tests**: Test component interactions within language boundaries  
- **Python scenario tests**: Test complete workflows through Python API
- **End-to-end tests**: Test real-world usage patterns via examples

### Template Management
- **Never edit generated files directly** - all changes will be overwritten
- **Modify templates** in `.angreal/templates/` for configuration changes
- **Use template variables** for backend-specific or version-specific content
- **Test template changes** with both backends to ensure compatibility

{{< hint warning >}}
**Development workflow reminder**: The Python binding development workflow requires generating files, testing, and cleaning up. Always use the provided angreal commands rather than manual file manipulation to maintain consistency and avoid conflicts.
{{< /hint >}}
