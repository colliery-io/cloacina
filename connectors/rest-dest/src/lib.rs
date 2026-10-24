//! The REST destination (COLLIERY-I-0634 / CLOACINA-T-0991), moved from weir:
//! the shared runtime of every declarative destination. It writes one object
//! of a destination definition, which the host compiled into a
//! `cloacina_constructor_contract::sync::rest::RestDestConfig`. Each record is
//! shaped (field map, then an optional body wrap) and sent to the object's
//! path, with `{{ record.<field> }}` replaced by the record's value,
//! percent-encoded. The method is the config's, or the write mode's: `PATCH`
//! for upsert, `POST` otherwise. A CDC delete goes to `delete_path`. Auth is
//! added by the host's egress. 429 and 5xx retry with backoff; a record the
//! API refuses is a dead-letter, and the write goes on. Capability: `http`.
#![allow(clippy::all)]

use cloacina_constructor_contract::sync::rest::RestDestConfig;
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

pub struct RestDest { cfg: cloacina_constructor_contract::sync::Config }

impl RestDest {
    fn configure(cfg: cloacina_constructor_contract::sync::Config) -> Self { Self { cfg } }
}

#[plugin_impl(Connector, crate = "fidius_guest", config = cloacina_constructor_contract::sync::Config)]
impl Connector for RestDest {
    fn spec(&self) -> ConnectorSpec {
        ConnectorSpec {
            name: "rest-dest".to_string(),
            connector_version: "0.1.0".to_string(),
            contract_version: 1,
            // The config is a RestDestConfig the host compiles from a
            // destination definition.
            config_schema: "{\"type\":\"object\",\"required\":[\"base_url\",\"path\"],\"description\":\"A RestDestConfig compiled from a destination definition\"}".to_string(),
            roles: vec![ConnectorRole::Destination],
            supported_sync_modes: vec![SyncMode::FullRefresh, SyncMode::Incremental],
        }
    }

    fn check(&self, _config: Config) -> CheckResult {
        // There is no cheap probe without spending a write: check the config.
        match parse_cfg(&self.cfg.json) {
            Ok(_) => CheckResult { success: true, message: None },
            Err(e) => CheckResult { success: false, message: Some(e) },
        }
    }

    fn discover(&self, _config: Config) -> DiscoverOutcome {
        // A destination discovers no streams; the object comes from its definition.
        DiscoverOutcome::Catalog(Catalog { streams: Vec::new() })
    }

    fn read(&self, _ctx: ReadContext) -> fidius_guest::Stream<ReadMessage> {
        fidius_guest::Stream::from_iter(vec![ReadMessage::Fatal(ConnectorError::fatal(
            "rest-dest is a write-only destination; it has no read side",
        ))])
    }

    fn write(&self, ctx: WriteContext, mut batches: fidius_guest::Stream<RecordBatch>) -> WriteOutcome {
        let mut c = match parse_cfg(&self.cfg.json) {
            Ok(c) => c,
            Err(message) => {
                return write_err(ConnectorError {
                    kind: ErrorKind::Config,
                    message,
                    retryable: false,
                    context: Vec::new(),
                });
            }
        };
        // Stream-driven upsert: unless the config pins a method, an
        // `Upsert` write mode means PATCH the keyed URL (create-or-update, idempotent on
        // replay); anything else creates with POST.
        if c.method.is_empty() {
            c.method = match ctx.stream.write_mode {
                WriteMode::Upsert { .. } => "PATCH".to_string(),
                _ => "POST".to_string(),
            };
        }
        let mut accepted: u64 = 0;
        let mut dead_letters: Vec<DeadLetter> = Vec::new();
        while let Some(batch) = batches.next_item() {
            match batch {
                RecordBatch::Rows(rows) => {
                    for row in rows {
                        match write_one(&c, &row) {
                            Ok(()) => accepted += 1,
                            Err(reason) => dead_letters.push(DeadLetter { record: row, reason }),
                        }
                    }
                }
                // CDC changes: Delete → DELETE the keyed URL; Insert/Update → write.
                RecordBatch::Changes(changes) => {
                    for ch in changes {
                        let res = match ch.op {
                            ChangeOp::Delete => delete_one(&c, &ch.data),
                            _ => write_one(&c, &ch.data),
                        };
                        match res {
                            Ok(()) => accepted += 1,
                            Err(reason) => dead_letters.push(DeadLetter { record: ch.data, reason }),
                        }
                    }
                }
                RecordBatch::Arrow(_) => {
                    return write_err(ConnectorError::fatal(
                        "rest-dest takes Rows/Changes (JSON text), not Arrow batches",
                    ));
                }
            }
        }
        WriteOutcome {
            state: StreamState { cursor: None, opaque: Vec::new() },
            diagnostics: Vec::new(),
            dead_letters,
            result: WriteResult::Ok(WriteReceipt { accepted }),
        }
    }
}

