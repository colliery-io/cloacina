---
id: cloacinactl-cli-redesign-and
level: initiative
title: "cloacinactl — CLI Redesign and Rebuild"
short_code: "CLOACI-I-0098"
created_at: 2026-04-17T17:00:46.328374+00:00
updated_at: 2026-04-17T18:01:32.849203+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: cloacinactl-cli-redesign-and
---

# cloacinactl — CLI Redesign and Rebuild Initiative

## Context

`cloacinactl` today has five commands: `daemon`, `serve`, `config`, `admin`, `status`. The server side exposes eight REST endpoint groups (workflows, keys, tenants, triggers, executions, health, ws, auth) that are not reachable through the CLI — users drive the platform with `curl`. There is no coherent command hierarchy, no output-format convention, no profile/target story, no completion generation, and no client/server binary separation.

This initiative designs and rebuilds the CLI. It's a prerequisite for the distribution work in T-0501 (we can't ship a stable install path around a CLI whose shape we're about to change).

## Goals & Non-Goals

**Goals:**
- Coherent noun-verb command hierarchy covering every server-side operation.
- Binary split: `cloacina-server` (HTTP API) as its own binary, `cloacinactl` as the client + thin daemon-mode wrapper.
- Profile-based server targeting in a single `~/.cloacina/config.toml`.
- Output conventions: human tables by default, `--json`/`-o` for scripting, standardized exit codes.
- Shell completions for bash, zsh, fish, powershell.
- Convenience `package publish` (build + pack + upload).

**Non-Goals:**
- Secure secret storage via OS keyring — deferred. Env vars and config-file api keys are sufficient for v1; daemon-local commands bypass auth entirely.
- New server-side functionality. The CLI exposes the existing API; new features come through separate initiatives.
- Separate `cloacina-daemon` binary. Daemon mode stays as a long-running subcommand of `cloacinactl` (matching today). T-0501's artifact list updates to match.
- Python bindings changes.

## Design decisions

### Binaries
Two binaries shipped:
- **`cloacina-server`** — the HTTP API service. Extracted from today's `cloacinactl serve`. Lives in `crates/cloacina-server` (new) or as a new `[[bin]]` target of an existing crate.
- **`cloacinactl`** — client + in-process daemon mode. `cloacinactl server start` execs `cloacina-server`; `cloacinactl daemon start` runs in-process. No back-compat aliases for today's `serve` / `daemon` top-level verbs — pre-1.0, clean slate.

### Command hierarchy (noun-verb)

Global flags (apply to every subcommand):
```
--verbose | --home <PATH> | --profile <NAME>
--server <URL> | --api-key <KEY> | --tenant <ID>
--json | -o <table|json|yaml|id>
```

```
# Runtime — strict noun-verb, same verbs on both
daemon
  start [--watch-dir PATH]... [--poll-interval MS]   — in-process foreground scheduler
  stop  [--force]                                    — SIGTERM via $home/daemon.pid
  status                                             — rich, via Unix socket
  health                                             — scripted probe, via Unix socket

server
  start [--bind ADDR] [--database-url URL]
        [--bootstrap-key KEY] [--require-signatures] — execs cloacina-server
  stop  [--force]                                    — SIGTERM via $home/server.pid (local only)
  status                                             — rich, via HTTP
  health                                             — scripted probe, via HTTP /health

# Composite top-level (only noun-verb exception)
status   — runs daemon status + server status, prints both

# Client — noun-verb against a running cloacina-server
package
  build <DIR> [--debug|--release]           — cargo build the package source
  pack  <DIR> [--out PATH] [--sign <KEY>]   — fidius pack the source → .cloacina
  publish <DIR> [--release] [--sign <KEY>]  — build + pack + upload in one shot
  upload <FILE>
  list
  inspect <ID>
  delete <ID>

workflow
  list
  inspect <NAME>
  run <NAME> [--context <FILE>]
  enable <NAME>
  disable <NAME>

graph
  list
  status <NAME>
  pause <NAME>
  resume <NAME>

execution
  list [--workflow <N>] [--status <S>]
  status <ID>
  events <ID>
  cancel <ID>

trigger
  list
  inspect <NAME>

tenant
  create <NAME>
  list
  delete <NAME>

key
  create [--role admin|write|read] [--ttl <DUR>]
  list
  revoke <ID>

# Local ops
config
  get <KEY>
  set <KEY> <VALUE>
  list
  profile set <NAME> <URL> [--api-key <KEY>]
  profile list
  profile use <NAME>

admin
  cleanup-events [--older-than <DUR>] [--dry-run]
  migrate

completions <shell>
```

