---
id: 003-cloacinactl-cli-surface-noun-verb
level: adr
title: "cloacinactl CLI surface — noun-verb hierarchy, binary split, profile model"
number: 3
short_code: "CLOACI-A-0003"
created_at: 2026-04-17T17:01:52.724967+00:00
updated_at: 2026-04-17T17:53:32.341527+00:00
decision_date:
decision_maker: Dylan Storey
parent:
archived: false

tags:
  - "#adr"
  - "#phase/discussion"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-0003: cloacinactl CLI surface — noun-verb hierarchy, binary split, profile model

## Context

`cloacinactl` exposes a small, incoherent command surface (5 top-level verbs: daemon / serve / config / admin / status). The server exposes 8 REST endpoint groups that are not reachable through the CLI; users `curl` today. There is no output convention, no server-target profile story, no standardized exit codes, and no split between the HTTP API daemon and the CLI client.

Before the distribution work in CLOACI-T-0501 ships binaries and install scripts, we need to fix the CLI shape so we're not baking the current mess into a release.

## Decision

### 1. Noun-verb command hierarchy

Every operation is `cloacinactl <noun> <verb> [args]`. No top-level-verb exceptions. Runtime services (`daemon`, `server`) are nouns with their own verbs (`start`/`stop`/`status`/`health`), same as client nouns.

**Client nouns:** `package`, `workflow`, `graph`, `execution`, `trigger`, `tenant`, `key`.
**Runtime nouns:** `daemon`, `server`.
**Local-ops nouns:** `config`, `admin`, `completions`.

A thin top-level `cloacinactl status` is kept as a composite "show me daemon + server together" convenience; it's the one exception to the noun-verb rule.

### 2. Binary split — two binaries

- **`cloacina-server`** — the HTTP API service. Extracted from today's `cloacinactl serve` path. Ships as a separate binary (with its own Docker image per T-0501).
- **`cloacinactl`** — CLI client + in-process daemon mode. `cloacinactl server start` execs `cloacina-server`; `cloacinactl daemon start` runs in-process.

The `cloacina-daemon` binary mentioned in T-0501's draft is dropped: daemon mode remains a subcommand of `cloacinactl`.

No backwards-compatible aliases (e.g., `cloacinactl serve` → `cloacinactl server start`). Pre-1.0, clean slate.

### 3. Profile-based server targeting

Single `~/.cloacina/config.toml` gains `[profiles.*]` sections alongside the existing `[daemon]` section. `default_profile` selects the default. Per-invocation overrides available as `--profile`, `--server`, `--api-key`, `--tenant`. Precedence: explicit flag > `--profile` > `default_profile`.

API-key source schemes: raw value, `env:VAR`, `file:PATH`. Keyring support deferred.

### 4. Tenant resolution rule

Tenant resolution is driven by key scope, not by implicit defaults:

- **Tenant-scoped key** — tenant implicit from the key. `--tenant` override permitted only when the key's ACL covers that tenant.
- **Admin key** — `--tenant` is **required** for any command that operates on tenant-scoped resources. No silent fallback.

### 5. Output conventions

- Default: human-readable tables. Timestamps in local TZ. Color on TTY, `--no-color` override.
- `-o <table|json|yaml|id>` flag; `--json` is shorthand for `-o json`.
- Errors go to stderr; structured JSON error objects when `-o json`.

### 6. Exit codes

```
0  success
1  user error (bad flags, validation, malformed input)
2  network / server unreachable
3  not found
4  auth failure
5  server-side rejection (business-logic error reported by API)
```

### 7. Shell completions

`cloacinactl completions <shell>` emits a completion script for `bash | zsh | fish | powershell` via `clap_complete`.

## Consequences

**Positive:**
- Every server-side operation is CLI-reachable; curl-based flows disappear.
- Multi-server workflows work via profiles without per-command URLs.
- Scripting is first-class (`-o json`, deterministic exit codes).
- Shipping the HTTP API as its own binary matches the Docker-based distribution story in T-0501.

**Negative:**
- Breaking change for anyone scripting against today's `cloacinactl serve`/`admin cleanup-events` surface. Pre-1.0, acceptable.
- Two binaries instead of one — slightly larger release artifacts.
- Keyring deferral means API keys live in config file or env. Documented "don't commit `config.toml`" rule.

**Neutral:**
- Daemon stays under `cloacinactl` (not a separate binary), so the T-0501 artifact list updates to drop `cloacina-daemon` as a distinct artifact.

## Alternatives Considered

- **Verb-noun (kubectl-style):** `cloacinactl list packages`. Rejected — noun-verb reads better for tab-completion-driven discovery (`cloacinactl workflow <tab>` shows every op on workflows).
- **Three binaries** (`cloacina-server`, `cloacina-daemon`, `cloacinactl`): Rejected as overkill; the daemon code shares heavily with the CLI and there's no operational need to split.
- **No profiles, `--server` on every call:** Rejected; tedious for multi-environment workflows.
- **Keyring in v1:** Deferred. Adds a cross-platform native dep for a problem env vars + config file already solve at this scale.

## Status

Draft — pending human decision.
