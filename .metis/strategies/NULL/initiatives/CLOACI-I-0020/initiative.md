---
id: cloaca-workflow-package-format
level: initiative
title: "Cloaca Workflow Package Format"
short_code: "CLOACI-I-0020"
created_at: 2026-01-28T05:21:08.781117+00:00
updated_at: 2026-01-28T18:42:06.484436+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
strategy_id: NULL
initiative_id: cloaca-workflow-package-format
---

# Cloaca Workflow Package Format Initiative

## Context

Cloacina currently supports Rust workflows via the `#[packaged_workflow]` macro, compiled to `.cloacina` packages (tar.gz with .so + manifest). Python workflows authored with the `cloaca` bindings need an equivalent packaging and distribution mechanism.

Key requirements identified:
- **Reproducibility**: Workflows must execute identically regardless of when/where deployed
- **Dependency isolation**: Each workflow brings its own dependencies (no cross-workflow conflicts)
- **Wheel-style packaging**: Vendor dependencies from lock file rather than requiring server-side resolution

The server provides the Python interpreter; packages bring isolated dependencies.

## Goals & Non-Goals

**Goals:**
- Define `.cloacina` package format for Python workflows
- Implement `cloaca build` command to create packages from Python projects
- Support wheel-style vendored dependencies from lock file
- Integrate with existing PackageLoader/registry infrastructure
- Enable local testing before deployment

**Non-Goals:**
- Compiling Python to native code (PyInstaller, Nuitka, etc.)
- Supporting Python 2.x
- Server-side dependency resolution (packages are self-contained)
- Hot-reloading of Python code in production

## Architecture

### Package Structure

```
my_workflow.cloacina (tar.gz)
├── manifest.json              # Package metadata (unified with Rust format)
├── workflow/                  # User's Python source
│   ├── __init__.py
│   ├── tasks.py              # Task definitions with @task decorator
│   └── helpers.py            # Supporting code
├── vendor/                    # Vendored dependencies (extracted wheels)
│   ├── requests/
│   ├── pandas/
│   └── ...
└── requirements.lock          # Locked dependency versions (for audit/rebuild)
```

### Manifest Schema (Python extension)

```json
{
  "format_version": "1",
  "package": {
    "name": "my-etl-workflow",
    "version": "1.2.0",
    "language": "python",
    "fingerprint": "sha256:abc123..."
  },
  "runtime": {
    "python": {
      "version": ">=3.10,<4.0",
      "entry_module": "workflow.tasks"
    }
  },
  "tasks": [
    {
      "id": "extract",
      "function": "workflow.tasks:extract_data",
      "dependencies": [],
      "description": "Extract data from source"
    },
    {
      "id": "transform",
      "function": "workflow.tasks:transform_data",
      "dependencies": ["extract"],
      "description": "Transform extracted data"
    }
  ]
}
```

### Build Process

```
┌─────────────────────────────────────────────────────────────┐
│ Developer Machine                                           │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  pyproject.toml          cloaca build                       │
│  requirements.lock   ──────────────────►  my_workflow.cloacina
│  workflow/                                                  │
│                                                             │
│  Steps:                                                     │
│  1. Parse pyproject.toml for metadata                       │
│  2. Discover @task decorated functions                      │
│  3. Resolve deps from lock file                             │
│  4. Download/extract wheels to vendor/                      │
│  5. Generate manifest.json                                  │
│  6. Create tar.gz archive                                   │
└─────────────────────────────────────────────────────────────┘
```

### Server-Side Loading

```
┌─────────────────────────────────────────────────────────────┐
│ Server (cloacina-server)                                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  PackageLoader                                              │
│  ├── Detect language from manifest                          │
│  ├── For Python packages:                                   │
│  │   ├── Extract to isolated directory                      │
│  │   ├── Create virtualenv or sys.path isolation            │
│  │   ├── Import entry module                                │
│  │   └── Register tasks with executor                       │
│  └── Workflow ready for execution                           │
│                                                             │
│  Task Execution:                                            │
│  ├── Server provides Context (via cloaca bindings)          │
│  ├── Calls Python task function                             │
│  └── Receives updated Context                               │
└─────────────────────────────────────────────────────────────┘
```

## Detailed Design

### Task Definition (Python side)

```python
# workflow/tasks.py
from cloaca import task, Context

@task(id="extract", dependencies=[])
async def extract_data(ctx: Context) -> None:
    """Extract data from configured source."""
    source = ctx.get("source_url")
    data = await fetch_data(source)
    ctx.insert("raw_data", data)

@task(id="transform", dependencies=["extract"])
async def transform_data(ctx: Context) -> None:
    """Transform the extracted data."""
    raw = ctx.get("raw_data")
    transformed = process(raw)
    ctx.insert("transformed_data", transformed)
```

### pyproject.toml Configuration

```toml
[project]
name = "my-etl-workflow"
version = "1.2.0"
requires-python = ">=3.10"

[project.dependencies]
requests = "^2.28"
pandas = "^2.0"

[tool.cloaca]
entry_module = "workflow.tasks"
description = "ETL workflow for daily data processing"
author = "Data Team"
```

### cloaca CLI Commands

```bash
# Build a package
cloaca build                    # Creates my-etl-workflow.cloacina

# Build with specific output
cloaca build -o dist/          # Output to dist/ directory

# Inspect a package
cloaca inspect my_workflow.cloacina

# Test locally before deployment
cloaca run my_workflow.cloacina --context '{"source_url": "..."}'

# Generate/update lock file
cloaca lock                     # Creates/updates requirements.lock
```

### Platform-Specific Wheels

For packages with native extensions (numpy, pandas, etc.):

