---
id: cloacinactl-command-surface
level: specification
title: "cloacinactl command surface"
short_code: "CLOACI-S-0009"
created_at: 2026-04-17T17:01:53.474365+00:00
updated_at: 2026-04-17T17:01:53.474365+00:00
parent: CLOACI-I-0098
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# cloacinactl Command Surface — Specification

Living specification for every `cloacinactl` subcommand: invocation, flags, HTTP endpoint it maps to, output schema, error behavior. The ADR (CLOACI-A-0003) locks the high-level decisions; this doc is the implementer's reference.

## Global flags

Applied to every subcommand.

| Flag | Type | Source | Description |
|---|---|---|---|
| `--verbose` / `-v` | bool | | Enable debug logging (`RUST_LOG=debug`-equivalent) |
| `--home <PATH>` | path | | Cloacina home directory (default `~/.cloacina`) |
| `--profile <NAME>` | string | config | Named profile from `~/.cloacina/config.toml` |
| `--server <URL>` | URL | profile | Server base URL, overrides profile |
| `--api-key <KEY>` | string/scheme | profile/env | API key; accepts raw, `env:VAR`, or `file:PATH` |
| `--tenant <ID>` | string | required for admin keys, optional for tenant keys | Target tenant |
| `--json` | bool | | Alias for `-o json` |
| `-o <fmt>` | `table\|json\|yaml\|id` | | Output format (default `table`) |
| `--no-color` | bool | TTY | Disable ANSI colors |

Precedence for server / api-key / tenant: explicit flag > `--profile` > `default_profile` in config > error.

## Exit codes

Every command follows the ADR-defined table:

```
0  success
1  user error (validation, bad flags, malformed input)
2  network / server unreachable
3  not found
4  auth failure (401/403)
5  server-side rejection (business-logic 4xx/5xx from API)
```

## Config file schema

`~/.cloacina/config.toml`:

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
api_key = "file:/run/secrets/cloacina-prod-key"
```

API-key schemes resolved at runtime:
- raw string (literal)
- `env:VAR` — read from env var at invocation
- `file:PATH` — read (first line, trimmed) from path at invocation
- `keyring:NAME` — **deferred** to v1.1, rejected with clear error in v1

---

## Runtime — `daemon` and `server` nouns

Both daemons have the same verb set: `start`, `stop`, `status`, `health`. The distinction is transport:

- **`daemon`** — in-process local scheduler. Health + status over a Unix socket at `$home/daemon.sock`. PID file at `$home/daemon.pid`.
- **`server`** — out-of-process HTTP API (`cloacina-server` binary). Health + status over HTTP. PID file at `$home/server.pid` when started locally; remote/containerized `server start` is not applicable.

### `cloacinactl daemon <verb>`

| Verb | Transport | Description |
|---|---|---|
| `start` | — | Runs in-process (foreground). Flags: `--watch-dir <PATH>...`, `--poll-interval <MS>` (default 500). Writes PID file on start, removes on clean exit. Blocks until SIGINT/SIGTERM. |
| `stop` | PID file + SIGTERM | Sends SIGTERM to PID in `$home/daemon.pid`. Waits up to 10s for exit, then exits 2. `--force` sends SIGKILL instead. Exit 0 on clean stop, 3 if no PID file. |
| `status` | Unix socket `$home/daemon.sock` | Rich human output: PID, uptime, packages loaded, last reconciliation timestamp, pending work. `-o json` returns structured object. Exit 2 if socket unreachable. |
| `health` | Unix socket, terse | Minimal `up`/`down` probe. No body unless `-o json`. Exit 0 if healthy, 2 if socket unreachable or daemon reports unhealthy. |

### `cloacinactl server <verb>`

| Verb | Transport | Description |
|---|---|---|
| `start` | — | `exec`s the `cloacina-server` binary with flags passed through: `--bind <ADDR>` (default `127.0.0.1:8080`), `--database-url <URL>` (env `DATABASE_URL`), `--bootstrap-key <K>` (env `CLOACINA_BOOTSTRAP_KEY`), `--require-signatures` (env `CLOACINA_REQUIRE_SIGNATURES`). Writes `$home/server.pid` before handing off. |
| `stop` | PID file + SIGTERM | Local-only: SIGTERM to `$home/server.pid`. Stubbed (print warning + exit 0) when no PID file exists — for remote/containerized servers, users stop via their orchestrator (Docker, K8s, systemd). `--force` sends SIGKILL. |
| `status` | HTTP | Rich human output: base URL, reachability, version, auth result (tenant + role if authenticated), key resources (loaded packages, active graphs, pending executions). `-o json` returns structured object. Exit codes: 0 reachable + auth ok, 2 unreachable, 4 auth failure. |
| `health` | HTTP `GET /health` | Minimal probe. Exit 0 if healthy, 2 if unreachable or non-200. No body unless `-o json`. |

### `cloacinactl status` (top-level composite)

Convenience that runs `daemon status` and `server status` and prints both side-by-side:

```
daemon:   running (pid 12345, uptime 4h12m, 3 packages loaded)
server:   http://localhost:8080
  reachable:  yes (HTTP 200 /health)
  auth:       ok (tenant 'public', role 'admin')
  version:    0.5.2
