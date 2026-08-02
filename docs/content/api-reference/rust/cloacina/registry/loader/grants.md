# cloacina::registry::loader::grants <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Constructor capability grants → fidius enforcement (CLOACI-T-0834).

Implements the tenant-authored, **default-closed** capability model decided in
[`CLOACI-A-0009`] and specified in [`CLOACI-S-0014`]. A workflow author writes a
`grants = { http=[..], tcp=[..], fs=[..], env=[..] }` at the constructor
instantiation site; this module translates that into the two keys fidius's
enforcement consumes:
1. a **capability allow-list** (`Vec<String>` — the `WasiCtx` key: `fs:ro:<path>`,
`fs:rw:<path>`, `env:<NAME>`, plus the `http` / `tcp` intent markers), and
2. an **[`EgressPolicy`]** (the per-request HTTP / per-peer TCP key) — supplied to
[`fidius_host::PluginHost::load_wasm_configured_with_grants`].
**Fail-closed by construction.** An empty [`GrantSpec`] yields an empty allow-list
and no policy: the guest gets a zero-grant `WasiCtx` and fidius's deny-all egress
default stands, so the constructor reaches nothing. A constructor can never widen
its own access — enforcement is entirely host-side, keyed on what the *tenant*
wrote here.

## Structs

### `cloacina::registry::loader::grants::GrantSpec`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Default`, `PartialEq`, `Eq`

The tenant's grants for one constructor instance, parsed from the `grants = { .. }` literal. Each list holds raw pattern strings in author order; validation happens in [`translate`]. An all-empty value (the [`Default`]) is the fail-closed default: no capability is granted.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `http` | `Vec < String >` | HTTP egress patterns: `host`, `host:port`, or a URL/path glob
(`https://api.example.com/v1/*`). `*` matches anything. |
| `tcp` | `Vec < String >` | TCP egress patterns: `host:port`, `ip:port`, or `*:port` / `*`. |
| `fs` | `Vec < String >` | Filesystem grants, each `ro:<path>` or `rw:<path>`. |
| `env` | `Vec < String >` | Environment variables to pass through from the host, by name. |
| `secrets` | `Vec < String >` | Named secret allow-list (CLOACI-T-0860, design D-3). Each entry is a
secret NAME the constructor may resolve at fire time. **Fail-closed:** an
absent/empty list means the holder may resolve NO secrets. Tenant-scope is
the outer boundary — a name here can only resolve within the caller's
tenant (the resolver carries the `org_id`). NOT ridden on the egress grant
(D-3): a passphrase-style secret has no network endpoint, and implicit
authz is exactly what we're avoiding. |

#### Methods

##### `from_lists` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_lists (http : Vec < String > , tcp : Vec < String > , fs : Vec < String > , env : Vec < String > , secrets : Vec < String > ,) -> Self
```

Build a [`GrantSpec`] from the grant lists (the shape the macro lowers to). A convenience over the struct literal so generated code is insulated from field additions.

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



##### `from_pairs` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_pairs (pairs : Vec < (String , Vec < String >) >) -> Self
```

Build a [`GrantSpec`] from raw `(kind, patterns)` pairs — the shape both consumer macros (`constructor!` and `#[reactor]`) lower the `grants = { .. }` literal to, mirroring how `config` is carried as raw pairs and bound at load. Recognized kinds: `http`, `tcp`, `fs`, `env`, `secrets`; an unrecognized kind is ignored here (the macro validates kinds at compile time, so this stays infallible). The `secrets` kind lowers the authored `secrets = ["db_prod", ..]` allow-list (CLOACI-T-0860, design D-3).

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



##### `is_empty` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_empty (& self) -> bool
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





### `cloacina::registry::loader::grants::GrantError`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `PartialEq`, `Eq`

A grant that could not be translated — fails the load closed.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `0` | `String` |  |



### `cloacina::registry::loader::grants::ResolvedGrants`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

The translated grants: the capability allow-list (`WasiCtx` key) and an optional [`EgressPolicy`] (the http/tcp per-request/per-peer key). Hand both to [`fidius_host::PluginHost::load_wasm_configured_with_grants`].

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `capabilities` | `Vec < String >` | The fidius capability allow-list — overrides the package manifest's
`[wasm].capabilities`. Empty ⇒ a zero-grant `WasiCtx` (deny-all). |
| `egress` | `Option < Arc < dyn EgressPolicy > >` | The egress policy for `http`/`tcp`. `None` ⇒ fidius's deny-all default
(no brokered HTTP/TCP), which is correct when neither is granted. |
| `secrets` | `Vec < String >` | The named secret allow-list the holder may resolve (CLOACI-T-0860, D-3).
Lowered verbatim from [`GrantSpec::secrets`]. **Fail-closed:** empty ⇒ the
holder may resolve NO secrets. The `cloacina` runtime turns this into the
[`SecretStoreResolver`](crate::security::SecretStoreResolver)'s gated
allow-list, which denies any un-granted name BEFORE any decrypt. Carries
NAMES ONLY (never values), so it is safe to log in an audit line. |

#### Methods

##### `deny_all` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn deny_all () -> Self
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





### `cloacina::registry::loader::grants::HttpPattern`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


**Derives:** `Debug`, `Clone`

One compiled HTTP allow pattern. `host` is matched against the request authority host (glob, `*` = any); `port` (when present) must match exactly; `path` (when present) is a glob over the request path.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `host` | `String` |  |
| `port` | `Option < u16 >` |  |
| `path` | `Option < String >` |  |



### `cloacina::registry::loader::grants::TcpRules`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


