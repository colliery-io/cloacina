---
id: toml-config-parsing-with-env-var
level: task
title: "TOML config parsing with env var and CLI flag overrides"
short_code: "CLOACI-T-0174"
created_at: 2026-03-16T01:35:05.897498+00:00
updated_at: 2026-03-16T01:55:28.966190+00:00
parent: CLOACI-I-0029
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0029
---

# TOML config parsing with env var and CLI flag overrides

## Objective

Implement a layered configuration system for the `cloacinactl serve` command that loads settings from a TOML file, overlays environment variables, and applies CLI flag overrides. This gives operators flexibility to configure Cloacina via config files for defaults, env vars for deployment-specific values (secrets, URLs), and CLI flags for ad-hoc overrides.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `ServerConfig` struct defined with serde `Deserialize` + `Default`, containing sections: `server` (bind, port, mode), `database` (url, pool_size), `scheduler` (poll_interval, enable_continuous), `worker` (max_concurrent, timeout), `logging` (level, format)
- [ ] TOML file deserialized into `ServerConfig` via `toml` crate
- [ ] Config file discovery chain: `--config` CLI flag > `./cloacina.toml` > `~/.config/cloacina/cloacina.toml` > built-in defaults
- [ ] Environment variable overlay using `CLOACINA_` prefix convention: `CLOACINA_DATABASE_URL`, `CLOACINA_SERVER_PORT`, `CLOACINA_SERVER_BIND`, `CLOACINA_SCHEDULER_POLL_INTERVAL`, `CLOACINA_WORKER_MAX_CONCURRENT`, `CLOACINA_LOG_LEVEL`, etc.
- [ ] CLI flags from the `serve` subcommand (`--port`, `--bind`, `--mode`) override both file and env values
- [ ] `toml` crate added as a dependency in `crates/cloacinactl/Cargo.toml`
- [ ] Unit tests covering: default config when no file exists, TOML parsing with all sections, env var overlay replacing specific fields, CLI flags taking precedence over env and file values
- [ ] Missing config file is not an error (defaults are used); malformed TOML is a clear error with file path in message

## Implementation Notes

Create a `config.rs` module in `crates/cloacinactl/src/`. The loading function signature should be roughly `fn load_config(cli_config_path: Option<&Path>, cli_overrides: &ServeArgs) -> Result<ServerConfig>`. The merge order is: defaults -> TOML file -> env vars -> CLI flags. Use `Option<T>` fields in an intermediate "partial config" struct for TOML/env layers so that unset values don't clobber defaults. For env vars, manually check `std::env::var("CLOACINA_DATABASE_URL")` etc. rather than pulling in a heavy config framework -- keep dependencies minimal. Depends on CLOACI-T-0173 for the `ServeArgs` struct.

## Status Updates

### 2026-03-16 — Completed
- Created `config.rs` with `ServerConfig` struct (5 sections: server, database, scheduler, worker, logging)
- All sections have `#[serde(default)]` for partial TOML files
- Layered loading: defaults → TOML file → env vars (CLOACINA_ prefix) → CLI flags
- Config discovery chain: --config flag > ./cloacina.toml > ~/.config/cloacina/cloacina.toml
- Added `toml`, `serde`, `dirs` dependencies
- Wired into serve.rs — config loaded before anything else
- 8 unit tests: defaults, TOML parsing, partial TOML, env overlay, CLI precedence, missing file handling
- 19 total cloacinactl tests pass