```

With `-o json`, a single object with `daemon` and `server` keys. Exits 0 if either is healthy, 2 if both unreachable.

This is the **only** exception to the strict noun-verb rule — kept because "show me everything local" is the most common first thing users type.

---

## Client — noun-verb subcommands

### `package`

| Verb | Flags | HTTP | Description |
|---|---|---|---|
| `build <DIR>` | `--release` \| `--debug` (default debug) | — (local) | Runs `cargo build [--release]` in `<DIR>`, honoring `package.toml` |
| `pack <DIR>` | `--out <PATH>`, `--sign <KEY_PATH>` | — (local) | `fidius_core::package::pack_package`, optionally signs |
| `publish <DIR>` | `--release`, `--sign <KEY_PATH>` | POST `/v1/workflows` | Convenience: build + pack + upload. Tmpdir cleanup on exit. |
| `upload <FILE>` | | POST `/v1/workflows` (multipart `package`) | Upload a pre-packed `.cloacina` |
| `list` | `--filter <PAT>` | GET `/v1/workflows` | List installed packages |
| `inspect <ID>` | | GET `/v1/workflows/{id}` | Package metadata, manifest, workflow/graph/trigger summary |
| `delete <ID>` | `--force` | DELETE `/v1/workflows/{id}` | Uninstall |

**Output — list (table default):**
```
ID                                    NAME           VERSION   UPLOADED             TENANT
a1b2...                               etl-pipeline   1.2.0     2026-04-17 10:14Z    public
```

**Output — inspect:** human-readable YAML-ish summary. JSON mode returns the raw metadata object.

### `workflow`

| Verb | Flags | HTTP | Description |
|---|---|---|---|
| `list` | `--package <ID>` | GET `/v1/workflows/index` | All registered workflows |
| `inspect <NAME>` | | GET `/v1/workflows/by-name/{name}` | Tasks, deps, trigger rules, schedules |
| `run <NAME>` | `--context <FILE\|->` (reads JSON) | POST `/v1/workflows/{name}/run` | Kick off an execution, returns execution ID |
| `enable <NAME>` | | POST `/v1/workflows/{name}/enable` | Allow new scheduled runs |
| `disable <NAME>` | | POST `/v1/workflows/{name}/disable` | Stop scheduler from starting new runs (does not cancel in-flight) |

### `graph`

| Verb | Flags | HTTP | Description |
|---|---|---|---|
| `list` | | GET `/v1/graphs` | All loaded computation graphs |
| `status <NAME>` | | GET `/v1/graphs/{name}` | Health (reactor, accumulators, last emission, backlog) |
| `pause <NAME>` | | POST `/v1/graphs/{name}/pause` | Stop consuming events |
| `resume <NAME>` | | POST `/v1/graphs/{name}/resume` | Resume |

### `execution`

| Verb | Flags | HTTP | Description |
|---|---|---|---|
| `list` | `--workflow <N>`, `--status <S>`, `--limit <N>` | GET `/v1/executions` | Recent executions |
| `status <ID>` | | GET `/v1/executions/{id}` | Current state, task summary |
| `events <ID>` | `--follow`, `--since <DUR>` | GET `/v1/executions/{id}/events` (SSE when `--follow`) | Event trail |
| `cancel <ID>` | | POST `/v1/executions/{id}/cancel` | Request cancellation |

### `trigger`

| Verb | Flags | HTTP | Description |
|---|---|---|---|
| `list` | | GET `/v1/triggers` | All registered triggers |
| `inspect <NAME>` | | GET `/v1/triggers/{name}` | Schedule, workflow binding, last fire, enabled status |

### `tenant`

| Verb | Flags | HTTP | Description |
|---|---|---|---|
| `create <NAME>` | `--description <TEXT>` | POST `/v1/tenants` | Admin-key only |
| `list` | | GET `/v1/tenants` | Admin-key only |
| `delete <NAME>` | `--force` | DELETE `/v1/tenants/{name}` | Admin-key only |

### `key`

| Verb | Flags | HTTP | Description |
|---|---|---|---|
| `create` | `--role <admin\|write\|read>`, `--ttl <DUR>`, `--description <TEXT>` | POST `/v1/keys` | Tenant-scoped unless admin role requested by an admin key |
| `list` | | GET `/v1/keys` | Admin key lists all; tenant key lists own-tenant keys |
| `revoke <ID>` | | DELETE `/v1/keys/{id}` | |

`key create` returns the secret in plain once; the CLI emits it to stdout in `-o id` format for piping into a subsequent `config profile set --api-key=-`.

---

## Local ops

### `config`

| Verb | Description |
|---|---|
| `get <KEY>` | Read one dotted key (e.g. `daemon.poll_interval_ms`) |
| `set <KEY> <VALUE>` | Write |
| `list` | Dump (secrets redacted) |
| `profile set <NAME> <URL> [--api-key <K>]` | Upsert profile |
| `profile list` | Show all profiles, default marker |
| `profile use <NAME>` | Set `default_profile` |
| `profile delete <NAME>` | Remove |

### `admin`

| Verb | Flags | HTTP / DB | Description |
|---|---|---|---|
| `cleanup-events` | `--older-than <DUR>`, `--dry-run` | direct DB | Prune execution events. Requires DB URL (from config/env). |
| `migrate` | `--dry-run` | direct DB | Run pending Diesel migrations. |

`admin` commands take `--database-url <URL>` as an override; they're DB-level, not API-level.

### `completions <shell>`

Emits a completion script for `bash | zsh | fish | powershell`. User pipes into their shell config. Generated from `clap_complete`.

---

## Output formats — per-format rules

### `-o table` (default)
- Left-align strings, right-align numbers.
- Truncate long IDs to first 8 chars with `…` suffix on non-`-o id` calls.
- Timestamps: local TZ, ISO-8601 short form (`2026-04-17 10:14Z`).
- Empty result sets print a single `No items.` line and exit 0.

### `-o json`
- Lists: JSON array.
- Single objects: bare object.
- Errors: `{"error": {"code": "...", "message": "...", "details": {...}}}` on stderr.
- Newline-terminated on stdout.

### `-o yaml`
- Same shapes as JSON, YAML-encoded. Uses `serde_yaml`.

### `-o id`
- One ID per line on stdout. Nothing else. Useful for piping:
  ```
  cloacinactl package list -o id | xargs -n1 cloacinactl package delete
  ```

## Error mapping

HTTP response → exit code:

| HTTP | Exit | Message prefix |
|---|---|---|
| 200/201/204 | 0 | — |
| 400 | 1 | `Error: bad request — ` |
| 401/403 | 4 | `Error: authentication — ` |
| 404 | 3 | `Error: not found — ` |
| 409 | 5 | `Error: conflict — ` |
| 422 | 1 | `Error: validation — ` |
| 429 | 2 | `Error: rate-limited — ` |
| 5xx | 2 | `Error: server — ` |
| network failure | 2 | `Error: network — ` |

Body `{"error": {...}}` from the server is surfaced after the colon when present.

## Auth handshake

1. Resolve API key (flag > profile > env).
2. Send `Authorization: Bearer <key>` header.
3. On 401 → exit 4 immediately, no retry.
4. On 403 → exit 4, include `X-Tenant` and required role in message when server provides them.

## Tenant resolution (client-side)

Before sending a request, determine whether the command is tenant-scoped. Tenant-scoped commands are every verb on `package`, `workflow`, `graph`, `execution`, `trigger`, plus `key list`.

Resolution:
1. Probe key scope via `GET /v1/keys/self` (cached per session).
2. If key is tenant-scoped:
   - Use key's tenant as implicit.
   - Reject with exit 1 if `--tenant` names a different tenant than the key allows.
3. If key is admin-scoped:
   - Require explicit `--tenant <ID>`; exit 1 with help text when missing.

Non-tenant-scoped commands (`tenant *`, `key create --role admin`, `admin *`, etc.) skip the tenant resolution step.

## Completion coverage

`clap_complete` generates completions for:
- Top-level and nested subcommands.
- Flag names.
- Value hints for `--home`, `--out`, `--context` (path hint), `--shell` (enum).

Dynamic completion (tenant names, package IDs from the server) is **not** in scope for v1.

## Open items

- `workflow run --context` input format: accept JSON on disk, `-` for stdin. `--param KEY=VALUE` flags for scalar overrides — v1 or v1.1?
- `execution events --follow` implementation — SSE or WebSocket? Pick whichever the server already supports; audit during implementation.
- Scrollable/paged long lists — deferred; rely on user piping through `less` for now.
- Retry on 5xx / 429: exponential backoff? v1 keeps simple exit-on-first-failure; revisit if it bites.

---

*Living document. Update as implementation lands; mark closed items with a ✓ in the Open items section above.*
