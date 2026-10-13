---
title: "cloacina::registry::loader::grants"
description: "Constructor capability grants → fidius enforcement (CLOACI-T-0834)."
editUrl: false
sidebar:
  order: 161
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Constructor capability grants → fidius enforcement (CLOACI-T-0834).

Implements the tenant-authored, **default-closed** capability model decided in
[`CLOACI-A-0009`] and specified in [`CLOACI-S-0014`]. A workflow author writes a
`grants = { http=[..], tcp=[..], fs=[..], env=[..] }` at the constructor
instantiation site; this module translates that into the two keys fidius's
enforcement consumes:

1. a **capability allow-list** (`Vec<String>` — the `WasiCtx` key: `fs:ro:<path>`,
   `fs:rw:<path>`, `env:<NAME>`, plus the `http` / `tcp` intent markers), and
2. an **`EgressPolicy`** (the per-request HTTP / per-peer TCP key) — supplied to
   `fidius_host::PluginHost::load_wasm_configured_with_grants`.

**Fail-closed by construction.** An empty [`GrantSpec`](../../../../../rust/cloacina/registry/loader/grants/#struct.GrantSpec) yields an empty allow-list
and no policy: the guest gets a zero-grant `WasiCtx` and fidius's deny-all egress
default stands, so the constructor reaches nothing. A constructor can never widen
its own access — enforcement is entirely host-side, keyed on what the *tenant*
wrote here.

**TCP scope (v1)**

fidius hands `EgressPolicy::authorize_tcp` a **resolved** `SocketAddr` (IP:port),
not the original hostname. So a `host:port` grant is enforced by resolving `host`
to its IP(s) **once at load** and matching `(ip, port)`; the port is always exact.
A DNS rebind after load is therefore not re-checked (the IP set is frozen at load).
For unconstrained host matching use HTTP (whose policy sees the full URI). See
[`CLOACI-S-0014`] "Open Items".

## Structs

### <span id="struct.GrantSpec"></span><code style="font-size: 0.85em">struct GrantSpec</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Default`, `PartialEq`, `Eq`

The tenant's grants for one constructor instance, parsed from the `grants = { .. }` literal. Each list holds raw pattern strings in author order; validation happens in [`translate`](../../../../../rust/cloacina/registry/loader/grants/#fn.translate). An all-empty value (the `Default`) is the fail-closed default: no capability is granted.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `http` | `Vec<String>` | HTTP egress patterns: `host`, `host:port`, or a URL/path glob (`https://api.example.com/v1/*`). `*` matches anything. |
| `tcp` | `Vec<String>` | TCP egress patterns: `host:port`, `ip:port`, or `*:port` / `*`. |
| `fs` | `Vec<String>` | Filesystem grants, each `ro:<path>` or `rw:<path>`. |
| `env` | `Vec<String>` | Environment variables to pass through from the host, by name. |
| `secrets` | `Vec<String>` | Named secret allow-list (CLOACI-T-0860, design D-3). Each entry is a secret NAME the constructor may resolve at fire time. **Fail-closed:** an absent/empty list means the holder may resolve NO secrets. Tenant-scope is the outer boundary — a name here can only resolve within the caller's tenant (the resolver carries the `org_id`). NOT ridden on the egress grant (D-3): a passphrase-style secret has no network endpoint, and implicit authz is exactly what we're avoiding. |

#### Methods

##### <span id="GrantSpec.from_lists"></span><code>from_​lists</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_lists(
    http: Vec<String>,
    tcp: Vec<String>,
    fs: Vec<String>,
    env: Vec<String>,
    secrets: Vec<String>,
) -> Self
```

Build a [`GrantSpec`](../../../../../rust/cloacina/registry/loader/grants/#struct.GrantSpec) from the grant lists (the shape the macro lowers to). A convenience over the struct literal so generated code is insulated from field additions.

<details>
<summary>Source</summary>

```rust
    pub fn from_lists(
        http: Vec<String>,
        tcp: Vec<String>,
        fs: Vec<String>,
        env: Vec<String>,
        secrets: Vec<String>,
    ) -> Self {
        Self {
            http,
            tcp,
            fs,
            env,
            secrets,
        }
    }
```

</details>

##### <span id="GrantSpec.from_pairs"></span><code>from_​pairs</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_pairs(pairs: Vec<(String, Vec<String>)>) -> Self
```

