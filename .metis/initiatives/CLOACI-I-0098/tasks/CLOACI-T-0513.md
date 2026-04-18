---
id: t4-output-formatting-foundation
level: task
title: "T4: Output formatting foundation + exit codes + HTTP client helper"
short_code: "CLOACI-T-0513"
parent: CLOACI-I-0098
blocked_by: [CLOACI-T-0512]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0098
---

# T4: Output formatting foundation + exit codes + HTTP client helper

## Parent Initiative

CLOACI-I-0098 — cloacinactl CLI redesign

## Objective

Build the shared plumbing every client command will call into: a typed output layer (table/json/yaml/id), standardized exit codes, and an HTTP client wrapper that handles auth, tenant resolution, and error-to-exit-code mapping. After this task, T5-T7 (package/workflow/etc. verbs) can be small glue on top.

## Acceptance Criteria

- [ ] `OutputFormat` enum: `Table`, `Json`, `Yaml`, `Id`. `--json` is a shortcut for `-o json`.
- [ ] `Renderable` trait implemented for every response shape. Render per selected format. Human tables aligned, timestamps in local TZ, TTY-aware colors.
- [ ] `Redacted` newtype wrapping `String` with the `abcd…wxyz` short form in table/yaml and raw in json.
- [ ] `CliError` enum mapping to exit codes per the ADR (0 ok / 1 user / 2 network / 3 not found / 4 auth / 5 server). `From<reqwest::Error>`, `From<std::io::Error>`, `From<clap::Error>` implementations.
- [ ] `CliClient` wrapping `reqwest::Client`:
  - Constructor takes `ClientContext`.
  - Attaches `Authorization: Bearer` and `X-Tenant` (when set) headers automatically.
  - `get<T>`, `post<T>`, `delete`, `stream` helpers return `Result<T, CliError>`.
  - HTTP status → `CliError` variant per the ADR table.
  - Surfaces server's `{"error": {...}}` body in error messages.
- [ ] Tenant pre-flight: `CliClient::whoami()` caches `/v1/keys/self` per session. Used by callers to decide whether `--tenant` is required (admin key → required, tenant key → implicit/validated).
- [ ] `main.rs` converts any returned `CliError` into the right exit code via `ExitCode::from(err)`.
- [ ] Unit tests for error mapping and the redacted formatter.

## Implementation Notes

### Renderable shape

```rust
pub trait Renderable {
    fn render(&self, format: OutputFormat, out: &mut dyn Write) -> io::Result<()>;
}
```

For tables, each renderable type declares its columns. `serde` + `serde_json`/`serde_yaml` covers the structured formats.

### Errors

```rust
pub enum CliError {
    UserError(String),
    Network(reqwest::Error),
    NotFound { resource: String, key: String },
    Auth(String),
    ServerReject { status: u16, body: serde_json::Value },
    Io(std::io::Error),
    Clap(clap::Error),
}
impl From<&CliError> for ExitCode { ... }
```

### Tenant rule enforcement

```rust
match (client.whoami().await?.scope, ctx.tenant.as_deref()) {
    (KeyScope::Tenant(t), None) => Some(t),
    (KeyScope::Tenant(t), Some(requested)) if t != requested =>
        return Err(CliError::UserError(format!("key is scoped to tenant {t}, cannot target {requested}"))),
    (KeyScope::Admin, None) =>
        return Err(CliError::UserError("admin key requires --tenant for this command".into())),
    (KeyScope::Admin, Some(t)) => Some(t.to_string()),
    _ => ctx.tenant.clone(),
}
```

Helper lives alongside `CliClient` so command handlers just call `client.require_tenant()`.

### HTTP client singleton

One `reqwest::Client` per CLI invocation (connection pool reuse). Timeouts: 30s request, 5s connect. `--timeout <DUR>` deferred.

## Status Updates

*To be added during implementation*
