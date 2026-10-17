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

//! A mock Snowflake SQL API for the Snowflake connector's tests
//! (CLOACINA-T-0986, CLOACINA-A-0014).
//!
//! An in-memory Snowflake that understands the statements the connector's
//! builders emit: `SELECT 1`; `CREATE TABLE IF NOT EXISTS`; `TRUNCATE TABLE IF
//! EXISTS`; multi-row `INSERT`; `MERGE ... USING (SELECT COLUMNn AS ... FROM
//! VALUES ...)` by key; `DELETE ... USING (...)` by key; and `SELECT *` from a
//! table, optionally wrapped with `WHERE "<col>" > ?` and `ORDER BY "<col>"`.
//! Tables are keyed by their last name part; the database and schema are
//! ignored. Anything else answers `422`.
//!
//! Every request must carry the key-pair JWT the host injects: `Authorization:
//! Bearer <jwt>` and `X-Snowflake-Authorization-Token-Type: KEYPAIR_JWT`. The
//! mock checks the JWT's algorithm, its `sub`, its `iss` (with the public key
//! fingerprint, when the options name one), and its expiry. It does not check
//! the signature: the fingerprint ties the token to the key. A failed check
//! answers `401` and is kept in [`SnowflakeMock::rejected`].
//!
//! Answers follow the SQL API: values come back as strings, a result can span
//! partitions fetched with `GET /api/v2/statements/<handle>?partition=<n>`,
//! and with [`MockOptions::asynchronous`] each statement first answers `202`
//! with a handle the client polls.

use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use super::conformance::Service;

/// What the mock expects and how it answers.
#[derive(Debug, Clone)]
pub struct MockOptions {
    /// The account and user the JWT must name (compared uppercased).
    pub account: String,
    pub user: String,
    /// `SHA256:<base64>` of the public key the JWT must be issued for; `None`
    /// accepts any key.
    pub fingerprint: Option<String>,
    /// Answer each statement `202` first, then settle on the first poll.
    pub asynchronous: bool,
    /// Rows per result partition; 0 means one partition.
    pub partition_rows: usize,
}

impl MockOptions {
    pub fn new(account: &str, user: &str) -> Self {
        Self {
            account: account.to_string(),
            user: user.to_string(),
            fingerprint: None,
            asynchronous: false,
            partition_rows: 0,
        }
    }
}

#[derive(Debug, Default, Clone)]
struct Table {
    columns: Vec<String>,
    rows: Vec<Vec<Value>>,
}

#[derive(Default)]
struct State {
    tables: HashMap<String, Table>,
    statements: Vec<String>,
    rejected: Vec<String>,
    /// Settled results by handle, with their partitions.
    results: HashMap<String, Vec<Value>>,
    /// Handles answered `202` and not yet polled.
    pending: HashMap<String, Value>,
    next_handle: u64,
}

/// A running mock. It stops when the test process ends.
pub struct SnowflakeMock {
    port: u16,
    options: MockOptions,
    state: Arc<Mutex<State>>,
}

impl SnowflakeMock {
    /// Start the mock on a free local port.
    pub fn start(options: MockOptions) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind the mock");
        let port = listener.local_addr().unwrap().port();
        let state = Arc::new(Mutex::new(State::default()));
        let (st, opts) = (state.clone(), options.clone());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let (st, opts) = (st.clone(), opts.clone());
                std::thread::spawn(move || serve(stream, &st, &opts));
            }
        });
        Self {
            port,
            options,
            state,
        }
    }

    /// The `base_url` a connector config names.
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// The http grant for the mock.
    pub fn grant(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn options(&self) -> &MockOptions {
        &self.options
    }

    /// Replace a table with `columns` and `rows`.
    pub fn load_table(&self, name: &str, columns: &[&str], rows: Vec<Vec<Value>>) {
        self.state.lock().unwrap().tables.insert(
            table_key(name),
            Table {
                columns: columns.iter().map(|c| c.to_uppercase()).collect(),
                rows,
            },
        );
    }

    /// A table's rows as JSON objects with lowercased names, or `None`.
    pub fn rows(&self, name: &str) -> Option<Vec<Value>> {
        let st = self.state.lock().unwrap();
        st.tables.get(&table_key(name)).map(|t| {
            t.rows
                .iter()
                .map(|r| {
                    Value::Object(
                        t.columns
                            .iter()
                            .map(|c| c.to_lowercase())
                            .zip(r.iter().cloned())
                            .collect(),
                    )
                })
                .collect()
        })
    }

    /// Every statement executed, in order.
    pub fn statements(&self) -> Vec<String> {
        self.state.lock().unwrap().statements.clone()
    }

    /// Why each rejected request was rejected.
    pub fn rejected(&self) -> Vec<String> {
        self.state.lock().unwrap().rejected.clone()
    }
}

