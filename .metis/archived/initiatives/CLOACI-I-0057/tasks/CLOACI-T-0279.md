---
id: directory-watcher-notify-crate
level: task
title: "Directory watcher — notify crate, debounced events, reconciler trigger"
short_code: "CLOACI-T-0279"
created_at: 2026-03-28T15:30:06.923823+00:00
updated_at: 2026-03-29T00:40:21.236411+00:00
parent: CLOACI-I-0057
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0057
---

# Directory watcher — notify crate, debounced events, reconciler trigger

## Parent Initiative

[[CLOACI-I-0057]]

## Objective

Add filesystem watching to the daemon so that when `.cloacina` packages are added, modified, or removed from the watch directory, the reconciler is triggered to load/unload them. Uses the `notify` crate for cross-platform filesystem events with debouncing to handle rapid file changes.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `notify` crate added as dependency to cloacinactl
- [ ] `PackageWatcher` struct that watches a directory for `.cloacina` file events
- [ ] Debounced events (500ms–1s) to avoid reconciling during partial writes
- [ ] On file create/modify: triggers reconciler `reconcile()` call
- [ ] On file delete: triggers reconciler `reconcile()` which detects removed packages
- [ ] Only watches for `.cloacina` extension (ignores other files)
- [ ] Watcher runs as a tokio task, communicates via channel
- [ ] Integration test: copy a package into watch dir → reconciler picks it up

## Implementation Notes

### Files to modify
- `crates/cloacinactl/Cargo.toml` — add `notify` dependency
- `crates/cloacinactl/src/commands/daemon.rs` — integrate watcher into daemon loop

### Key design points
- `notify::RecommendedWatcher` with debounced events
- Watcher sends events to an `mpsc` channel, daemon loop receives and calls `reconciler.reconcile()`
- Archive from prior work: commit `43968dd` on `archive/cloacina-server-week1`

### Depends on
- T-0278 (daemon subcommand — the loop to integrate into)

## Status Updates

**2026-03-28**: Implementation complete, smoke tested.

### Changes:
- `watcher.rs` — `PackageWatcher` with `notify::RecommendedWatcher`, 500ms debounce, `.cloacina` extension filter, `mpsc` channel signaling
- `daemon.rs` — Event-driven loop: watcher signals + periodic tick drive `reconciler.reconcile()`
- `Cargo.toml` — added `notify` v7
