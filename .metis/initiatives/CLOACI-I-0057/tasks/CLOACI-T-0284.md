---
id: hot-reload-sighup-config-re-read
level: task
title: "Hot reload — SIGHUP config re-read without restart"
short_code: "CLOACI-T-0284"
created_at: 2026-03-28T15:38:56.663995+00:00
updated_at: 2026-03-28T15:38:56.663995+00:00
parent: CLOACI-I-0057
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0057
---

# Hot reload — SIGHUP config re-read without restart

## Parent Initiative

[[CLOACI-I-0057]]

## Objective

Allow the daemon to reload its configuration without restarting. On SIGHUP (or config file change), the daemon re-reads `~/.cloacina/config.toml`, updates watch directories, adjusts log level, and triggers a reconciliation pass to pick up any changes.

## Acceptance Criteria

- [ ] SIGHUP signal triggers config reload
- [ ] `~/.cloacina/config.toml` defines daemon configuration (watch dirs, poll interval, log level)
- [ ] CLI args override config file values (CLI takes precedence)
- [ ] On reload: new watch directories are added to the watcher, removed dirs stop being watched
- [ ] On reload: log level changes take effect immediately
- [ ] On reload: poll interval changes take effect on next scheduler cycle
- [ ] Reconciler runs a full pass after reload to pick up packages in new watch dirs
- [ ] Logs reload: "Reloading configuration...", "Added watch dir: ...", "Reload complete"
- [ ] Invalid config file doesn't crash the daemon — logs error, keeps running with previous config

## Implementation Notes

### Config file format (`~/.cloacina/config.toml`)
```toml
[daemon]
poll_interval_ms = 50
log_level = "info"

[watch]
directories = [
    "~/.cloacina/packages",
    "/opt/workflows",
    "~/my-project/packages",
]
```

### Files to modify
- `crates/cloacinactl/src/commands/daemon.rs` — SIGHUP handler, config loading
- May need a `DaemonConfig` struct for serialization

### Key design points
- Use `tokio::signal::unix::signal(SignalKind::hangup())` for SIGHUP
- Config file is optional — daemon works fine with just CLI args
- Hot-reloadable settings: watch dirs, log level, poll interval
- NOT hot-reloadable: DB path, home directory (requires restart)

### Depends on
- T-0278 (daemon subcommand)
- T-0279 (directory watcher — to add/remove watch dirs)
- T-0283 (file logging — to change log level dynamically)

## Status Updates

*To be added during implementation*