Build a [`GrantSpec`](../../../../../rust/cloacina/registry/loader/grants/#struct.GrantSpec) from raw `(kind, patterns)` pairs — the shape both consumer macros (`constructor!` and `#[reactor]`) lower the `grants = { .. }` literal to, mirroring how `config` is carried as raw pairs and bound at load. Recognized kinds: `http`, `tcp`, `fs`, `env`, `secrets`; an unrecognized kind is ignored here (the macro validates kinds at compile time, so this stays infallible). The `secrets` kind lowers the authored `secrets = ["db_prod", ..]` allow-list (CLOACI-T-0860, design D-3).

<details>
<summary>Source</summary>

```rust
    pub fn from_pairs(pairs: Vec<(String, Vec<String>)>) -> Self {
        let mut spec = Self::default();
        for (kind, patterns) in pairs {
            match kind.as_str() {
                "http" => spec.http.extend(patterns),
                "tcp" => spec.tcp.extend(patterns),
                "fs" => spec.fs.extend(patterns),
                "env" => spec.env.extend(patterns),
                "secrets" => spec.secrets.extend(patterns),
                _ => {}
            }
        }
        spec
    }
```

</details>

##### <span id="GrantSpec.is_empty"></span><code>is_​empty</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_empty(&self) -> bool
```

True when no capability of any kind is granted (the default-closed case).

<details>
<summary>Source</summary>

```rust
    pub fn is_empty(&self) -> bool {
        self.http.is_empty()
            && self.tcp.is_empty()
            && self.fs.is_empty()
            && self.env.is_empty()
            && self.secrets.is_empty()
    }
```

</details>

### <span id="struct.GrantError"></span><code style="font-size: 0.85em">struct GrantError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `PartialEq`, `Eq`

A grant that could not be translated — fails the load closed.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `0` | `String` |  |

### <span id="struct.ResolvedGrants"></span><code style="font-size: 0.85em">struct ResolvedGrants</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

The translated grants: the capability allow-list (`WasiCtx` key) and an optional `EgressPolicy` (the http/tcp per-request/per-peer key). Hand both to `fidius_host::PluginHost::load_wasm_configured_with_grants`.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `capabilities` | `Vec<String>` | The fidius capability allow-list — overrides the package manifest's `[wasm].capabilities`. Empty ⇒ a zero-grant `WasiCtx` (deny-all). |
| `egress` | `Option<Arc<dyn EgressPolicy>>` | The egress policy for `http`/`tcp`. `None` ⇒ fidius's deny-all default (no brokered HTTP/TCP), which is correct when neither is granted. |
| `secrets` | `Vec<String>` | The named secret allow-list the holder may resolve (CLOACI-T-0860, D-3). Lowered verbatim from `GrantSpec::secrets`. **Fail-closed:** empty ⇒ the holder may resolve NO secrets. The `cloacina` runtime turns this into the [`SecretStoreResolver`](../../../../../rust/cloacina/security/secret_resolver/#struct.SecretStoreResolver)'s gated allow-list, which denies any un-granted name BEFORE any decrypt. Carries NAMES ONLY (never values), so it is safe to log in an audit line. |

#### Methods

##### <span id="ResolvedGrants.deny_all"></span><code>deny_​all</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn deny_all() -> Self
```

The fail-closed default: empty allow-list (zero-grant `WasiCtx`), no egress policy, and no granted secrets. A constructor loaded with this reaches nothing. Used at every load site that isn't handed explicit tenant grants.

<details>
<summary>Source</summary>

```rust
    pub fn deny_all() -> Self {
        Self {
            capabilities: Vec::new(),
            egress: None,
            secrets: Vec::new(),
        }
    }
```

</details>

## Functions

### <span id="fn.translate"></span><code style="font-size: 0.85em">fn translate</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn translate(spec: &GrantSpec) -> Result<ResolvedGrants, GrantError>
```

Translate a [`GrantSpec`](../../../../../rust/cloacina/registry/loader/grants/#struct.GrantSpec) into the [`ResolvedGrants`](../../../../../rust/cloacina/registry/loader/grants/#struct.ResolvedGrants) fidius enforces.

Default-closed: an empty spec yields an empty allow-list and `None` egress. Fails
closed ([`GrantError`](../../../../../rust/cloacina/registry/loader/grants/#struct.GrantError)) on a malformed grant (e.g. `fs` without an `ro:`/`rw:`
prefix, an empty env name, a `tcp` pattern without a port).

<details>
<summary>Source</summary>

```rust
pub fn translate(spec: &GrantSpec) -> Result<ResolvedGrants, GrantError> {
    let mut capabilities: Vec<String> = Vec::new();

    // Filesystem: each entry is `ro:<path>` / `rw:<path>` → fidius `fs:ro:`/`fs:rw:`.
    for entry in &spec.fs {
        let cap = if let Some(path) = entry.strip_prefix("ro:") {
            non_empty(path, "fs", "ro:<path>")?;
            format!("fs:ro:{path}")
        } else if let Some(path) = entry.strip_prefix("rw:") {
            non_empty(path, "fs", "rw:<path>")?;
            format!("fs:rw:{path}")
        } else {
            return Err(GrantError(format!(
                "fs grant '{entry}' must start with 'ro:' or 'rw:' (e.g. 'ro:/data')"
            )));
        };
        capabilities.push(cap);
    }

    // Environment: each entry is a variable NAME → fidius `env:<NAME>` (host
    // passthrough). Reject an empty name and the `=value` form (literal injection is
    // not supported in v1 — see S-0014).
    for name in &spec.env {
        if name.is_empty() {
            return Err(GrantError("env grant has an empty variable name".into()));
        }
        if name.contains('=') {
            return Err(GrantError(format!(
                "env grant '{name}' must be a bare variable NAME (host passthrough); \
                 literal `KEY=value` injection is not supported yet"
            )));
        }
        capabilities.push(format!("env:{name}"));
    }

    // HTTP + TCP: the allow-list carries only the *intent* marker (`http` / `tcp`);
    // the per-request/per-peer specifics live in the egress policy below.
    let http = compile_http(&spec.http)?;
    let tcp = compile_tcp(&spec.tcp)?;
    if http.is_some() {
        capabilities.push("http".into());
    }
    if tcp.is_some() {
        capabilities.push("tcp".into());
    }

    let egress: Option<Arc<dyn EgressPolicy>> = if http.is_some() || tcp.is_some() {
        Some(Arc::new(GrantEgressPolicy {
            http: http.unwrap_or_default(),
            tcp: tcp.unwrap_or_default(),
        }))
    } else {
        None
    };

    // Secrets (CLOACI-T-0860, D-3): each entry is a secret NAME the holder may
    // resolve. Names flow verbatim into `ResolvedGrants.secrets`; the resolver
    // enforces membership before any decrypt. Reject an empty name — it can never
    // match a real secret and only muddies the audit line (mirrors the env check).
    for name in &spec.secrets {
        if name.is_empty() {
            return Err(GrantError("secrets grant has an empty secret name".into()));
        }
    }

    Ok(ResolvedGrants {
        capabilities,
        egress,
        secrets: spec.secrets.clone(),
    })
}
```

</details>

### <span id="fn.lint_unmet_intents"></span><code style="font-size: 0.85em">fn lint_​unmet_​intents</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn lint_unmet_intents(
    manifest_caps: &[String],
    spec: &GrantSpec,
) -> Vec<String>
```

Load-time capability lint (REQ-1.3.1): compare the package manifest's declared `[wasm].capabilities` (the author's stated intent) against the tenant's grants, and return a human-readable warning for each capability the component declares an intent to use but the tenant did **not** grant. Advisory only — enforcement still fails closed at runtime; this just surfaces "this constructor wants `http` you didn't grant" early.

<details>
<summary>Source</summary>

```rust
pub fn lint_unmet_intents(manifest_caps: &[String], spec: &GrantSpec) -> Vec<String> {
    let mut warnings = Vec::new();
    for cap in manifest_caps {
        let unmet = match cap.as_str() {
            "http" => spec.http.is_empty(),
            "tcp" | "udp" | "network" | "sockets" => spec.tcp.is_empty(),
            _ if cap.starts_with("fs:") => spec.fs.is_empty(),
            _ if cap.starts_with("env:") => spec.env.is_empty(),
            _ => false,
        };
        if unmet {
            warnings.push(format!(
                "constructor declares capability '{cap}' but the workflow granted no \
                 matching access; it will be denied at runtime (add it to `grants`)"
            ));
        }
    }
    warnings
}
```

</details>
