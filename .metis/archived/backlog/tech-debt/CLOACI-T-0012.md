---
id: refactor-repo-structure-move
level: task
title: "Refactor Repo Structure - Move Crates to crates/ directory"
short_code: "CLOACI-T-0012"
created_at: 2025-12-04T00:01:12.808096+00:00
updated_at: 2025-12-05T22:34:25.145577+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: NULL
---

# Refactor Repo Structure - Move Crates to crates/ directory

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

Reorganize the repository structure to follow Rust workspace conventions by moving all crates into a `crates/` directory. This improves discoverability, separates library code from configuration/tooling, and aligns with common Rust project patterns.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P2 - Medium (nice to have)

### Technical Debt Impact
- **Current Problems**:
  - Crates are scattered at repository root level mixed with config files
  - Hard to distinguish between library crates and tooling/examples
  - Non-standard layout makes navigation harder for contributors
- **Benefits of Fixing**:
  - Clear separation: `crates/` for libraries, `examples/` for demos, root for config
  - Easier to understand project structure at a glance
  - Better alignment with Rust ecosystem conventions
- **Risk Assessment**: Low risk - mostly path updates in Cargo.toml files and CI

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All library crates moved to `crates/` directory
- [ ] Workspace Cargo.toml updated with new paths
- [ ] CI workflows updated to reference new paths
- [ ] All tests pass after restructure
- [ ] Documentation updated to reflect new structure

## Implementation Notes

### Final Structure

```
cloacina/
  # ─── CRATES ───────────────────────────────────────
  crates/
    cloacina/                  # Main library
    cloacina-macros/           # Proc macros
    # cloacina-workflow/       # Added by T-0013

  # ─── BINDINGS ─────────────────────────────────────
  bindings/
    cloaca-backend/            # Python bindings (keep name for PyPI distribution)

  # ─── EXAMPLES ─────────────────────────────────────
  examples/
    tutorials/                 # Learning path
      01-basic-workflow/       # (was tutorial-01)
      02-multi-task/           # (was tutorial-02)
      03-dependencies/         # (was tutorial-03)
      04-error-handling/       # (was tutorial-04)
      05-advanced/             # (was tutorial-05)
      06-multi-tenancy/        # (was tutorial-06)
    features/                  # Feature showcases
      complex-dag/             # (was complex-dag-example)
      cron-scheduling/
      multi-tenant/            # (was multi_tenant)
      packaged-workflows/      # (was packaged-workflow-example)
      per-tenant-credentials/
      registry-execution/      # (was registry-execution-demo)
      simple-packaged/         # (was simple-packaged-demo)
      validation-failures/
    performance/               # Benchmarks
      simple/                  # (was performance-simple)
      parallel/                # (was performance-parallel)
      pipeline/                # (was performance-pipeline)

  # ─── TESTING ──────────────────────────────────────
  tests/
    python/                    # (was python-tests/)

  # ─── DOCUMENTATION ────────────────────────────────
  docs/                        # Hugo site (unchanged)
  ARCHITECTURE.md              # NEW: High-level architecture guide

  # ─── TOOLING ──────────────────────────────────────
  .angreal/                    # Task automation
  .github/                     # CI workflows
  docker/                      # Docker configs

  # ─── CONFIG ───────────────────────────────────────
  Cargo.toml, LICENSE, README.md, etc.
```

### Technical Approach

#### Phase 1: Crate Reorganization
1. Create `crates/` directory
2. Move `cloacina/` -> `crates/cloacina/`
3. Move `cloacina-macros/` -> `crates/cloacina-macros/`
4. Update workspace `Cargo.toml`:
   ```toml
   [workspace]
   members = ["crates/cloacina", "crates/cloacina-macros"]
   exclude = ["examples/*", "bindings/*"]
   ```
5. Update inter-crate dependency paths in Cargo.toml files

#### Phase 2: Bindings Reorganization
1. Create `bindings/` directory
2. Move `cloaca-backend/` -> `bindings/cloaca-backend/`
3. Update any path references in cloaca-backend

#### Phase 3: Example Reorganization
1. Create `examples/tutorials/`, `examples/features/`, `examples/performance/`
2. Move and rename examples:
   - `tutorial-0X/` -> `examples/tutorials/0X-descriptive-name/`
   - Feature examples -> `examples/features/`
   - Performance examples -> `examples/performance/`
3. Update example Cargo.toml dependency paths
4. Update any documentation references

#### Phase 4: Test Reorganization
1. Create `tests/` directory
2. Move `python-tests/` -> `tests/python/`
3. Update pytest configuration paths
4. Update CI workflow paths

