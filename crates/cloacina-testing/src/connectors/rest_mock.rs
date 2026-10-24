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

//! A mock JSON API for the REST runtime's tests (CLOACINA-T-0990,
//! CLOACINA-A-0014).
//!
//! Each endpoint serves a list of records, paged one way: by page number, by
//! offset, by a next-page token, or by a `Link` header. Records can be
//! wrapped under a key, filtered by an incremental parameter, and guarded by
//! a header the host must inject. Parameters are read from the query string
//! and, for a POST, from the JSON body (flattened to dot-paths). Every request
//! is kept, so a test can see what the guest sent.
//!
//! A **sink** takes writes: any request under its path prefix is kept with
//! its JSON body and answered `200`, unless the body's field at a dot-path
//! holds a refused value (`400`).
//!
//! It is also a conformance [`Service`]: a step with `endpoint` configures one
//! (the fields of [`Endpoint`]); `sink` adds a sink, with an optional
//! `refuse_when = [dot-path, value]`; `requests_to` returns the requests made
//! to a path, as `{method, path, params, headers}` objects; `requests_under`
//! returns the requests under a prefix as `{method, path, body, headers}`.

use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::conformance::Service;

/// How an endpoint pages.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Paging {
    /// Everything in one answer.
    #[default]
    None,
    /// `page` from 1 and `size`; a page past the end is empty.
    Page {
        page_param: String,
        size_param: String,
    },
    /// `offset` and `limit`.
    Offset {
        offset_param: String,
        limit_param: String,
    },
    /// A token: the answer carries the next token at `next_path` (absent on
    /// the last page); the request sends it as `token_param`.
    Cursor {
        token_param: String,
        next_path: String,
        size_param: String,
    },
    /// Page numbers, with the next page's URL in a `Link` header.
    Link {
        page_param: String,
        size_param: String,
    },
}

/// One endpoint of the mock.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub records: Vec<Value>,
    #[serde(default)]
    pub paging: Paging,
    /// The page size when the request sends none.
    #[serde(default = "default_size")]
    pub default_size: usize,
    /// Wrap the page's records as `{"<wrap>": [...]}`.
    #[serde(default)]
    pub wrap: Option<String>,
    /// Answer the whole body as it is, ignoring `records` and paging (a
    /// header-row sheet, a single object).
    #[serde(default)]
    pub body: Option<Value>,
    /// Incremental: keep the records whose `field` is greater than the value
    /// of the request's `param`.
    #[serde(default)]
    pub filter: Option<Filter>,
    /// The header (name, value) the request must carry; otherwise `401`.
    #[serde(default)]
    pub require_header: Option<(String, String)>,
    /// Answer `429` to this many first requests.
    #[serde(default)]
    pub throttle_first: usize,
}

fn default_size() -> usize {
    100
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filter {
    pub param: String,
    pub field: String,
}

/// A request the mock received.
#[derive(Debug, Clone)]
pub struct Seen {
    pub method: String,
    pub path: String,
    /// Query parameters, and a POST body's fields as dot-paths.
    pub params: BTreeMap<String, String>,
    pub headers: BTreeMap<String, String>,
    /// The JSON body, when there was one.
    pub body: Option<Value>,
}

/// A path prefix that takes writes.
#[derive(Debug, Clone)]
struct Sink {
    prefix: String,
    refuse_when: Option<(String, Value)>,
}

#[derive(Default)]
struct State {
    endpoints: HashMap<String, Endpoint>,
    sinks: Vec<Sink>,
    throttled: HashMap<String, usize>,
    seen: Vec<Seen>,
}

/// A running mock. It stops when the test process ends.
pub struct RestMock {
    port: u16,
    state: Arc<Mutex<State>>,
}

impl Default for RestMock {
    fn default() -> Self {
        Self::start()
    }
}

impl RestMock {
    /// Start the mock on a free local port.
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind the mock");
        let port = listener.local_addr().unwrap().port();
        let state = Arc::new(Mutex::new(State::default()));
        let st = state.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let st = st.clone();
                std::thread::spawn(move || serve(stream, port, &st));
            }
        });
        Self { port, state }
    }

    /// The `base_url` a REST config names.
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// The http grant for the mock.
    pub fn grant(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    /// Serve `endpoint` at `path`, replacing what was there.
    pub fn serve(&self, path: &str, endpoint: Endpoint) {
        self.state
            .lock()
            .unwrap()
            .endpoints
            .insert(path.to_string(), endpoint);
    }

    /// Take writes under `prefix`, refusing a body whose `field` (a dot-path)
    /// is `value`.
    pub fn sink(&self, prefix: &str, refuse_when: Option<(String, Value)>) {
        self.state.lock().unwrap().sinks.push(Sink {
            prefix: prefix.to_string(),
            refuse_when,
        });
    }

    /// The requests under `prefix`, in order.
    pub fn requests_under(&self, prefix: &str) -> Vec<Seen> {
        self.state
            .lock()
            .unwrap()
            .seen
            .iter()
            .filter(|s| s.path.starts_with(prefix))
            .cloned()
            .collect()
    }

    /// The requests made to `path`, in order.
    pub fn requests_to(&self, path: &str) -> Vec<Seen> {
        self.state
            .lock()
            .unwrap()
            .seen
            .iter()
            .filter(|s| s.path == path)
            .cloned()
            .collect()
    }
}

