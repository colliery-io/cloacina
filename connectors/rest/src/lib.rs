//! The REST source (COLLIERY-I-0634 / CLOACINA-T-0990), moved from weir: the
//! shared runtime of every declarative source. It reads one stream of a
//! connector definition, which the host compiled into a
//! `cloacina_constructor_contract::sync::rest::RestConfig` and bound as this
//! guest's config. Templates over the user's config are already rendered; the
//! guest renders only `{{ stream_partition.* }}`.
//!
//! The read streams: one page per pull, each followed by a checkpoint whose
//! `opaque` holds the paginator's resume position, so a failure at page N
//! loses only page N. Pagination by page, offset, cursor token or `Link`
//! header; incremental reads by a cursor field; list and substream partition
//! routers. Auth is added by the host's egress; the guest sends plain
//! requests and never holds a credential. The cursor crosses the contract as
//! JSON text. Capability: `http`.
#![allow(clippy::all)]

use cloacina_constructor_contract::sync::cursor_cmp;
use cloacina_constructor_contract::sync::rest::{HttpMethod, InjectInto, Pagination, PartitionRouter, RestConfig};
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

pub struct Rest { cfg: cloacina_constructor_contract::sync::Config }

#[plugin_impl(Connector, crate = "fidius_guest", config = cloacina_constructor_contract::sync::Config)]
impl Connector for Rest {
    fn spec(&self) -> ConnectorSpec {
        ConnectorSpec {
            name: "rest".to_string(),
            connector_version: "0.1.0".to_string(),
            contract_version: 1,
            // The config is a RestConfig the host compiles from a connector
            // definition, not one a person writes.
            config_schema: "{\"type\":\"object\",\"required\":[\"base_url\",\"path\"],\"description\":\"A RestConfig compiled from a connector definition\"}".to_string(),
            // Auth is added by the host's egress; the guest declares no auth.
            roles: vec![ConnectorRole::Source],
            supported_sync_modes: vec![SyncMode::FullRefresh, SyncMode::Incremental],
        }
    }

    /// The config parses, and the first page of the first slice answers.
    fn check(&self, _config: Config) -> CheckResult {
        let result = RestCfg::from_json(&self.cfg.json).and_then(|c| {
            let path = match c.partition_kind.as_deref() {
                Some("list") => match c.partition_values.first() {
                    Some(v) => render_partition(&c.path, v),
                    None => return Ok(()),
                },
                Some("substream") => c.parent_path.clone().unwrap_or_default(),
                _ => c.path.clone(),
            };
            let mut rs = ResumeState::start(0, c.cursor_start.clone());
            fetch_page(&c, &path, &[], &mut rs).map(|_| ()).map_err(|e| e.message)
        });
        match result {
            Ok(()) => CheckResult { success: true, message: None },
            Err(e) => CheckResult { success: false, message: Some(e) },
        }
    }

    fn discover(&self, _config: Config) -> DiscoverOutcome {
        // Generic source: the stream is defined by config (path + record_path), so
        // discover advertises one configurable "records" stream.
        DiscoverOutcome::Catalog(Catalog { streams: vec![
            StreamInfo { name: "records".to_string(), namespace: None, schema: ArrowSchemaIpc { ipc: Vec::new() }, supported_sync_modes: vec![SyncMode::FullRefresh, SyncMode::Incremental], source_defined_cursor: false, default_cursor_field: None, source_defined_primary_key: None, partitioning: PartitionScheme::Unpartitioned },
        ] })
    }

    fn read(&self, ctx: ReadContext) -> fidius_guest::Stream<ReadMessage> {
        // Streaming read: pages are fetched lazily, one per pull,
        // each followed by a Checkpoint carrying the paginator's resume position in
        // `StreamState.opaque` — the engine commits per Checkpoint, so a failure at
        // page N loses only page N and memory is bounded by page size.
        let c = match RestCfg::from_json(&self.cfg.json) {
            Ok(c) => c,
            Err(message) => {
                return fidius_guest::Stream::from_iter(vec![ReadMessage::Fatal(ConnectorError {
                    kind: ErrorKind::Config,
                    message,
                    retryable: false,
                    context: Vec::new(),
                })]);
            }
        };
        // The committed cursor is JSON text; the paginator works on the value.
        let base_cursor = ctx.state.cursor.as_deref().map(cursor_value);
        let mut pending = std::collections::VecDeque::new();
        let rs = if ctx.state.opaque.is_empty() {
            ResumeState::start(0, base_cursor.clone().or_else(|| c.cursor_start.clone()))
        } else {
            match serde_json::from_slice::<ResumeState>(&ctx.state.opaque) {
                Ok(rs) => rs,
                Err(_) => {
                    pending.push_back(ReadMessage::Log(LogEntry {
                        level: LogLevel::Warn,
                        message: "unrecognized resume state in stream opaque; restarting the read from the committed cursor".to_string(),
                    }));
                    ResumeState::start(0, base_cursor.clone().or_else(|| c.cursor_start.clone()))
                }
            }
        };
        fidius_guest::Stream::from_iter(ReadIter {
            c,
            base_cursor,
            slices: None,
            rs,
            pages_read: 0,
            pending,
            finished: false,
        })
    }