fn write_err(e: ConnectorError) -> WriteOutcome {
    WriteOutcome {
        state: StreamState { cursor: None, opaque: Vec::new() },
        diagnostics: Vec::new(),
        dead_letters: Vec::new(),
        result: WriteResult::Err(e),
    }
}

/// The guest's settings, from the compiled `RestDestConfig`.
struct DestCfg {
    base_url: String,
    /// Templated with `{{ record.<field> }}`.
    path: String,
    /// Empty until `write` takes it from the write mode.
    method: String,
    /// Record field to API property. Empty means the record as it is.
    field_map: Vec<(String, String)>,
    /// Nest the shaped fields under this key (HubSpot's `properties`).
    body_wrap: Option<String>,
    /// Where a CDC delete goes; without it a delete is a dead-letter.
    delete_path: Option<String>,
    delete_method: String,
    max_retries: u32,
}

fn parse_cfg(json: &str) -> Result<DestCfg, String> {
    let c: RestDestConfig = serde_json::from_str(json)
        .map_err(|e| format!("the REST destination config does not parse: {e}"))?;
    Ok(DestCfg {
        base_url: c.base_url,
        path: c.path,
        method: c.method.unwrap_or_default(),
        field_map: c.field_map,
        body_wrap: c.body_wrap,
        delete_path: c.delete_path,
        delete_method: c.delete_method.unwrap_or_else(|| "DELETE".to_string()),
        max_retries: c.max_retries,
    })
}

/// Replace `{{ record.<field> }}` (or `record['field']`) with the record's
/// value, percent-encoded for a path: a value like `a+b@x.com` or one with a
/// slash must arrive as it is. A field the record lacks renders empty.
fn render_path(path: &str, rec: &serde_json::Value) -> String {
    let mut out = String::with_capacity(path.len());
    let mut rest = path;
    while let Some(start) = rest.find("{{") {
        let Some(len) = rest[start..].find("}}") else { break };
        out.push_str(&rest[..start]);
        let expr = rest[start + 2..start + len].trim();
        let field = expr
            .strip_prefix("record")
            .map(str::trim)
            .and_then(|r| match r.strip_prefix('[') {
                Some(b) => Some(b.trim_end_matches(']').trim().trim_matches(|c| c == '\'' || c == '"')),
                None => r.strip_prefix('.').map(str::trim),
            })
            .unwrap_or(expr);
        let value = rec
            .get(field)
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .unwrap_or_default();
        out.push_str(&encode_segment(&value));
        rest = &rest[start + len + 2..];
    }
    out.push_str(rest);
    out
}