**Option A: Build on server (deferred)**
- Server rebuilds platform-specific wheels on first load
- Requires build tools on server
- Complexity: High

**Option B: Multi-platform packages (recommended for v1)**
- Package includes wheels for target platforms
- `cloaca build --platform linux-x86_64`
- Larger packages but simpler deployment

**Option C: Server-provided base packages**
- Common heavy deps (numpy, pandas) pre-installed on server
- Workflows can use but not override versions
- Trade-off: Less isolation, simpler packages

Recommended: Start with Option B, evaluate Option C for common cases.

### Isolation Strategy

Each workflow loads into isolated environment:

```python
# Server-side pseudocode
def load_python_package(package_path: Path) -> LoadedWorkflow:
    # Extract package
    extract_dir = temp_dir / package.fingerprint
    extract_archive(package_path, extract_dir)

    # Isolate via sys.path manipulation
    old_path = sys.path.copy()
    sys.path.insert(0, str(extract_dir / "vendor"))
    sys.path.insert(0, str(extract_dir / "workflow"))

    try:
        # Import entry module
        entry = importlib.import_module(manifest.runtime.python.entry_module)

        # Discover and register tasks
        tasks = discover_tasks(entry)
        return LoadedWorkflow(tasks=tasks, cleanup=lambda: cleanup(extract_dir))
    finally:
        sys.path = old_path
```

## Alternatives Considered

### Alternative A: Source-only packages (no vendored deps)

- **Pros**: Smaller packages, server manages dependencies
- **Cons**: Version drift, server needs pip/network access, reproducibility issues
- **Decision**: Rejected. Lock file + vendored deps ensures reproducibility.

### Alternative B: Compiled Python (PyInstaller/Nuitka)

- **Pros**: Single binary, fast startup, no interpreter needed
- **Cons**: Huge binaries, platform-specific, complex build
- **Decision**: Rejected. Server provides interpreter; not worth the complexity.

### Alternative C: Container-per-workflow

- **Pros**: Perfect isolation, any runtime
- **Cons**: Resource overhead, slow startup, complex orchestration
- **Decision**: Rejected for v1. May revisit for heavy isolation requirements.

## Implementation Plan

### Phase 1: Package Format & Build
- [ ] Define manifest.json schema for Python packages
- [ ] Implement `cloaca build` command
- [ ] Implement dependency vendoring from lock file
- [ ] Add `cloaca lock` for lock file generation
- [ ] Create example Python workflow project

### Phase 2: Server-Side Loading
- [ ] Extend PackageLoader to detect Python packages
- [ ] Implement Python package extraction
- [ ] Implement sys.path isolation
- [ ] Implement task discovery from entry module
- [ ] Bridge Context between Rust and Python

### Phase 3: Execution Integration
- [ ] Register Python tasks with scheduler
- [ ] Execute Python tasks via cloaca bindings
- [ ] Handle async Python tasks
- [ ] Error handling and logging

### Phase 4: Testing & Polish
- [ ] Integration tests with Python packages
- [ ] Test dependency isolation
- [ ] Test platform-specific wheels
- [ ] Documentation for Python workflow authors

## Technology Decisions

### Lock File & Dependency Resolution

**Tool**: `uv` (https://github.com/astral-sh/uv)
- Fast, modern Python package manager
- Generates lock files with hashes
- Picking up steam in ecosystem
- Fixes many Python packaging pain points

### Wheel Vendoring

**Strategy**: Extract wheels to `vendor/` directory (unpacked site-packages style)
- Simpler sys.path manipulation
- Faster imports than zip-import
- Easier debugging
- Can switch to non-extracted later if needed

### Platform Targeting

**Approach**: Build-time flag with auto-detect default

```bash
# Default: build for current machine
cloaca build                           # auto-detects platform

# Explicit target
cloaca build --target linux-x86_64

# Multi-target (larger package)
cloaca build --target linux-x86_64 --target macos-arm64
```

**Supported targets (v1):**
- `linux-x86_64` (most servers)
- `linux-arm64` (AWS Graviton, etc.)
- `macos-arm64` (M1/M2 dev machines)
- `macos-x86_64` (older Macs)

Windows deferred.

**Wheel tag mapping:**

| Our Target | Wheel Tag Pattern |
|------------|-------------------|
| `linux-x86_64` | `manylinux*_x86_64` |
| `linux-arm64` | `manylinux*_aarch64` |
| `macos-arm64` | `macosx_*_arm64` |
| `macos-x86_64` | `macosx_*_x86_64` |

**Server validates** package targets against runtime platform at load time.

### Python Version Compatibility

- Server declares its Python version (e.g., 3.11)
- Package manifest specifies `requires_python: ">=3.10"`
- Server checks compatibility at load time
- Incompatible packages rejected with clear error

### CLI Naming

**`cloaca`** CLI for Python workflows (separate from `cloacina`)
- Minimizes cognitive burden
- Python developers use Python-named tool
- Clear separation of concerns

## Decisions Log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Dependency strategy | Wheel-style vendored via uv | Reproducibility; fast; modern tooling |
| Wheel extraction | Extracted to vendor/ | Simpler imports, easier debugging |
| Platform targeting | Auto-detect default, explicit override | Matches where built, allows cross-platform |
| Platform support | linux/macos x arm64/x86_64 | Common server + dev combos; Windows deferred |
| Python version | Server declares, package requires | Clear contract; server controls runtime |
| CLI naming | `cloaca` (not cloacina-py) | Cognitive separation for Python devs |
| Isolation | sys.path manipulation | Lightweight; avoids virtualenv overhead |
| Entry point | Module path in manifest | Explicit; discoverable; Python conventions |