    fn write(&self, _ctx: WriteContext, mut batches: fidius_guest::Stream<RecordBatch>) -> WriteOutcome {
        while batches.next_item().is_some() {}
        WriteOutcome {
            state: StreamState { cursor: None, opaque: Vec::new() },
            diagnostics: Vec::new(),
            dead_letters: Vec::new(),
            result: WriteResult::Err(ConnectorError::fatal("rest is a source, not a destination")),
        }
    }
}

impl Rest {
    fn configure(cfg: cloacina_constructor_contract::sync::Config) -> Self { Self { cfg } }
}

/// Parsed config for the generic REST source. Mirrors the declarative manifest
/// (url_base + path + record selector + pagination + datetime cursor), so a
/// vendored manifest's stream maps straight onto a connection config.
struct RestCfg {
    base_url: String,
    path: String,
    /// Dpath to the records array (`["data","items"]`); empty = top-level array.
    record_path: Vec<String>,
    /// Query param for the page number (None = single request, no pagination).
    page_param: Option<String>,
    /// Query param for a row offset (`?offset=N`); advances by `page_size` each page.
    /// Mutually exclusive with `page_param` in practice.
    offset_param: Option<String>,
    page_size_param: Option<String>,
    page_size: Option<u64>,
    /// Opaque cursor pagination: dpath to the next-page token in the response (empty = none)
    /// + the query param to send it as. Stop when the token is absent/empty.
    page_cursor_path: Vec<String>,
    page_cursor_param: Option<String>,
    /// Stripe-style: the next-page token is this field of the LAST
    /// record on the page (`starting_after = last id`). Wins over `page_cursor_path`
    /// being empty; dot-paths allowed.
    page_cursor_record_field: Option<String>,
    /// Response bool dpath; `false` there = last page (`has_more`) — stops without
    /// wasting an empty-page request.
    page_stop_on_false_path: Vec<String>,
    /// Link-header pagination: follow `Link: <url>; rel="next"` from the
    /// response headers (GitHub-style) until absent. The next URL is absolute.
    page_link_header: bool,
    /// Record field whose max advances the incremental cursor (None = full refresh).
    cursor_field: Option<String>,
    /// Raw-record dot-path the cursor value is read from when the response is columnar
    /// (, e.g. GA4's `dimensionValues.0.value`); None = `cursor_field`.
    cursor_value_path: Option<String>,
    /// Query/body param the cursor is sent as. None = **track-only**:
    /// the checkpoint advances but nothing is injected (window re-read APIs like GA4).
    cursor_param: Option<String>,
    /// Richer datetime cursor: initial lower bound (`cursor_start`, used
    /// only on the first run when there's no state) + an upper bound
    /// (`cursor_end_param`=`cursor_end`, sent on every request).
    cursor_start: Option<String>,
    cursor_end_param: Option<String>,
    cursor_end: Option<String>,
    /// Max retries on a transient failure (429 / 5xx / transport) with exponential backoff
    ///; honors `Retry-After`. Default 4.
    max_retries: u32,
    /// Max pages fetched **per run**. Hitting it commits a resumable
    /// checkpoint + a Warn log; the next run continues from that position. Default 1000.
    max_pages: u64,
    /// Request options: HTTP method (`GET` default | `POST`), a POST body
    /// (config-templated), and static request headers (e.g. `Notion-Version`). Auth headers
    /// are still host-injected.
    http_method: String,
    request_body: Option<String>,
    request_headers: Vec<(String, String)>,
    /// Body injection: carry the pagination params / incremental-cursor
    /// params as dot-path fields of the JSON POST body instead of the query string
    /// (`page_inject_into` / `cursor_inject_into` = `body`). The body is rebuilt per page.
    page_inject_body: bool,
    cursor_inject_body: bool,
    /// Header-row response shape (, Google-Sheets-style): records are
    /// row-arrays whose first row is the header — zip the rest into objects (snake_cased
    /// header cells as keys, ragged rows padded with null) + a `_row` index field.
    header_row: bool,
    /// Partition router: `list` (one request per `partition_values`) or
    /// `substream` (read `parent_path`, one request per `parent_key` of each parent
    /// record). The slice value is templated into the path as `{{ stream_partition.* }}`.
    partition_kind: Option<String>,
    partition_values: Vec<String>,
    parent_path: Option<String>,
    parent_record_path: Vec<String>,
    parent_key: Option<String>,
}

