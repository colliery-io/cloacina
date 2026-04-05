---
id: reorganize-tutorials-and-docs-into
level: task
title: "Reorganize tutorials and docs into library/service and workflows/computation-graphs hierarchy"
short_code: "CLOACI-T-0385"
created_at: 2026-04-05T12:49:30.119214+00:00
updated_at: 2026-04-05T13:06:02.634697+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Reorganize tutorials and docs into library/service and workflows/computation-graphs hierarchy

## Objective

Introduce two-axis hierarchy to tutorials, examples, and docs before computation graph tutorials are written. The axes are:

1. **Capability**: Workflows (unified scheduler) vs Computation Graphs (reactive scheduler)
2. **Mode**: Library (embedded, manual wiring) vs Service (API server, packaging, reconciler)

Everything currently published is workflows/library or workflows/service. Computation graph content is about to land. Without hierarchy, users arriving at the docs won't know which path they're on.

## Current State

### Examples (`examples/`)
```
tutorials/
  01-basic-workflow/          # workflows / library
  02-multi-task/              # workflows / library
  03-dependencies/            # workflows / library
  04-error-handling/          # workflows / library
  05-advanced/                # workflows / library
  06-multi-tenancy/           # workflows / library (with DB)
  python/                     # workflows / library (Python)
features/
  complex-dag/                # workflows / library
  continuous-scheduling/      # computation graphs (misplaced?)
  cron-scheduling/            # workflows / service
  deferred-tasks/             # workflows / library
  event-triggers/             # workflows / service
  multi-tenant/               # workflows / service
  packaged-triggers/          # workflows / service
  packaged-workflows/         # workflows / service
  per-tenant-credentials/     # workflows / service
  python-workflow/            # workflows / library (Python)
  registry-execution/         # workflows / service
  simple-packaged/            # workflows / service
  validation-failures/        # workflows / library
```

### Docs (`docs/content/`)
```
tutorials/
  01-first-workflow.md        # workflows / library
  02-context-handling.md      # workflows / library
  03-complex-workflows.md     # workflows / library
  04-error-handling.md        # workflows / library
  05-cron-scheduling.md       # workflows / service
  06-multi-tenancy.md         # workflows / service
  07-packaged-workflows.md    # workflows / service
  08-workflow-registry.md     # workflows / service
  09-event-triggers.md        # workflows / service
  10-task-deferral.md         # workflows / library
explanation/                  # all workflows, flat
how-to-guides/                # all workflows/service, flat
```

## Target State

### Examples (`examples/`)
```
tutorials/
  workflows/
    library/
      01-basic-workflow/
      02-multi-task/
      03-dependencies/
      04-error-handling/
      05-advanced/
      06-multi-tenancy/
    service/
      07-packaged-workflows/      # promote from features/
      08-workflow-registry/       # promote from features/
  computation-graphs/
    library/
      01-computation-graph/       # new (I-0072)
      02-accumulators/            # new (I-0072)
      03-full-pipeline/           # new (I-0072)
      04-routing/                 # new (I-0072)
    service/                      # empty for now
  python/
    workflows/                    # existing 01-08
    computation-graphs/           # empty for now
features/
  workflows/
    complex-dag/
    cron-scheduling/
    deferred-tasks/
    event-triggers/
    multi-tenant/
    packaged-triggers/
    per-tenant-credentials/
    python-workflow/
    simple-packaged/
    validation-failures/
  computation-graphs/
    continuous-scheduling/
```

