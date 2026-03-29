---
id: expose-remaining-hardcoded-daemon
level: task
title: "Expose remaining hardcoded daemon settings in config.toml and CLI"
short_code: "CLOACI-T-0287"
created_at: 2026-03-29T11:21:20.880695+00:00
updated_at: 2026-03-29T11:21:20.880695+00:00
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

# Expose remaining hardcoded daemon settings in config.toml and CLI

## Objective

Several daemon settings are currently hardcoded. They should be configurable via `config.toml` and/or CLI args, following the existing pattern where CLI overrides config file overrides defaults.

### Tech Debt Impact
- **Current problems**: Users can't tune shutdown timeout, watcher debounce, trigger poll interval, or catchup limits without rebuilding
- **Benefits of fixing**: Fully configurable daemon for different deployment scenarios (fast dev loop vs production stability)
- **Risk**: P3 — sensible defaults work for most cases, but power users need knobs

## Currently hardcoded settings

| Setting | Current value | Location |
|---|---|---|
| Shutdown timeout | 30s | `daemon.rs` |
| Watcher debounce | 500ms | `daemon.rs` |
| Cron max catchup executions | `usize::MAX` (run_all) | `daemon.rs` |
| Trigger scheduler base poll | 1s (DefaultRunner default) | `config.rs` in DefaultRunnerConfig |
| Cron recovery interval | 300s | DefaultRunnerConfig |
| Cron lost threshold | 10 min | DefaultRunnerConfig |

## Acceptance Criteria

- [ ] All settings above configurable in `config.toml` under `[daemon]` section
- [ ] Corresponding CLI args where appropriate (shutdown timeout, debounce)
- [ ] `DaemonConfig` struct updated with new fields + defaults
- [ ] Hot reload (SIGHUP) picks up changed values where safe
- [ ] Documented in config.toml comments or example config

## Status Updates

*To be added during implementation*