/// The conformance kit's steps against the mock: `table` with `columns` and
/// `data` (a list of rows) loads a table; `rows_of` returns a table's rows.
/// (`rows` is the kit's own field: the rows a step must return.)
impl Service for SnowflakeMock {
    fn run(&self, step: &Map<String, Value>) -> Result<Vec<Value>, String> {
        if let Some(name) = step.get("rows_of").and_then(Value::as_str) {
            return self
                .rows(name)
                .ok_or_else(|| format!("the mock has no table `{name}`"));
        }
        let name = step
            .get("table")
            .and_then(Value::as_str)
            .ok_or("a mock step needs `table` or `rows_of`")?;
        let columns: Vec<String> = step
            .get("columns")
            .and_then(Value::as_array)
            .ok_or("a `table` step needs `columns`")?
            .iter()
            .map(|c| c.as_str().unwrap_or_default().to_string())
            .collect();
        let rows: Vec<Vec<Value>> = step
            .get("data")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|r| r.as_array().cloned().unwrap_or_default())
            .collect();
        let cols: Vec<&str> = columns.iter().map(String::as_str).collect();
        self.load_table(name, &cols, rows);
        Ok(Vec::new())
    }
}

fn table_key(name: &str) -> String {
    name.rsplit('.')
        .next()
        .unwrap_or(name)
        .trim_matches('"')
        .to_uppercase()
}

// ---- HTTP ----

fn serve(mut stream: TcpStream, state: &Mutex<State>, opts: &MockOptions) {
    let Some((head, body)) = read_request(&mut stream) else {
        return;
    };
    let (status, answer) = answer(&head, &body, state, opts);
    let text = answer.to_string();
    let _ = stream.write_all(
        format!(
            "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{text}",
            text.len()
        )
        .as_bytes(),
    );
    let _ = stream.flush();
}

/// Read one request: the head, and the body by content-length or chunked.
fn read_request(stream: &mut TcpStream) -> Option<(String, String)> {
    stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
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
    let lower = head.to_ascii_lowercase();
    let mut raw = data[end + 4..].to_vec();
    if lower.contains("transfer-encoding: chunked") {
        while !raw.ends_with(b"0\r\n\r\n") {
            let n = stream.read(&mut buf).ok()?;
            if n == 0 {
                break;
            }
            raw.extend_from_slice(&buf[..n]);
        }
        return Some((head, dechunk(&String::from_utf8_lossy(&raw))));
    }
    let len = header(&head, "content-length")
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

fn header<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    head.lines().skip(1).find_map(|l| {
        let (k, v) = l.split_once(':')?;
        k.trim().eq_ignore_ascii_case(name).then(|| v.trim())
    })
}

