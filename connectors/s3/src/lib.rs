//! The S3 connector (COLLIERY-I-0633 / CLOACINA-T-0985), moved from weir: lists
//! and reads NDJSON objects from an S3-compatible endpoint over HTTP. Requests
//! leave the guest UNSIGNED: the host's egress signs them with SigV4, so the
//! secret never enters the sandbox. A **source** only: FullRefresh, and
//! Incremental by object key. Config: `endpoint`, `bucket`, `prefix`,
//! `max_keys`. Capability: `http`.
#![allow(clippy::all)]

use fidius_macro::{WitType, plugin_impl, plugin_interface};
use sync_guest_types::*;

mod sync_guest_types;

#[plugin_interface(version = 1, buffer = PluginAllocated, crate = "fidius_guest")]
pub trait Connector: Send + Sync {
    fn spec(&self) -> ConnectorSpec;
    fn check(&self, config: Config) -> CheckResult;
    #[optional(since = 1)]
    fn discover(&self, config: Config) -> DiscoverOutcome;
    #[optional(since = 1)]
    fn read(&self, ctx: ReadContext) -> fidius_guest::Stream<ReadMessage>;
    #[optional(since = 1)]
    fn write(&self, ctx: WriteContext, batches: fidius_guest::Stream<RecordBatch>) -> WriteOutcome;
}

pub struct S3 {
    cfg: cloacina_constructor_contract::sync::Config,
}

impl S3 {
    fn configure(cfg: cloacina_constructor_contract::sync::Config) -> Self {
        Self { cfg }
    }
}

#[plugin_impl(Connector, crate = "fidius_guest", config = cloacina_constructor_contract::sync::Config)]
impl Connector for S3 {
    fn spec(&self) -> ConnectorSpec {
        ConnectorSpec {
            name: "s3".to_string(),
            connector_version: "0.1.0".to_string(),
            contract_version: 1,
            config_schema: "{\"type\":\"object\",\"required\":[\"endpoint\",\"bucket\"],\"properties\":{\
                \"endpoint\":{\"type\":\"string\",\"title\":\"S3 endpoint (e.g. https://s3.amazonaws.com or http://minio:9000)\"},\
                \"bucket\":{\"type\":\"string\",\"title\":\"Bucket\"},\
                \"prefix\":{\"type\":\"string\",\"title\":\"Key prefix (blank = whole bucket)\"},\
                \"max_keys\":{\"type\":\"integer\",\"title\":\"Listing page size (default: S3's 1000; listing follows continuation tokens either way)\"}}}".to_string(),
            // Auth is a SigV4 credential the host applies; the guest declares none.
            roles: vec![ConnectorRole::Source],
            supported_sync_modes: vec![SyncMode::FullRefresh, SyncMode::Incremental],
        }
    }

    /// List one key: a wrong endpoint, bucket or key fails.
    fn check(&self, _config: Config) -> CheckResult {
        let result = parse_cfg(&self.cfg).and_then(|cfg| {
            http_get(&format!("{}/{}?list-type=2&max-keys=1", cfg.endpoint, cfg.bucket)).map(|_| ())
        });
        match result {
            Ok(()) => CheckResult { success: true, message: None },
            Err(e) => CheckResult { success: false, message: Some(e.message) },
        }
    }

    fn discover(&self, _config: Config) -> DiscoverOutcome {
        // One "objects" stream; the incremental cursor is the object key.
        DiscoverOutcome::Catalog(Catalog {
            streams: vec![StreamInfo {
                name: "objects".to_string(),
                namespace: None,
                schema: ArrowSchemaIpc { ipc: Vec::new() },
                supported_sync_modes: vec![SyncMode::FullRefresh, SyncMode::Incremental],
                source_defined_cursor: true,
                default_cursor_field: Some("_key".to_string()),
                source_defined_primary_key: None,
                partitioning: PartitionScheme::Unpartitioned,
            }],
        })
    }

    fn read(&self, ctx: ReadContext) -> fidius_guest::Stream<ReadMessage> {
        let msgs: Vec<ReadMessage> = match read_objects(&self.cfg, &ctx) {
            Ok((rows, cursor)) => vec![
                ReadMessage::Records(RecordBatch::Rows(rows)),
                ReadMessage::Checkpoint(StreamState { cursor, opaque: Vec::new() }),
            ],
            Err(e) => vec![ReadMessage::Fatal(e)],
        };
        fidius_guest::Stream::from_iter(msgs)
    }

    fn write(&self, _ctx: WriteContext, mut batches: fidius_guest::Stream<RecordBatch>) -> WriteOutcome {
        while batches.next_item().is_some() {}
        WriteOutcome {
            state: StreamState { cursor: None, opaque: Vec::new() },
            diagnostics: Vec::new(),
            dead_letters: Vec::new(),
            result: WriteResult::Err(ConnectorError::fatal("s3 is a source, not a destination")),
        }
    }
}

struct S3Cfg {
    endpoint: String,
    bucket: String,
    prefix: String,
    /// The listing page size (`max-keys`; None means S3's 1000). Mostly a test
    /// lever to force multi-page listings cheaply.
    max_keys: Option<u64>,
}