impl Service for RestMock {
    fn run(&self, step: &Map<String, Value>) -> Result<Vec<Value>, String> {
        if let Some(path) = step.get("requests_to").and_then(Value::as_str) {
            return Ok(self
                .requests_to(path)
                .into_iter()
                .map(|s| {
                    json!({
                        "method": s.method,
                        "path": s.path,
                        "params": s.params,
                        "headers": s.headers,
                    })
                })
                .collect());
        }
        if let Some(prefix) = step.get("requests_under").and_then(Value::as_str) {
            return Ok(self
                .requests_under(prefix)
                .into_iter()
                .map(|s| json!({ "method": s.method, "path": s.path, "body": s.body, "headers": s.headers }))
                .collect());
        }
        if let Some(prefix) = step.get("sink").and_then(Value::as_str) {
            let refuse = match step.get("refuse_when") {
                None => None,
                Some(Value::Array(a)) if a.len() == 2 && a[0].is_string() => {
                    Some((a[0].as_str().unwrap().to_string(), a[1].clone()))
                }
                Some(other) => {
                    return Err(format!(
                        "refuse_when must be [dot-path, value], got {other}"
                    ));
                }
            };
            self.sink(prefix, refuse);
            return Ok(Vec::new());
        }
        let mut fields = step.clone();
        let path = fields
            .remove("endpoint")
            .and_then(|p| p.as_str().map(str::to_string))
            .ok_or("a mock step needs `endpoint` or `requests_to`")?;
        let endpoint: Endpoint = serde_json::from_value(Value::Object(fields))
            .map_err(|e| format!("endpoint `{path}`: {e}"))?;
        self.serve(&path, endpoint);
        Ok(Vec::new())
    }
}

// ---- HTTP ----

fn serve(mut stream: TcpStream, port: u16, state: &Mutex<State>) {
    let Some((head, body)) = read_request(&mut stream) else {
        return;
    };
    let (status, headers, answer) = answer(&head, &body, port, state);
    let text = answer.to_string();
    let mut out = format!(
        "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n",
        text.len()
    );
    for (k, v) in headers {
        out.push_str(&format!("{k}: {v}\r\n"));
    }
    out.push_str("\r\n");
    out.push_str(&text);
    let _ = stream.write_all(out.as_bytes());
    let _ = stream.flush();
}

fn read_request(stream: &mut TcpStream) -> Option<(String, String)> {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .ok();
    let mut data = Vec::new();
    let mut buf = [0u8; 8192];
    let end = loop {
        let n = stream.read(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        data.extend_from_slice(&buf[..n]);
        if let Some(end) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            break end;
        }
    };
    let head = String::from_utf8_lossy(&data[..end]).into_owned();
    let mut raw = data[end + 4..].to_vec();
    if head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        while !raw.ends_with(b"0\r\n\r\n") {
            let n = stream.read(&mut buf).ok()?;
            if n == 0 {
                break;
            }
            raw.extend_from_slice(&buf[..n]);
        }
        return Some((head, dechunk(&String::from_utf8_lossy(&raw))));
    }
    let len = head_value(&head, "content-length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    while raw.len() < len {
        let n = stream.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        raw.extend_from_slice(&buf[..n]);
    }
    Some((head, String::from_utf8_lossy(&raw).into_owned()))
}