impl RestCfg {
    /// The guest's settings from the compiled `RestConfig`.
    fn from_json(json: &str) -> Result<Self, String> {
        let c: RestConfig = serde_json::from_str(json)
            .map_err(|e| format!("the REST config does not parse: {e}"))?;
        let dpath = |p: &str| -> Vec<String> {
            if p.is_empty() { Vec::new() } else { p.split('.').map(str::to_string).collect() }
        };
        let mut cfg = RestCfg {
            base_url: c.base_url,
            path: c.path,
            record_path: dpath(c.record_selector.as_deref().unwrap_or("")),
            page_param: None,
            offset_param: None,
            page_size_param: None,
            page_size: None,
            page_cursor_path: Vec::new(),
            page_cursor_param: None,
            page_cursor_record_field: None,
            page_stop_on_false_path: Vec::new(),
            page_link_header: false,
            cursor_field: None,
            cursor_value_path: None,
            cursor_param: None,
            cursor_start: None,
            cursor_end_param: None,
            cursor_end: None,
            max_retries: c.max_retries,
            max_pages: c.max_pages,
            http_method: match c.http_method {
                HttpMethod::Get => "GET".to_string(),
                HttpMethod::Post => "POST".to_string(),
            },
            request_body: c.request_body,
            request_headers: c.request_headers,
            page_inject_body: false,
            cursor_inject_body: false,
            header_row: c.header_row,
            partition_kind: None,
            partition_values: Vec::new(),
            parent_path: None,
            parent_record_path: Vec::new(),
            parent_key: None,
        };
        match c.pagination {
            Some(Pagination::Page { page_param, size_param, size, inject_into }) => {
                cfg.page_param = Some(page_param);
                cfg.page_size_param = Some(size_param);
                cfg.page_size = Some(size as u64);
                cfg.page_inject_body = inject_into == InjectInto::Body;
            }
            Some(Pagination::Offset { offset_param, limit_param, size, inject_into }) => {
                cfg.offset_param = Some(offset_param);
                cfg.page_size_param = Some(limit_param);
                cfg.page_size = Some(size as u64);
                cfg.page_inject_body = inject_into == InjectInto::Body;
            }
            Some(Pagination::Cursor {
                cursor_path,
                token_param,
                cursor_record_field,
                stop_on_false_path,
                size_param,
                size,
                inject_into,
            }) => {
                cfg.page_cursor_path = dpath(&cursor_path);
                cfg.page_cursor_param = Some(token_param);
                cfg.page_cursor_record_field = cursor_record_field;
                cfg.page_stop_on_false_path = dpath(stop_on_false_path.as_deref().unwrap_or(""));
                cfg.page_size_param = size_param;
                cfg.page_size = size.map(u64::from);
                cfg.page_inject_body = inject_into == InjectInto::Body;
            }
            Some(Pagination::LinkHeader) => cfg.page_link_header = true,
            None => {}
        }
        if let Some(i) = c.incremental {
            cfg.cursor_field = Some(i.cursor_field);
            cfg.cursor_value_path = i.cursor_value_path;
            cfg.cursor_param = Some(i.cursor_param).filter(|p| !p.is_empty());
            cfg.cursor_start = i.start_value;
            cfg.cursor_end_param = i.end_param;
            cfg.cursor_end = i.end_value;
            cfg.cursor_inject_body = i.inject_into == InjectInto::Body;
        }
        match c.partition {
            Some(PartitionRouter::List { values, .. }) => {
                cfg.partition_kind = Some("list".to_string());
                cfg.partition_values = values;
            }
            Some(PartitionRouter::Substream { parent_path, parent_record_selector, parent_key, .. }) => {
                cfg.partition_kind = Some("substream".to_string());
                cfg.parent_path = Some(parent_path);
                cfg.parent_record_path = dpath(parent_record_selector.as_deref().unwrap_or(""));
                cfg.parent_key = Some(parent_key);
            }
            None => {}
        }
        Ok(cfg)
    }
}

/// A cursor's value from its JSON text: a JSON string unquoted, anything else
/// as written.
fn cursor_value(text: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(serde_json::Value::String(s)) => s,
        Ok(other) => other.to_string(),
        Err(_) => text.to_string(),
    }
}

/// A cursor value as the JSON text the contract carries: a JSON string.
fn cursor_text(value: &str) -> String {
    serde_json::Value::String(value.to_string()).to_string()
}

/// Substitute `{{ stream_partition.* }}` placeholders in `path` with this slice's `value`
///. There is one partition value per slice, so the exact field inside
/// the braces doesn't need matching.
fn render_partition(path: &str, value: &str) -> String {
    let mut out = path.to_string();
    let mut from = 0;
    while let Some(rel_start) = out[from..].find("{{") {
        let start = from + rel_start;
        let Some(rel_end) = out[start..].find("}}") else { break };
        let end = start + rel_end + 2;
        if out[start + 2..end - 2].trim().starts_with("stream_partition") {
            out.replace_range(start..end, value);
            from = start + value.len();
        } else {
            from = end;
        }
    }
    out
}

