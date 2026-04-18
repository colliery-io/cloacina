---
id: t2-cloacinactl-top-level-reshape
level: task
title: "T2: cloacinactl top-level reshape — global flags + daemon/server noun-verb"
short_code: "CLOACI-T-0511"
created_at: 2026-04-17T17:00:00+00:00
updated_at: 2026-04-18T01:40:05.007927+00:00
parent: CLOACI-I-0098
blocked_by: [CLOACI-T-0510]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0098
---

# T2: cloacinactl top-level reshape — global flags + daemon/server noun-verb

## Parent Initiative

CLOACI-I-0098 — cloacinactl CLI redesign

## Objective

Rewrite `cloacinactl`'s `main.rs` and top-level command structure to match the ADR/spec. Every operation becomes noun-verb; runtime services (`daemon`, `server`) get their own verbs (`start`/`stop`/`status`/`health`). Global flag handling is put in place so downstream tasks (T3-T7) can hang their subcommands off a stable skeleton.

This task produces a working CLI with `daemon` and `server` nouns wired end-to-end. Client nouns (package, workflow, etc.) come in later tasks.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Global flags parsed at the root level and available to every subcommand: `--verbose`, `--home`, `--profile`, `--server`, `--api-key`, `--tenant`, `--json`, `-o <fmt>`, `--no-color`. (Profile resolution is in T3 — T2 just wires the flags through.)
- [ ] `daemon` noun with verbs:
  - `start [--watch-dir PATH]... [--poll-interval MS]`
  - `stop [--force]` — SIGTERM (or SIGKILL with `--force`) to PID in `$home/daemon.pid`. Exit 3 if no PID file, 2 if process didn't exit within 10s.
  - `status` — queries `$home/daemon.sock` (existing transport), human output by default.
  - `health` — same socket, terse `up`/`down`, exit 0/2.
- [ ] `server` noun with verbs:
  - `start` — `exec`s `cloacina-server` (from T1), writing `$home/server.pid` before hand-off. Forwards flags: `--bind`, `--database-url`, `--bootstrap-key`, `--require-signatures`.
  - `stop [--force]` — SIGTERM / SIGKILL via `$home/server.pid`. Stubbed with "no local PID file — stop via your orchestrator" message when file absent. Exit 0 in that stubbed case so CI scripts don't break.
  - `status` — HTTP `/health` + auth probe. Rich human output. Exit 0/2/4.
  - `health` — HTTP `/health`. Terse. Exit 0/2.
- [ ] Top-level `cloacinactl status` — composite that runs `daemon status` + `server status` and prints both. Exits 0 if either is healthy.
- [ ] Old top-level `serve`, `daemon`, `status`, `admin`, `config` handling removed or relocated (`admin`, `config` stay as nouns; polish is T3+).
- [ ] `angreal check all-crates` passes. `cloacinactl --help` shows the new surface.

## Implementation Notes

### Structure

Reorganize `crates/cloacinactl/src/`:
```
main.rs                — clap Cli + global flags + dispatch
nouns/
  daemon/{start,stop,status,health}.rs
  server/{start,stop,status,health}.rs
  config/…             (T3)
  package/…            (T5)
  workflow/…           (T6)
  ...
shared/
  pid.rs               — PID file read/write/signal helpers
  sockets.rs           — Unix socket client for daemon
  exec.rs              — exec cloacina-server helper
```

`shared/pid.rs` centralizes the SIGTERM-with-timeout logic used by both `daemon stop` and `server stop`.

### Server start handoff

Use `std::os::unix::process::CommandExt::exec` so the `cloacinactl` process is replaced by `cloacina-server`. Server inherits the parent's stdio and signals go to the right place. Write the PID file before exec.

### Daemon stop

Confirm `daemon start` writes `$home/daemon.pid` today; if not, add it.

### Global flags plumbing

`GlobalOpts` struct derived via `clap::Args`, `#[command(flatten)]` on the top-level `Cli`, passed down to every subcommand handler as `&GlobalOpts`.

### Composite `status`

Calls the same handlers as `daemon status` and `server status` internally, collects results, prints side-by-side. Pure orchestration.

## Status Updates

*To be added during implementation*
