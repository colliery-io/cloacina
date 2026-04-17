---
id: validate-server-package-lifecycle
level: task
title: "Validate server package lifecycle — repeated uploads, upgrades, rollbacks"
short_code: "CLOACI-T-0497"
created_at: 2026-04-16T12:38:22.860340+00:00
updated_at: 2026-04-16T15:45:01.498723+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/active"


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

## Acceptance Criteria

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

### Design (from audit discussion)

**Package identity model:**
- `(name, version)` is the identity — one active row per pair
- `content_hash` (SHA256 of archive bytes) determines whether content changed
- UUID is immutable per content — new content = new UUID, old row superseded

**Upload handler flow:**
1. Unpack manifest → `(name, version)`
2. Compute `SHA256(archive bytes)`
3. Check DB for active `(name, version)`:
   - **Same hash** → return existing UUID (idempotent no-op)
   - **Different hash** → mark old row `superseded = true`, insert new row with new UUID (transactional)
   - **Not exists** → insert new row
4. Version upgrade (same name, different version): same flow — old version row gets superseded, new row inserted

**Reconciler — no changes needed:**
- Old UUID disappears from active set → existing unload path fires
- New UUID appears → existing load path fires
- The diff-by-UUID logic handles both sides naturally

**Schema changes:**
```sql
ALTER TABLE workflow_packages ADD COLUMN content_hash TEXT NOT NULL;
ALTER TABLE workflow_packages ADD COLUMN superseded BOOLEAN NOT NULL DEFAULT FALSE;
-- Partial unique: only one active row per (name, version)
CREATE UNIQUE INDEX idx_active_package ON workflow_packages(package_name, version) WHERE NOT superseded;
```

**Fixes all three bugs:**
- Bug 1 (no upgrade path): supersede old row → reconciler unloads old UUID, loads new UUID
- Bug 2 (concurrent upload race): partial unique index prevents duplicates at DB level
- Bug 3 (rollback): re-uploading old version with different content supersedes current, creates new UUID

## Status Updates

### 2026-04-16: Implementation landed — supersede-based upload flow

Design: name is identity (one active row per package_name), version is a
monotonically-increasing string the client uses to communicate recency, and
content_hash (SHA256 of the archive) drives idempotency.

Schema changes (migrations postgres/021, sqlite/018):
- `content_hash TEXT NOT NULL DEFAULT ''`
- `superseded BOOLEAN NOT NULL DEFAULT FALSE`
- Partial unique index `(package_name) WHERE NOT superseded` — enforces one
  active row per name at the DB level, closes Bug 2 (concurrent upload race).
- Existing `UNIQUE(package_name, version)` kept as defense-in-depth.

Upload handler (`register_workflow`) rewritten:
1. Unpack manifest → (name, version).
2. SHA256 the archive bytes → content_hash.
3. Look up the active row for `name`:
   - Same hash → return existing UUID (idempotent no-op; no storage churn).
   - Different hash → transactional supersede-and-insert (UPDATE old row
     `superseded = TRUE`, INSERT new row with new UUID + content_hash).
   - None → INSERT new row.

Reconciler requires no changes — its diff-by-UUID logic naturally unloads the
superseded UUID and loads the newly-inserted UUID because the DAL read methods
(`get_package_metadata`, `get_package_metadata_by_id`, `list_all_packages`) all
filter on `superseded = FALSE`.

Fixes all three bugs:
- **Bug 1 (no upgrade path)**: supersede flips old row; reconciler unloads old
  and loads new.
- **Bug 2 (concurrent race)**: partial unique index rejects the second insert;
  surfaced as `PackageExists`.
- **Bug 3 (rollback)**: same path as upgrade — client uploads a new version with
  old content; new row supersedes whatever was active.

Unit tests added to `workflow_registry/database.rs`:
- `test_supersede_and_insert_fresh_name`
- `test_supersede_and_insert_replaces_old_active` (old UUID invisible via all
  filtered reads; new UUID visible; list_all_packages returns only active)
- `test_partial_unique_rejects_second_active_for_same_name`

Server-mode end-to-end validation (repeated/upgrade/rollback/concurrent uploads
against the HTTP API) is deferred to a separate soak-style task or the server
soak harness.

### 2026-04-15: Code audit complete — 4 bugs found

**Audit scope**: `workflows.rs` (upload handler), `workflow_registry/mod.rs` (register_workflow), `reconciler/mod.rs` (reconcile loop), `reconciler/loading.rs` (load/unload).

#### Bug 1: No upgrade path — versions accumulate, never replace (P1)

The reconciler diffs by **package UUID**, not by package name. Uploading v2 of a package creates a new DB row with a new UUID. The reconciler sees it as a brand new package and loads it alongside v1. Both versions run simultaneously.

For CGs, `ReactiveScheduler::load_graph()` rejects duplicate graph names, so v2's graph silently fails to load while v1 keeps running. For workflows, both versions register — last-write-wins in the global workflow registry, but tasks from both versions remain in the task registry.

**Fix needed**: The reconciler (or upload handler) needs "replace" semantics — when uploading a new version of an existing package name, the old version should be unloaded before the new one is loaded. Options:
- Upload handler: on new version, mark old version as superseded (soft delete or status column)
- Reconciler: group packages by name, only load the latest version, unload older ones
- Explicit API: `PUT` to replace, `POST` to create. Reject if name exists with different version unless explicit upgrade flag.

**Code refs**: `register_workflow` (`workflow_registry/mod.rs:282-292`) — duplicate check is `(name, version)` exact match. `reconcile()` (`reconciler/mod.rs:314`) — diffs by UUID set, no name-awareness.

#### Bug 2: Concurrent upload race condition (P2)

The duplicate check in `register_workflow` is SELECT-then-INSERT, not atomic:
```
1. Check: get_package_metadata(name, version) → None
2. Insert: store_binary + store_package_metadata
```
Two concurrent uploads of the same `(name, version)` can both pass step 1 and both insert. Result: two DB rows with same name+version, different UUIDs. Reconciler loads both.

**Fix needed**: Use a unique constraint on `(package_name, version)` in the DB schema, or use `INSERT ... ON CONFLICT DO NOTHING` and check the result.

**Code refs**: `register_workflow` (`workflow_registry/mod.rs:283-314`).

#### Bug 3: Rollback is same broken path as upgrade (P1)

Re-uploading an old version (e.g., v1 after v2 is running) creates a third DB row. Now v1 and v2 are both loaded. There's no way to "roll back" without manually deleting v2 via the API first.

**Fix needed**: Same as Bug 1 — replace semantics.

#### Bug 4: Delete while running — works correctly (not a bug)

The reconciler's `unload_package()` path is correct:
- Removes from `loaded_packages` HashMap
- Unregisters tasks from global task registry
- Unregisters workflow from global workflow registry
- Unregisters triggers
- Calls `scheduler.unload_graph()` for CGs (shutdown accumulators/reactor)

No fix needed. The unload path is solid.

#### Summary

| Scenario | Current behavior | Correct behavior | Bug? |
|----------|-----------------|-------------------|------|
| Repeated upload (same name+version) | Rejected with PackageExists | Rejected | OK |
| Version upgrade (same name, new version) | Both versions loaded simultaneously | Old unloaded, new loaded | BUG |
| Rollback (re-upload old version) | All versions accumulate | Old replaced by target version | BUG |
| Concurrent uploads (same name+version) | Both succeed, both loaded | One wins, other rejected | BUG |
| Delete while running | Clean unload of all components | Clean unload | OK |
| Upload during active execution | N/A (upgrade doesn't replace) | Graceful shutdown then reload | BUG (no upgrade path) |
