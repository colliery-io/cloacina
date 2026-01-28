---
id: python-package-integration-tests
level: task
title: "Python package integration tests and example project"
short_code: "CLOACI-T-0071"
created_at: 2026-01-28T14:29:04.367894+00:00
updated_at: 2026-01-28T14:29:04.367894+00:00
parent: CLOACI-I-0020
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0020
---

# Python package integration tests and example project

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0020]]

## Objective

Create comprehensive integration tests for the Python package workflow and a complete example project that demonstrates the end-to-end flow: writing Python tasks, building a package with `cloaca build`, loading the package on the server, and executing tasks via the workflow engine.

## Acceptance Criteria

- [ ] Example project with realistic Python workflow (multiple tasks, dependencies)
- [ ] Integration test: `cloaca build` produces valid package archive
- [ ] Integration test: Server loads Python package and discovers tasks
- [ ] Integration test: Task execution with context passing works end-to-end
- [ ] Integration test: Cross-package dependencies (Python task depends on Rust task)
- [ ] Documentation: README for the example project
- [ ] CI integration: Tests run on Linux and macOS

## Implementation Notes

### Example Project Structure

```
examples/features/python-workflow/
├── pyproject.toml           # Project metadata and dependencies
├── README.md                # Documentation for the example
├── src/
│   └── data_pipeline/
│       ├── __init__.py
│       └── tasks.py         # @task decorated functions
└── tests/
    └── test_local.py        # Local unit tests for tasks
```

### Example pyproject.toml

```toml
# examples/features/python-workflow/pyproject.toml

[project]
name = "data-pipeline-example"
version = "1.0.0"
description = "Example Python workflow for Cloacina"
requires-python = ">=3.11"
dependencies = [
    "httpx>=0.25.0",
    "pydantic>=2.0.0",
]

[project.optional-dependencies]
dev = [
    "pytest>=7.0.0",
    "pytest-asyncio>=0.21.0",
]

[tool.cloaca]
entry_module = "data_pipeline.tasks"

[tool.cloaca.tasks]
# Task metadata can also be defined here as alternative to decorators
# "fetch-data" = { dependencies = [] }
# "transform-data" = { dependencies = ["fetch-data"] }
```

### Example Tasks Implementation

```python
# examples/features/python-workflow/src/data_pipeline/tasks.py

from cloaca import task
import httpx
from pydantic import BaseModel
from typing import Any


class DataRecord(BaseModel):
    id: int
    name: str
    value: float


@task(id="fetch-data", dependencies=[])
async def fetch_data(context: dict[str, Any]) -> dict[str, Any]:
    """
    Fetch data from an API endpoint.

    Expects:
        context["api_url"]: str - URL to fetch from

    Produces:
        context["raw_data"]: list[dict] - Raw API response
    """
    api_url = context.get("api_url", "https://api.example.com/data")

    async with httpx.AsyncClient() as client:
        response = await client.get(api_url)
        response.raise_for_status()

    context["raw_data"] = response.json()
    context["fetch_timestamp"] = datetime.utcnow().isoformat()
    return context


@task(id="validate-data", dependencies=["fetch-data"])
def validate_data(context: dict[str, Any]) -> dict[str, Any]:
    """
    Validate and parse raw data into typed records.

    Expects:
        context["raw_data"]: list[dict] - Raw data from API

    Produces:
        context["validated_records"]: list[dict] - Validated records
        context["validation_errors"]: list[str] - Any validation errors
    """
    raw_data = context["raw_data"]
    validated = []
    errors = []

    for i, item in enumerate(raw_data):
        try:
            record = DataRecord(**item)
            validated.append(record.model_dump())
        except Exception as e:
            errors.append(f"Record {i}: {e}")

    context["validated_records"] = validated
    context["validation_errors"] = errors
    return context


@task(id="aggregate-data", dependencies=["validate-data"])
def aggregate_data(context: dict[str, Any]) -> dict[str, Any]:
    """
    Compute aggregations on validated data.

    Expects:
        context["validated_records"]: list[dict] - Validated records

    Produces:
        context["aggregations"]: dict - Computed statistics
    """
    records = context["validated_records"]

    if not records:
        context["aggregations"] = {"count": 0, "sum": 0, "avg": 0}
        return context

    values = [r["value"] for r in records]
    context["aggregations"] = {
        "count": len(values),
        "sum": sum(values),
        "avg": sum(values) / len(values),
        "min": min(values),
        "max": max(values),
    }
    return context


@task(id="generate-report", dependencies=["aggregate-data"])
def generate_report(context: dict[str, Any]) -> dict[str, Any]:
    """
    Generate a summary report.

    Expects:
        context["aggregations"]: dict - Computed statistics
        context["validation_errors"]: list[str] - Any errors

    Produces:
        context["report"]: str - Human-readable report
    """
    agg = context["aggregations"]
    errors = context.get("validation_errors", [])

    report_lines = [
        "=== Data Pipeline Report ===",
        f"Records processed: {agg['count']}",
        f"Sum: {agg['sum']:.2f}",
        f"Average: {agg['avg']:.2f}",
        f"Range: {agg.get('min', 0):.2f} - {agg.get('max', 0):.2f}",
    ]

    if errors:
        report_lines.append(f"\nValidation errors: {len(errors)}")
        for err in errors[:5]:  # Show first 5
            report_lines.append(f"  - {err}")

    context["report"] = "\n".join(report_lines)
    return context
```