/// Set `path` (dot-separated, intermediate objects created — or overwritten if not
/// objects) in `v` to `val`: body-injected pagination/cursor params
/// land at their dot-path in the JSON request body.
fn set_json_path(v: &mut serde_json::Value, path: &str, val: serde_json::Value) {
    let segs: Vec<&str> = path.split('.').collect();
    let Some((last, parents)) = segs.split_last() else { return };
    let mut cur = v;
    for seg in parents {
        if !cur.is_object() {
            *cur = serde_json::Value::Object(Default::default());
        }
        cur = cur
            .as_object_mut()
            .expect("just ensured object")
            .entry(seg.to_string())
            .or_insert(serde_json::Value::Null);
    }
    if !cur.is_object() {
        *cur = serde_json::Value::Object(Default::default());
    }
    cur.as_object_mut()
        .expect("just ensured object")
        .insert(last.to_string(), val);
}

/// Descend `path` into `v` (dpath); empty path returns `v` itself.
fn navigate<'a>(v: &'a serde_json::Value, path: &[String]) -> Option<&'a serde_json::Value> {
    let mut cur = v;
    for seg in path {
        cur = cur.get(seg)?;
    }
    Some(cur)
}

/// A header cell → a snake_case field name: trim, lowercase,
/// non-alphanumerics → `_`; an empty cell falls back to its column position.
fn header_name(cell: &serde_json::Value, i: usize) -> String {
    let raw = cell.as_str().map(str::to_string).unwrap_or_else(|| cell.to_string());
    let name: String = raw
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if name.chars().all(|ch| ch == '_') {
        format!("col_{i}")
    } else {
        name
    }
}

/// Descend a **dot-path** where a numeric segment indexes an array:
/// `dimensionValues.0.value` reaches `{"dimensionValues": [{"value": …}]}`.
fn get_path<'a>(v: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut cur = v;
    for seg in path.split('.') {
        cur = match (cur, seg.parse::<usize>()) {
            (serde_json::Value::Array(a), Ok(i)) => a.get(i)?,
            (x, _) => x.get(seg)?,
        };
    }
    Some(cur)
}

/// Percent-encode a query VALUE per RFC 3986 (unreserved `A-Za-z0-9-._~` pass
/// through) — exactly once, at build time. Values arrive RAW
/// (response next-tokens, record cursor fields, config datetimes): an ISO
/// `+02:00` offset would otherwise land as a space, and a base64 cursor's `+/=`
/// (or any `&`/`#`) corrupts the query. Param NAMES are author-controlled config
/// and go verbatim (e.g. `filter[updated]` stays literal). Numeric page/offset/
/// size params need no encoding; Link-header next URLs are spliced verbatim
/// (already server-encoded); body-injected params are JSON, not URL, values.
fn url_encode(s: &str) -> String {
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

/// Exponential backoff for retry `attempt` (0-based): 200ms, 400, 800, … capped at 8s.
fn backoff_ms(attempt: u32) -> u64 {
    (200u64 << attempt.min(5)).min(8000)
}

/// Parse a `Retry-After` header (integer seconds) → milliseconds, if present.
fn retry_after_ms(headers: &[(String, String)]) -> Option<u64> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("retry-after"))
        .and_then(|(_, v)| v.trim().parse::<u64>().ok())
        .map(|secs| secs * 1000)
}

/// Send `url` with an optional POST `body` (/: `Some` ⇒ POST —
/// the caller builds it per page when params inject into the body), retrying transient
/// failures (429 / 5xx / transport error) up to `max_retries` with exponential backoff,
/// honoring `Retry-After`. A non-retryable status is returned as `Ok` for
/// the caller to handle; a persistent transient one errors. Auth is injected host-side
///; static `request_headers` (e.g. `Notion-Version`) are set by the guest.
fn send_with_retry(
    c: &RestCfg,
    url: &str,
    body: Option<&str>,
) -> Result<fidius_guest::http::Response, ConnectorError> {
    let mut attempt: u32 = 0;
    loop {
        let mut req = if let Some(b) = body {
            fidius_guest::http::Request::post(url, b.as_bytes().to_vec())
        } else {
            fidius_guest::http::Request::get(url)
        };
        // Default content-type for a JSON POST body — the manifest's `request_body` /
        // body-injected params are JSON by construction. `request_headers` overrides it.
        if body.is_some()
            && !c
                .request_headers
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        {
            req = req.header("content-type", "application/json");
        }
        // Default User-Agent — some APIs (notably GitHub) reject requests without one
        // with a non-JSON error body. A manifest `request_headers` entry overrides it.
        let has_ua = c
            .request_headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("user-agent"));
        if !has_ua {
            req = req.header("user-agent", concat!("cloacina-rest/", env!("CARGO_PKG_VERSION")));
        }
        for (k, v) in &c.request_headers {
            req = req.header(k.as_str(), v.as_str());
        }
        match fidius_guest::http::send(req) {
            Ok(r) => {
                let retryable = r.status == 429 || (500..=599).contains(&r.status);
                if !retryable {
                    return Ok(r);
                }
                if attempt >= c.max_retries {
                    return Err(ConnectorError::transient(format!(
                        "request to {url} failed after {} retries (HTTP {})",
                        c.max_retries, r.status
                    )));
                }
                let wait = retry_after_ms(&r.headers).unwrap_or_else(|| backoff_ms(attempt));
                std::thread::sleep(std::time::Duration::from_millis(wait));
                attempt += 1;
            }
            Err(e) => {
                if attempt >= c.max_retries {
                    return Err(ConnectorError::transient(format!(
                        "request to {url} failed: {e}"
                    )));
                }
                std::thread::sleep(std::time::Duration::from_millis(backoff_ms(attempt)));
                attempt += 1;
            }
        }
    }
}

