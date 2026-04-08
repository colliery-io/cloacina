---
id: remove-credential-logging-from
level: task
title: "Remove credential logging from Python bindings (OPS-03)"
short_code: "CLOACI-T-0442"
created_at: 2026-04-08T13:35:06.465161+00:00
updated_at: 2026-04-08T14:00:07.382166+00:00
parent: CLOACI-I-0085
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0085
---

# Remove credential logging from Python bindings (OPS-03)

## Parent Initiative

[[CLOACI-I-0085]] Security Foundation

## Objective

Remove debug instrumentation and credential exposure from the Python bindings. The `PyDefaultRunner` logs full database URLs with passwords via both `info!()` and `eprintln!()`, bypassing the `mask_db_url()` protection correctly applied in `serve.rs`. Addresses OPS-03 (Major).

**Effort estimate**: 1-2 hours

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All `eprintln!("THREAD: ...")` debug statements removed from `python/bindings/runner.rs`
- [ ] `mask_db_url()` moved to a shared utility (e.g., `logging.rs` or `database/mod.rs`)
- [ ] All `info!()` and `debug!()` calls that log database URLs in Python bindings use the masked version
- [ ] `serve.rs` updated to use the shared `mask_db_url()` instead of its local copy
- [ ] No database credentials appear in log output when running Python tutorials with `RUST_LOG=info`
- [ ] Python binding tests (`angreal cloaca test`) still pass

## Implementation Notes

### Technical Approach

1. Move `mask_db_url()` from `crates/cloacinactl/src/commands/serve.rs` to `crates/cloacina/src/logging.rs` (or `database/mod.rs`) and make it `pub`.
2. Update `serve.rs` to import the shared version.
3. Search `crates/cloacina/src/python/bindings/runner.rs` for all `database_url` references in log/print statements (at least lines 308-312 and 1150-1151). Replace with masked versions.
4. Remove all `eprintln!("THREAD: ...")` calls -- these are leftover debug instrumentation.

### Dependencies
None -- independent of other I-0085 tasks.

## Follow-up

After the immediate fix, a tech debt item (backlog) should be created for a defensive practice to prevent credential leakage from recurring. See backlog item created alongside this task.

## Status Updates

- **2026-04-08**: Removed all 84 `eprintln!` debug statements from `runner.rs` (118 lines total including multi-line calls). Added `pub fn mask_db_url()` to `crates/cloacina/src/logging.rs`. Updated `serve.rs` to delegate to the shared version. Masked 2 `info!` calls that logged raw `database_url` (lines 302, 1072). Zero `eprintln!` remaining. Zero raw `database_url` in any log/print call. Tech debt backlog item T-0443 filed for defensive practice.
