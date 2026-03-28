---
id: file-logging-structured-logs-to
level: task
title: "File logging — structured logs to ~/.cloacina/logs/ with rotation"
short_code: "CLOACI-T-0283"
created_at: 2026-03-28T15:38:55.788522+00:00
updated_at: 2026-03-28T15:38:55.788522+00:00
parent: CLOACI-I-0057
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0057
---

# File logging — structured logs to ~/.cloacina/logs/ with rotation

## Parent Initiative

[[CLOACI-I-0057]]

## Objective

Set up file-based logging for the daemon so that it writes structured logs to `~/.cloacina/logs/` with rotation. The daemon runs as a long-lived process — stdout alone isn't sufficient for production debugging.

## Acceptance Criteria

- [ ] Daemon writes logs to `~/.cloacina/logs/cloacina.log` (or date-stamped variant)
- [ ] Structured format (JSON lines or similar) for machine parsing
- [ ] Also outputs to stderr for foreground use (dual output)
- [ ] Log rotation: size-based or daily rotation, configurable max files
- [ ] `--log-level` flag controls verbosity (trace, debug, info, warn, error)
- [ ] `tracing-appender` or similar crate for non-blocking file writes
- [ ] Logs directory auto-created on daemon startup

## Implementation Notes

### Crates to consider
- `tracing-appender` — non-blocking file appender with rotation (already in tracing ecosystem)
- `tracing-subscriber` — layered subscribers for dual output (file + stderr)

### Files to modify
- `crates/cloacinactl/src/commands/daemon.rs` — logging setup during startup
- `crates/cloacinactl/Cargo.toml` — add `tracing-appender` dependency

### Depends on
- T-0278 (daemon subcommand — the startup to add logging to)

## Status Updates

*To be added during implementation*
