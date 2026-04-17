---
id: t3-profile-model-config-profile
level: task
title: "T3: Profile model + config profile commands"
short_code: "CLOACI-T-0512"
parent: CLOACI-I-0098
blocked_by: [CLOACI-T-0511]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0098
---

# T3: Profile model + config profile commands

## Parent Initiative

CLOACI-I-0098 — cloacinactl CLI redesign

## Objective

Implement profile-based server targeting per ADR-0003 §3. Profiles live in `~/.cloacina/config.toml` alongside the existing `[daemon]` section. Flag precedence resolves `--server`, `--api-key`, `--tenant` for every client command before it makes HTTP calls. Introduce `config profile` verbs for management.

## Acceptance Criteria

- [ ] `~/.cloacina/config.toml` parses into a struct with `default_profile`, `[daemon]`, `[profiles.*]`. Existing `[daemon]` semantics unchanged.
- [ ] Profile struct fields: `server` (URL), `api_key` (string with scheme prefix support).
- [ ] API-key scheme resolution: raw string → literal, `env:VAR` → read env var, `file:PATH` → read file (first line, trimmed). `keyring:NAME` returns an explicit "deferred to v1.1" error.
- [ ] `ClientContext` resolver takes `GlobalOpts` + config and returns `{ server, api_key, tenant, output, ... }`. Precedence: explicit flag > named profile > default profile > error.
- [ ] `config profile` verbs:
  - `set <NAME> <URL> [--api-key <KEY>] [--default]` — upsert; `--default` sets as `default_profile`.
  - `list` — table of profiles, default marked with `*`. Secrets redacted to first/last 4 chars in `table`/`yaml`, preserved in `json`.
  - `use <NAME>` — set `default_profile`.
  - `delete <NAME>` — remove; if default, clear `default_profile`.
- [ ] `config get`, `config set`, `config list` keep working against non-profile sections.
- [ ] Unit tests for resolver precedence + API-key schemes.

## Implementation Notes

### Resolver precedence

```rust
pub struct ClientContext {
    pub server: Url,
    pub api_key: String,
    pub tenant: Option<String>,
    pub output: OutputFormat,
}

impl ClientContext {
    pub fn resolve(opts: &GlobalOpts, config: &Config) -> Result<Self> {
        let profile_name = opts.profile.as_deref().or(config.default_profile.as_deref());
        let profile = profile_name.and_then(|n| config.profiles.get(n));

        let server = opts.server.clone()
            .or_else(|| profile.map(|p| p.server.clone()))
            .ok_or_else(|| err("no server configured"))?;

        let api_key = opts.api_key.clone()
            .or_else(|| profile.map(|p| p.api_key.clone()))
            .map(resolve_api_key_scheme)
            .transpose()?
            .ok_or_else(|| err("no api key configured"))?;

        let tenant = opts.tenant.clone(); // key-scope resolution lives in T4
        Ok(Self { server, api_key, tenant, output: opts.output() })
    }
}
```

### Writes to config.toml

`toml_edit` (not `toml`) so comments and ordering survive `config set` calls.

### Secret redaction

Output layer (T4) gets a `Redacted` newtype that prints as `abcd…wxyz` in human output and the raw value in `-o json`. Applied to `api_key` fields.

## Status Updates

*To be added during implementation*