fn parse_cfg(config: &cloacina_constructor_contract::sync::Config) -> Result<S3Cfg, ConnectorError> {
    let v: serde_json::Value = serde_json::from_str(&config.json).unwrap_or(serde_json::Value::Null);
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let cfg = S3Cfg {
        endpoint: s("endpoint").trim_end_matches('/').to_string(),
        bucket: s("bucket"),
        prefix: s("prefix"),
        max_keys: v.get("max_keys").and_then(|x| x.as_u64()),
    };
    if cfg.endpoint.is_empty() || cfg.bucket.is_empty() {
        return Err(ConnectorError {
            kind: ErrorKind::Config,
            message: "s3: `endpoint` and `bucket` are required".to_string(),
            retryable: false,
            context: Vec::new(),
        });
    }
    Ok(cfg)
}

/// List the objects (ListObjectsV2), then read each new one as NDJSON.
/// Incremental uses the object key as the cursor (`start-after`). Returns the
/// rows and the new cursor: the last key read, as a JSON string.
fn read_objects(
    config: &cloacina_constructor_contract::sync::Config,
    ctx: &ReadContext,
) -> Result<(Vec<String>, Option<String>), ConnectorError> {
    let cfg = parse_cfg(config)?;
    let incremental = matches!(ctx.stream.sync_mode, SyncMode::Incremental);
    // The cursor is JSON text: a string key.
    let after: Option<String> = ctx.state.cursor.as_deref().map(|c| {
        serde_json::from_str::<String>(c).unwrap_or_else(|_| c.to_string())
    });

    let mut list_url = format!("{}/{}?list-type=2", cfg.endpoint, cfg.bucket);
    if !cfg.prefix.is_empty() {
        list_url.push_str(&format!("&prefix={}", uri_escape(&cfg.prefix)));
    }
    if let Some(n) = cfg.max_keys {
        list_url.push_str(&format!("&max-keys={n}"));
    }
    if incremental {
        if let Some(a) = &after {
            list_url.push_str(&format!("&start-after={}", uri_escape(a)));
        }
    }

    // Follow `IsTruncated` and `NextContinuationToken` to the end of the
    // listing: a single page stops at 1000 objects and reports success.
    let mut keys = Vec::new();
    let mut token: Option<String> = None;
    loop {
        let url = match &token {
            Some(t) => format!("{list_url}&continuation-token={}", uri_escape(t)),
            None => list_url.clone(),
        };
        let listing = http_get(&url)?;
        keys.extend(parse_list_keys(&listing));
        let truncated = xml_tag(&listing, "IsTruncated").as_deref() == Some("true");
        token = xml_tag(&listing, "NextContinuationToken");
        if !truncated || token.is_none() {
            break;
        }
    }
    keys.sort();

    let mut rows = Vec::new();
    let mut last = after.clone();
    for key in keys {
        if key.ends_with('/') {
            continue; // a "directory" placeholder, not an object
        }
        if incremental {
            if let Some(a) = &after {
                if &key <= a {
                    continue;
                }
            }
        }
        let obj_url = format!("{}/{}/{}", cfg.endpoint, cfg.bucket, uri_escape_path(&key));
        let body = http_get(&obj_url)?;
        for line in body.lines() {
            let t = line.trim();
            if !t.is_empty() {
                rows.push(t.to_string());
            }
        }
        last = Some(key);
    }
    let cursor = last.map(|k| serde_json::Value::String(k).to_string());
    Ok((rows, cursor))
}

/// The `<Key>` values of a ListObjectsV2 response. S3's listing schema is
/// stable, so a minimal scan does.
fn parse_list_keys(xml: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<Key>") {
        let tail = &rest[start + 5..];
        match tail.find("</Key>") {
            Some(end) => {
                keys.push(xml_unescape(&tail[..end]));
                rest = &tail[end + 6..];
            }
            None => break,
        }
    }
    keys
}

/// The unescaped text of the first `<tag>...</tag>` in `xml`.
fn xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml_unescape(&xml[start..end]))
}

fn xml_unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Percent-encode a query value; unreserved characters pass through.
fn uri_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Percent-encode an object key for the request path, keeping `/`.
fn uri_escape_path(s: &str) -> String {
    s.split('/').map(uri_escape).collect::<Vec<_>>().join("/")
}

/// A plain GET; the host adds the SigV4 signature. Retries 429, 5xx and
/// transport failures with backoff; any other non-2xx status fails.
fn http_get(url: &str) -> Result<String, ConnectorError> {
    let mut attempt: u32 = 0;
    loop {
        match fidius_guest::http::send(fidius_guest::http::Request::get(url)) {
            Ok(r) if (200..300).contains(&r.status) => return Ok(r.text()),
            Ok(r) if (r.status == 429 || (500..=599).contains(&r.status)) && attempt < 4 => {
                std::thread::sleep(std::time::Duration::from_millis(200u64 << attempt));
                attempt += 1;
            }
            Ok(r) if r.status == 429 || (500..=599).contains(&r.status) => {
                return Err(ConnectorError::transient(format!("GET {url}: HTTP {}", r.status)));
            }
            Ok(r) => {
                return Err(ConnectorError::fatal(format!("GET {url}: HTTP {}", r.status)));
            }
            Err(_) if attempt < 4 => {
                std::thread::sleep(std::time::Duration::from_millis(200u64 << attempt));
                attempt += 1;
            }
            Err(e) => return Err(ConnectorError::transient(format!("GET {url} failed: {e}"))),
        }
    }
}
