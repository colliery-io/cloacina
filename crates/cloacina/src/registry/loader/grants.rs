/*
 *  Copyright 2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! Constructor capability grants → fidius enforcement (CLOACI-T-0834).
//!
//! Implements the tenant-authored, **default-closed** capability model decided in
//! [`CLOACI-A-0009`] and specified in [`CLOACI-S-0014`]. A workflow author writes a
//! `grants = { http=[..], tcp=[..], fs=[..], env=[..] }` at the constructor
//! instantiation site; this module translates that into the two keys fidius's
//! enforcement consumes:
//!
//! 1. a **capability allow-list** (`Vec<String>` — the `WasiCtx` key: `fs:ro:<path>`,
//!    `fs:rw:<path>`, `env:<NAME>`, plus the `http` / `tcp` intent markers), and
//! 2. an **[`EgressPolicy`]** (the per-request HTTP / per-peer TCP key) — supplied to
//!    [`fidius_host::PluginHost::load_wasm_configured_with_grants`].
//!
//! **Fail-closed by construction.** An empty [`GrantSpec`] yields an empty allow-list
//! and no policy: the guest gets a zero-grant `WasiCtx` and fidius's deny-all egress
//! default stands, so the constructor reaches nothing. A constructor can never widen
//! its own access — enforcement is entirely host-side, keyed on what the *tenant*
//! wrote here.
//!
//! ## TCP scope (v1)
//! fidius hands [`EgressPolicy::authorize_tcp`] a **resolved** [`SocketAddr`] (IP:port),
//! not the original hostname. So a `host:port` grant is enforced by resolving `host`
//! to its IP(s) **once at load** and matching `(ip, port)`; the port is always exact.
//! A DNS rebind after load is therefore not re-checked (the IP set is frozen at load).
//! For unconstrained host matching use HTTP (whose policy sees the full URI). See
//! [`CLOACI-S-0014`] "Open Items".

use std::net::SocketAddr;
use std::sync::Arc;

use fidius_host::http_types::request::Parts as HttpParts;
use fidius_host::{EgressDenied, EgressPolicy};

/// The tenant's grants for one constructor instance, parsed from the
/// `grants = { .. }` literal. Each list holds raw pattern strings in author order;
/// validation happens in [`translate`]. An all-empty value (the [`Default`]) is the
/// fail-closed default: no capability is granted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GrantSpec {
    /// HTTP egress patterns: `host`, `host:port`, or a URL/path glob
    /// (`https://api.example.com/v1/*`). `*` matches anything.
    pub http: Vec<String>,
    /// TCP egress patterns: `host:port`, `ip:port`, or `*:port` / `*`.
    pub tcp: Vec<String>,
    /// Filesystem grants, each `ro:<path>` or `rw:<path>`.
    pub fs: Vec<String>,
    /// Environment variables to pass through from the host, by name.
    pub env: Vec<String>,
    /// Named secret allow-list (CLOACI-T-0860, design D-3). Each entry is a
    /// secret NAME the constructor may resolve at fire time. **Fail-closed:** an
    /// absent/empty list means the holder may resolve NO secrets. Tenant-scope is
    /// the outer boundary — a name here can only resolve within the caller's
    /// tenant (the resolver carries the `org_id`). NOT ridden on the egress grant
    /// (D-3): a passphrase-style secret has no network endpoint, and implicit
    /// authz is exactly what we're avoiding.
    pub secrets: Vec<String>,
}

impl GrantSpec {
    /// Build a [`GrantSpec`] from the grant lists (the shape the macro lowers
    /// to). A convenience over the struct literal so generated code is insulated
    /// from field additions.
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

    /// Build a [`GrantSpec`] from raw `(kind, patterns)` pairs — the shape both
    /// consumer macros (`constructor!` and `#[reactor]`) lower the `grants = { .. }`
    /// literal to, mirroring how `config` is carried as raw pairs and bound at load.
    /// Recognized kinds: `http`, `tcp`, `fs`, `env`, `secrets`; an unrecognized kind
    /// is ignored here (the macro validates kinds at compile time, so this stays
    /// infallible). The `secrets` kind lowers the authored `secrets = ["db_prod", ..]`
    /// allow-list (CLOACI-T-0860, design D-3).
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

    /// True when no capability of any kind is granted (the default-closed case).
    pub fn is_empty(&self) -> bool {
        self.http.is_empty()
            && self.tcp.is_empty()
            && self.fs.is_empty()
            && self.env.is_empty()
            && self.secrets.is_empty()
    }
}

/// A grant that could not be translated — fails the load closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantError(pub String);

impl std::fmt::Display for GrantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid constructor grant: {}", self.0)
    }
}

impl std::error::Error for GrantError {}

/// The translated grants: the capability allow-list (`WasiCtx` key) and an optional
/// [`EgressPolicy`] (the http/tcp per-request/per-peer key). Hand both to
/// [`fidius_host::PluginHost::load_wasm_configured_with_grants`].
#[derive(Clone)]
pub struct ResolvedGrants {
    /// The fidius capability allow-list — overrides the package manifest's
    /// `[wasm].capabilities`. Empty ⇒ a zero-grant `WasiCtx` (deny-all).
    pub capabilities: Vec<String>,
    /// The egress policy for `http`/`tcp`. `None` ⇒ fidius's deny-all default
    /// (no brokered HTTP/TCP), which is correct when neither is granted.
    pub egress: Option<Arc<dyn EgressPolicy>>,
    /// The named secret allow-list the holder may resolve (CLOACI-T-0860, D-3).
    /// Lowered verbatim from [`GrantSpec::secrets`]. **Fail-closed:** empty ⇒ the
    /// holder may resolve NO secrets. The `cloacina` runtime turns this into the
    /// [`SecretStoreResolver`](crate::security::SecretStoreResolver)'s gated
    /// allow-list, which denies any un-granted name BEFORE any decrypt. Carries
    /// NAMES ONLY (never values), so it is safe to log in an audit line.
    pub secrets: Vec<String>,
}

impl ResolvedGrants {
    /// The fail-closed default: empty allow-list (zero-grant `WasiCtx`), no
    /// egress policy, and no granted secrets. A constructor loaded with this
    /// reaches nothing. Used at every load site that isn't handed explicit tenant
    /// grants.
    pub fn deny_all() -> Self {
        Self {
            capabilities: Vec::new(),
            egress: None,
            secrets: Vec::new(),
        }
    }
}

impl Default for ResolvedGrants {
    fn default() -> Self {
        Self::deny_all()
    }
}

/// Translate a [`GrantSpec`] into the [`ResolvedGrants`] fidius enforces.
///
/// Default-closed: an empty spec yields an empty allow-list and `None` egress. Fails
/// closed ([`GrantError`]) on a malformed grant (e.g. `fs` without an `ro:`/`rw:`
/// prefix, an empty env name, a `tcp` pattern without a port).
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

fn non_empty(s: &str, kind: &str, example: &str) -> Result<(), GrantError> {
    if s.is_empty() {
        Err(GrantError(format!(
            "{kind} grant '{example}' requires a value"
        )))
    } else {
        Ok(())
    }
}

// ===========================================================================
// HTTP matching
// ===========================================================================

/// One compiled HTTP allow pattern. `host` is matched against the request authority
/// host (glob, `*` = any); `port` (when present) must match exactly; `path` (when
/// present) is a glob over the request path.
#[derive(Debug, Clone)]
struct HttpPattern {
    host: String,
    port: Option<u16>,
    path: Option<String>,
}

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

// ===========================================================================
// TCP matching
// ===========================================================================

/// Compiled TCP allow rules. `any` (a bare `*`) authorizes every peer; `ports`
/// authorizes any host on that port (`*:PORT`); `addrs` authorizes exact resolved
/// peers (a `host:port` resolved once at load, or a literal `ip:port`).
#[derive(Debug, Clone, Default)]
struct TcpRules {
    any: bool,
    ports: Vec<u16>,
    addrs: Vec<SocketAddr>,
}

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

/// The [`EgressPolicy`] cloacina hands fidius for a constructor's granted egress.
/// Default-deny: a request/peer is authorized only if it matches a compiled grant.
struct GrantEgressPolicy {
    http: Vec<HttpPattern>,
    tcp: TcpRules,
}

impl std::fmt::Debug for GrantEgressPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrantEgressPolicy")
            .field("http_patterns", &self.http.len())
            .field("tcp", &self.tcp)
            .finish()
    }
}

impl EgressPolicy for GrantEgressPolicy {
    fn authorize(&self, parts: &mut HttpParts) -> Result<(), EgressDenied> {
        let uri = &parts.uri;
        let host = uri.host().unwrap_or("");
        let port = uri.port_u16();
        let path = uri.path();
        for p in &self.http {
            let host_ok = p.host == "*" || glob_match(&p.host, host);
            let port_ok = match p.port {
                Some(want) => Some(want) == port,
                None => true,
            };
            let path_ok = match &p.path {
                Some(pat) => glob_match(pat, path),
                None => true,
            };
            if host_ok && port_ok && path_ok {
                return Ok(());
            }
        }
        Err(EgressDenied::new(format!(
            "http egress to {host}{} {path} not in the constructor's grants",
            port.map(|p| format!(":{p}")).unwrap_or_default()
        )))
    }

    fn authorize_tcp(&self, addr: &SocketAddr) -> Result<(), EgressDenied> {
        if self.tcp.any || self.tcp.ports.contains(&addr.port()) || self.tcp.addrs.contains(addr) {
            return Ok(());
        }
        Err(EgressDenied::new(format!(
            "tcp egress to {addr} not in the constructor's grants"
        )))
    }
}

// ===========================================================================
// Glob + lint
// ===========================================================================

/// Minimal glob match supporting `*` (any run, including empty) anywhere in the
/// pattern. Case-sensitive; no `?`/character classes (not needed for host/path).
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

/// Load-time capability lint (REQ-1.3.1): compare the package manifest's declared
/// `[wasm].capabilities` (the author's stated intent) against the tenant's grants,
/// and return a human-readable warning for each capability the component declares an
/// intent to use but the tenant did **not** grant. Advisory only — enforcement still
/// fails closed at runtime; this just surfaces "this constructor wants `http` you
/// didn't grant" early.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn parts(uri: &str) -> HttpParts {
        let req = fidius_host::http_types::Request::builder()
            .uri(uri)
            .body(())
            .unwrap();
        req.into_parts().0
    }

    #[test]
    fn empty_spec_is_deny_all() {
        let r = translate(&GrantSpec::default()).unwrap();
        assert!(r.capabilities.is_empty());
        assert!(r.egress.is_none(), "no grant ⇒ no policy ⇒ fidius deny-all");
    }

    #[test]
    fn fs_grants_map_to_scoped_caps() {
        let spec = GrantSpec::from_lists(
            vec![],
            vec![],
            vec!["ro:/data".into(), "rw:/out".into()],
            vec![],
            vec![],
        );
        let r = translate(&spec).unwrap();
        assert_eq!(r.capabilities, vec!["fs:ro:/data", "fs:rw:/out"]);
        assert!(r.egress.is_none());
    }

    #[test]
    fn fs_without_mode_prefix_fails_closed() {
        let spec = GrantSpec::from_lists(vec![], vec![], vec!["/data".into()], vec![], vec![]);
        assert!(translate(&spec).is_err());
    }

    #[test]
    fn env_grants_map_to_scoped_caps_and_reject_literals() {
        let ok = GrantSpec::from_lists(vec![], vec![], vec![], vec!["STRIPE_KEY".into()], vec![]);
        assert_eq!(translate(&ok).unwrap().capabilities, vec!["env:STRIPE_KEY"]);

        let literal = GrantSpec::from_lists(vec![], vec![], vec![], vec!["K=v".into()], vec![]);
        assert!(translate(&literal).is_err());
        let empty = GrantSpec::from_lists(vec![], vec![], vec![], vec!["".into()], vec![]);
        assert!(translate(&empty).is_err());
    }

    #[test]
    fn http_intent_marker_and_policy_present() {
        let spec = GrantSpec::from_lists(
            vec!["api.example.com:443".into()],
            vec![],
            vec![],
            vec![],
            vec![],
        );
        let r = translate(&spec).unwrap();
        assert_eq!(r.capabilities, vec!["http"]);
        assert!(r.egress.is_some());
    }

    #[test]
    fn http_policy_matches_host_port_and_path() {
        let spec = GrantSpec::from_lists(
            vec!["https://api.example.com/v1/*".into()],
            vec![],
            vec![],
            vec![],
            vec![],
        );
        let policy = translate(&spec).unwrap().egress.unwrap();

        // In-scope path on the granted host.
        assert!(policy
            .authorize(&mut parts("https://api.example.com/v1/charge"))
            .is_ok());
        // Out-of-scope path is denied.
        assert!(policy
            .authorize(&mut parts("https://api.example.com/v2/admin"))
            .is_err());
        // Different host is denied.
        assert!(policy
            .authorize(&mut parts("https://evil.example.com/v1/charge"))
            .is_err());
    }

    #[test]
    fn http_host_only_grant_allows_any_path() {
        let spec = GrantSpec::from_lists(
            vec!["api.example.com".into()],
            vec![],
            vec![],
            vec![],
            vec![],
        );
        let policy = translate(&spec).unwrap().egress.unwrap();
        assert!(policy
            .authorize(&mut parts("https://api.example.com/anything"))
            .is_ok());
        assert!(policy
            .authorize(&mut parts("https://other.example.com/anything"))
            .is_err());
    }

    #[test]
    fn http_star_allows_all() {
        let spec = GrantSpec::from_lists(vec!["*".into()], vec![], vec![], vec![], vec![]);
        let policy = translate(&spec).unwrap().egress.unwrap();
        assert!(policy
            .authorize(&mut parts("https://anywhere.test/x"))
            .is_ok());
    }

    #[test]
    fn tcp_star_port_matches_any_host_on_that_port() {
        let spec = GrantSpec::from_lists(vec![], vec!["*:5432".into()], vec![], vec![], vec![]);
        let r = translate(&spec).unwrap();
        assert_eq!(r.capabilities, vec!["tcp"]);
        let policy = r.egress.unwrap();
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 9));
        assert!(policy.authorize_tcp(&SocketAddr::new(ip, 5432)).is_ok());
        assert!(policy.authorize_tcp(&SocketAddr::new(ip, 5433)).is_err());
    }

    #[test]
    fn tcp_literal_ip_port_exact_match() {
        let spec =
            GrantSpec::from_lists(vec![], vec!["10.0.0.5:5432".into()], vec![], vec![], vec![]);
        let policy = translate(&spec).unwrap().egress.unwrap();
        let ok = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5)), 5432);
        let wrong_ip = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 6)), 5432);
        let wrong_port = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5)), 1);
        assert!(policy.authorize_tcp(&ok).is_ok());
        assert!(policy.authorize_tcp(&wrong_ip).is_err());
        assert!(policy.authorize_tcp(&wrong_port).is_err());
    }

    #[test]
    fn tcp_without_port_fails_closed() {
        let spec =
            GrantSpec::from_lists(vec![], vec!["db.internal".into()], vec![], vec![], vec![]);
        assert!(translate(&spec).is_err());
    }

    #[test]
    fn combined_grants_accumulate_caps() {
        let spec = GrantSpec::from_lists(
            vec!["api.example.com:443".into()],
            vec!["*:5432".into()],
            vec!["ro:/data".into()],
            vec!["TOKEN".into()],
            vec![],
        );
        let r = translate(&spec).unwrap();
        assert!(r.capabilities.contains(&"http".to_string()));
        assert!(r.capabilities.contains(&"tcp".to_string()));
        assert!(r.capabilities.contains(&"fs:ro:/data".to_string()));
        assert!(r.capabilities.contains(&"env:TOKEN".to_string()));
        assert!(r.egress.is_some());
    }

    // ── Secrets allow-list (CLOACI-T-0860, D-3) ─────────────────────────────

    #[test]
    fn secrets_flow_into_resolved_grants() {
        let spec = GrantSpec::from_lists(
            vec![],
            vec![],
            vec![],
            vec![],
            vec!["db_prod".into(), "stripe".into()],
        );
        let r = translate(&spec).unwrap();
        assert_eq!(r.secrets, vec!["db_prod", "stripe"]);
        // Secrets are not a WASI capability and carry no egress policy.
        assert!(r.capabilities.is_empty());
        assert!(r.egress.is_none());
    }

    #[test]
    fn empty_secrets_grant_is_empty() {
        // Fail-closed: an absent secrets list ⇒ no secret may be resolved.
        let r = translate(&GrantSpec::default()).unwrap();
        assert!(r.secrets.is_empty());
    }

    #[test]
    fn secrets_lower_from_raw_pairs() {
        // The authoring path: the macro lowers `secrets = ["db_prod"]` to a
        // ("secrets", [..]) pair carried through `from_pairs`.
        let spec = GrantSpec::from_pairs(vec![(
            "secrets".to_string(),
            vec!["db_prod".to_string(), "cache".to_string()],
        )]);
        assert_eq!(spec.secrets, vec!["db_prod", "cache"]);
        assert!(!spec.is_empty());
        assert_eq!(translate(&spec).unwrap().secrets, vec!["db_prod", "cache"]);
    }

    #[test]
    fn empty_secret_name_fails_closed() {
        let spec = GrantSpec::from_lists(vec![], vec![], vec![], vec![], vec!["".into()]);
        assert!(translate(&spec).is_err());
    }

    #[test]
    fn lint_flags_unmet_intent_only() {
        let manifest = vec!["http".to_string(), "env:TOKEN".to_string()];
        // Granted http but not env → exactly one warning (the env intent).
        let spec = GrantSpec::from_lists(vec!["*".into()], vec![], vec![], vec![], vec![]);
        let w = lint_unmet_intents(&manifest, &spec);
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("env:TOKEN"));

        // Grant both → no warnings.
        let spec2 = GrantSpec::from_lists(
            vec!["*".into()],
            vec![],
            vec![],
            vec!["TOKEN".into()],
            vec![],
        );
        assert!(lint_unmet_intents(&manifest, &spec2).is_empty());
    }

    #[test]
    fn glob_match_basics() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("/v1/*", "/v1/charge"));
        assert!(glob_match("/v1/*", "/v1/"));
        assert!(!glob_match("/v1/*", "/v2/charge"));
        assert!(glob_match("a*c", "abc"));
        assert!(glob_match("a*c", "ac"));
        assert!(!glob_match("a*c", "ab"));
        assert!(glob_match("*.example.com", "api.example.com"));
        assert!(!glob_match("*.example.com", "example.com"));
    }
}
