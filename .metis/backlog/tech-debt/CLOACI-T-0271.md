---
id: update-angreal-cloaca-tasks-for
level: task
title: "Update angreal cloaca tasks for native Python in core"
short_code: "CLOACI-T-0271"
created_at: 2026-03-27T13:06:55.853917+00:00
updated_at: 2026-03-27T13:06:55.853917+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Update angreal cloaca tasks for native Python in core

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Related Initiative

I-0050 (Native Python in Core) — this should have been included in the original initiative scope. The cloaca Python module is now served natively from cloacina core, but the angreal task group still references the deleted `bindings/cloaca-backend/` path.

## Objective

Update or rework the angreal `cloaca` task group (smoke, test, package, release, scrub) to work with the new native Python model where the `cloaca` module is embedded in cloacina core via PyO3, not built as a separate maturin wheel from `bindings/cloaca-backend`.

## Current State

- `bindings/cloaca-backend/` deleted in I-0050
- `angreal cloaca smoke` fails: tries to run `maturin build` in `bindings/cloaca-backend`
- `angreal cloaca test` likely fails for same reason
- `angreal cloaca package` / `release` reference the old wheel build path
- The `cloaca` Python module is now registered via `ensure_cloaca_module()` in cloacina core at runtime
- Python tutorials still `import cloaca` — this works at runtime when loaded through cloacina
- `unified_release.yml` already had cloaca wheel/sdist jobs removed (TODO comments left)

## Acceptance Criteria

- [ ] `angreal cloaca smoke` passes — verifies `import cloaca` works through cloacina core
- [ ] `angreal cloaca test` passes — runs Python binding tests against native cloacina
- [ ] `angreal cloaca package` reworked or removed (no separate wheel needed)
- [ ] `angreal cloaca release` reworked or removed
- [ ] `angreal cloaca scrub` updated for new paths
- [ ] Python tutorials run successfully via cloacina core
- [ ] `unified_release.yml` TODO comments resolved — decide on Python packaging story

## Implementation Notes

### Key Decision
The `cloaca` Python module is no longer a standalone PyPI package. It's embedded in the cloacina binary. This means:
- No separate wheel build
- `import cloaca` only works when running under a cloacina-powered binary (cloacinactl, server, or a user binary with cloacina-build)
- The smoke/test tasks need to build a test binary that embeds cloacina, then run Python tests against it
- OR the tasks verify that `cloaca` module is importable from within the cloacina test harness

### Files to Update
- `.angreal/cloaca/smoke.py`
- `.angreal/cloaca/test.py`
- `.angreal/cloaca/package.py`
- `.angreal/cloaca/release.py`
- `.angreal/cloaca/scrub.py`
- `.angreal/cloaca/cloaca_utils.py`
- `.github/workflows/unified_release.yml` (PyPI TODO)

## Status Updates

*To be added during implementation*
