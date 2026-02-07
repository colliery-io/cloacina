---
id: cloaca-build-cli-command
level: task
title: "cloaca build CLI command implementation"
short_code: "CLOACI-T-0067"
created_at: 2026-01-28T14:29:02.217522+00:00
updated_at: 2026-01-28T18:31:16.703113+00:00
parent: CLOACI-I-0020
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0020
---

# cloaca build CLI command implementation

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0020]]

## Objective

Implement the `cloaca build` CLI command that creates `.cloacina` packages from Python projects. Parses pyproject.toml, discovers tasks, invokes dependency vendoring, generates manifest, and creates the final tar.gz archive.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `cloaca build` command creates `.cloacina` package
- [x] Parses `pyproject.toml` for package metadata
- [x] Discovers `@task` decorated functions from entry module (AST-based)
- [x] `--target` flag for platform targeting (default: auto-detect)
- [x] `--output` flag for output directory
- [x] Invokes dependency vendoring (stubbed — T-0068 provides real impl)
- [x] Generates `manifest.json`
- [x] Creates tar.gz archive with correct structure
- [x] Computes package fingerprint (SHA256)
- [x] CLI help and error messages (click-based)

## CLI Interface

```bash
# Basic usage - build for current platform
cloaca build

# Specify output directory
cloaca build -o dist/

# Explicit target platform
cloaca build --target linux-x86_64

# Multiple targets (creates multi-platform package)
cloaca build --target linux-x86_64 --target macos-arm64

# Verbose output
cloaca build -v

# Show what would be built (dry run)
cloaca build --dry-run
```

## Build Process

```python
# cloaca/cli/build.py
import click
import tarfile
import hashlib
from pathlib import Path
from datetime import datetime, timezone

from cloaca.manifest import Manifest, PackageInfo, PythonRuntime, TaskDefinition
from cloaca.discovery import discover_tasks
from cloaca.vendor import vendor_dependencies
from cloaca.platform import detect_platform, SUPPORTED_PLATFORMS

@click.command()
@click.option('-o', '--output', type=click.Path(), default='.',
              help='Output directory for the package')
@click.option('--target', 'targets', multiple=True,
              help='Target platform(s). Default: current platform')
@click.option('-v', '--verbose', is_flag=True)
@click.option('--dry-run', is_flag=True, help='Show what would be built')
def build(output: str, targets: tuple[str], verbose: bool, dry_run: bool):
    """Build a .cloacina package from the current Python project."""

    project_dir = Path.cwd()

    # 1. Parse pyproject.toml
    pyproject = parse_pyproject(project_dir / 'pyproject.toml')

    # 2. Determine targets
    if not targets:
        targets = [detect_platform()]

    for target in targets:
        if target not in SUPPORTED_PLATFORMS:
            raise click.ClickException(f"Unsupported target: {target}")

    # 3. Discover tasks from entry module
    entry_module = pyproject.get('tool', {}).get('cloaca', {}).get('entry_module')
    if not entry_module:
        raise click.ClickException("Missing [tool.cloaca] entry_module in pyproject.toml")

    tasks = discover_tasks(entry_module, project_dir)
    if not tasks:
        raise click.ClickException(f"No @task decorated functions found in {entry_module}")

    click.echo(f"Found {len(tasks)} task(s): {', '.join(t.id for t in tasks)}")

    # 4. Vendor dependencies
    if not dry_run:
        vendor_dir = project_dir / '.cloaca_build' / 'vendor'
        lock_file = vendor_dependencies(
            project_dir=project_dir,
            vendor_dir=vendor_dir,
            targets=list(targets),
            verbose=verbose,
        )

    # 5. Build manifest
    package_name = pyproject['project']['name']
    package_version = pyproject['project']['version']
    requires_python = pyproject['project'].get('requires-python', '>=3.10')

    manifest = Manifest(
        format_version="2",
        package=PackageInfo(
            name=package_name,
            version=package_version,
            description=pyproject['project'].get('description'),
            fingerprint="",  # Computed after archive creation
            targets=list(targets),
        ),
        language="python",
        python=PythonRuntime(
            requires_python=requires_python,
            entry_module=entry_module,
        ),
        tasks=tasks,
        created_at=datetime.now(timezone.utc),
    )

    if dry_run:
        click.echo("\nDry run - would create package with:")
        click.echo(f"  Name: {package_name}")
        click.echo(f"  Version: {package_version}")
        click.echo(f"  Targets: {', '.join(targets)}")
        click.echo(f"  Tasks: {len(tasks)}")
        return

    # 6. Create archive
    output_dir = Path(output)
    output_dir.mkdir(parents=True, exist_ok=True)

    archive_name = f"{package_name}-{package_version}.cloacina"
    archive_path = output_dir / archive_name

    build_dir = project_dir / '.cloaca_build'

    # Copy workflow source
    workflow_dir = build_dir / 'workflow'
    copy_workflow_source(project_dir, workflow_dir, entry_module)

    # Write manifest
    manifest_path = build_dir / 'manifest.json'
    manifest_path.write_text(manifest.model_dump_json(indent=2))

    # Copy lock file
    (build_dir / 'requirements.lock').write_text(lock_file.read_text())

    # Create tar.gz
    with tarfile.open(archive_path, 'w:gz') as tar:
        tar.add(manifest_path, arcname='manifest.json')
        tar.add(workflow_dir, arcname='workflow')
        tar.add(vendor_dir, arcname='vendor')
        tar.add(build_dir / 'requirements.lock', arcname='requirements.lock')

    # 7. Compute fingerprint and update manifest
    fingerprint = f"sha256:{compute_sha256(archive_path)}"
    manifest.package.fingerprint = fingerprint

    # Rewrite archive with fingerprint
    # (Or embed fingerprint differently - TBD)

    click.echo(f"\nCreated: {archive_path}")
    click.echo(f"Fingerprint: {fingerprint}")


def compute_sha256(path: Path) -> str:
    sha256 = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(8192), b''):
            sha256.update(chunk)
    return sha256.hexdigest()
```

