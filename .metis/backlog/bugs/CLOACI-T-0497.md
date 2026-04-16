---
id: validate-server-package-lifecycle
level: task
title: "Validate server package lifecycle — repeated uploads, upgrades, rollbacks"
short_code: "CLOACI-T-0497"
created_at: 2026-04-16T12:38:22.860340+00:00
updated_at: 2026-04-16T12:38:22.860340+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# Validate server package lifecycle — repeated uploads, upgrades, rollbacks

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Objective

Validate the server's behavior for package lifecycle edge cases that have not been tested. The reconciler and upload handler were built for the happy path — upload once, load, run. The behavior on repeated uploads, version upgrades, downgrades, and concurrent uploads is unknown and likely buggy.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (important for user experience)

### Impact Assessment
- **Affected Users**: Any operator upgrading or redeploying packages in a running server
- **Expected vs Actual**: Unknown — these paths have never been exercised. Likely issues: duplicate graph loading, stale cdylib references, leaked accumulator/reactor tasks, reconciler confusion on version changes.

## Acceptance Criteria

- [ ] Repeated upload of identical package: server should be idempotent (no duplicate graphs)
- [ ] Upload of new version of existing package: server should unload old, load new (hot upgrade)
- [ ] Rollback to previous version: same as upgrade, old version loads cleanly
- [ ] Upload during active graph execution: accumulators/reactor shut down gracefully before reload
- [ ] Concurrent uploads of same package: only one wins, no corruption
- [ ] Package deletion while graph is running: clean shutdown of graph components
- [ ] All scenarios verified with integration tests

## Implementation Notes

### Test scenarios to build
1. Upload package A v1, verify loaded. Upload A v1 again — no duplicate.
2. Upload A v1, push events, verify firing. Upload A v2 — old graph stops, new graph starts, events resume.
3. Upload A v2, then re-upload A v1 — rollback works.
4. Two concurrent uploads of A v1 — one succeeds, no split-brain.
5. Delete package while graph is actively processing — clean shutdown.

### Likely code paths to audit
- Reconciler `reconcile()`: how does it handle version changes vs. same-version re-upload?
- `ReactiveScheduler::load_graph()`: rejects duplicates by name — but does the reconciler call `unload_graph` first on upgrade?
- Upload handler: does it overwrite or create a new row? How does the reconciler detect the change?

## Status Updates

*To be added during implementation*
