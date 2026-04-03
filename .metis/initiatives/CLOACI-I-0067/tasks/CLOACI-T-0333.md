---
id: t2-daemon-watcher-and-lifecycle
level: task
title: "T2: Daemon watcher and lifecycle tests"
short_code: "CLOACI-T-0333"
created_at: 2026-04-03T02:36:30.947869+00:00
updated_at: 2026-04-03T10:29:21.855628+00:00
parent: CLOACI-I-0067
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0067
---

# T2: Daemon watcher and lifecycle tests

## Parent Initiative
[[CLOACI-I-0067]] — Tier 1 (highest impact)

## Objective
Add tests for the daemon's watcher, config parsing, hot-reload, and lifecycle management. Currently zero test coverage.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] PackageWatcher: creates watcher on valid directories
- [ ] PackageWatcher: signals on .cloacina file create/modify/remove
- [ ] PackageWatcher: ignores non-.cloacina file changes
- [ ] PackageWatcher: debounces rapid changes into single signal
- [ ] PackageWatcher: watch_dir/unwatch_dir add/remove directories
- [ ] CloacinaConfig: loads valid TOML config
- [ ] CloacinaConfig: returns defaults for missing file
- [ ] CloacinaConfig: returns defaults for invalid TOML (no crash)
- [ ] CloacinaConfig: resolves watch directories
- [ ] DaemonSection: default values are sensible

## Source Files
- crates/cloacinactl/src/commands/watcher.rs (PackageWatcher)
- crates/cloacinactl/src/commands/config.rs (CloacinaConfig, DaemonSection)
- crates/cloacinactl/src/commands/daemon.rs (run, register_triggers_from_reconcile)

## Implementation Notes

The daemon `run()` function is a monolithic event loop — not directly unit-testable.
Focus on testing the composable components: PackageWatcher, CloacinaConfig, and DaemonSection defaults.
The watcher tests need real filesystem operations (tempdir + file create/modify/remove).

## Status Updates

### 2026-04-03 — Implementation complete (23 tests, all passing)

**Refactored `daemon.rs`** — extracted 3 functions from monolithic `run()`:
- `collect_watch_dirs()` — pure function for merging/deduplicating watch dirs (was inline 2x)
- `apply_watch_dir_changes()` — SIGHUP watch dir diffing logic
- `handle_reconcile()` — reconcile result handling + trigger registration (was copy-pasted 3x)

**Tests added:**
- `daemon.rs` (4 tests): `collect_watch_dirs` dedup, ordering, packages-dir-first, empty inputs
- `watcher.rs` (9 tests): create on valid dir, signals on create/modify/remove, ignores non-.cloacina, debouncing, watch_dir/unwatch_dir, nonexistent dirs
- `config.rs` (10 tests): defaults, load missing/valid/invalid/partial TOML, tilde expansion, save/reload roundtrip, get/set/list dotted keys

**Findings:**
- macOS kqueue needs ~500ms settle time between watcher creation and file operations
- kqueue doesn't fire modify/remove events for files that existed before the watcher started — files must be created while watcher is active
