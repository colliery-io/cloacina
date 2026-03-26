---
id: cleanup-remove-v1-code-update
level: task
title: "Cleanup — remove v1 code, update angreal commands, evaluate old perf examples"
short_code: "CLOACI-T-0261"
created_at: 2026-03-26T02:36:49.513902+00:00
updated_at: 2026-03-26T03:24:12.507172+00:00
parent: CLOACI-I-0046
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0046
---

# Cleanup — remove v1 code, update angreal commands, evaluate old perf examples

## Parent Initiative

[[CLOACI-I-0046]]

## Objective

Evaluate whether `examples/performance/{simple,pipeline,parallel}` should be kept or removed now that the Python bench covers real e2e performance testing. Clean up any remaining v1 artifacts.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `examples/performance/{simple,pipeline,parallel}` evaluated — document decision (keep as user-facing examples or remove)
- [ ] If removing: delete directories, remove from angreal demos, remove angreal performance simple/pipeline/parallel commands
- [ ] If keeping: ensure they still compile and run, update descriptions to clarify they're examples not benchmarks
- [ ] Verify no stale references to old Rust scheduler-bench in CI, docs, or angreal
- [ ] Root `Cargo.toml` exclude list updated if scheduler-bench directory removed

## Implementation Notes

### Decision: old perf examples
The 3 old examples (`simple`, `pipeline`, `parallel`) are standalone binaries that run N iterations of a workflow using in-process `runner.execute()`. They're useful as **user-facing examples** showing how to use the library API programmatically, even though they don't test real deployment performance. Recommend keeping them as examples but removing them from the `angreal performance` commands (which should now only invoke the Python bench).

### Dependencies
- T-0258, T-0259, T-0260 (all implementation complete)

## Status Updates

- 2026-03-26: Old scheduler-bench Rust binary fully removed (T-0258). No stale references in CI, angreal, or config. Decision: keep `examples/performance/{simple,pipeline,parallel}` as user-facing examples of programmatic API usage — they're not benchmarks, they demonstrate how to use DefaultRunner directly. They compile and run correctly.
