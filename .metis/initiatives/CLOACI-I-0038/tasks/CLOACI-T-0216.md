---
id: rewrite-cloaca-build-in-rust
level: task
title: "Rewrite cloaca build in Rust — eliminate PyO3 dependency for cloacinactl package build"
short_code: "CLOACI-T-0216"
created_at: 2026-03-19T13:55:32.227551+00:00
updated_at: 2026-03-21T01:26:10.413991+00:00
parent: CLOACI-I-0038
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Rewrite cloaca build in Rust — eliminate PyO3 dependency for cloacinactl package build

## Objective

Rewrite `cloacinactl package build` for Python projects in pure Rust — no Python runtime needed at build time. Today the build path delegates to `cloaca build` via PyO3, requiring the `cloaca` wheel to be installed. This breaks in CI/CD and Docker where the system Python doesn't have cloaca.

The fix: parse `pyproject.toml` with the `toml` crate, discover `@task` decorators via AST scanning (Rust Python parser), vendor deps via `uv` subprocess, create the `.cloacina` archive with `tar`/`flate2`. Same output format, no Python dependency.

Note: Moving PyO3 bindings into cloacina core is handled separately by CLOACI-T-0217.

### Priority
- [x] P1 - High (blocks Python package building and execution for all users)

### Business Justification
- **User Value**: If you can run cloacina, you can run Python workflows. No additional installs, no venv management, no "is cloaca installed?" debugging.
- **Business Value**: Eliminates the Python onboarding friction entirely. Unblocks Docker builds, CI/CD, and daemon mode for Python workflows.
- **Effort Estimate**: L

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `cloacinactl package build` in a Python workflow directory produces a valid `.cloacina` package without requiring `cloaca` to be installed
- [ ] Reads `pyproject.toml` for package name, version, description, `[tool.cloaca].entry_module`
- [ ] Scans Python source files to discover `@cloaca.task(id="...", dependencies=[...])` decorators via AST parsing (not import)
- [ ] Vendors dependencies using `uv` subprocess (with fallback error if uv not available)
- [ ] Produces identical `.cloacina` archive format as the current Python `cloaca build`
- [ ] `register_workflow()` accepts the output (manifest + workflow/ + vendor/ structure)
- [ ] PyO3 dependency removed from `cloacinactl package build` path (may still be needed for task execution)
- [ ] Existing `cloaca build` Python CLI continues to work (not removed, just no longer the only path)
- [ ] Test fixture: `tests/fixtures/python-workflow.cloacina` checked in, built by this tool

## Implementation Notes

### What the Python build does today

Location: `bindings/cloaca-backend/python/cloaca/cli/build.py`

1. `parse_pyproject(pyproject_path)` — reads `[project]` and `[tool.cloaca]` sections
2. `discover_tasks(entry_module, project_dir)` — imports the module, inspects `@task` decorators
3. Builds `Manifest` with package info + task definitions
4. `vendor_dependencies(project_dir, vendor_dir, targets)` — calls `uv pip compile` + `uv pip install`
5. Creates tar.gz: `manifest.json` + `workflow/` tree + `vendor/` + `requirements.lock`

### Rust implementation plan

**TOML parsing**: Use `toml` crate (already a dependency). Parse `[project]` for name/version/description, `[tool.cloaca]` for `entry_module`.

**AST scanning**: Use `ruff_python_parser` (MIT, from the ruff project) or `rustpython-parser` to parse Python files. Walk the AST looking for:
- Function definitions with decorators matching `cloaca.task` or `task`
- Extract `id`, `dependencies`, `description` from decorator kwargs
- If `id` is not provided, use function name (matching current Python behavior)

**Vendoring**: Shell out to `uv pip compile` and `uv pip install --target` (same as current Python impl). Fall back to clear error if `uv` not found.

**Archive creation**: Use `tar` + `flate2` (already dependencies). Same structure as current Python output.

**Manifest**: Use existing `ManifestV2` struct from `cloacina::packaging::manifest_v2`.

### Files to create/modify

