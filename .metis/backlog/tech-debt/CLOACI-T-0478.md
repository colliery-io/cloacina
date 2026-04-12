---
id: add-configuration-validation-and
level: task
title: "Add configuration validation and return Result from config builder"
short_code: "CLOACI-T-0478"
created_at: 2026-04-11T13:51:57.264960+00:00
updated_at: 2026-04-11T13:51:57.264960+00:00
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

# Add configuration validation and return Result from config builder

## Objective

Add bounds-checking validation to `DefaultRunnerConfig`, return `Result` from builder instead of panicking, cap dangerous defaults, and catch TOML typos.

## Review Finding References

OPS-003, API-003, PERF-004 (from architecture review `review/10-recommendations.md` REC-006)

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P3 - Low (when time permits)

### Technical Debt Impact
- **Current Problems**: Builder panics via `assert!` on invalid config. Zero-value configs silently deadlock or busy-loop. `cron_max_catchup_executions` defaults to `usize::MAX`. TOML typos silently ignored.
- **Benefits of Fixing**: Config errors caught at startup with actionable messages instead of runtime panics/deadlocks.
- **Risk Assessment**: Low risk of not addressing — most users use defaults which work fine.

## Acceptance Criteria

- [ ] `DefaultRunnerConfigBuilder::build()` returns `Result<DefaultRunnerConfig, ConfigError>`
- [ ] Validation rules: `max_concurrent_tasks > 0`, `scheduler_poll_interval >= 10ms`, `stale_claim_threshold > heartbeat_interval`, `cron_max_catchup_executions <= 1000`, `db_pool_size > 0`
- [ ] `cron_max_catchup_executions` default changed from `usize::MAX` to `100`
- [ ] `#[serde(deny_unknown_fields)]` on TOML config struct
- [ ] No regression in existing tests

## Implementation Notes

### Key Files
- `crates/cloacina/src/config.rs` — `DefaultRunnerConfig`, `DefaultRunnerConfigBuilder`
- `crates/cloacinactl/src/commands/config.rs` — TOML deserialization

### Dependencies
None.

## Status Updates

*To be added during implementation*