fn answer(head: &str, body: &str, state: &Mutex<State>, opts: &MockOptions) -> (u16, Value) {
    if let Err(why) = check_jwt(head, opts) {
        state.lock().unwrap().rejected.push(why.clone());
        return (401, json!({ "code": "390144", "message": why }));
    }
    let target = head.split_whitespace().nth(1).unwrap_or("");
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if head.starts_with("GET ") {
        let handle = path.rsplit('/').next().unwrap_or("");
        let partition = query
            .split('&')
            .find_map(|kv| kv.strip_prefix("partition="))
            .and_then(|p| p.parse::<usize>().ok());
        let mut st = state.lock().unwrap();
        if let Some(settled) = st.pending.remove(handle) {
            return (200, settled);
        }
        return match (st.results.get(handle), partition) {
            (Some(parts), Some(p)) if p < parts.len() => (200, json!({ "data": parts[p] })),
            (Some(parts), None) => (200, json!({ "data": parts[0] })),
            _ => (
                404,
                json!({ "message": format!("no statement or partition for {target}") }),
            ),
        };
    }
    if !head.starts_with("POST ") || !path.ends_with("/api/v2/statements") {
        return (
            404,
            json!({ "message": format!("mock: no route {target}") }),
        );
    }
    let request: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return (400, json!({ "message": format!("body is not JSON: {e}") })),
    };
    let statement = request["statement"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let bindings = request["bindings"].as_object().cloned().unwrap_or_default();

    let mut st = state.lock().unwrap();
    st.statements.push(statement.clone());
    st.next_handle += 1;
    let handle = format!("h-{}", st.next_handle);
    let (columns, rows) = match execute(&mut st, &statement, &bindings) {
        Ok(r) => r,
        Err(e) => {
            return (
                422,
                json!({ "code": "002003", "message": e, "statementHandle": handle }),
            );
        }
    };
    // The SQL API answers every value as a string.
    let rows: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            Value::Array(
                r.into_iter()
                    .map(|v| match v {
                        Value::Null => Value::Null,
                        Value::String(s) => Value::String(s),
                        other => Value::String(other.to_string()),
                    })
                    .collect(),
            )
        })
        .collect();
    let parts: Vec<Value> = if opts.partition_rows == 0 || rows.is_empty() {
        vec![Value::Array(rows)]
    } else {
        rows.chunks(opts.partition_rows)
            .map(|c| Value::Array(c.to_vec()))
            .collect()
    };
    let result = json!({
        "statementHandle": handle,
        "message": "Statement executed successfully.",
        "resultSetMetaData": {
            "numRows": parts.iter().map(|p| p.as_array().map_or(0, Vec::len)).sum::<usize>(),
            "rowType": columns.iter().map(|c| json!({ "name": c })).collect::<Vec<_>>(),
            "partitionInfo": parts.iter().map(|_| json!({})).collect::<Vec<_>>(),
        },
        "data": parts[0],
    });
    st.results.insert(handle.clone(), parts);
    if opts.asynchronous {
        st.pending.insert(handle.clone(), result);
        return (
            202,
            json!({ "statementHandle": handle, "message": "Asynchronous execution in progress." }),
        );
    }
    (200, result)
}

// ---- the JWT ----

fn check_jwt(head: &str, opts: &MockOptions) -> Result<(), String> {
    let token_type = header(head, "x-snowflake-authorization-token-type")
        .ok_or("no X-Snowflake-Authorization-Token-Type header")?;
    if token_type != "KEYPAIR_JWT" {
        return Err(format!("token type is {token_type}, not KEYPAIR_JWT"));
    }
    let jwt = header(head, "authorization")
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or("no bearer token")?;
    let mut parts = jwt.split('.');
    let (Some(h), Some(p), Some(sig)) = (parts.next(), parts.next(), parts.next()) else {
        return Err("the bearer token is not a JWT".into());
    };
    if sig.is_empty() {
        return Err("the JWT has no signature".into());
    }
    let decode = |s: &str| -> Result<Value, String> {
        let bytes = base64url_decode(s).ok_or("a JWT part is not base64url")?;
        serde_json::from_slice(&bytes).map_err(|e| format!("a JWT part is not JSON: {e}"))
    };
    let (h, p) = (decode(h)?, decode(p)?);
    if h["alg"] != "RS256" {
        return Err(format!("JWT alg is {}, not RS256", h["alg"]));
    }
    let who = format!(
        "{}.{}",
        opts.account.to_uppercase(),
        opts.user.to_uppercase()
    );
    if p["sub"] != Value::String(who.clone()) {
        return Err(format!("JWT sub is {}, not {who}", p["sub"]));
    }
    let iss = p["iss"].as_str().unwrap_or_default();
    match &opts.fingerprint {
        Some(fp) if iss != format!("{who}.{fp}") => {
            return Err(format!("JWT iss {iss} is not for the key {fp}"));
        }
        None if !iss.starts_with(&format!("{who}.SHA256:")) => {
            return Err(format!("JWT iss {iss} names no key fingerprint"));
        }
        _ => {}
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if p["exp"].as_u64().is_none_or(|exp| exp <= now) {
        return Err("the JWT has expired".into());
    }
    Ok(())
}

fn base64url_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut acc = 0u32;
    let mut bits = 0u32;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'-' | b'+' => 62,
            b'_' | b'/' => 63,
            b'=' => break,
            _ => return None,
        } as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