/// Parse an RFC 5988 `Link` header for the `rel="next"` URL, if present. e.g.
/// `Link: <https://api…?page=2>; rel="next", <…>; rel="last"` → the page-2 URL.
fn next_link_url(headers: &[(String, String)]) -> Option<String> {
    let link = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("link"))
        .map(|(_, v)| v.as_str())?;
    for part in link.split(',') {
        // A segment like `<url>; rel="next"`.
        let is_next = part
            .split(';')
            .skip(1)
            .any(|p| p.trim().trim_start_matches("rel=").trim_matches(['"', ' ']) == "next");
        if is_next {
            let start = part.find('<')?;
            let end = part[start..].find('>')? + start;
            return Some(part[start + 1..end].trim().to_string());
        }
    }
    None
}

/// The paginator's resume position, serialized as JSON into
/// `StreamState.opaque` at every per-page checkpoint. A run that dies mid-pagination
/// resumes exactly here; the clean-end checkpoint clears it. `req_cursor` snapshots
/// the incremental cursor this slice's requests are issued with (fixed at slice
/// start, so every page — and a resumed run — issues the same query shape), while
/// `cursor_seen` carries the advancing max, published as the committed cursor only
/// when the read completes.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct ResumeState {
    v: u32,
    /// Partition slice index; 0 for an unrouted read.
    slice: usize,
    /// Next page number to fetch (1-based; also drives the past-page-1 heuristics).
    page: u64,
    /// Next row offset (offset pagination).
    offset: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    page_cursor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    link_next: Option<String>,
    /// Header-row data-row counter.
    row_n: u64,
    /// Header-row field names — carried so a resumed slice doesn't need page 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    header: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    req_cursor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cursor_seen: Option<String>,
}

impl ResumeState {
    /// Fresh position at the start of `slice`, with the incremental cursor the
    /// slice's requests will carry.
    fn start(slice: usize, cursor: Option<String>) -> Self {
        Self {
            v: 1,
            slice,
            page: 1,
            offset: 0,
            page_cursor: None,
            link_next: None,
            row_n: 0,
            header: None,
            req_cursor: cursor.clone(),
            cursor_seen: cursor,
        }
    }
}

/// The streaming read driver: an iterator the host pulls one
/// message at a time — each step fetches ONE page and yields `Records` +
/// `Checkpoint`, so the engine commits progress per page. Hitting `max_pages`
/// stops LOUDLY (Warn log) at a committed, resumable checkpoint.
struct ReadIter {
    c: RestCfg,
    /// The committed cursor as of read start: mid-run checkpoints re-publish it
    /// (with the resume position in `opaque`), so a lost/cleared `opaque` can only
    /// re-read — never skip.
    base_cursor: Option<String>,
    /// Resolved slice paths (partition router, ); `None` until the
    /// first step — resolving substream parents fetches the parent stream.
    slices: Option<Vec<String>>,
    rs: ResumeState,
    /// Pages fetched by THIS run (`max_pages` is per run, so resumed chunked reads
    /// make full progress across runs).
    pages_read: u64,
    pending: std::collections::VecDeque<ReadMessage>,
    finished: bool,
}

impl Iterator for ReadIter {
    type Item = ReadMessage;
    fn next(&mut self) -> Option<ReadMessage> {
        loop {
            if let Some(m) = self.pending.pop_front() {
                return Some(m);
            }
            if self.finished {
                return None;
            }
            self.step();
        }
    }
}

impl ReadIter {
    /// Mid-run checkpoint: the safe base cursor + the resume position in `opaque`.
    fn checkpoint(&self) -> ReadMessage {
        let opaque = serde_json::to_vec(&self.rs).unwrap_or_default();
        ReadMessage::Checkpoint(StreamState { cursor: self.base_cursor.as_deref().map(cursor_text), opaque })
    }

    /// Clean end: publish the advanced incremental cursor, clear the resume position.
    fn finish_clean(&mut self) {
        let cursor = self
            .rs
            .cursor_seen
            .clone()
            .or_else(|| self.base_cursor.clone())
            .as_deref()
            .map(cursor_text);
        self.pending
            .push_back(ReadMessage::Checkpoint(StreamState { cursor, opaque: Vec::new() }));
        self.finished = true;
    }