## pyproject.toml Parsing

```python
# cloaca/pyproject.py
import tomllib
from pathlib import Path

def parse_pyproject(path: Path) -> dict:
    """Parse pyproject.toml and validate required fields."""

    if not path.exists():
        raise ValueError(f"pyproject.toml not found at {path}")

    with open(path, 'rb') as f:
        data = tomllib.load(f)

    # Validate required fields
    if 'project' not in data:
        raise ValueError("Missing [project] section in pyproject.toml")

    project = data['project']
    for field in ['name', 'version']:
        if field not in project:
            raise ValueError(f"Missing project.{field} in pyproject.toml")

    if 'tool' not in data or 'cloaca' not in data['tool']:
        raise ValueError("Missing [tool.cloaca] section in pyproject.toml")

    cloaca = data['tool']['cloaca']
    if 'entry_module' not in cloaca:
        raise ValueError("Missing tool.cloaca.entry_module in pyproject.toml")

    return data
```

## Package Structure Output

```
my-workflow-1.2.0.cloacina (tar.gz)
├── manifest.json
├── workflow/
│   ├── __init__.py
│   ├── tasks.py
│   └── helpers.py
├── vendor/
│   ├── requests/
│   ├── urllib3/
│   └── ...
└── requirements.lock
```

## File Locations

- CLI entry: `crates/cloaca/cloaca/cli/__init__.py`
- Build command: `crates/cloaca/cloaca/cli/build.py`
- pyproject parser: `crates/cloaca/cloaca/pyproject.py`

## Dependencies

```toml
# pyproject.toml for cloaca package
[project.dependencies]
click = ">=8.0"
pydantic = ">=2.0"
```

## Requires

- CLOACI-T-0066 (manifest schema)
- CLOACI-T-0068 (dependency vendoring)

## Status Updates

### Completed
- Created `bindings/cloaca-backend/python/cloaca/cli/__init__.py` — click group entry point
- Created `bindings/cloaca-backend/python/cloaca/cli/build.py` — `cloaca build` command with `--output`, `--target`, `--verbose`, `--dry-run` flags; stages build dir, copies workflow source, writes manifest, creates tar.gz, computes SHA256 fingerprint
- Created `bindings/cloaca-backend/python/cloaca/pyproject.py` — `parse_pyproject()` with validation for `[project]`, `[tool.cloaca]`, `entry_module`
- Created `bindings/cloaca-backend/python/cloaca/discovery.py` — AST-based `discover_tasks()` that finds `@task` decorated functions without importing user code; extracts id, dependencies, description, retries, timeout_seconds from decorator kwargs
- Vendor directory creation is stubbed (T-0068 provides real `uv`-based vendoring)