### Integration Test: Package Building

```python
# tests/integration/python_package/test_build.py

import subprocess
import tempfile
import tarfile
import json
from pathlib import Path
import pytest


EXAMPLE_PROJECT = Path(__file__).parent.parent.parent.parent / "examples/features/python-workflow"


@pytest.fixture
def build_output_dir():
    with tempfile.TemporaryDirectory() as tmpdir:
        yield Path(tmpdir)


def test_cloaca_build_produces_package(build_output_dir):
    """Test that cloaca build creates a valid package archive."""
    result = subprocess.run(
        ["cloaca", "build", "-o", str(build_output_dir)],
        cwd=EXAMPLE_PROJECT,
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0, f"Build failed: {result.stderr}"

    # Find the output package
    packages = list(build_output_dir.glob("*.cloaca"))
    assert len(packages) == 1, f"Expected 1 package, found {len(packages)}"

    package_path = packages[0]
    assert "data-pipeline-example" in package_path.name


def test_package_contains_required_files(build_output_dir):
    """Test that the package archive has correct structure."""
    subprocess.run(
        ["cloaca", "build", "-o", str(build_output_dir)],
        cwd=EXAMPLE_PROJECT,
        check=True,
    )

    package_path = next(build_output_dir.glob("*.cloaca"))

    with tarfile.open(package_path, "r:gz") as tar:
        names = tar.getnames()

        # Required files
        assert "manifest.json" in names
        assert "requirements.lock" in names

        # Source directory
        assert any(n.startswith("src/") for n in names)

        # Vendor directory (if dependencies exist)
        # Note: may be empty if no deps


def test_manifest_is_valid_json(build_output_dir):
    """Test that manifest.json is valid and contains expected fields."""
    subprocess.run(
        ["cloaca", "build", "-o", str(build_output_dir)],
        cwd=EXAMPLE_PROJECT,
        check=True,
    )

    package_path = next(build_output_dir.glob("*.cloaca"))

    with tarfile.open(package_path, "r:gz") as tar:
        manifest_file = tar.extractfile("manifest.json")
        manifest = json.load(manifest_file)

    # Check required fields
    assert manifest["format_version"] == "2"
    assert manifest["language"] == "python"
    assert "package" in manifest
    assert manifest["package"]["name"] == "data-pipeline-example"

    # Check Python config
    assert "python" in manifest
    assert manifest["python"]["entry_module"] == "data_pipeline.tasks"

    # Check tasks
    assert "tasks" in manifest
    task_ids = [t["id"] for t in manifest["tasks"]]
    assert "fetch-data" in task_ids
    assert "validate-data" in task_ids
    assert "aggregate-data" in task_ids
    assert "generate-report" in task_ids


def test_build_for_multiple_platforms(build_output_dir):
    """Test building for multiple target platforms."""
    result = subprocess.run(
        [
            "cloaca", "build",
            "-o", str(build_output_dir),
            "--target", "linux-x86_64",
            "--target", "macos-arm64",
        ],
        cwd=EXAMPLE_PROJECT,
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0, f"Build failed: {result.stderr}"

    packages = list(build_output_dir.glob("*.cloaca"))
    assert len(packages) == 2

    names = [p.name for p in packages]
    assert any("linux-x86_64" in n for n in names)
    assert any("macos-arm64" in n for n in names)
```