// ---- the statements ----

type Answer = Result<(Vec<String>, Vec<Vec<Value>>), String>;

/// The quoted identifiers of `s`, unquoted, in order.
fn idents(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut id = String::new();
        while let Some(c) = chars.next() {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    id.push('"');
                    continue;
                }
                break;
            }
            id.push(c);
        }
        out.push(id);
    }
    out
}

/// The table named by the first fully qualified name after `keyword`.
fn table_after(statement: &str, keyword: &str) -> Option<String> {
    let at = statement.find(keyword)? + keyword.len();
    let rest = &statement[at..];
    let end = rest.find([' ', '(']).unwrap_or(rest.len());
    let fq = &rest[..end];
    idents(fq).pop()
}

/// Bindings `start..start+n` as typed values.
fn bound(bindings: &Map<String, Value>, index: usize) -> Value {
    let b = &bindings[&index.to_string()];
    let v = &b["value"];
    match (b["type"].as_str(), v) {
        (_, Value::Null) => Value::Null,
        (Some("FIXED"), Value::String(s)) => s
            .parse::<i64>()
            .map(Value::from)
            .unwrap_or_else(|_| Value::String(s.clone())),
        (Some("REAL"), Value::String(s)) => s
            .parse::<f64>()
            .map(Value::from)
            .unwrap_or_else(|_| Value::String(s.clone())),
        (Some("BOOLEAN"), Value::String(s)) => Value::Bool(s == "true"),
        (_, v) => v.clone(),
    }
}

/// Rows of `ncols` values from the bindings, row-major.
fn bound_rows(bindings: &Map<String, Value>, ncols: usize) -> Vec<Vec<Value>> {
    if ncols == 0 {
        return Vec::new();
    }
    let n = bindings.len() / ncols;
    (0..n)
        .map(|r| {
            (0..ncols)
                .map(|c| bound(bindings, r * ncols + c + 1))
                .collect()
        })
        .collect()
}

/// The `COLUMNn AS "X"` names of a `FROM VALUES` source.
fn values_columns(statement: &str) -> Vec<String> {
    let start = statement
        .find("USING (SELECT ")
        .map(|i| i + 14)
        .unwrap_or(0);
    let end = statement[start..]
        .find(" FROM VALUES")
        .map(|i| start + i)
        .unwrap_or(start);
    idents(&statement[start..end])
}

fn rows_affected(n: usize) -> Answer {
    Ok((
        vec!["number of rows affected".to_string()],
        vec![vec![Value::from(n as u64)]],
    ))
}