### Profile + auth model

Single `~/.cloacina/config.toml` with a `[daemon]` section (unchanged today) and new `[profiles.*]` sections:

```toml
default_profile = "local"

[daemon]
poll_interval_ms = 500
watch_dirs = ["~/.cloacina/packages"]

[profiles.local]
server = "http://localhost:8080"
api_key = "env:CLOACINA_LOCAL_KEY"

[profiles.prod]
server = "https://cloacina.corp.net"
api_key = "2f7a..."  # raw value also accepted; env: and file: schemes supported
```

Precedence: explicit `--server`/`--api-key`/`--tenant` flags > `--profile <NAME>` > `default_profile` in config > error.

Auth-key source schemes: raw string, `env:VAR`, `file:PATH`. Keyring (`keyring:NAME`) deferred to v1.1.

### Tenant resolution

Drive by key scope:
- **Tenant-scoped key** — tenant is implicit from the key's scope. `--tenant <ID>` is only accepted when the key's ACL covers that tenant; otherwise rejected client-side with a clear error.
- **Admin key** — `--tenant <ID>` is **required** for any command that operates on tenant-scoped resources. No silent default, because there's no sensible one.

Commands that are not tenant-scoped (e.g., `tenant create`, `key create --role admin`) do not require `--tenant`.

### Output conventions

- Default: human-readable tables with aligned columns. Timestamps in local TZ.
- `-o <format>` flag: `table` (default), `json`, `yaml`, `id` (just the ID column for piping into other commands). `--json` is a shortcut for `-o json`.
- Errors: human message to stderr; structured JSON error object to stderr when `-o json` is in effect.
- TTY detection for color; `--no-color` override.

### Exit codes

```
0  success
1  user error (bad flags, validation, malformed input)
2  network / server unreachable
3  not found
4  auth failure
5  server-side rejection (business-logic error reported by API)
```

### Shell completions

`completions <shell>` emits a completion script. Supports `bash | zsh | fish | powershell` via `clap_complete`.

## Alternatives Considered

- **kubectl-style verb-noun (`cloacinactl list packages`):** Rejected. Modern CLIs (`docker`, `gcloud`, `gh`) converge on noun-verb, which reads better for discovery (`cloacinactl workflow <tab>` lists every operation).
- **Three separate binaries** (`cloacina-server`, `cloacina-daemon`, `cloacinactl`): Rejected as overkill. The daemon shares most code with the CLI and there's no operational reason to split them.
- **Per-command server target flags (no profiles):** Rejected. Users have multi-server setups (dev/staging/prod); forcing `--server` on every invocation is tedious.
- **Keyring-backed auth secrets in v1:** Deferred. Adds a cross-platform native dep (`keyring` crate, DBus/Keychain/Credential Manager integration) for a problem we can solve with env vars and a documented "don't commit your config.toml" rule.

## Implementation Plan

The design content above (command tree, profile model, tenant rule, conventions) is captured in two child documents before decomposition:

- **CLOACI-A-0003** (ADR) — locks the top-level architecture decisions (binary split, noun-verb, profile model, tenant rule, exit codes). Short, decisional.
- **CLOACI-S-0009** (specification) — the full command-surface spec: every subcommand mapped to its HTTP endpoint, flag schemas, output schemas, error mapping. Living doc.

Decomposition happens after the ADR is accepted and the spec is reviewed. Expected task shape:
1. Extract `cloacina-server` binary (no behavior change).
2. Rework `cloacinactl` top-level, global flag handling, and runtime nouns (`daemon`, `server` with `start/stop/status/health`).
3. Implement profile model + `config profile *` commands.
4. Implement package verbs (build/pack/publish/upload/list/inspect/delete).
5. Implement workflow + execution + graph verbs.
6. Implement tenant + key + trigger verbs.
7. Output formatting + exit codes across all commands.
8. Shell completions.
9. Integration tests driving the CLI end-to-end against a running server fixture.