#### Phase 5: Documentation
1. Create `ARCHITECTURE.md` with:
   - Crate dependency graph
   - Key concepts (Task, Workflow, Context, Registry, DAL)
   - Module overview for each crate
   - Entry points for common tasks
2. Update README.md with new structure
3. Update Hugo docs if needed

### Files Requiring Updates

**Cargo Files:**
- `Cargo.toml` (workspace members, excludes)
- `crates/cloacina/Cargo.toml` (path to cloacina-macros)
- `crates/cloacina-macros/Cargo.toml` (if any path deps)
- All example `Cargo.toml` files (path to cloacina)
- `bindings/cloaca-backend/Cargo.toml` (path to cloacina)

**CI Workflows:**
- `.github/workflows/ci.yml`
- `.github/workflows/cloacina.yml`
- `.github/workflows/cloaca-matrix.yml`
- `.github/workflows/performance.yml`
- `.github/workflows/docs.yml`
- `.github/workflows/examples-docs.yml`
- `.github/workflows/unified_release.yml`

**Angreal Scripts:**
- `.angreal/task_check.py`
- `.angreal/task_docs.py`
- `.angreal/utils.py`
- `.angreal/demos/rust_demos.py`
- Any other scripts with hardcoded paths

**Documentation:**
- `README.md` (structure section)
- `docs/` Hugo content referencing paths

**Python Testing:**
- `tests/python/conftest.py` (if any path references)
- pytest.ini or pyproject.toml (if exists)

### Example Renaming Map

| Current | New Location |
|---------|--------------|
| `tutorial-01/` | `examples/tutorials/01-basic-workflow/` |
| `tutorial-02/` | `examples/tutorials/02-multi-task/` |
| `tutorial-03/` | `examples/tutorials/03-dependencies/` |
| `tutorial-04/` | `examples/tutorials/04-error-handling/` |
| `tutorial-05/` | `examples/tutorials/05-advanced/` |
| `tutorial-06/` | `examples/tutorials/06-multi-tenancy/` |
| `complex-dag-example/` | `examples/features/complex-dag/` |
| `cron-scheduling/` | `examples/features/cron-scheduling/` |
| `multi_tenant/` | `examples/features/multi-tenant/` |
| `packaged-workflow-example/` | `examples/features/packaged-workflows/` |
| `per_tenant_credentials/` | `examples/features/per-tenant-credentials/` |
| `registry-execution-demo/` | `examples/features/registry-execution/` |
| `simple-packaged-demo/` | `examples/features/simple-packaged/` |
| `validation_failures/` | `examples/features/validation-failures/` |
| `performance-simple/` | `examples/performance/simple/` |
| `performance-parallel/` | `examples/performance/parallel/` |
| `performance-pipeline/` | `examples/performance/pipeline/` |

## Status Updates **[REQUIRED]**

### 2025-12-04: Comprehensive Restructure Plan
- Expanded scope to include examples, bindings, tests reorganization
- Designed logical grouping for 17 examples into tutorials/features/performance
- Added ARCHITECTURE.md to improve onboarding
- Created detailed implementation phases and file update checklist

## Related Future Work: Large File Splitting

**Deferred from this task** - Recommend creating a separate task for splitting large source files to improve maintainability:

### Files to Consider Splitting

| File | Lines | Suggested Split |
|------|-------|-----------------|
| `workflow.rs` | 1,811 | `workflow/mod.rs`, `workflow/builder.rs`, `workflow/validation.rs`, `workflow/metadata.rs` |
| `task_scheduler.rs` | 1,730 | `scheduler/mod.rs`, `scheduler/engine.rs`, `scheduler/orchestration.rs`, `scheduler/state.rs` |
| `default_runner.rs` | 1,728 | `runner/mod.rs`, `runner/config.rs`, `runner/execution.rs`, `runner/lifecycle.rs` |
| `task_execution.rs` (dal) | 1,622 | Consider splitting CRUD operations into separate files |
| `task.rs` | 1,243 | `task/mod.rs`, `task/trait.rs`, `task/state.rs`, `task/registry.rs` |

### Benefits
- Easier navigation and code review
- Better separation of concerns
- Smaller compilation units
- Improved testability

### Note on T-0013 Relationship
The file splitting is independent of T-0013 (cloacina-workflow crate). T-0013 extracts types to a new crate; file splitting refactors within existing crates. These can be done in any order.

### Recommendation
Create a new task under an appropriate initiative (e.g., unarchive CLOACI-I-0016 "Technical Debt and Repository Improvements") when ready to prioritize this work.