### Docs (`docs/content/`)
```
tutorials/
  workflows/
    library/
      _index.md
      01-first-workflow.md
      02-context-handling.md
      03-complex-workflows.md
      04-error-handling.md
    service/
      _index.md
      05-cron-scheduling.md
      06-multi-tenancy.md
      07-packaged-workflows.md
      08-workflow-registry.md
      09-event-triggers.md
      10-task-deferral.md
  computation-graphs/
    library/
      _index.md               # placeholder for I-0072 tutorials
    service/
      _index.md               # placeholder
explanation/
  workflows/
    _index.md
    architecture-overview.md
    context-management.md
    cron-scheduling.md
    dispatcher-architecture.md
    guaranteed-execution-architecture.md
    horizontal-scaling.md
    macro-system.md
    task-deferral.md
    task-execution-sequence.md
    trigger-rules.md
    workflow-versioning.md
  computation-graphs/
    _index.md                 # placeholder for reactive scheduler docs
  platform/
    _index.md
    database-backends.md
    ffi-system.md
    multi-tenancy.md
    package-format.md
    packaged-workflow-architecture.md
    performance-characteristics.md
how-to-guides/
  library/
    _index.md
    testing-workflows.md
  service/
    _index.md
    cleaning-up-events.md
    deploying-the-api-server.md
    monitoring-executions.md
    multi-tenant-recovery.md
    multi-tenant-setup.md
    running-the-daemon.md
    security/
```

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `examples/tutorials/` restructured into `workflows/library/`, `workflows/service/`, `computation-graphs/library/`, `computation-graphs/service/`
- [ ] `examples/features/` restructured into `workflows/`, `computation-graphs/`
- [ ] `examples/tutorials/python/` restructured into `python/workflows/`, `python/computation-graphs/`
- [ ] `docs/content/tutorials/` restructured with `workflows/library/`, `workflows/service/`, `computation-graphs/library/`, `computation-graphs/service/` subdirs
- [ ] `docs/content/explanation/` restructured into `workflows/`, `computation-graphs/`, `platform/`
- [ ] `docs/content/how-to-guides/` restructured into `library/`, `service/`
- [ ] All `_index.md` files updated with new section descriptions
- [ ] Hugo frontmatter `weight` values updated so sections render in correct order
- [ ] Angreal demo tasks updated to reflect new paths (or left as-is with symlinks if less risky)
- [ ] Workspace `Cargo.toml` member paths updated for moved example crates
- [ ] `angreal demos` commands still work after the move
- [ ] `docs build` still works after the move
- [ ] All internal doc cross-references (`{{< ref >}}`) updated for new paths
- [ ] No broken links in docs site

## Implementation Notes

### Approach
1. Move example crates first (no workspace Cargo.toml changes — examples are `exclude`d)
2. Move doc files, create `_index.md` for new sections
3. Update Hugo cross-references (`{{< ref >}}`)
4. Update angreal demo utils — the system is dynamic (scans dirs at runtime), so update:
   - `.angreal/demos/demos_utils.py`: `get_rust_tutorial_directories()`, `get_rust_feature_directories()`, `get_python_tutorial_files()` — update scan paths for new nested structure
   - `.angreal/demos/rust_demos.py`: `create_rust_tutorial_command()`, `create_rust_feature_command()` — update path construction
   - `.angreal/demos/python_demos.py` — update scan path
5. Update CI: `.github/workflows/cloacina.yml` references `examples/features/packaged-workflows/` and `examples/features/simple-packaged/` paths
6. Ensure example→doc naming alignment (e.g., tutorial dir name matches doc filename)
7. Verify `angreal docs build` and `angreal demos` still work

### Example/Doc naming alignment
Current mismatches to fix during move:
- `examples/tutorials/02-multi-task/` → Cargo.toml name is `data-pipeline-example` (should be `tutorial-02`)
- Doc tutorials numbered 01-10 but only examples 01-06 exist as code — 07-10 are doc-only (packaged workflows, registry, triggers, deferral)
- `examples/features/packaged-workflows/` and `examples/features/registry-execution/` map to doc tutorials 07 and 08 but names don't match

### Risk
- Hugo uses directory structure for URL generation — moving files changes URLs (acceptable, not yet public)
- Internal cross-references via `{{< ref >}}` will break and need updating
- CI references specific example paths — must update
- Angreal is dynamic but scan root paths change

## Status Updates

- 2026-04-05: Phase 1 complete — examples restructured. Tutorials 01-06 moved to `tutorials/workflows/library/`. Python tutorials moved to `tutorials/python/workflows/`. All 12 workflow features moved to `features/workflows/`. Continuous-scheduling moved to `features/computation-graphs/`. Empty placeholder dirs created for computation-graphs/library, computation-graphs/service, python/computation-graphs. Starting Phase 2 — docs.