fn dechunk(raw: &str) -> String {
    let mut out = String::new();
    let mut rest = raw;
    while let Some((size_line, tail)) = rest.split_once("\r\n") {
        let Ok(size) = usize::from_str_radix(size_line.trim(), 16) else {
            break;
        };
        if size == 0 || tail.len() < size {
            break;
        }
        out.push_str(&tail[..size]);
        rest = tail.get(size + 2..).unwrap_or("");
    }
    out
}

fn head_value<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    head.lines().skip(1).find_map(|l| {
        let (k, v) = l.split_once(':')?;
        k.trim().eq_ignore_ascii_case(name).then(|| v.trim())
    })
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => match u8::from_str_radix(&s[i + 1..i + 3], 16) {
                Ok(b) => {
                    out.push(b);
                    i += 3;
                }
                Err(_) => {
                    out.push(b'%');
                    i += 1;
                }
            },
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A JSON body's fields as dot-paths, scalars as text.
fn flatten(prefix: &str, v: &Value, out: &mut BTreeMap<String, String>) {
    match v {
        Value::Object(o) => {
            for (k, v) in o {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten(&key, v, out);
            }
        }
        Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        other => {
            out.insert(prefix.to_string(), other.to_string());
        }
    }
}

/// Numbers compare as numbers, anything else as text.
fn greater(a: &Value, bound: &str) -> bool {
    let a = match a {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    match (a.parse::<f64>(), bound.parse::<f64>()) {
        (Ok(x), Ok(y)) => x > y,
        _ => a.as_str() > bound,
    }
}

fn answer(
    head: &str,
    body: &str,
    port: u16,
    state: &Mutex<State>,
) -> (u16, Vec<(String, String)>, Value) {
    let mut words = head.split_whitespace();
    let method = words.next().unwrap_or("").to_string();
    let target = words.next().unwrap_or("").to_string();
    let (path, query) = target.split_once('?').unwrap_or((&target, ""));
    let mut params = BTreeMap::new();
    for kv in query.split('&').filter(|kv| !kv.is_empty()) {
        let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
        params.insert(percent_decode(k), percent_decode(v));
    }
    let json_body = serde_json::from_str::<Value>(body)
        .ok()
        .filter(|_| !body.trim().is_empty());
    if let Some(v) = &json_body {
        flatten("", v, &mut params);
    }
    let headers: BTreeMap<String, String> = head
        .lines()
        .skip(1)
        .filter_map(|l| {
            let (k, v) = l.split_once(':')?;
            Some((k.trim().to_ascii_lowercase(), v.trim().to_string()))
        })
        .collect();
    let mut st = state.lock().unwrap();
    st.seen.push(Seen {
        method,
        path: path.to_string(),
        params: params.clone(),
        headers: headers.clone(),
        body: json_body.clone(),
    });
    if !st.endpoints.contains_key(path)
        && let Some(sink) = st.sinks.iter().find(|k| path.starts_with(&k.prefix))
    {
        if let (Some((field, value)), Some(b)) = (&sink.refuse_when, &json_body)
            && get_path(b, field) == Some(value)
        {
            return (
                400,
                vec![],
                json!({ "error": format!("{field} is refused") }),
            );
        }
        return (200, vec![], json!({ "ok": true }));
    }
    let Some(ep) = st.endpoints.get(path).cloned() else {
        return (
            404,
            vec![],
            json!({ "error": format!("no endpoint {path}") }),
        );
    };
    if let Some((name, value)) = &ep.require_header
        && headers.get(&name.to_ascii_lowercase()) != Some(value)
    {
        return (
            401,
            vec![],
            json!({ "error": format!("{name} is missing or wrong") }),
        );
    }
    let throttled = st.throttled.entry(path.to_string()).or_insert(0);
    if *throttled < ep.throttle_first {
        *throttled += 1;
        return (
            429,
            vec![("retry-after".to_string(), "0".to_string())],
            json!({ "error": "slow down" }),
        );
    }
    drop(st);
    if let Some(body) = ep.body {
        return (200, vec![], body);
    }

    let mut records = ep.records.clone();
    if let Some(f) = &ep.filter
        && let Some(bound) = params.get(&f.param)
    {
        records.retain(|r| r.get(&f.field).is_some_and(|v| greater(v, bound)));
    }
    let num = |name: &str, default: usize| -> usize {
        params
            .get(name)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    let slice = |from: usize, size: usize| -> Vec<Value> {
        records.iter().skip(from).take(size).cloned().collect()
    };
    let mut extra = Vec::new();
    let mut next_token = None;
    let page: Vec<Value> = match &ep.paging {
        Paging::None => records.clone(),
        Paging::Page {
            page_param,
            size_param,
        } => {
            let size = num(size_param, ep.default_size);
            slice((num(page_param, 1).max(1) - 1) * size, size)
        }
        Paging::Offset {
            offset_param,
            limit_param,
        } => slice(num(offset_param, 0), num(limit_param, ep.default_size)),
        Paging::Cursor {
            token_param,
            size_param,
            ..
        } => {
            let size = num(size_param, ep.default_size);
            let from = params
                .get(token_param)
                .and_then(|t| t.strip_prefix("tok-"))
                .and_then(|t| t.parse().ok())
                .unwrap_or(0);
            if from + size < records.len() {
                next_token = Some(format!("tok-{}", from + size));
            }
            slice(from, size)
        }
        Paging::Link {
            page_param,
            size_param,
        } => {
            let size = num(size_param, ep.default_size);
            let n = num(page_param, 1).max(1);
            if n * size < records.len() {
                extra.push((
                    "link".to_string(),
                    format!(
                        "<http://127.0.0.1:{port}{path}?{page_param}={}&{size_param}={size}>; rel=\"next\", <http://127.0.0.1:{port}{path}?{page_param}=1&{size_param}={size}>; rel=\"first\"",
                        n + 1
                    ),
                ));
            }
            slice((n - 1) * size, size)
        }
    };
    let mut answer = match &ep.wrap {
        Some(key) => json!({ key.as_str(): page }),
        None => Value::Array(page),
    };
    if let Paging::Cursor { next_path, .. } = &ep.paging {
        if !answer.is_object() {
            answer = json!({ "records": answer });
        }
        if let Some(tok) = next_token {
            set_path(&mut answer, next_path, Value::String(tok));
        }
    }
    (200, extra, answer)
}

fn get_path<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(v, |cur, seg| cur.get(seg))
}

fn set_path(v: &mut Value, path: &str, val: Value) {
    let mut cur = v;
    let segs: Vec<&str> = path.split('.').collect();
    for (i, seg) in segs.iter().enumerate() {
        if !cur.is_object() {
            *cur = json!({});
        }
        let obj = cur.as_object_mut().unwrap();
        if i == segs.len() - 1 {
            obj.insert(seg.to_string(), val);
            return;
        }
        cur = obj.entry(seg.to_string()).or_insert(json!({}));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(url: &str, headers: &[(&str, &str)]) -> (u16, String, Value) {
        let addr = url.strip_prefix("http://").unwrap();
        let (host, rest) = addr.split_once('/').unwrap();
        let mut s = TcpStream::connect(host).unwrap();
        let mut req = format!("GET /{rest} HTTP/1.1\r\nhost: {host}\r\n");
        for (k, v) in headers {
            req.push_str(&format!("{k}: {v}\r\n"));
        }
        req.push_str("\r\n");
        s.write_all(req.as_bytes()).unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        let (head, body) = out.split_once("\r\n\r\n").unwrap();
        let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        (
            status,
            head.to_string(),
            serde_json::from_str(body).unwrap(),
        )
    }

    fn records(n: i64) -> Vec<Value> {
        (1..=n).map(|i| json!({ "id": i })).collect()
    }

    #[test]
    fn page_offset_cursor_and_link_paging() {
        let m = RestMock::start();
        let page = Paging::Page {
            page_param: "page".into(),
            size_param: "per".into(),
        };
        m.serve(
            "/p",
            Endpoint {
                records: records(5),
                paging: page,
                ..Default::default()
            },
        );
        let (_, _, v) = get(&format!("{}/p?page=2&per=2", m.base_url()), &[]);
        assert_eq!(v, json!([{ "id": 3 }, { "id": 4 }]));
        let (_, _, v) = get(&format!("{}/p?page=4&per=2", m.base_url()), &[]);
        assert_eq!(v, json!([]));

        m.serve(
            "/o",
            Endpoint {
                records: records(5),
                paging: Paging::Offset {
                    offset_param: "offset".into(),
                    limit_param: "limit".into(),
                },
                wrap: Some("data".into()),
                ..Default::default()
            },
        );
        let (_, _, v) = get(&format!("{}/o?offset=4&limit=2", m.base_url()), &[]);
        assert_eq!(v, json!({ "data": [{ "id": 5 }] }));

        m.serve(
            "/c",
            Endpoint {
                records: records(3),
                paging: Paging::Cursor {
                    token_param: "after".into(),
                    next_path: "meta.next".into(),
                    size_param: "limit".into(),
                },
                wrap: Some("data".into()),
                ..Default::default()
            },
        );
        let (_, _, v) = get(&format!("{}/c?limit=2", m.base_url()), &[]);
        assert_eq!(v["meta"]["next"], "tok-2");
        let (_, _, v) = get(&format!("{}/c?limit=2&after=tok-2", m.base_url()), &[]);
        assert_eq!(v["data"], json!([{ "id": 3 }]));
        assert!(v.get("meta").is_none());

        m.serve(
            "/l",
            Endpoint {
                records: records(3),
                paging: Paging::Link {
                    page_param: "page".into(),
                    size_param: "per".into(),
                },
                ..Default::default()
            },
        );
        let (_, head, _) = get(&format!("{}/l?per=2", m.base_url()), &[]);
        assert!(head.contains("page=2&per=2>; rel=\"next\""), "{head}");
        let (_, head, _) = get(&format!("{}/l?page=2&per=2", m.base_url()), &[]);
        assert!(!head.contains("rel=\"next\""), "{head}");
    }

    #[test]
    fn filter_header_and_throttle() {
        let m = RestMock::start();
        m.serve(
            "/e",
            Endpoint {
                records: (1..=12).map(|i| json!({ "n": i })).collect(),
                filter: Some(Filter {
                    param: "since".into(),
                    field: "n".into(),
                }),
                require_header: Some(("Authorization".into(), "Bearer t".into())),
                throttle_first: 1,
                ..Default::default()
            },
        );
        let url = format!("{}/e?since=9", m.base_url());
        assert_eq!(get(&url, &[]).0, 401);
        assert_eq!(get(&url, &[("Authorization", "Bearer t")]).0, 429);
        let (status, _, v) = get(&url, &[("Authorization", "Bearer t")]);
        assert_eq!(status, 200);
        assert_eq!(v, json!([{ "n": 10 }, { "n": 11 }, { "n": 12 }]));
        let seen = m.requests_to("/e");
        assert_eq!(seen.len(), 3);
        assert_eq!(seen[2].params["since"], "9");
    }

    #[test]
    fn a_step_configures_an_endpoint_and_reads_requests() {
        let m = RestMock::start();
        let step = |v: Value| v.as_object().unwrap().clone();
        m.run(&step(json!({ "endpoint": "/s", "records": [{ "id": 1 }] })))
            .unwrap();
        get(&format!("{}/s?x=1", m.base_url()), &[]);
        let rows = m.run(&step(json!({ "requests_to": "/s" }))).unwrap();
        assert_eq!(rows[0]["params"]["x"], "1");
        assert!(
            m.run(&step(json!({ "endpoint": "/t", "recrds": [] })))
                .is_err()
        );
    }
}
