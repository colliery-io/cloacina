---
title: "Use CLI profiles"
description: "Manage named cloacinactl profiles in ~/.cloacina/config.toml: create, switch, secret schemes (env: / file: / raw), resolution precedence, secret rotation."
weight: 82
---

# Use CLI profiles

`cloacinactl` resolves server URL + API key through **named profiles** stored in `~/.cloacina/config.toml`. Profiles let one CLI install talk to multiple environments (local daemon, staging server, production server) without rewriting flags on every invocation. This recipe covers the profile commands, the API-key schemes profiles accept, the resolution precedence, and rotation patterns.

## Prerequisites

- `cloacinactl` installed (see [Installing cloacinactl]({{< ref "/quick-start/install" >}})).
- At least one API key for at least one server (or daemon) you want to target.

## Background

A profile is a `[profiles.<name>]` block in `~/.cloacina/config.toml` (or `<home>/config.toml` if `--home` is set). Each block carries two required fields — a server URL and an API key — and a top-level `default_profile = "<name>"` selects which profile is used when `--profile` is not supplied on the command line.

Per ADR-0003 §3, server-targeting flags resolve in this order (highest precedence first):

1. Explicit `--server` / `--api-key` flags on the command line.
2. The named profile from `--profile <name>`.
3. The default profile from `default_profile`.
4. Error: "no server/key configured."

API keys in profiles accept several schemes (see [API Key Schemes]({{< ref "/platform/reference/cli" >}}#api-key-schemes)):

| Scheme | Example | Behavior |
|---|---|---|
| Raw | `clk_a1b2c3...` | The literal API key in the config file. |
| `env:VAR` | `env:CLOACINA_API_KEY` | Read the key from the named environment variable at command time. |
| `file:PATH` | `file:/etc/cloacina/key` | Read the first non-empty line of the file at command time. |
| `keyring:NAME` | `keyring:prod` | **Reserved for v1.1**; rejected today with a clear error. |

The `env:` and `file:` schemes are the right choice for production — neither plants the secret in the config file itself.

## Steps

### 1. Create a profile

```sh
cloacinactl config profile set prod \
  --server https://api.example.com \
  --api-key env:CLOACINA_PROD_KEY
```

This writes a `[profiles.prod]` block to `~/.cloacina/config.toml`:

```toml
[profiles.prod]
server = "https://api.example.com"
api_key = "env:CLOACINA_PROD_KEY"
```

Pass `--default` to also make it the resolution default:

```sh
cloacinactl config profile set prod \
  --server https://api.example.com \
  --api-key env:CLOACINA_PROD_KEY \
  --default
```

The config file's top level then gets `default_profile = "prod"`.

### 2. Create more profiles

Repeat for each environment. Common shapes:

```sh
# Staging — file-based key
cloacinactl config profile set staging \
  --server https://staging.example.com \
  --api-key file:/etc/cloacina/staging-key

# Local daemon — raw key (acceptable for hobbyist single-user mode per ADR-0005)
cloacinactl config profile set local \
  --server http://127.0.0.1:8080 \
  --api-key clk_localdev_key_a1b2c3d4
```

### 3. List and inspect profiles

```sh
cloacinactl config profile list
# table of profile names, server URLs, and key schemes (never the plaintext key value)
```

### 4. Switch the default profile

```sh
cloacinactl config profile use staging
# updates default_profile = "staging" in ~/.cloacina/config.toml
```

Subsequent commands without `--profile` resolve `staging`. Verify with `cloacinactl status` — it'll show which server it's about to talk to.

### 5. Override per-command

For a one-off command against a non-default profile:

```sh
cloacinactl --profile prod execution list
```

For a one-off against an entirely different server (overriding both `--profile` and the default's URL):

```sh
cloacinactl --server https://other.example.com --api-key env:OTHER_KEY tenant list
```

### 6. Delete a profile

```sh
cloacinactl config profile delete staging
# removes [profiles.staging] from ~/.cloacina/config.toml.
# If the deleted profile was the default, default_profile is also cleared.
```

## Secret rotation patterns

The `env:` and `file:` schemes mean **rotating the secret doesn't require editing the config file**:

- **`env:` rotation** — change the env var in your shell rc / systemd unit / container env, and the next `cloacinactl` invocation picks it up. No config file touched.
- **`file:` rotation** — overwrite the file (e.g., from your secrets-manager fetch), and the next invocation picks it up. Useful in CI: a pre-job step writes the secret to a tmpfs path, the job runs, and the file is gone at cleanup.

For credential storage in shared homes / multi-user systems, prefer `file:` over `env:` — env vars leak through `/proc/PID/environ` to anyone who can read the process state.

The `keyring:` scheme will (when shipped in v1.1) integrate with OS keychains (macOS Keychain, Linux Secret Service, Windows Credential Manager). Until then, `file:` with a tmpfs-backed path is the closest stand-in.

## Recipes

### Switch between local daemon and remote server in one shell

```sh
# Default is `prod`; one-off command against the local daemon.
cloacinactl --profile local daemon status

# Or set a shell alias for the duration of the session:
alias cloaca-local='cloacinactl --profile local'
cloaca-local daemon status
cloaca-local workflow list
```

### Use a CI-only profile that materializes its key from the CI secret

```yaml
# .github/workflows/deploy.yml
- name: Deploy workflow package
  env:
    CLOACINA_API_KEY: ${{ secrets.CLOACINA_API_KEY }}
  run: |
    cloacinactl config profile set ci \
      --server https://api.example.com \
      --api-key env:CLOACINA_API_KEY \
      --default
    cloacinactl package publish ./my-workflow
```

The job ends and the config file (in the runner's `$HOME`) is discarded with the runner — no secret persists.

### Per-tenant scoping with a single profile

The profile carries the server URL + admin/operator key; the tenant comes from the per-command `--tenant` flag (defaults to `public`):

```sh
cloacinactl --profile prod --tenant tenant_acme workflow list
cloacinactl --profile prod --tenant tenant_globex execution list
```

If you find yourself always passing `--tenant <x>` for a specific server, create a wrapper alias rather than a per-tenant profile — profiles target servers, not tenants.

## What this how-to does NOT cover

- **The actual `~/.cloacina/config.toml` schema beyond profiles.** See [CLI Reference: Configuration File]({{< ref "/platform/reference/cli" >}}#configuration-file) for the full schema (daemon settings, watch directories, etc.).
- **Server-side key creation.** See `cloacinactl key create` documented in [CLI Reference]({{< ref "/platform/reference/cli" >}}#key) for minting the keys you'll then reference from profiles.
- **The `keyring:` scheme** (reserved for v1.1; currently rejected with a clear error message at parse time).

## See also

- [CLI Reference: Profile Resolution]({{< ref "/platform/reference/cli" >}}#profile-resolution) — full precedence rules and the formal config schema.
- [CLI Reference: API Key Schemes]({{< ref "/platform/reference/cli" >}}#api-key-schemes) — the four schemes in detail.
- [Configure multi-tenant deployment]({{< ref "configure-multi-tenant-deployment" >}}) — uses profiles end-to-end with tenant-scoped keys.
- **ADR-0003 §3** — the precedence ordering this page describes.
- **CLOACI-I-0098** — `cloacinactl` redesign initiative (defined the profile model).
- **CLOACI-T-0538** — angreal harness reorganization that surfaced the profile commands.
