/*
 *  Copyright 2025-2026 Colliery Software
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

//! Host-side credential injection (COLLIERY-I-0633 / CLOACINA-T-0982): a
//! credential the egress policy applies to each outbound request a sandboxed
//! connector makes, so the secret never enters the guest. Moved from weir's
//! runtime with four variants: a header, a query parameter, AWS SigV4 and the
//! Snowflake key-pair JWT. OAuth2, session tokens and Google service accounts
//! come with the REST connectors in Sync 3.
//!
//! [`CredentialEgress`] wraps the grant policy: the allow-list decides first,
//! then the credential is applied. A request the grants deny never sees the
//! credential.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use fidius_host::http_types::request::Parts as HttpParts;
use fidius_host::{EgressDenied, EgressPolicy, TcpTarget};

/// A credential the host applies to each outbound HTTP request.
pub enum Credential {
    /// Inject a fixed header. A bearer is `name = "Authorization"`,
    /// `value = "Bearer <token>"`.
    Header { name: String, value: String },
    /// Append `?<param>=<value>` to the outbound URI, the value percent-encoded once.
    Query { param: String, value: String },
    /// Sign each request with AWS SigV4 (S3 and S3-compatible stores). GET-only:
    /// the payload hash is the empty one.
    AwsSigV4 {
        access_key: String,
        secret_key: String,
        region: String,
        service: String,
    },
    /// Snowflake key-pair JWT: a self-issued RS256 JWT injected as a bearer plus
    /// `X-Snowflake-Authorization-Token-Type: KEYPAIR_JWT`.
    SnowflakeKeypairJwt(SnowflakeKeypairProvider),
    /// HTTP Basic: `Authorization: Basic base64(username:password)`.
    Basic { username: String, password: String },
    /// OAuth2: the host runs the grant, caches the access token and adds it as a
    /// bearer (CLOACINA-T-0989).
    OAuth2(OAuth2Provider),
    /// A session token: the host logs in, reads the token from the answer, and
    /// adds it as a header.
    Session(SessionProvider),
    /// A Google service account: the host signs a JWT-bearer grant with the
    /// account's key and adds the access token as a bearer.
    GoogleServiceAccount(GoogleServiceAccountProvider),
}

impl std::fmt::Debug for Credential {
    // Never print a secret.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            Credential::Header { name, .. } => format!("Header({name})"),
            Credential::Query { param, .. } => format!("Query({param})"),
            Credential::AwsSigV4 {
                region, service, ..
            } => format!("AwsSigV4({region}/{service})"),
            Credential::SnowflakeKeypairJwt(p) => {
                format!("SnowflakeKeypairJwt({}.{})", p.account, p.user)
            }
            Credential::Basic { username, .. } => format!("Basic({username})"),
            Credential::OAuth2(p) => format!("OAuth2({})", p.token_url),
            Credential::Session(p) => format!("Session({})", p.login_url),
            Credential::GoogleServiceAccount(p) => {
                format!("GoogleServiceAccount({})", p.client_email)
            }
        };
        f.write_str(&kind)
    }
}

impl Credential {
    /// Apply the credential to one outbound request.
    pub fn apply(&self, parts: &mut HttpParts) -> Result<(), EgressDenied> {
        match self {
            Credential::Header { name, value } => inject_header(parts, name, value),
            Credential::Query { param, value } => append_query(parts, param, value),
            Credential::SnowflakeKeypairJwt(p) => {
                let token = p.bearer()?;
                inject_header(parts, "Authorization", &format!("Bearer {token}"))?;
                inject_header(parts, "X-Snowflake-Authorization-Token-Type", "KEYPAIR_JWT")
            }
            Credential::AwsSigV4 {
                access_key,
                secret_key,
                region,
                service,
            } => {
                // The signed `host` must equal the Host header the client sends: the
                // URI authority, including a non-default port (MinIO on :9000).
                let host = parts
                    .uri
                    .authority()
                    .map(|a| a.as_str().to_string())
                    .or_else(|| {
                        parts
                            .headers
                            .get("host")
                            .and_then(|h| h.to_str().ok())
                            .map(str::to_string)
                    })
                    .ok_or_else(|| EgressDenied::new("sigv4: request has no host"))?;
                let (amz_date, datestamp) = now_amz();
                let payload_hash = sigv4::EMPTY_PAYLOAD_SHA256;
                let canonical_uri = canonical_uri(parts.uri.path());
                let canonical_query = canonical_query(parts.uri.query().unwrap_or(""));
                inject_header(parts, "x-amz-date", &amz_date)?;
                inject_header(parts, "x-amz-content-sha256", payload_hash)?;
                let signed = vec![
                    ("host".to_string(), host),
                    ("x-amz-content-sha256".to_string(), payload_hash.to_string()),
                    ("x-amz-date".to_string(), amz_date.clone()),
                ];
                let auth = sigv4::SigV4 {
                    access_key,
                    secret_key,
                    region,
                    service,
                }
                .authorization(
                    parts.method.as_str(),
                    &canonical_uri,
                    &canonical_query,
                    &signed,
                    payload_hash,
                    &amz_date,
                    &datestamp,
                );
                inject_header(parts, "Authorization", &auth)
            }
            Credential::Basic { username, password } => {
                use base64::Engine;
                let encoded = base64::engine::general_purpose::STANDARD
                    .encode(format!("{username}:{password}"));
                inject_header(parts, "Authorization", &format!("Basic {encoded}"))
            }
            Credential::OAuth2(p) => {
                let token = p.bearer()?;
                inject_header(parts, "Authorization", &format!("Bearer {token}"))
            }
            Credential::Session(p) => {
                let token = p.token()?;
                inject_header(parts, &p.inject_header, &format!("{}{token}", p.prefix))
            }
            Credential::GoogleServiceAccount(p) => {
                let token = p.bearer()?;
                inject_header(parts, "Authorization", &format!("Bearer {token}"))
            }
        }
    }
}

/// The grant policy with a credential applied after it: the allow-list decides,
/// then the request is decorated. TCP egress passes straight through.
pub struct CredentialEgress {
    inner: Option<Arc<dyn EgressPolicy>>,
    credential: Credential,
}

impl CredentialEgress {
    pub fn new(inner: Option<Arc<dyn EgressPolicy>>, credential: Credential) -> Self {
        Self { inner, credential }
    }
}

impl std::fmt::Debug for CredentialEgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialEgress")
            .field("credential", &self.credential)
            .finish()
    }
}

impl EgressPolicy for CredentialEgress {
    fn authorize(&self, parts: &mut HttpParts) -> Result<(), EgressDenied> {
        match &self.inner {
            Some(inner) => inner.authorize(parts)?,
            None => return Err(EgressDenied::new("no http egress granted")),
        }
        // The guest only sees "request denied", so the reason goes to the log:
        // a refused grant or a rotated refresh token is the operator's to fix.
        // Reasons name URLs and fields, never secret values.
        self.credential.apply(parts).inspect_err(|e| {
            tracing::warn!(credential = ?self.credential, reason = %e.reason, "credential not applied; request denied");
        })
    }

    fn authorize_tcp(&self, addr: &SocketAddr) -> Result<(), EgressDenied> {
        match &self.inner {
            Some(inner) => inner.authorize_tcp(addr),
            None => Err(EgressDenied::new("no tcp egress granted")),
        }
    }

    fn authorize_tcp_target(&self, target: &TcpTarget) -> Result<(), EgressDenied> {
        match &self.inner {
            Some(inner) => inner.authorize_tcp_target(target),
            None => Err(EgressDenied::new("no tcp egress granted")),
        }
    }
}

/// Current UTC as SigV4's `(YYYYMMDDTHHMMSSZ, YYYYMMDD)`.
fn now_amz() -> (String, String) {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    epoch_to_amz(secs)
}

/// Convert Unix seconds → SigV4 timestamps (civil date via Howard Hinnant's `days → y/m/d`).
fn epoch_to_amz(secs: u64) -> (String, String) {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (
        format!("{y:04}{m:02}{d:02}T{h:02}{mi:02}{s:02}Z"),
        format!("{y:04}{m:02}{d:02}"),
    )
}

/// Percent-encode per RFC 3986 (unreserved `A-Za-z0-9-._~` pass through). `keep_slash` leaves `/`
/// unescaped, as S3's canonical URI requires.
fn uri_encode(s: &str, keep_slash: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        let unreserved = b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~');
        if unreserved || (keep_slash && b == b'/') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// S3 canonical URI: the path with each segment percent-encoded, `/` preserved.
fn canonical_uri(path: &str) -> String {
    if path.is_empty() {
        "/".to_string()
    } else {
        uri_encode(path, true)
    }
}

/// SigV4 canonical query: each `k=v` DECODED then percent-encoded exactly once,
/// sorted by encoded key. The wire query is already percent-encoded by the
/// caller — encoding it again double-encodes (`%3D` → `%253D`) and the
/// signature no longer matches what the server computes. ListObjectsV2
/// continuation tokens exposed this; any reserved character in
/// `prefix`/`start-after` hits it too.
fn canonical_query(query: &str) -> String {
    if query.is_empty() {
        return String::new();
    }
    let mut pairs: Vec<(String, String)> = query
        .split('&')
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (
                uri_encode(&uri_decode(k), false),
                uri_encode(&uri_decode(v), false),
            )
        })
        .collect();
    pairs.sort();
    pairs
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&")
}

/// Percent-decode (`%XX` only; a literal `+` stays a plus — S3 object keys with
/// `+` arrive as `%2B`). Malformed escapes pass through untouched.
fn uri_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let (Some(h), Some(l)) = (
                bytes.get(i + 1).and_then(|b| (*b as char).to_digit(16)),
                bytes.get(i + 2).and_then(|b| (*b as char).to_digit(16)),
            )
        {
            out.push((h * 16 + l) as u8);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Insert `name: value` into the request headers, failing closed on an invalid name/value.
fn inject_header(parts: &mut HttpParts, name: &str, value: &str) -> Result<(), EgressDenied> {
    use fidius_host::http_types::header::{HeaderName, HeaderValue};
    let n = HeaderName::from_bytes(name.as_bytes())
        .map_err(|e| EgressDenied::new(format!("invalid header name `{name}`: {e}")))?;
    let v = HeaderValue::from_str(value)
        .map_err(|_| EgressDenied::new("credential is not a valid header value"))?;
    parts.headers.insert(n, v);
    Ok(())
}

/// Append `?<param>=<value>` (or `&…`) to the outbound URI. The value is the RAW
/// secret and is percent-encoded exactly once here — a key with
/// `& = + #` would otherwise corrupt the query (or truncate it as a fragment). The
/// param name is author-controlled config and goes verbatim. SigV4 canonicalization
/// stays consistent: it decodes the wire query then re-encodes once.
fn append_query(parts: &mut HttpParts, param: &str, value: &str) -> Result<(), EgressDenied> {
    let mut s = parts.uri.to_string();
    s.push(if parts.uri.query().is_some() {
        '&'
    } else {
        '?'
    });
    s.push_str(param);
    s.push('=');
    s.push_str(&uri_encode(value, false));
    parts.uri = s
        .parse()
        .map_err(|e| EgressDenied::new(format!("uri rewrite for query-param auth failed: {e}")))?;
    Ok(())
}

/// Snowflake key-pair JWT auth: the host signs a self-issued RS256 JWT; there is
/// no token endpoint, the JWT is the bearer. Claims per Snowflake's key-pair
/// spec: `iss` = `ACCOUNT.USER.SHA256:<public-key fingerprint>`, `sub` =
/// `ACCOUNT.USER` (uppercase), one hour validity. Minting is a local sign;
/// the token is cached and re-signed a minute before it expires.
pub struct SnowflakeKeypairProvider {
    account: String,
    user: String,
    private_key_pem: String,
    cached: Mutex<Option<(String, u64)>>,
}

impl SnowflakeKeypairProvider {
    pub fn new(account: String, user: String, private_key_pem: String) -> Self {
        Self {
            account: account.to_uppercase(),
            user: user.to_uppercase(),
            private_key_pem,
            cached: Mutex::new(None),
        }
    }

    fn bearer(&self) -> Result<String, EgressDenied> {
        let now = unix_now_secs();
        let mut slot = self.cached.lock().unwrap();
        if let Some((token, exp)) = slot.as_ref()
            && now + 60 < *exp
        {
            return Ok(token.clone());
        }
        let jwt = snowflake_jwt(&self.account, &self.user, now, &self.private_key_pem)
            .map_err(EgressDenied::new)?;
        *slot = Some((jwt.clone(), now + 3600));
        Ok(jwt)
    }
}

// ---- minted tokens: OAuth2, sessions, Google service accounts ----
//
// A token the host mints is kept for the process: on an agent, every load of
// a connector with the same credential shares it, so a sync does not mint a
// token per segment. An entry is keyed by a digest of who the token is for
// (`slot`) and holds a digest of the whole credential (`credential`); a
// lookup with another credential, a rotated secret, mints afresh and replaces
// the entry. Neither digest is secret text.

/// Mint again this long before a token expires.
const REFRESH_MARGIN: std::time::Duration = std::time::Duration::from_secs(60);

struct Minted {
    credential: String,
    token: String,
    expires_at: std::time::Instant,
}

static MINTED: std::sync::OnceLock<Mutex<std::collections::HashMap<String, Minted>>> =
    std::sync::OnceLock::new();

/// Where a provider's token lives in [`MINTED`].
struct TokenIds {
    slot: String,
    credential: String,
}

impl TokenIds {
    fn new(identity: &[&str], secret: &[&str]) -> Self {
        let digest = |parts: &[&str]| {
            use sha2::Digest;
            let mut h = sha2::Sha256::new();
            h.update(b"cloacina-minted-token-v1");
            for p in parts {
                h.update((p.len() as u64).to_le_bytes());
                h.update(p.as_bytes());
            }
            hex::encode(h.finalize())
        };
        let all: Vec<&str> = identity.iter().chain(secret).copied().collect();
        Self {
            slot: digest(identity),
            credential: digest(&all),
        }
    }
}

/// A cached token for `ids`, or one from `mint` (a token and its lifetime).
/// The lock is not held while minting; two racers both minting is harmless.
fn cached_token(
    ids: &TokenIds,
    mint: impl FnOnce() -> Result<(String, std::time::Duration), EgressDenied>,
) -> Result<String, EgressDenied> {
    let cache = MINTED.get_or_init(Default::default);
    {
        let map = cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(m) = map.get(&ids.slot)
            && m.credential == ids.credential
            && std::time::Instant::now() + REFRESH_MARGIN < m.expires_at
        {
            return Ok(m.token.clone());
        }
    }
    let (token, ttl) = mint()?;
    cache.lock().unwrap_or_else(|e| e.into_inner()).insert(
        ids.slot.clone(),
        Minted {
            credential: ids.credential.clone(),
            token: token.clone(),
            expires_at: std::time::Instant::now() + ttl,
        },
    );
    Ok(token)
}

/// Run a blocking request on its own thread: the egress hook can fire on an
/// async executor's thread, where a blocking client must not run.
fn on_own_thread<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, EgressDenied> {
    std::thread::spawn(f)
        .join()
        .map_err(|_| EgressDenied::new("the credential request thread panicked"))?
        .map_err(EgressDenied::new)
}

/// POST a form and read a JSON answer. Errors carry the status and the
/// answer's `error` field, never the form.
fn post_form(url: &str, form: Vec<(String, String)>) -> Result<serde_json::Value, String> {
    let resp = reqwest::blocking::Client::new()
        .post(url)
        .form(&form)
        .send()
        .map_err(|e| format!("{url}: {e}"))?;
    json_answer(url, resp)
}

fn json_answer(url: &str, resp: reqwest::blocking::Response) -> Result<serde_json::Value, String> {
    let status = resp.status();
    let body: serde_json::Value = resp
        .json()
        .map_err(|e| format!("{url}: the answer is not JSON: {e}"))?;
    if !status.is_success() {
        let why = body
            .get("error_description")
            .or_else(|| body.get("error"))
            .and_then(|e| e.as_str())
            .unwrap_or("no reason given");
        return Err(format!("{url}: HTTP {}: {why}", status.as_u16()));
    }
    Ok(body)
}

/// `access_token` and `expires_in` (default an hour) of a token answer.
fn access_token(
    url: &str,
    body: &serde_json::Value,
) -> Result<(String, std::time::Duration), String> {
    let token = body
        .get("access_token")
        .and_then(|t| t.as_str())
        .ok_or_else(|| format!("{url}: the answer has no `access_token`"))?
        .to_string();
    let ttl = body
        .get("expires_in")
        .and_then(|e| e.as_u64())
        .unwrap_or(3600);
    Ok((token, std::time::Duration::from_secs(ttl)))
}

/// The OAuth2 grant the host runs.
#[derive(Clone)]
pub enum OAuthGrant {
    /// `grant_type=refresh_token`.
    RefreshToken { refresh_token: String },
    /// `grant_type=client_credentials`.
    ClientCredentials,
}

/// Mints and caches an OAuth2 access token on the host.
pub struct OAuth2Provider {
    token_url: String,
    grant: OAuthGrant,
    client_id: String,
    client_secret: String,
    scopes: Vec<String>,
    ids: TokenIds,
}

impl OAuth2Provider {
    pub fn new(
        token_url: String,
        grant: OAuthGrant,
        client_id: String,
        client_secret: String,
        scopes: Vec<String>,
    ) -> Self {
        let scope = scopes.join(" ");
        let refresh = match &grant {
            OAuthGrant::RefreshToken { refresh_token } => refresh_token.as_str(),
            OAuthGrant::ClientCredentials => "",
        };
        let ids = TokenIds::new(
            &["oauth2", &token_url, &client_id, &scope],
            &[&client_secret, refresh],
        );
        Self {
            token_url,
            grant,
            client_id,
            client_secret,
            scopes,
            ids,
        }
    }

    fn bearer(&self) -> Result<String, EgressDenied> {
        cached_token(&self.ids, || self.mint())
    }

    fn mint(&self) -> Result<(String, std::time::Duration), EgressDenied> {
        let url = self.token_url.clone();
        let mut form = vec![
            ("client_id".to_string(), self.client_id.clone()),
            ("client_secret".to_string(), self.client_secret.clone()),
        ];
        let ours = match &self.grant {
            OAuthGrant::RefreshToken { refresh_token } => {
                form.push(("grant_type".into(), "refresh_token".into()));
                form.push(("refresh_token".into(), refresh_token.clone()));
                Some(refresh_token.clone())
            }
            OAuthGrant::ClientCredentials => {
                form.push(("grant_type".into(), "client_credentials".into()));
                None
            }
        };
        if !self.scopes.is_empty() {
            form.push(("scope".into(), self.scopes.join(" ")));
        }
        on_own_thread(move || {
            let body = post_form(&url, form).map_err(|e| format!("oauth2 grant: {e}"))?;
            // A provider that rotates refresh tokens answers with a new one and
            // retires ours. Cloacina does not store the new one yet, so the next
            // grant would fail: say so now.
            if let (Some(ours), Some(new)) =
                (ours, body.get("refresh_token").and_then(|t| t.as_str()))
                && new != ours
            {
                return Err(format!(
                    "oauth2 grant: {url} rotated the refresh token; cloacina does not store a rotated refresh token yet, so this credential cannot be refreshed again"
                ));
            }
            access_token(&url, &body).map_err(|e| format!("oauth2 grant: {e}"))
        })
    }
}

/// Logs in on the host and adds the returned session token as a header.
pub struct SessionProvider {
    login_url: String,
    method: String,
    /// The JSON login body, built on the host from the secret's fields.
    body: Option<String>,
    token_path: Vec<String>,
    inject_header: String,
    prefix: String,
    ttl: std::time::Duration,
    ids: TokenIds,
}

impl SessionProvider {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        login_url: String,
        method: String,
        body: Option<String>,
        token_path: Vec<String>,
        inject_header: String,
        prefix: String,
        ttl: std::time::Duration,
    ) -> Self {
        let path = token_path.join(".");
        let ids = TokenIds::new(
            &["session", &login_url, &method, &path, &inject_header],
            &[body.as_deref().unwrap_or("")],
        );
        Self {
            login_url,
            method,
            body,
            token_path,
            inject_header,
            prefix,
            ttl,
            ids,
        }
    }

    fn token(&self) -> Result<String, EgressDenied> {
        cached_token(&self.ids, || {
            let (url, method, body, path) = (
                self.login_url.clone(),
                self.method.clone(),
                self.body.clone(),
                self.token_path.clone(),
            );
            let ttl = self.ttl;
            on_own_thread(move || {
                let method = reqwest::Method::from_bytes(method.as_bytes())
                    .map_err(|_| format!("session login: `{method}` is not an HTTP method"))?;
                let mut req = reqwest::blocking::Client::new().request(method, &url);
                if let Some(body) = body {
                    req = req.header("content-type", "application/json").body(body);
                }
                let resp = req
                    .send()
                    .map_err(|e| format!("session login: {url}: {e}"))?;
                let answer = json_answer(&url, resp).map_err(|e| format!("session login: {e}"))?;
                let mut at = &answer;
                for seg in &path {
                    at = at.get(seg).ok_or_else(|| {
                        format!(
                            "session login: {url}: the answer has no `{}`",
                            path.join(".")
                        )
                    })?;
                }
                let token = at
                    .as_str()
                    .ok_or_else(|| format!("session login: `{}` is not a string", path.join(".")))?
                    .to_string();
                Ok((token, ttl))
            })
        })
    }
}

/// Mints and caches a Google service account's access token on the host.
pub struct GoogleServiceAccountProvider {
    client_email: String,
    private_key_pem: String,
    token_uri: String,
    scopes: Vec<String>,
    ids: TokenIds,
}

impl GoogleServiceAccountProvider {
    /// From the account's JSON key: `client_email`, `private_key` and
    /// `token_uri` (default Google's).
    pub fn from_key_json(key_json: &str, scopes: Vec<String>) -> Result<Self, String> {
        let key: serde_json::Value = serde_json::from_str(key_json)
            .map_err(|e| format!("the service account key is not JSON: {e}"))?;
        let field = |k: &str| {
            key.get(k)
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .filter(|v| !v.is_empty())
        };
        let client_email =
            field("client_email").ok_or("the service account key has no `client_email`")?;
        let private_key_pem =
            field("private_key").ok_or("the service account key has no `private_key`")?;
        let token_uri =
            field("token_uri").unwrap_or_else(|| "https://oauth2.googleapis.com/token".to_string());
        let scope = scopes.join(" ");
        let ids = TokenIds::new(
            &["google", &client_email, &scope, &token_uri],
            &[&private_key_pem],
        );
        Ok(Self {
            client_email,
            private_key_pem,
            token_uri,
            scopes,
            ids,
        })
    }

    fn bearer(&self) -> Result<String, EgressDenied> {
        cached_token(&self.ids, || {
            let assertion = google_assertion(
                &self.client_email,
                &self.scopes.join(" "),
                &self.token_uri,
                unix_now_secs(),
                &self.private_key_pem,
            )
            .map_err(EgressDenied::new)?;
            let url = self.token_uri.clone();
            on_own_thread(move || {
                let form = vec![
                    (
                        "grant_type".to_string(),
                        "urn:ietf:params:oauth:grant-type:jwt-bearer".to_string(),
                    ),
                    ("assertion".to_string(), assertion),
                ];
                let body = post_form(&url, form)
                    .map_err(|e| format!("google service account grant: {e}"))?;
                access_token(&url, &body).map_err(|e| format!("google service account grant: {e}"))
            })
        })
    }
}

/// The signed RS256 JWT-bearer assertion of Google's server-to-server flow.
/// Pure: tests fix `iat`.
fn google_assertion(
    client_email: &str,
    scope: &str,
    aud: &str,
    iat: u64,
    private_key_pem: &str,
) -> Result<String, String> {
    let key = parse_pkcs8_keypair(private_key_pem)?;
    rs256_sign_jwt(
        &key,
        &serde_json::json!({
            "iss": client_email,
            "scope": scope,
            "aud": aud,
            "iat": iat,
            "exp": iat + 3600,
        })
        .to_string(),
    )
}

/// Build Snowflake's self-signed key-pair JWT. Pure — tests fix `iat`.
fn snowflake_jwt(
    account: &str,
    user: &str,
    iat: u64,
    private_key_pem: &str,
) -> Result<String, String> {
    let key = parse_pkcs8_keypair(private_key_pem)?;
    let fingerprint = snowflake_fingerprint(&key);
    rs256_sign_jwt(
        &key,
        &serde_json::json!({
            "iss": format!("{account}.{user}.{fingerprint}"),
            "sub": format!("{account}.{user}"),
            "iat": iat,
            "exp": iat + 3600,
        })
        .to_string(),
    )
}

/// Snowflake's public-key fingerprint: `SHA256:` + base64 of the SHA-256 over the
/// **SubjectPublicKeyInfo** DER — the same encoding `ALTER USER … SET RSA_PUBLIC_KEY`
/// stores (what `openssl rsa -pubout -outform DER` emits). ring exposes the public key
/// as PKCS#1, so wrap it into SPKI first.
fn snowflake_fingerprint(key: &ring::signature::RsaKeyPair) -> String {
    use base64::Engine;
    use sha2::Digest;
    let spki = spki_from_pkcs1(ring::signature::KeyPair::public_key(key).as_ref());
    let digest = sha2::Sha256::digest(&spki);
    format!(
        "SHA256:{}",
        base64::engine::general_purpose::STANDARD.encode(digest)
    )
}

/// Wrap a PKCS#1 `RSAPublicKey` DER into an X.509 `SubjectPublicKeyInfo` DER:
/// `SEQUENCE { AlgorithmIdentifier(rsaEncryption, NULL), BIT STRING { 0, pkcs1 } }`.
fn spki_from_pkcs1(pkcs1: &[u8]) -> Vec<u8> {
    // DER definite-length encoding for `len` bytes of content.
    fn der_len(len: usize) -> Vec<u8> {
        match len {
            0..=0x7f => vec![len as u8],
            0x80..=0xff => vec![0x81, len as u8],
            _ => vec![0x82, (len >> 8) as u8, (len & 0xff) as u8],
        }
    }
    // AlgorithmIdentifier for rsaEncryption (OID 1.2.840.113549.1.1.1) + NULL params.
    const ALG_ID: [u8; 15] = [
        0x30, 0x0d, 0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01, 0x05, 0x00,
    ];
    let mut bit_string = vec![0x03];
    bit_string.extend(der_len(pkcs1.len() + 1)); // +1 for the leading unused-bits byte
    bit_string.push(0x00);
    bit_string.extend_from_slice(pkcs1);
    let content_len = ALG_ID.len() + bit_string.len();
    let mut spki = vec![0x30];
    spki.extend(der_len(content_len));
    spki.extend_from_slice(&ALG_ID);
    spki.extend(bit_string);
    spki
}

/// Decode a PKCS#8 PEM (`-----BEGIN PRIVATE KEY-----`) body to DER. SA JSON keys embed
/// the PEM with `\n` escapes — serde unescapes them before this sees the string.
fn pkcs8_pem_to_der(pem: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    let body: String = pem
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .collect::<Vec<_>>()
        .join("");
    base64::engine::general_purpose::STANDARD
        .decode(body.trim())
        .map_err(|e| format!("private key is not valid PEM base64: {e}"))
}

fn parse_pkcs8_keypair(private_key_pem: &str) -> Result<ring::signature::RsaKeyPair, String> {
    let der = pkcs8_pem_to_der(private_key_pem)?;
    ring::signature::RsaKeyPair::from_pkcs8(&der).map_err(|e| format!("private key rejected: {e}"))
}

/// Assemble + RS256-sign a JWT (`header.claims.signature`, base64url-no-pad) over the
/// given claims JSON.
fn rs256_sign_jwt(key: &ring::signature::RsaKeyPair, claims_json: &str) -> Result<String, String> {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header = b64.encode(r#"{"alg":"RS256","typ":"JWT"}"#);
    let claims = b64.encode(claims_json);
    let signing_input = format!("{header}.{claims}");
    let mut sig = vec![0u8; key.public().modulus_len()];
    key.sign(
        &ring::signature::RSA_PKCS1_SHA256,
        &ring::rand::SystemRandom::new(),
        signing_input.as_bytes(),
        &mut sig,
    )
    .map_err(|e| format!("RS256 signing failed: {e}"))?;
    Ok(format!("{signing_input}.{}", b64.encode(sig)))
}

/// Current Unix time in seconds.
fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// AWS SigV4 signing.
pub mod sigv4 {
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};

    type HmacSha256 = Hmac<Sha256>;

    /// SHA-256 of an empty body — the payload hash every S3 **GET** carries.
    pub const EMPTY_PAYLOAD_SHA256: &str =
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    pub fn sha256_hex(bytes: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(bytes);
        hex::encode(h.finalize())
    }

    fn hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
        let mut m = <HmacSha256 as hmac::KeyInit>::new_from_slice(key)
            .expect("HMAC accepts any key length");
        m.update(data);
        m.finalize().into_bytes().to_vec()
    }

    /// The signer for one (credential, region, service).
    pub struct SigV4<'a> {
        pub access_key: &'a str,
        pub secret_key: &'a str,
        pub region: &'a str,
        pub service: &'a str,
    }

    impl SigV4<'_> {
        /// The `aws4_request` signing key for a date (the HMAC chain over the secret).
        fn signing_key(&self, datestamp: &str) -> Vec<u8> {
            let k_date = hmac(
                format!("AWS4{}", self.secret_key).as_bytes(),
                datestamp.as_bytes(),
            );
            let k_region = hmac(&k_date, self.region.as_bytes());
            let k_service = hmac(&k_region, self.service.as_bytes());
            hmac(&k_service, b"aws4_request")
        }

        /// Build the `Authorization` header value for a request.
        ///
        /// `signed_headers` must be **lowercased names + trimmed values, sorted by name**;
        /// `canonical_query` must be sorted + percent-encoded; `amz_date` is `YYYYMMDDTHHMMSSZ`
        /// and `datestamp` is `YYYYMMDD` (both derived from one instant).
        #[allow(clippy::too_many_arguments)]
        pub fn authorization(
            &self,
            method: &str,
            canonical_uri: &str,
            canonical_query: &str,
            signed_headers: &[(String, String)],
            payload_hash: &str,
            amz_date: &str,
            datestamp: &str,
        ) -> String {
            let canonical_headers: String = signed_headers
                .iter()
                .map(|(k, v)| format!("{k}:{v}\n"))
                .collect();
            let signed_names = signed_headers
                .iter()
                .map(|(k, _)| k.as_str())
                .collect::<Vec<_>>()
                .join(";");
            let canonical_request = format!(
                "{method}\n{canonical_uri}\n{canonical_query}\n{canonical_headers}\n{signed_names}\n{payload_hash}"
            );

            let scope = format!("{datestamp}/{}/{}/aws4_request", self.region, self.service);
            let string_to_sign = format!(
                "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
                sha256_hex(canonical_request.as_bytes())
            );
            let signature = hex::encode(hmac(
                &self.signing_key(datestamp),
                string_to_sign.as_bytes(),
            ));
            format!(
                "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_names}, Signature={signature}",
                self.access_key
            )
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn empty_payload_hash_matches_the_known_constant() {
            assert_eq!(sha256_hex(b""), EMPTY_PAYLOAD_SHA256);
        }

        // AWS's published worked example ("Examples of a signed AWS API request"): GET
        // https://iam.amazonaws.com/?Action=ListUsers&Version=2010-05-08 at 20150830T123600Z.
        // Expected signature is fixed by AWS — proves the whole canonical-request → STS → HMAC chain.
        #[test]
        fn matches_aws_published_test_vector() {
            let signer = SigV4 {
                access_key: "AKIDEXAMPLE",
                secret_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
                region: "us-east-1",
                service: "iam",
            };
            let signed = vec![
                (
                    "content-type".to_string(),
                    "application/x-www-form-urlencoded; charset=utf-8".to_string(),
                ),
                ("host".to_string(), "iam.amazonaws.com".to_string()),
                ("x-amz-date".to_string(), "20150830T123600Z".to_string()),
            ];
            let auth = signer.authorization(
                "GET",
                "/",
                "Action=ListUsers&Version=2010-05-08",
                &signed,
                EMPTY_PAYLOAD_SHA256,
                "20150830T123600Z",
                "20150830",
            );
            assert!(
                auth.ends_with(
                    "Signature=5d672d79c15b13162d9279b0855cfba6789a8edb4c82c400e06b5924a6f2b5d7"
                ),
                "SigV4 authorization did not match the AWS test vector: {auth}"
            );
            assert!(auth.contains("Credential=AKIDEXAMPLE/20150830/us-east-1/iam/aws4_request"));
            assert!(auth.contains("SignedHeaders=content-type;host;x-amz-date"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A throwaway RSA key used only in tests (from weir's test fixtures).
    const TEST_KEY: &str = include_str!("../../../tests/fixtures/test_rsa_key.pem");

    fn parts(uri: &str) -> HttpParts {
        let req = fidius_host::http_types::Request::builder()
            .uri(uri)
            .body(())
            .unwrap();
        req.into_parts().0
    }

    /// An allow-all inner policy, to test the credential on its own.
    struct AllowAll;
    impl EgressPolicy for AllowAll {
        fn authorize(&self, _parts: &mut HttpParts) -> Result<(), EgressDenied> {
            Ok(())
        }
        fn authorize_tcp(&self, _addr: &SocketAddr) -> Result<(), EgressDenied> {
            Ok(())
        }
    }

    /// A deny-all inner policy: the credential must never be applied.
    struct DenyAll;
    impl EgressPolicy for DenyAll {
        fn authorize(&self, _parts: &mut HttpParts) -> Result<(), EgressDenied> {
            Err(EgressDenied::new("denied"))
        }
        fn authorize_tcp(&self, _addr: &SocketAddr) -> Result<(), EgressDenied> {
            Err(EgressDenied::new("denied"))
        }
    }

    #[test]
    fn header_is_injected_after_the_allow_list() {
        let egress = CredentialEgress::new(
            Some(Arc::new(AllowAll)),
            Credential::Header {
                name: "Authorization".into(),
                value: "Bearer s3cr3t".into(),
            },
        );
        let mut p = parts("https://api.example.com/v1/items");
        egress.authorize(&mut p).unwrap();
        assert_eq!(p.headers["authorization"], "Bearer s3cr3t");
    }

    #[test]
    fn a_denied_request_never_carries_the_credential() {
        let egress = CredentialEgress::new(
            Some(Arc::new(DenyAll)),
            Credential::Header {
                name: "Authorization".into(),
                value: "Bearer s3cr3t".into(),
            },
        );
        let mut p = parts("https://evil.example.com/");
        assert!(egress.authorize(&mut p).is_err());
        assert!(p.headers.get("authorization").is_none());

        let no_grant = CredentialEgress::new(
            None,
            Credential::Header {
                name: "X-Key".into(),
                value: "k".into(),
            },
        );
        let mut p = parts("https://api.example.com/");
        assert!(no_grant.authorize(&mut p).is_err());
        assert!(p.headers.get("x-key").is_none());
    }

    #[test]
    fn append_query_percent_encodes_the_credential_value() {
        let mut p = parts("https://api.example.com/v1/items?page=2");
        append_query(&mut p, "apikey", "k+&=#z").expect("append");
        assert_eq!(
            p.uri.query().expect("query"),
            "page=2&apikey=k%2B%26%3D%23z"
        );
    }

    #[test]
    fn canonical_query_encodes_exactly_once() {
        assert_eq!(
            canonical_query("list-type=2&continuation-token=abc%3D%3D&max-keys=1"),
            "continuation-token=abc%3D%3D&list-type=2&max-keys=1",
            "sorted, and %3D stays single-encoded"
        );
        assert_eq!(canonical_query("a=x/y"), "a=x%2Fy");
    }

    #[test]
    fn epoch_to_amz_formats_the_aws_example_date() {
        // 2015-08-30T12:36:00Z, the date of AWS's published SigV4 example.
        assert_eq!(
            epoch_to_amz(1_440_938_160),
            ("20150830T123600Z".to_string(), "20150830".to_string())
        );
    }

    #[test]
    fn sigv4_signs_with_the_port_and_the_amz_headers() {
        let egress = CredentialEgress::new(
            Some(Arc::new(AllowAll)),
            Credential::AwsSigV4 {
                access_key: "AKIDEXAMPLE".into(),
                secret_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY".into(),
                region: "us-east-1".into(),
                service: "s3".into(),
            },
        );
        let mut p = parts("http://127.0.0.1:9000/bucket/key.ndjson");
        egress.authorize(&mut p).unwrap();
        let auth = p.headers["authorization"].to_str().unwrap();
        assert!(
            auth.starts_with("AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/"),
            "{auth}"
        );
        assert!(auth.contains("/us-east-1/s3/aws4_request"), "{auth}");
        assert!(
            auth.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date"),
            "{auth}"
        );
        assert!(
            !auth.contains("wJalrXUtnFEMI"),
            "the secret key is never on the wire"
        );
        assert_eq!(
            p.headers["x-amz-content-sha256"],
            sigv4::EMPTY_PAYLOAD_SHA256
        );
        assert!(p.headers.get("x-amz-date").is_some());
    }

    #[test]
    fn snowflake_jwt_is_a_bearer_with_the_keypair_header_and_uppercase_claims() {
        let egress = CredentialEgress::new(
            Some(Arc::new(AllowAll)),
            Credential::SnowflakeKeypairJwt(SnowflakeKeypairProvider::new(
                "my_acct".into(),
                "loader".into(),
                TEST_KEY.into(),
            )),
        );
        let mut p = parts("https://my_acct.snowflakecomputing.com/api/v2/statements");
        egress.authorize(&mut p).unwrap();
        assert_eq!(
            p.headers["x-snowflake-authorization-token-type"],
            "KEYPAIR_JWT"
        );
        let bearer = p.headers["authorization"].to_str().unwrap();
        let jwt = bearer.strip_prefix("Bearer ").expect("bearer");
        let mut segs = jwt.split('.');
        let (_h, claims, _s) = (
            segs.next().unwrap(),
            segs.next().unwrap(),
            segs.next().unwrap(),
        );
        use base64::Engine;
        let claims: serde_json::Value = serde_json::from_slice(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(claims)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(claims["sub"], "MY_ACCT.LOADER");
        let iss = claims["iss"].as_str().unwrap();
        assert!(iss.starts_with("MY_ACCT.LOADER.SHA256:"), "{iss}");
        assert_eq!(
            claims["exp"].as_u64().unwrap() - claims["iat"].as_u64().unwrap(),
            3600
        );
    }

    #[test]
    fn snowflake_token_is_cached_between_requests() {
        let provider = SnowflakeKeypairProvider::new("a".into(), "u".into(), TEST_KEY.into());
        let first = provider.bearer().unwrap();
        assert_eq!(provider.bearer().unwrap(), first);
    }

    #[test]
    fn debug_never_prints_a_secret() {
        let c = Credential::Header {
            name: "Authorization".into(),
            value: "Bearer s3cr3t".into(),
        };
        assert!(!format!("{c:?}").contains("s3cr3t"));
        let c = Credential::AwsSigV4 {
            access_key: "AK".into(),
            secret_key: "SECRETKEY".into(),
            region: "r".into(),
            service: "s3".into(),
        };
        assert!(!format!("{c:?}").contains("SECRETKEY"));
    }
}

/// The minted-token credentials against a local token server (CLOACINA-T-0989).
#[cfg(test)]
mod minted_tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    const TEST_KEY: &str = include_str!("../../../tests/fixtures/test_rsa_key.pem");

    type Handler = dyn Fn(&str, &str) -> (u16, String) + Send + Sync;

    /// A local server: `handler(path, body)` answers each request; each
    /// request's path and body are kept.
    struct TokenServer {
        port: u16,
        seen: Arc<Mutex<Vec<(String, String)>>>,
    }

    impl TokenServer {
        fn start(handler: impl Fn(&str, &str) -> (u16, String) + Send + Sync + 'static) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let seen = Arc::new(Mutex::new(Vec::new()));
            let (log, handler): (_, Arc<Handler>) = (seen.clone(), Arc::new(handler));
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(mut stream) = stream else { continue };
                    let mut data = Vec::new();
                    let mut buf = [0u8; 4096];
                    let end = loop {
                        let n = stream.read(&mut buf).unwrap_or(0);
                        if n == 0 {
                            break None;
                        }
                        data.extend_from_slice(&buf[..n]);
                        if let Some(e) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                            break Some(e);
                        }
                    };
                    let Some(end) = end else { continue };
                    let head = String::from_utf8_lossy(&data[..end]).to_string();
                    let len = head
                        .lines()
                        .find_map(|l| {
                            let (k, v) = l.split_once(':')?;
                            k.eq_ignore_ascii_case("content-length")
                                .then(|| v.trim().parse::<usize>().ok())?
                        })
                        .unwrap_or(0);
                    while data.len() < end + 4 + len {
                        let n = stream.read(&mut buf).unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        data.extend_from_slice(&buf[..n]);
                    }
                    let body = String::from_utf8_lossy(&data[end + 4..]).to_string();
                    let path = head.split_whitespace().nth(1).unwrap_or("").to_string();
                    let (status, answer) = handler(&path, &body);
                    log.lock().unwrap().push((path, body));
                    let _ = stream.write_all(
                        format!(
                            "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{answer}",
                            answer.len()
                        )
                        .as_bytes(),
                    );
                }
            });
            Self { port, seen }
        }

        fn url(&self, path: &str) -> String {
            format!("http://127.0.0.1:{}{path}", self.port)
        }

        fn requests(&self) -> Vec<(String, String)> {
            self.seen.lock().unwrap().clone()
        }
    }

    fn parts() -> HttpParts {
        fidius_host::http_types::Request::builder()
            .uri("https://api.example.com/v1/items")
            .body(())
            .unwrap()
            .into_parts()
            .0
    }

    fn header(c: &Credential, name: &str) -> Result<String, EgressDenied> {
        let mut p = parts();
        c.apply(&mut p)?;
        Ok(p.headers[name].to_str().unwrap().to_string())
    }

    /// Counts grants and answers with token `tok-<n>`.
    fn counting(expires_in: u64) -> TokenServer {
        let n = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        TokenServer::start(move |_, _| {
            let i = n.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            (
                200,
                format!(r#"{{"access_token":"tok-{i}","expires_in":{expires_in}}}"#),
            )
        })
    }

    fn client_credentials(server: &TokenServer, secret: &str) -> Credential {
        Credential::OAuth2(OAuth2Provider::new(
            server.url("/token"),
            OAuthGrant::ClientCredentials,
            "client-1".into(),
            secret.into(),
            vec!["read".into(), "write".into()],
        ))
    }

    #[test]
    fn a_client_credentials_token_is_minted_once_and_shared() {
        let server = counting(3600);
        let first = client_credentials(&server, "s3cr3t");
        assert_eq!(header(&first, "authorization").unwrap(), "Bearer tok-1");
        // Another load of the same credential reuses the token.
        let second = client_credentials(&server, "s3cr3t");
        assert_eq!(header(&second, "authorization").unwrap(), "Bearer tok-1");
        let reqs = server.requests();
        assert_eq!(reqs.len(), 1, "{reqs:?}");
        let form = &reqs[0].1;
        assert!(form.contains("grant_type=client_credentials"), "{form}");
        assert!(form.contains("client_id=client-1"), "{form}");
        assert!(form.contains("scope=read+write"), "{form}");
        // A changed secret is another credential: it mints afresh.
        let rotated = client_credentials(&server, "n3w-s3cr3t");
        assert_eq!(header(&rotated, "authorization").unwrap(), "Bearer tok-2");
    }

    #[test]
    fn a_token_near_its_expiry_is_minted_again() {
        // 30 seconds is inside the one-minute margin, so every use mints.
        let server = counting(30);
        let c = client_credentials(&server, "s");
        assert_eq!(header(&c, "authorization").unwrap(), "Bearer tok-1");
        assert_eq!(header(&c, "authorization").unwrap(), "Bearer tok-2");
    }

    #[test]
    fn a_refresh_token_grant_and_a_rotated_refresh_token() {
        let server = TokenServer::start(|_, body| {
            if body.contains("refresh_token=rotating") {
                (
                    200,
                    r#"{"access_token":"a","refresh_token":"brand-new"}"#.to_string(),
                )
            } else {
                (
                    200,
                    r#"{"access_token":"refreshed","refresh_token":"keeps"}"#.to_string(),
                )
            }
        });
        let grant = |rt: &str| {
            Credential::OAuth2(OAuth2Provider::new(
                server.url("/oauth/token"),
                OAuthGrant::RefreshToken {
                    refresh_token: rt.into(),
                },
                "c".into(),
                "s".into(),
                vec![],
            ))
        };
        assert_eq!(
            header(&grant("keeps"), "authorization").unwrap(),
            "Bearer refreshed"
        );
        let form = &server.requests()[0].1;
        assert!(form.contains("grant_type=refresh_token"), "{form}");
        let err = header(&grant("rotating"), "authorization")
            .unwrap_err()
            .reason;
        assert!(err.contains("rotated the refresh token"), "{err}");
        assert!(
            !err.contains("brand-new") && !err.contains("rotating"),
            "{err}"
        );
    }

    #[test]
    fn a_refused_grant_names_the_reason_not_the_secret() {
        let server = TokenServer::start(|_, _| {
            (
                401,
                r#"{"error":"invalid_client","error_description":"bad client"}"#.to_string(),
            )
        });
        let err = header(&client_credentials(&server, "top-s3cr3t"), "authorization")
            .unwrap_err()
            .reason;
        assert!(
            err.contains("HTTP 401") && err.contains("bad client"),
            "{err}"
        );
        assert!(!err.contains("top-s3cr3t"), "{err}");
    }

    #[test]
    fn a_session_login_token_is_sent_as_a_header() {
        let server = TokenServer::start(|path, body| {
            assert_eq!(path, "/login");
            assert!(body.contains("\"username\":\"robin\""), "{body}");
            (200, r#"{"data":{"token":"sess-1"}}"#.to_string())
        });
        let c = Credential::Session(SessionProvider::new(
            server.url("/login"),
            "POST".into(),
            Some(r#"{"username":"robin","password":"pw"}"#.into()),
            vec!["data".into(), "token".into()],
            "X-Session".into(),
            "Token ".into(),
            std::time::Duration::from_secs(900),
        ));
        assert_eq!(header(&c, "x-session").unwrap(), "Token sess-1");
        assert_eq!(header(&c, "x-session").unwrap(), "Token sess-1");
        assert_eq!(server.requests().len(), 1, "the session is reused");
    }

    #[test]
    fn a_google_service_account_runs_the_jwt_bearer_grant() {
        let server = TokenServer::start(|_, body| {
            assert!(
                body.contains("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer"),
                "{body}"
            );
            let assertion = body.split("assertion=").nth(1).unwrap_or("");
            assert_eq!(assertion.split('.').count(), 3, "{assertion}");
            (
                200,
                r#"{"access_token":"ya29.test","expires_in":3600}"#.to_string(),
            )
        });
        let key = serde_json::json!({
            "client_email": "sync@project.iam.gserviceaccount.com",
            "private_key": TEST_KEY,
            "token_uri": server.url("/token"),
        })
        .to_string();
        let c = Credential::GoogleServiceAccount(
            GoogleServiceAccountProvider::from_key_json(
                &key,
                vec!["https://www.googleapis.com/auth/analytics.readonly".into()],
            )
            .unwrap(),
        );
        assert_eq!(header(&c, "authorization").unwrap(), "Bearer ya29.test");
        assert!(format!("{c:?}").contains("sync@project"));
        assert!(!format!("{c:?}").contains("PRIVATE KEY"));
        assert!(GoogleServiceAccountProvider::from_key_json("{}", vec![]).is_err());
    }

    #[test]
    fn the_google_assertion_claims() {
        use base64::Engine;
        let jwt = google_assertion(
            "a@b.iam",
            "s1 s2",
            "https://t/token",
            1_700_000_000,
            TEST_KEY,
        )
        .unwrap();
        let claims = jwt.split('.').nth(1).unwrap();
        let claims: serde_json::Value = serde_json::from_slice(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(claims)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(claims["iss"], "a@b.iam");
        assert_eq!(claims["scope"], "s1 s2");
        assert_eq!(claims["aud"], "https://t/token");
        assert_eq!(claims["exp"], 1_700_003_600u64);
    }

    #[test]
    fn basic_auth_encodes_the_pair() {
        let c = Credential::Basic {
            username: "robin".into(),
            password: "pw".into(),
        };
        assert_eq!(header(&c, "authorization").unwrap(), "Basic cm9iaW46cHc=");
        assert_eq!(format!("{c:?}"), "Basic(robin)");
    }
}