**Derives:** `Debug`, `Clone`, `Default`

Compiled TCP allow rules. `any` (a bare `*`) authorizes every peer; `ports` authorizes any host on that port (`*:PORT`); `addrs` authorizes exact resolved peers (a `host:port` resolved once at load, or a literal `ip:port`).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `any` | `bool` |  |
| `ports` | `Vec < u16 >` |  |
| `addrs` | `Vec < SocketAddr >` |  |



### `cloacina::registry::loader::grants::GrantEgressPolicy`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


The [`EgressPolicy`] cloacina hands fidius for a constructor's granted egress. Default-deny: a request/peer is authorized only if it matches a compiled grant.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `http` | `Vec < HttpPattern >` |  |
| `tcp` | `TcpRules` |  |



## Functions

### `cloacina::registry::loader::grants::translate`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn translate (spec : & GrantSpec) -> Result < ResolvedGrants , GrantError >
```

Translate a [`GrantSpec`] into the [`ResolvedGrants`] fidius enforces.

Default-closed: an empty spec yields an empty allow-list and `None` egress. Fails
closed ([`GrantError`]) on a malformed grant (e.g. `fs` without an `ro:`/`rw:`
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



### `cloacina::registry::loader::grants::non_empty`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn non_empty (s : & str , kind : & str , example : & str) -> Result < () , GrantError >
```

<details>
<summary>Source</summary>

```rust
fn non_empty(s: &str, kind: &str, example: &str) -> Result<(), GrantError> {
    if s.is_empty() {
        Err(GrantError(format!(
            "{kind} grant '{example}' requires a value"
        )))
    } else {
        Ok(())
    }
}
```

</details>



### `cloacina::registry::loader::grants::compile_http`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn compile_http (patterns : & [String]) -> Result < Option < Vec < HttpPattern > > , GrantError >
```

<details>
<summary>Source</summary>

```rust
fn compile_http(patterns: &[String]) -> Result<Option<Vec<HttpPattern>>, GrantError> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut out = Vec::with_capacity(patterns.len());
    for raw in patterns {
        // Strip an optional scheme (`https://`); we match host/port/path, not scheme.
        let rest = raw.split_once("://").map(|(_, r)| r).unwrap_or(raw);
        // Split authority from path at the first '/'.
        let (authority, path) = match rest.split_once('/') {
            Some((a, p)) => (a, Some(format!("/{p}"))),
            None => (rest, None),
        };
        // Split host:port (a bare `*` authority means any host).
        let (host, port) = match authority.rsplit_once(':') {
            Some((h, p)) if !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => {
                let port = p
                    .parse::<u16>()
                    .map_err(|_| GrantError(format!("http grant '{raw}' has an invalid port")))?;
                (h.to_string(), Some(port))
            }
            _ => (authority.to_string(), None),
        };
        if host.is_empty() {
            return Err(GrantError(format!("http grant '{raw}' has an empty host")));
        }
        out.push(HttpPattern { host, port, path });
    }
    Ok(Some(out))
}
```

</details>



### `cloacina::registry::loader::grants::compile_tcp`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn compile_tcp (patterns : & [String]) -> Result < Option < TcpRules > , GrantError >
```

<details>
<summary>Source</summary>

```rust
fn compile_tcp(patterns: &[String]) -> Result<Option<TcpRules>, GrantError> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut rules = TcpRules::default();
    for raw in patterns {
        if raw == "*" {
            rules.any = true;
            continue;
        }
        let (host, port_s) = raw.rsplit_once(':').ok_or_else(|| {
            GrantError(format!(
                "tcp grant '{raw}' must be 'host:port' (or '*:port' / '*')"
            ))
        })?;
        let port = port_s
            .parse::<u16>()
            .map_err(|_| GrantError(format!("tcp grant '{raw}' has an invalid port '{port_s}'")))?;
        if host == "*" {
            rules.ports.push(port);
            continue;
        }
        // Literal IP → exact match, no DNS. Otherwise resolve host→IPs once at load.
        if let Ok(ip) = host.parse::<std::net::IpAddr>() {
            rules.addrs.push(SocketAddr::new(ip, port));
        } else {
            // Resolve at load; a resolution failure is non-fatal — the rule simply
            // authorizes nothing (fail-closed) and we log it. (Avoids a load hard-
            // failing on transient DNS while still denying by default.)
            match std::net::ToSocketAddrs::to_socket_addrs(&(host, port)) {
                Ok(addrs) => rules.addrs.extend(addrs),
                Err(_e) => {
                    tracing::warn!(
                        grant = %raw,
                        "tcp grant host did not resolve at load; it authorizes no peer (fail-closed)"
                    );
                }
            }
        }
    }
    Ok(Some(rules))
}
```

</details>



### `cloacina::registry::loader::grants::glob_match`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn glob_match (pattern : & str , text : & str) -> bool
```

Minimal glob match supporting `*` (any run, including empty) anywhere in the pattern. Case-sensitive; no `?`/character classes (not needed for host/path).

<details>
<summary>Source</summary>

```rust
fn glob_match(pattern: &str, text: &str) -> bool {
    // Classic two-pointer wildcard match.
    let (p, t) = (pattern.as_bytes(), text.as_bytes());
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut star, mut mark) = (None, 0usize);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == b'*') {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if pi < p.len() && p[pi] == t[ti] {
            pi += 1;
            ti += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }
    pi == p.len()
}
```

</details>



### `cloacina::registry::loader::grants::lint_unmet_intents`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn lint_unmet_intents (manifest_caps : & [String] , spec : & GrantSpec) -> Vec < String >
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