fn execute(st: &mut State, statement: &str, bindings: &Map<String, Value>) -> Answer {
    let s = statement.trim();
    if s.eq_ignore_ascii_case("SELECT 1") {
        return Ok((vec!["1".to_string()], vec![vec![Value::from(1)]]));
    }
    if s.starts_with("CREATE TABLE IF NOT EXISTS ") {
        let name = table_after(s, "CREATE TABLE IF NOT EXISTS ").ok_or("no table name")?;
        let open = s.find('(').ok_or("no column list")?;
        let columns: Vec<String> = s[open..]
            .split(',')
            .filter_map(|def| idents(def).into_iter().next())
            .collect();
        st.tables.entry(name).or_insert(Table {
            columns,
            rows: Vec::new(),
        });
        return rows_affected(0);
    }
    if s.starts_with("TRUNCATE TABLE IF EXISTS ") {
        let name = table_after(s, "TRUNCATE TABLE IF EXISTS ").ok_or("no table name")?;
        if let Some(t) = st.tables.get_mut(&name) {
            t.rows.clear();
        }
        return rows_affected(0);
    }
    if s.starts_with("INSERT INTO ") {
        let name = table_after(s, "INSERT INTO ").ok_or("no table name")?;
        let open = s.find(" (").ok_or("no column list")? + 2;
        let close = s[open..].find(')').ok_or("no column list")? + open;
        let cols = idents(&s[open..close]);
        let rows = bound_rows(bindings, cols.len());
        let t = st
            .tables
            .get_mut(&name)
            .ok_or_else(|| format!("Table '{name}' does not exist"))?;
        let n = rows.len();
        for r in rows {
            t.rows.push(place(&t.columns, &cols, r));
        }
        return rows_affected(n);
    }
    if s.starts_with("MERGE INTO ") {
        let name = table_after(s, "MERGE INTO ").ok_or("no table name")?;
        let cols = values_columns(s);
        let on_start = s.find(") AS s ON ").ok_or("no ON clause")? + 10;
        let on_end = s[on_start..]
            .find(" WHEN ")
            .map(|i| on_start + i)
            .unwrap_or(s.len());
        let keys: Vec<String> = idents(&s[on_start..on_end])
            .into_iter()
            .step_by(2)
            .collect();
        let rows = bound_rows(bindings, cols.len());
        let t = st
            .tables
            .get_mut(&name)
            .ok_or_else(|| format!("Table '{name}' does not exist"))?;
        let n = rows.len();
        for r in rows {
            let row = place(&t.columns, &cols, r);
            match t
                .rows
                .iter()
                .position(|old| same_keys(&t.columns, &keys, old, &row))
            {
                Some(i) => t.rows[i] = row,
                None => t.rows.push(row),
            }
        }
        return rows_affected(n);
    }
    if s.starts_with("DELETE FROM ") {
        let name = table_after(s, "DELETE FROM ").ok_or("no table name")?;
        let keys = values_columns(s);
        let rows = bound_rows(bindings, keys.len());
        let t = st
            .tables
            .get_mut(&name)
            .ok_or_else(|| format!("Table '{name}' does not exist"))?;
        let before = t.rows.len();
        for r in rows {
            let probe = place(&t.columns, &keys, r);
            t.rows
                .retain(|old| !same_keys(&t.columns, &keys, old, &probe));
        }
        return rows_affected(before - t.rows.len());
    }
    if s.starts_with("SELECT * FROM ") {
        // `SELECT * FROM <fq>`, or wrapped: `SELECT * FROM (SELECT * FROM <fq>)
        // [WHERE "C" > ?] [ORDER BY "C"]`.
        let inner = s.rfind("SELECT * FROM ").unwrap();
        let name = table_after(&s[inner..], "SELECT * FROM ").ok_or("no table name")?;
        let t = st
            .tables
            .get(&name)
            .ok_or_else(|| format!("Table '{name}' does not exist"))?
            .clone();
        let idx = |col: &str| t.columns.iter().position(|c| c.eq_ignore_ascii_case(col));
        let mut rows = t.rows.clone();
        if let Some(w) = s.find(" WHERE ") {
            let col = idents(&s[w..])
                .into_iter()
                .next()
                .ok_or("no WHERE column")?;
            let i = idx(&col).ok_or_else(|| format!("invalid identifier '{col}'"))?;
            let bound = text(
                &bindings
                    .get("1")
                    .map(|b| b["value"].clone())
                    .unwrap_or(Value::Null),
            );
            rows.retain(|r| cmp_text(&text(&r[i]), &bound).is_gt());
        }
        if let Some(o) = s.find(" ORDER BY ") {
            let col = idents(&s[o..])
                .into_iter()
                .next()
                .ok_or("no ORDER BY column")?;
            let i = idx(&col).ok_or_else(|| format!("invalid identifier '{col}'"))?;
            rows.sort_by(|a, b| cmp_text(&text(&a[i]), &text(&b[i])));
        }
        return Ok((t.columns.clone(), rows));
    }
    Err(format!("mock: unsupported statement: {s}"))
}

