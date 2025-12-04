---
title: "Repository Structure"
description: "Overview of the Cloacina repository organization and crate architecture"
weight: 10
---

## Directory Layout

```
cloacina/
  crates/                    # Rust library crates
    cloacina/                # Core workflow engine
    cloacina-macros/         # Procedural macros

  bindings/                  # Language bindings
    cloaca-backend/          # Python bindings (PyPI: cloaca)

  examples/                  # Example projects
    tutorials/               # Step-by-step learning path
    features/                # Feature showcases
    performance/             # Benchmarks

  tests/                     # Integration tests
    python/                  # Python binding tests

  docs/                      # Hugo documentation site
  docker/                    # Docker configurations
  .angreal/                  # Task automation scripts
  .github/                   # CI workflows
```

## Crates

### cloacina

The core workflow orchestration library. Provides:

- **Task system**: `Task` trait, `TaskState`, retry policies
- **Workflow engine**: DAG construction, validation, execution
- **Context management**: Type-safe data passing between tasks
- **Persistence**: PostgreSQL and SQLite backends via Diesel
- **Registry**: Workflow registration and discovery
- **Scheduling**: Cron-based workflow scheduling
- **Multi-tenancy**: Isolated execution environments

**Key modules:**
- `src/task/` - Task trait and implementations
- `src/workflow/` - Workflow builder and validation
- `src/context/` - Context management
- `src/dal/` - Data access layer
- `src/executor/` - Pipeline execution engine
- `src/runner/` - High-level workflow runner
- `src/registry/` - Workflow registry

**Features:**
- `postgres` - PostgreSQL backend (default)
- `sqlite` - SQLite backend (default)
- `macros` - Procedural macro support (default)
- `auth` - Authentication support

### cloacina-macros

Procedural macros for workflow definition:

- `#[task]` - Define tasks from async functions
- `#[workflow]` - Define workflows declaratively
- `#[packaged_workflow]` - Create distributable workflow packages

## Bindings

### cloaca-backend (Python)

Python bindings distributed as `cloaca` on PyPI. Built with PyO3.

Exposes the full Cloacina API to Python including:
- Workflow and task definition
- Context management
- Runner configuration
- Database administration

## Examples

### tutorials/

Progressive learning path for Rust workflows:

| Directory | Topic |
|-----------|-------|
| `01-basic-workflow/` | Single task workflow |
| `02-multi-task/` | Multiple tasks with dependencies |
| `03-dependencies/` | Complex dependency patterns |
| `04-error-handling/` | Error handling and recovery |
| `05-advanced/` | Advanced patterns |
| `06-multi-tenancy/` | Multi-tenant workflows |
| `python/` | Python tutorial scripts |

### features/

Feature demonstrations:

| Directory | Feature |
|-----------|---------|
| `complex-dag/` | Complex DAG topologies |
| `cron-scheduling/` | Scheduled workflow execution |
| `multi-tenant/` | Tenant isolation |
| `packaged-workflows/` | Distributable workflow packages |
| `per-tenant-credentials/` | Tenant-specific configuration |
| `registry-execution/` | Registry-based execution |
| `simple-packaged/` | Minimal packaged workflow |
| `validation-failures/` | Validation error examples |

### performance/

Performance benchmarks:

| Directory | Benchmark |
|-----------|-----------|
| `simple/` | Single task baseline |
| `parallel/` | Parallel task execution |
| `pipeline/` | Sequential pipeline |

## Tests

### tests/python/

Integration tests for Python bindings using pytest. Tests cover:
- Basic API functionality
- Workflow execution patterns
- Context propagation
- Error handling
- Performance characteristics
- Multi-tenancy

## Configuration Files

| File | Purpose |
|------|---------|
| `Cargo.toml` | Workspace configuration |
| `pyproject.toml` | Python project configuration |
| `rustfmt.toml` | Rust formatting rules |
| `.pre-commit-config.yaml` | Pre-commit hooks |

## Development

### Building

```bash
# Build all crates
cargo build

# Build with specific backend
cargo build --no-default-features --features postgres
cargo build --no-default-features --features sqlite

# Build Python bindings
cd bindings/cloaca-backend
maturin develop
```

### Testing

```bash
# Rust tests
cargo test

# Python tests
cd tests/python
pytest
```

### Running Examples

```bash
# Run a tutorial
cargo run -p tutorial-01

# Run a feature example
cargo run -p cron-scheduling
```