### Integration Test: Server Loading

```rust
// crates/cloacina/tests/integration/python_package/loader_test.rs

use cloacina::registry::loader::{PackageLoaderFactory, LoadedPackage};
use std::path::PathBuf;
use tempfile::TempDir;

fn get_test_package() -> Vec<u8> {
    // Read pre-built test package from fixtures
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/python-workflow.cloaca");
    std::fs::read(fixture_path).expect("Test fixture not found")
}

#[test]
fn test_load_python_package() {
    let staging_dir = TempDir::new().unwrap();
    let factory = PackageLoaderFactory::new(staging_dir.path().to_path_buf());

    let package_data = get_test_package();
    let loaded = factory.load(&package_data).expect("Failed to load package");

    match loaded {
        LoadedPackage::Python(pkg) => {
            assert_eq!(pkg.entry_module, "data_pipeline.tasks");
            assert!(pkg.source_dir.exists());
        }
        LoadedPackage::Rust(_) => panic!("Expected Python package"),
    }
}

#[test]
fn test_discover_python_tasks() {
    let staging_dir = TempDir::new().unwrap();
    let factory = PackageLoaderFactory::new(staging_dir.path().to_path_buf());

    let package_data = get_test_package();
    let loaded = factory.load(&package_data).unwrap();

    let manifest = loaded.manifest();
    let task_ids: Vec<&str> = manifest.tasks.iter()
        .map(|t| t.id.as_str())
        .collect();

    assert!(task_ids.contains(&"fetch-data"));
    assert!(task_ids.contains(&"validate-data"));
    assert!(task_ids.contains(&"aggregate-data"));
    assert!(task_ids.contains(&"generate-report"));
}

#[test]
fn test_task_dependencies_parsed() {
    let staging_dir = TempDir::new().unwrap();
    let factory = PackageLoaderFactory::new(staging_dir.path().to_path_buf());

    let package_data = get_test_package();
    let loaded = factory.load(&package_data).unwrap();

    let manifest = loaded.manifest();

    let validate_task = manifest.tasks.iter()
        .find(|t| t.id == "validate-data")
        .expect("validate-data task not found");

    assert_eq!(validate_task.dependencies, vec!["fetch-data"]);
}
```

### Integration Test: End-to-End Execution

