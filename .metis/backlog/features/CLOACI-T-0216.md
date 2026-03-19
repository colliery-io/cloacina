---
id: rewrite-cloaca-build-in-rust
level: task
title: "Rewrite cloaca build in Rust — eliminate PyO3 dependency for cloacinactl package build"
short_code: "CLOACI-T-0216"
created_at: 2026-03-19T13:55:32.227551+00:00
updated_at: 2026-03-19T13:55:32.227551+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Rewrite cloaca build in Rust — eliminate PyO3 dependency for cloacinactl package build

## Objective

`cloacinactl package build` for Python workflows currently delegates to `cloaca.cli.build` via PyO3. This means the system Python that cloacinactl links against must have the `cloaca` package installed — a hidden dependency that fails with a confusing "No module named 'cloaca'" error. Users shouldn't need to install anything beyond `cloacinactl` to build packages.

Rewrite the Python package builder in pure Rust. The build process is entirely file-based — no Python runtime needed:

1. Parse `pyproject.toml` for package metadata and `[tool.cloaca]` config
2. Scan Python AST for `@cloaca.task()` decorated functions
3. Vendor dependencies (call `uv` as a subprocess)
4. Create tar.gz archive with `manifest.json` + `workflow/` + `vendor/`

### Priority
- [x] P1 - High (blocks Python package building for all users)

### Business Justification
- **User Value**: `cloacinactl package build` just works for Python workflows without any Python package installation
- **Business Value**: Eliminates the #1 Python onboarding friction point. Also unblocks Docker-based builds (Dockerfile.soak) and CI/CD where managing Python venvs is impractical.
- **Effort Estimate**: M

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

*To be added during implementation*