    /// Resolve the partition router into child slice paths: `list`
    /// = one per static value; `substream` = one per parent `parent_key` (parents
    /// are (re-)fetched whole — parent lists are small, and a resumed run needs the
    /// same slice indexing; parent-set drift between attempts is inherent). No
    /// router → the one configured path.
    fn resolve_slices(&self) -> Result<Vec<String>, ConnectorError> {
        match self.c.partition_kind.as_deref() {
            Some("list") => Ok(self
                .c
                .partition_values
                .iter()
                .map(|val| render_partition(&self.c.path, val))
                .collect()),
            Some("substream") => {
                let parent_path = self.c.parent_path.clone().unwrap_or_default();
                let parents = fetch_all(&self.c, &parent_path, &self.c.parent_record_path)?;
                let key = self.c.parent_key.clone().unwrap_or_else(|| "id".to_string());
                let mut paths = Vec::new();
                for prow in &parents {
                    let Ok(pv) = serde_json::from_str::<serde_json::Value>(prow) else {
                        continue;
                    };
                    let id = match pv.get(&key) {
                        Some(serde_json::Value::String(s)) => s.clone(),
                        Some(other) => other.to_string(),
                        None => continue,
                    };
                    paths.push(render_partition(&self.c.path, &id));
                }
                Ok(paths)
            }
            _ => Ok(vec![self.c.path.clone()]),
        }
    }

    /// One driver step: fetch the next page of the current slice, queue its
    /// `Records` + the follow-up `Checkpoint`, and advance slice/finish state.
    fn step(&mut self) {
        if self.slices.is_none() {
            match self.resolve_slices() {
                Ok(paths) => {
                    if self.rs.slice >= paths.len() {
                        // Nothing (left) to read — an empty partition list, or a
                        // resumed slice index past a drifted parent set.
                        self.finish_clean();
                        return;
                    }
                    self.slices = Some(paths);
                }
                Err(e) => {
                    self.pending.push_back(ReadMessage::Fatal(e));
                    self.finished = true;
                    return;
                }
            }
        }
        let path = self.slices.as_ref().expect("resolved above")[self.rs.slice].clone();
        match fetch_page(&self.c, &path, &self.c.record_path, &mut self.rs) {
            Ok((rows, more)) => {
                self.pages_read += 1;
                if !rows.is_empty() {
                    self.pending.push_back(ReadMessage::Records(RecordBatch::Rows(rows)));
                }
                if more && self.pages_read >= self.c.max_pages {
                    // Loud, resumable stop — never a silent cap: the checkpoint
                    // commits this position and the next run continues from it.
                    self.pending.push_back(self.checkpoint());
                    self.pending.push_back(ReadMessage::Log(LogEntry {
                        level: LogLevel::Warn,
                        message: format!(
                            "max_pages={} reached mid-read; progress is checkpointed — the next run resumes from page {}",
                            self.c.max_pages, self.rs.page
                        ),
                    }));
                    self.finished = true;
                } else if more {
                    self.pending.push_back(self.checkpoint());
                } else {
                    // Slice exhausted → next slice (the running cursor carries over,
                    // matching the pre-streaming behavior) or clean end.
                    let nslices = self.slices.as_ref().map(Vec::len).unwrap_or(0);
                    self.rs = ResumeState::start(self.rs.slice + 1, self.rs.cursor_seen.clone());
                    if self.rs.slice >= nslices {
                        self.finish_clean();
                    } else {
                        self.pending.push_back(self.checkpoint());
                    }
                }
            }
            Err(e) => {
                self.pending.push_back(ReadMessage::Fatal(e));
                self.finished = true;
            }
        }
    }
}

/// Buffered fetch of a whole (paginated) stream — used only for substream PARENT
/// records, whose ids define the child slices. Bounded by `max_pages`.
fn fetch_all(
    c: &RestCfg,
    path: &str,
    record_path: &[String],
) -> Result<Vec<String>, ConnectorError> {
    let mut rs = ResumeState::start(0, c.cursor_start.clone());
    let mut rows = Vec::new();
    let mut pages: u64 = 0;
    loop {
        let (r, more) = fetch_page(c, path, record_path, &mut rs)?;
        rows.extend(r);
        pages += 1;
        if !more || pages >= c.max_pages {
            return Ok(rows);
        }
    }
}

