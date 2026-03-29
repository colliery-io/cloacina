---
id: daemon-soak-test-angreal-task
level: task
title: "Daemon soak test — angreal task, sustained package loading and execution"
short_code: "CLOACI-T-0282"
created_at: 2026-03-28T15:30:10.697154+00:00
updated_at: 2026-03-29T02:06:31.716920+00:00
parent: CLOACI-I-0057
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0057
---

# Daemon soak test — angreal task, sustained package loading and execution

## Parent Initiative

[[CLOACI-I-0057]]

## Objective

Create an angreal soak test that runs the daemon under sustained load: spawns the daemon process, drops packages into the watch directory, verifies they get loaded and executed, removes packages and verifies unload, checks for leaks/crashes over time.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `angreal soak --mode daemon` angreal task (or similar)
- [ ] Builds the daemon binary and pre-built test packages
- [ ] Spawns `cloacinactl daemon` as a subprocess
- [ ] Drops `.cloacina` packages into the watch directory over time
- [ ] Verifies packages are loaded (check SQLite for execution records)
- [ ] Removes packages and verifies unload
- [ ] Runs for configurable duration (default 60s for CI, longer for manual)
- [ ] Asserts: no crashes, no resource leaks, all packages loaded/unloaded correctly
- [ ] Sends SIGINT at end, verifies clean shutdown

## Implementation Notes

### Files to create
- `.angreal/soak/daemon_soak.py` — Python script for the soak test
- Uses `subprocess.Popen` to spawn daemon, file operations to add/remove packages, SQLite queries to verify

### Pattern to follow
- Prior soak test on `archive/cloacina-server-week1` commit `5c4387a`
- Similar to existing `angreal cloacina integration` pattern but runs daemon as a process

### Depends on
- T-0277, T-0278, T-0279, T-0280, T-0281 (all daemon tasks — this validates everything)

## Status Updates

**2026-03-28**: Implementation complete, soak test passes.

### Changes:
- `soak.py` — `angreal cloacina soak --duration N` task
- `task_project.py` — added soak import

### Verified:
Daemon starts, accepts 5 packages, handles removal, stays alive, produces 181KB of logs, exits cleanly with code 0 on SIGINT.