```rust
// crates/cloacina/tests/integration/python_package/execution_test.rs

use cloacina::{Context, TaskRegistry, WorkflowBuilder};
use cloacina::registry::loader::PackageLoaderFactory;
use serde_json::json;
use tempfile::TempDir;

#[tokio::test]
async fn test_python_task_execution_e2e() {
    let staging_dir = TempDir::new().unwrap();
    let factory = PackageLoaderFactory::new(staging_dir.path().to_path_buf());

    // Load package
    let package_data = get_test_package();
    let loaded = factory.load(&package_data).unwrap();

    // Build workflow
    let mut registry = TaskRegistry::new();
    register_package_tasks(&mut registry, &loaded).unwrap();

    // Create context with test data
    let mut context = Context::new();
    context.insert("raw_data", json!([
        {"id": 1, "name": "test1", "value": 10.0},
        {"id": 2, "name": "test2", "value": 20.0},
        {"id": 3, "name": "test3", "value": 30.0},
    ])).unwrap();

    // Execute workflow (skip fetch-data since we provided raw_data)
    let workflow = WorkflowBuilder::new()
        .with_registry(registry)
        .build()
        .unwrap();

    let result = workflow.execute(context).await.unwrap();

    // Verify results
    let aggregations = result.get("aggregations").unwrap();
    assert_eq!(aggregations["count"], 3);
    assert_eq!(aggregations["sum"], 60.0);
    assert_eq!(aggregations["avg"], 20.0);

    let report = result.get("report").unwrap().as_str().unwrap();
    assert!(report.contains("Records processed: 3"));
}

#[tokio::test]
async fn test_python_task_error_handling() {
    let staging_dir = TempDir::new().unwrap();
    let factory = PackageLoaderFactory::new(staging_dir.path().to_path_buf());

    let package_data = get_test_package();
    let loaded = factory.load(&package_data).unwrap();

    let mut registry = TaskRegistry::new();
    register_package_tasks(&mut registry, &loaded).unwrap();

    // Create context WITHOUT required raw_data
    let context = Context::new();

    let workflow = WorkflowBuilder::new()
        .with_registry(registry)
        .start_from_task("validate-data")  // Skip fetch, missing data
        .build()
        .unwrap();

    let result = workflow.execute(context).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("KeyError") || err.to_string().contains("raw_data"));
}
```

### CI Configuration

```yaml
# .github/workflows/python-package-tests.yml (additions)

python-package-integration:
  runs-on: ${{ matrix.os }}
  strategy:
    matrix:
      os: [ubuntu-latest, macos-latest]
      python-version: ["3.11", "3.12"]

  steps:
    - uses: actions/checkout@v4

    - name: Set up Python
      uses: actions/setup-python@v5
      with:
        python-version: ${{ matrix.python-version }}

    - name: Install uv
      run: curl -LsSf https://astral.sh/uv/install.sh | sh

    - name: Install cloaca CLI
      run: pip install -e python/cloaca

    - name: Build example package
      run: |
        cd examples/features/python-workflow
        cloaca build -o dist/

    - name: Run Rust integration tests
      run: cargo test --package cloacina --test python_package
```

### Example Project README

```markdown
# Python Workflow Example

This example demonstrates how to create a Cloacina workflow using Python.

## Prerequisites

- Python 3.11+
- uv (for dependency management)
- cloaca CLI

## Project Structure

```
src/data_pipeline/
├── __init__.py
└── tasks.py      # @task decorated functions
```

## Tasks

1. **fetch-data**: Fetches data from an API endpoint
2. **validate-data**: Validates and parses raw data
3. **aggregate-data**: Computes statistics
4. **generate-report**: Creates a summary report

## Building

```bash
# Build for current platform
cloaca build

# Build for specific platforms
cloaca build --target linux-x86_64 --target macos-arm64

# Output to specific directory
cloaca build -o dist/
```

## Local Testing

```bash
# Install dev dependencies
uv pip install -e ".[dev]"

# Run tests
pytest tests/
```

## Deploying

Upload the built `.cloaca` package to your Cloacina server:

```bash
cloacina package upload dist/data-pipeline-example-1.0.0-linux-x86_64.cloaca
```
```

### Technical Dependencies

- **T-0066**: Manifest schema for task definitions
- **T-0067**: `cloaca build` CLI command
- **T-0068**: Dependency vendoring
- **T-0069**: Server-side package loader
- **T-0070**: Python task executor

### Risk Considerations

1. **Test isolation**: Python import caching may cause test interference. Use fresh interpreters.
2. **Fixture management**: Pre-built test packages need rebuilding when format changes.
3. **CI matrix**: Cross-platform testing increases CI time. Consider caching.
4. **Mock vs real**: Some tests may need mock HTTP server for fetch-data task.

## Status Updates

*To be added during implementation*