/// Fetch ONE page of a slice: build the request from the resume
/// position in `rs`, send it, extract the page's records **whole** (the engine's
/// mapping stage shapes them), and advance `rs`. Returns the page's rows + whether
/// the slice has more pages. The incremental cursor param is `rs.req_cursor` —
/// snapshotted at slice start, so every page (and a resumed run) issues the same
/// query shape; the advancing max lands in `rs.cursor_seen`.
fn fetch_page(
    c: &RestCfg,
    path: &str,
    record_path: &[String],
    rs: &mut ResumeState,
) -> Result<(Vec<String>, bool), ConnectorError> {
    // Params split between the query string and the JSON body:
    // body-injected param names are dot-paths overlaid onto `request_body` per page.
    let mut body_params: Vec<(String, serde_json::Value)> = Vec::new();
    // A Link-header page carries an absolute next URL; otherwise build base_url+path+query.
    let url = if let Some(u) = rs.link_next.take() {
        u
    } else {
        let mut url = format!("{}{}", c.base_url, path);
        let mut q: Vec<String> = Vec::new();
        if let Some(p) = &c.page_param {
            if c.page_inject_body {
                body_params.push((p.clone(), rs.page.into()));
            } else {
                q.push(format!("{p}={}", rs.page));
            }
        }
        if let Some(p) = &c.offset_param {
            if c.page_inject_body {
                body_params.push((p.clone(), rs.offset.into()));
            } else {
                q.push(format!("{p}={}", rs.offset));
            }
        }
        if let (Some(p), Some(tok)) = (&c.page_cursor_param, &rs.page_cursor) {
            if c.page_inject_body {
                body_params.push((p.clone(), tok.clone().into()));
            } else {
                q.push(format!("{p}={}", url_encode(tok)));
            }
        }
        if let (Some(p), Some(sz)) = (&c.page_size_param, c.page_size) {
            if c.page_inject_body {
                body_params.push((p.clone(), sz.into()));
            } else {
                q.push(format!("{p}={sz}"));
            }
        }
        if let (Some(p), Some(cur)) = (&c.cursor_param, &rs.req_cursor) {
            if c.cursor_inject_body {
                body_params.push((p.clone(), cur.clone().into()));
            } else {
                q.push(format!("{p}={}", url_encode(cur)));
            }
        }
        if let (Some(p), Some(e)) = (&c.cursor_end_param, &c.cursor_end) {
            if c.cursor_inject_body {
                body_params.push((p.clone(), e.clone().into()));
            } else {
                q.push(format!("{p}={}", url_encode(e)));
            }
        }
        if !q.is_empty() {
            url.push('?');
            url.push_str(&q.join("&"));
        }
        url
    };

    // POST body: the (config-templated) `request_body`, overlaid with body-injected
    // params at their dot-paths — rebuilt per page as they advance.
    // Injected params imply POST even if `http_method` was left unset.
    let body = if body_params.is_empty() {
        c.http_method
            .eq_ignore_ascii_case("POST")
            .then(|| c.request_body.clone().unwrap_or_default())
    } else {
        let mut v: serde_json::Value = c
            .request_body
            .as_deref()
            .and_then(|b| serde_json::from_str(b).ok())
            .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
        for (path, val) in body_params {
            set_json_path(&mut v, &path, val);
        }
        Some(v.to_string())
    };

    // Auth is injected host-side by the egress policy; the guest
    // sends a plain request. Transient failures (429 / 5xx / transport) retry with
    // exponential backoff, honoring `Retry-After`.
    let resp = send_with_retry(c, &url, body.as_deref())?;
    // Status-aware end-of-data: an error page must NEVER read as a
    // clean end — a mid-pagination auth expiry would otherwise end the run
    // "successfully" with partial data. Failing here is cheap: prior pages are
    // committed per checkpoint and the retry resumes from them.
    // The one sanctioned non-2xx stop: a 404 PAST page 1 is how page-probing APIs
    // signal one-past-the-end (the same URL shape already succeeded, so it cannot
    // be a config error). 429/5xx never reach here — `send_with_retry` retries them
    // and errors transient on exhaustion.
    if !(200..=299).contains(&resp.status) {
        if resp.status == 404 && rs.page > 1 {
            return Ok((Vec::new(), false));
        }
        if resp.status == 401 {
            // Likely an expired host-side credential — transient, so the worker's
            // run-level retry re-resolves the credential and resumes from the last
            // committed page.
            return Err(ConnectorError::transient(format!(
                "HTTP 401 at page {} of {url} (credential likely expired mid-run)",
                rs.page
            )));
        }
        return Err(ConnectorError::fatal(format!(
            "HTTP {} at page {} of {url}",
            resp.status, rs.page
        )));
    }
    let body = resp.text();
    // A 2xx with an EMPTY body past page 1 is end-of-data (some APIs answer the
    // one-past-the-end probe with 200 and no body); any other unparseable 2xx body
    // is a real error at ANY page.
    let val: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) if body.trim().is_empty() && rs.page > 1 => return Ok((Vec::new(), false)),
        Err(e) => return Err(ConnectorError::fatal(format!("invalid JSON response: {e}"))),
    };
    // Past page 1, a missing record_path / non-array body is end-of-data: some APIs
    // answer the one-past-the-end probe with a **200** error object (e.g.
    // `{"error": "..."}`) instead of an empty array — real error statuses are
    // already handled by the gate above. On page 1 it's a real config error.
    let node = match navigate(&val, record_path) {
        Some(n) => n,
        None if rs.page > 1 => return Ok((Vec::new(), false)),
        None => {
            return Err(ConnectorError::fatal(format!(
                "record_path {record_path:?} not found in response"
            )))
        }
    };
    let records: Vec<&serde_json::Value> = match node {
        serde_json::Value::Array(a) => a.iter().collect(),
        // A single-object resource (e.g. xkcd `/info.0.json`) — emit it as one record.
        serde_json::Value::Object(_) if rs.page == 1 => vec![node],
        // Past page 1, a non-array body is end-of-data (a terminal error/empty page).
        _ if rs.page > 1 => return Ok((Vec::new(), false)),
        _ => {
            return Err(ConnectorError::fatal(
                "record_path did not resolve to a JSON array or object",
            ))
        }
    };
    if records.is_empty() {
        return Ok((Vec::new(), false));
    }
    let n = records.len() as u64;
    // Stripe-style: the next-page token is a field of the LAST
    // record on the page — capture it before the loop consumes `records`.
    let last_record_cursor = c.page_cursor_record_field.as_ref().and_then(|f| {
        records.last().and_then(|r| get_path(r, f)).map(|v| {
            v.as_str()
                .map(str::to_string)
                .unwrap_or_else(|| v.to_string())
        })
    });
    let mut rows: Vec<String> = Vec::new();
    for rec in records {
        // Header-row zip: first row-array = the field names (held in
        // `rs` across pages AND resumes); later row-arrays become objects (ragged rows
        // pad with null) + `_row`.
        if c.header_row {
            let Some(cells) = rec.as_array() else { continue };
            let Some(hdr) = &rs.header else {
                rs.header = Some(
                    cells
                        .iter()
                        .enumerate()
                        .map(|(i, cell)| header_name(cell, i))
                        .collect(),
                );
                continue;
            };
            rs.row_n += 1;
            let mut obj = serde_json::Map::new();
            obj.insert("_row".to_string(), rs.row_n.into());
            for (i, name) in hdr.iter().enumerate() {
                obj.insert(
                    name.clone(),
                    cells.get(i).cloned().unwrap_or(serde_json::Value::Null),
                );
            }
            rows.push(serde_json::Value::Object(obj).to_string());
            continue;
        }
        if let Some(cf) = &c.cursor_field {
            // Columnar responses carry the cursor nested — read it
            // at `cursor_value_path` when set, else at the flat `cursor_field`.
            let cv = match &c.cursor_value_path {
                Some(path) => get_path(rec, path),
                None => rec.get(cf),
            };
            if let Some(cv) = cv {
                let cs = cv.as_str().map(str::to_string).unwrap_or_else(|| cv.to_string());
                // Numeric-aware max: `"9" > "12"` lexicographically.
                if rs
                    .cursor_seen
                    .as_deref()
                    .map_or(true, |cur| cursor_cmp(&cs, cur).is_gt())
                {
                    rs.cursor_seen = Some(cs);
                }
            }
        }
        rows.push(rec.to_string());
    }
    // Link-header pagination: follow `Link: <url>; rel="next"` from the
    // response headers until it's absent (GitHub-style). The next URL is absolute.
    if c.page_link_header {
        return match next_link_url(&resp.headers) {
            Some(u) => {
                rs.link_next = Some(u);
                rs.page += 1;
                Ok((rows, true))
            }
            None => Ok((rows, false)),
        };
    }
    // has_more-style stop: a false at the configured response path
    // is the last page — stop now instead of wasting an empty-page request.
    if !c.page_stop_on_false_path.is_empty()
        && navigate(&val, &c.page_stop_on_false_path).and_then(|x| x.as_bool()) == Some(false)
    {
        return Ok((rows, false));
    }
    // Advance: opaque cursor (`?cursor=<token>` from the response or the last record),
    // page-increment (`?page=N`), or offset (`?offset=N`, by page_size).
    let cursor_paginating =
        !c.page_cursor_path.is_empty() || c.page_cursor_record_field.is_some();
    if c.page_param.is_none() && c.offset_param.is_none() && !cursor_paginating {
        return Ok((rows, false)); // no pagination → single request
    }
    if cursor_paginating {
        // Next-page token: the response path when configured, else the captured
        // last-record field; absent/empty → last page.
        let tok = if !c.page_cursor_path.is_empty() {
            navigate(&val, &c.page_cursor_path)
                .and_then(|x| x.as_str())
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        } else {
            last_record_cursor.filter(|s| !s.is_empty())
        };
        match tok {
            Some(tok) => rs.page_cursor = Some(tok),
            None => return Ok((rows, false)),
        }
    } else if let Some(sz) = c.page_size {
        if n < sz {
            return Ok((rows, false)); // short page → last page
        }
    }
    rs.page += 1;
    rs.offset += c.page_size.unwrap_or(n);
    Ok((rows, true))
}