/// Percent-encode a path segment: unreserved characters and `@` stay.
fn encode_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~' | b'@') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Upsert one record. `Ok(())` = accepted (2xx); `Err(reason)` = dead-letter (4xx / exhausted).
fn write_one(c: &DestCfg, row: &str) -> Result<(), String> {
    let rec: serde_json::Value =
        serde_json::from_str(row).map_err(|e| format!("record json: {e}"))?;
    let path = render_path(&c.path, &rec);
    let url = format!("{}{}", c.base_url.trim_end_matches('/'), path);
    let body = shape_body(c, &rec)?;
    let resp = send_with_retry(c, &url, &c.method, body.as_bytes())?;
    if (200..300).contains(&resp.status) {
        Ok(())
    } else {
        Err(format!("HTTP {} from {url}: {}", resp.status, truncate(&resp.text(), 200)))
    }
}

/// Apply a CDC delete: render the `delete_path` template with the record's key and
/// issue an HTTP `DELETE` (or the configured `delete_method`). Missing `delete_path` → dead-letter.
fn delete_one(c: &DestCfg, row: &str) -> Result<(), String> {
    let rec: serde_json::Value =
        serde_json::from_str(row).map_err(|e| format!("record json: {e}"))?;
    let Some(delete_path) = c.delete_path.as_deref() else {
        return Err("delete on a change but no `delete_path` configured".to_string());
    };
    let path = render_path(delete_path, &rec);
    let url = format!("{}{}", c.base_url.trim_end_matches('/'), path);
    let resp = send_with_retry(c, &url, &c.delete_method, &[])?;
    // 2xx or 404 (already gone) both settle a delete.
    if (200..300).contains(&resp.status) || resp.status == 404 {
        Ok(())
    } else {
        Err(format!("HTTP {} deleting {url}: {}", resp.status, truncate(&resp.text(), 200)))
    }
}

/// Shape a record into the SaaS object body: apply the field map (or identity), then wrap.
fn shape_body(c: &DestCfg, rec: &serde_json::Value) -> Result<String, String> {
    let obj = rec.as_object().ok_or_else(|| "record is not a JSON object".to_string())?;
    let shaped: serde_json::Map<String, serde_json::Value> = if c.field_map.is_empty() {
        obj.clone()
    } else {
        c.field_map
            .iter()
            .filter_map(|(from, to)| obj.get(from).map(|v| (to.clone(), v.clone())))
            .collect()
    };
    let value = match &c.body_wrap {
        Some(wrap) => {
            let mut outer = serde_json::Map::new();
            outer.insert(wrap.clone(), serde_json::Value::Object(shaped));
            serde_json::Value::Object(outer)
        }
        None => serde_json::Value::Object(shaped),
    };
    serde_json::to_string(&value).map_err(|e| format!("shape: {e}"))
}


fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n { s.to_string() } else { format!("{}…", &s[..n]) }
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

/// Send the shaped body to `url` with the configured method, retrying transient failures
/// (429 / 5xx / transport) with backoff, honoring `Retry-After`. Auth is host-injected; the
/// guest sends `Content-Type: application/json` + a plain request.
fn send_with_retry(
    c: &DestCfg,
    url: &str,
    method: &str,
    body: &[u8],
) -> Result<fidius_guest::http::Response, String> {
    let mut attempt: u32 = 0;
    loop {
        let mut req = fidius_guest::http::Request::post(url, body.to_vec());
        req.method = method.to_uppercase();
        req = req.header("content-type", "application/json");
        match fidius_guest::http::send(req) {
            Ok(r) => {
                let retryable = r.status == 429 || (500..=599).contains(&r.status);
                if !retryable || attempt >= c.max_retries {
                    return Ok(r);
                }
                let wait = retry_after_ms(&r.headers).unwrap_or_else(|| backoff_ms(attempt));
                std::thread::sleep(std::time::Duration::from_millis(wait));
                attempt += 1;
            }
            Err(e) => {
                if attempt >= c.max_retries {
                    return Err(format!("request to {url} failed: {e}"));
                }
                std::thread::sleep(std::time::Duration::from_millis(backoff_ms(attempt)));
                attempt += 1;
            }
        }
    }
}