/// A row in `columns` order from values given for `given` columns.
fn place(columns: &[String], given: &[String], values: Vec<Value>) -> Vec<Value> {
    let by: BTreeMap<String, Value> = given.iter().map(|g| g.to_uppercase()).zip(values).collect();
    columns
        .iter()
        .map(|c| by.get(&c.to_uppercase()).cloned().unwrap_or(Value::Null))
        .collect()
}

fn same_keys(columns: &[String], keys: &[String], a: &[Value], b: &[Value]) -> bool {
    keys.iter().all(|k| {
        columns
            .iter()
            .position(|c| c.eq_ignore_ascii_case(k))
            .is_some_and(|i| text(&a[i]) == text(&b[i]))
    })
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Numbers compare as numbers, anything else as text.
fn cmp_text(a: &str, b: &str) -> std::cmp::Ordering {
    match (a.parse::<f64>(), b.parse::<f64>()) {
        (Ok(x), Ok(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        _ => a.cmp(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exec(st: &mut State, s: &str, b: Value) -> Answer {
        execute(st, s, b.as_object().unwrap())
    }

    #[test]
    fn the_builders_statements_round_trip() {
        let mut st = State::default();
        exec(
            &mut st,
            r#"CREATE TABLE IF NOT EXISTS "D"."S"."T" ("ID" NUMBER, "NAME" VARCHAR)"#,
            json!({}),
        )
        .unwrap();
        exec(
            &mut st,
            r#"INSERT INTO "D"."S"."T" ("ID", "NAME") VALUES (?, ?), (?, ?)"#,
            json!({"1": {"type": "FIXED", "value": "1"}, "2": {"type": "TEXT", "value": "a"},
                   "3": {"type": "FIXED", "value": "2"}, "4": {"type": "TEXT", "value": "b"}}),
        )
        .unwrap();
        exec(
            &mut st,
            r#"MERGE INTO "D"."S"."T" AS t USING (SELECT COLUMN1 AS "ID", COLUMN2 AS "NAME" FROM VALUES (?, ?), (?, ?)) AS s ON t."ID" = s."ID" WHEN MATCHED THEN UPDATE SET t."NAME" = s."NAME" WHEN NOT MATCHED THEN INSERT ("ID", "NAME") VALUES (s."ID", s."NAME")"#,
            json!({"1": {"type": "FIXED", "value": "2"}, "2": {"type": "TEXT", "value": "b2"},
                   "3": {"type": "FIXED", "value": "3"}, "4": {"type": "TEXT", "value": "c"}}),
        )
        .unwrap();
        exec(
            &mut st,
            r#"DELETE FROM "D"."S"."T" USING (SELECT COLUMN1 AS "ID" FROM VALUES (?)) AS s WHERE "D"."S"."T"."ID" = s."ID""#,
            json!({"1": {"type": "FIXED", "value": "1"}}),
        )
        .unwrap();
        let (cols, rows) = exec(
            &mut st,
            r#"SELECT * FROM (SELECT * FROM "D"."S"."T") WHERE "ID" > ? ORDER BY "ID""#,
            json!({"1": {"type": "TEXT", "value": "2"}}),
        )
        .unwrap();
        assert_eq!(cols, vec!["ID", "NAME"]);
        assert_eq!(rows, vec![vec![json!(3), json!("c")]]);
        let (_, all) = exec(&mut st, r#"SELECT * FROM "D"."S"."T""#, json!({})).unwrap();
        assert_eq!(all.len(), 2, "{all:?}");
        assert!(exec(&mut st, "DROP TABLE x", json!({})).is_err());
    }

    #[test]
    fn numbers_order_as_numbers() {
        assert!(cmp_text("10", "9").is_gt());
        assert!(cmp_text("b", "a").is_gt());
    }

    #[test]
    fn base64url_decodes_without_padding() {
        assert_eq!(base64url_decode("eyJhIjoxfQ").unwrap(), br#"{"a":1}"#);
    }
}