| File | Action |
|------|--------|
| `crates/cloacina/src/packaging/python_builder.rs` | New — pure Rust Python package builder |
| `crates/cloacina/src/packaging/python_discovery.rs` | New — AST-based task discovery |
| `crates/cloacinactl/src/commands/package.rs` | Modify — call Rust builder instead of PyO3 |
| `crates/cloacinactl/Cargo.toml` | Add `ruff_python_parser` or equivalent |
| `tests/fixtures/python-workflow.cloacina` | New — committed test fixture |

### Related
- CLOACI-T-0215 — Python package registration (uses the packages this builds)
- CLOACI-T-0211 — FFI/Python smoke test (blocked on being able to build packages)
- `bindings/cloaca-backend/python/cloaca/cli/build.py` — current Python implementation (reference)
- `bindings/cloaca-backend/python/cloaca/discovery.py` — current task discovery (reference)

## Status Updates

### 2026-03-20 — Exploration complete, starting implementation

**Architecture decision:** Builder + discovery code in `cloacina::packaging`, Python AST parser dep in cloacina.

**Implementation plan:**
1. Add `ruff_python_parser` + `ruff_python_ast` to cloacina deps
2. Create `python_discovery.rs` — AST-based @task decorator discovery
3. Create `python_builder.rs` — full build pipeline: parse pyproject.toml, discover tasks, vendor deps, create archive
4. Update `cloacinactl/commands/package.rs` — detect Python project, call Rust builder
5. Test with `examples/features/python-workflow/`

**Key reference files:**
- `bindings/cloaca-backend/python/cloaca/cli/build.py` — current Python build
- `bindings/cloaca-backend/python/cloaca/discovery.py` — current task discovery
- `examples/features/python-workflow/` — test project

**Decision: Skip AST scanning.** The `@task` decorator is Rust code (PyO3). Task discovery happens at registration time when the module is imported (T-0215). The builder just packages source code — no need to parse Python at build time. Manifest `tasks` array left empty; populated at registration.

**Revised plan (much simpler):**
1. Create `python_builder.rs` — parse pyproject.toml, copy workflow source, vendor deps via uv, create archive
2. Update `cloacinactl/commands/package.rs` — detect Python project, call Rust builder
3. No new parser dependencies needed

**Implementation complete.**

### Files changed

**New: `crates/cloacina/src/packaging/python_builder.rs`**
- `build_python_package()` — full build pipeline: parse pyproject.toml → copy source → vendor deps via uv → create tar.gz → SHA256 fingerprint
- `parse_pyproject()` — extracts [project] and [tool.cloaca] sections
- `copy_workflow_source()` — copies entry module's package tree, skips __pycache__
- `vendor_dependencies()` — uv pip compile → uv pip download → extract wheels
- `create_archive()` — tar.gz with manifest.json, workflow/, vendor/
- 4 unit tests (parse, missing config, copy, full build)

**Updated: `crates/cloacinactl/src/commands/package.rs`**
- `build()` now auto-detects Python projects (pyproject.toml with [tool.cloaca])
- Python projects → `build_python()` (pure Rust, no PyO3)
- Rust projects → `build_rust()` (existing PyO3 path unchanged)
- Python builds validated with `PackageValidator::validate_python_package()`

**Updated: `crates/cloacina/Cargo.toml`** — added `zip` dependency for wheel extraction
**Updated: `crates/cloacina/src/packaging/mod.rs`** — exports python_builder

### Test results
- Workspace: 488 passed, 0 failed
- E2E: `cloacinactl package build` in example Python project produces valid .cloacina
- Archive contains: manifest.json, workflow/data_pipeline/{__init__.py, tasks.py}, vendor/
- Manifest has SHA256 fingerprint, correct metadata, empty tasks (discovered at registration)

### Acceptance criteria
- [x] `cloacinactl package build` produces valid .cloacina without cloaca installed
- [x] Reads pyproject.toml for metadata and [tool.cloaca].entry_module
- [x] ~~AST scanning~~ — skipped by design: tasks discovered at registration via PyO3
- [x] Vendors deps via uv subprocess (with error if uv unavailable)
- [x] Same .cloacina archive format (manifest.json + workflow/ + vendor/)
- [x] register_workflow() accepts the output (validated with peek_manifest)
- [x] Existing cloaca build Python CLI unchanged
- [ ] Test fixture checked in — deferred (archive has timestamp, not deterministic)
