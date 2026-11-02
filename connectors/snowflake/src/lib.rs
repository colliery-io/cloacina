//! The Snowflake connector (COLLIERY-I-0633 / CLOACINA-T-0986), moved from
//! weir: a **destination** and a **source** over the Snowflake SQL API.
//!
//! **Write:** ensure the table, then chunked multi-row `INSERT` (append) or
//! `MERGE` (upsert, idempotent on replay), and CDC deletes by key. **Read:** a
//! table or a configured SELECT runs as one statement; incremental mode wraps
//! it with a bound cursor lower bound. The result's array rows are zipped with
//! `resultSetMetaData.rowType` names into JSON object records (field names
//! lowercased), across every result partition. Statement text and numbered
//! bindings come from the pure builders in `sql.rs`. Auth is injected on the
//! host (a key-pair JWT and its token-type header): the guest sends plain
//! requests. A `202` polls the statement handle; 429 and 5xx retry with
//! backoff. Capability: `http`.
#![allow(clippy::all)]

use cloacina_constructor_contract::sync::cursor_cmp;
use fidius_macro::{WitType, plugin_impl, plugin_interface};
use sync_guest_types::*;

mod sql;
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

pub struct Snowflake {
    cfg: cloacina_constructor_contract::sync::Config,
}

impl Snowflake {
    fn configure(cfg: cloacina_constructor_contract::sync::Config) -> Self {
        Self { cfg }
    }
}

#[plugin_impl(Connector, crate = "fidius_guest", config = cloacina_constructor_contract::sync::Config)]
impl Connector for Snowflake {
    fn spec(&self) -> ConnectorSpec {
        ConnectorSpec {
            name: "snowflake".to_string(),
            connector_version: "0.1.0".to_string(),
            contract_version: 1,
            config_schema: "{\"type\":\"object\",\"required\":[\"account\",\"database\",\"schema\",\"warehouse\"],\"properties\":{\
                \"account\":{\"type\":\"string\",\"title\":\"Account identifier (org-account)\"},\
                \"database\":{\"type\":\"string\",\"title\":\"Database\"},\
                \"schema\":{\"type\":\"string\",\"title\":\"Schema\"},\
                \"warehouse\":{\"type\":\"string\",\"title\":\"Warehouse\"},\
                \"role\":{\"type\":\"string\",\"title\":\"Role (optional)\"},\
                \"table\":{\"type\":\"string\",\"title\":\"Table (blank = stream name)\"},\
                \"query\":{\"type\":\"string\",\"title\":\"Source SELECT (blank = SELECT * FROM table)\"},\
                \"batch_rows\":{\"type\":\"integer\",\"title\":\"Rows per write statement (default 500)\"}}}".to_string(),
                // Auth is a key-pair JWT the host injects; the guest declares
                // no auth config and never sees the private key.
            roles: vec![ConnectorRole::Source, ConnectorRole::Destination],
            supported_sync_modes: vec![SyncMode::FullRefresh, SyncMode::Incremental],
        }
    }

    /// Run `SELECT 1`: the config, the endpoint and the injected key are all
    /// exercised, for one trivial statement.
    fn check(&self, _config: Config) -> CheckResult {
        match parse_cfg(&self.cfg.json).and_then(|c| exec(&c, "SELECT 1", serde_json::Map::new())) {
            Ok(_) => CheckResult { success: true, message: None },
            Err(e) => CheckResult { success: false, message: Some(e) },
        }
    }

    fn discover(&self, _config: Config) -> DiscoverOutcome {
        // The stream is defined by config (table or query), so discover
        // advertises one configurable "records" stream.
        DiscoverOutcome::Catalog(Catalog {
            streams: vec![StreamInfo {
                name: "records".to_string(),
                namespace: None,
                schema: ArrowSchemaIpc { ipc: Vec::new() },
                supported_sync_modes: vec![SyncMode::FullRefresh, SyncMode::Incremental],
                source_defined_cursor: false,
                default_cursor_field: None,
                source_defined_primary_key: None,
                partitioning: PartitionScheme::Unpartitioned,
            }],
        })
    }

    fn read(&self, ctx: ReadContext) -> fidius_guest::Stream<ReadMessage> {
        let msgs: Vec<ReadMessage> = match fetch_records(&self.cfg, &ctx) {
            Ok((rows, cursor)) => vec![
                ReadMessage::Records(RecordBatch::Rows(rows)),
                ReadMessage::Checkpoint(StreamState { cursor, opaque: Vec::new() }),
            ],
            Err(e) => vec![ReadMessage::Fatal(ConnectorError::fatal(e))],
        };
        fidius_guest::Stream::from_iter(msgs)
    }

    fn write(&self, ctx: WriteContext, mut batches: fidius_guest::Stream<RecordBatch>) -> WriteOutcome {
        let c = match parse_cfg(&self.cfg.json) {
            Ok(c) => c,
            Err(e) => return write_err(ConnectorError::fatal(e)),
        };
        let table = c.table.clone().unwrap_or_else(|| ctx.stream.stream.clone());
        let fq = sql::qualified_table(&c.database, &c.schema, &table);
        let keys: Vec<String> = match &ctx.stream.write_mode {
            WriteMode::Upsert { business_keys } => business_keys.clone(),
            _ => Vec::new(),
        };
        let mut w = Writer {
            c,
            fq,
            keys,
            write_mode: ctx.stream.write_mode.clone(),
            cols: None,
            accepted: 0,
            dead_letters: Vec::new(),
        };
        while let Some(batch) = batches.next_item() {
            let res = match batch {
                RecordBatch::Rows(rows) => w.write_rows(&rows),
                RecordBatch::Changes(changes) => w.write_changes(&changes),
                RecordBatch::Arrow(_) => Err(
                    "snowflake-dest takes Rows/Changes (JSON text), not Arrow batches".to_string(),
                ),
            };
            // A statement failure is schema/config-level (it affects every row in the
            // chunk the same way) — fail the sync rather than silently dead-letter.
            if let Err(e) = res {
                return write_err(ConnectorError::fatal(e));
            }
        }
        WriteOutcome {
            state: StreamState { cursor: None, opaque: Vec::new() },
            diagnostics: Vec::new(),
            dead_letters: w.dead_letters,
            result: WriteResult::Ok(WriteReceipt { accepted: w.accepted }),
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

/// The connection config. The private key never reaches this guest: the host
/// signs the JWT.
struct SfCfg {
    account: String,
    database: String,
    schema: String,
    warehouse: String,
    role: Option<String>,
    table: Option<String>,
    /// The source SELECT. Blank or absent means `SELECT * FROM <table>`.
    query: Option<String>,
    /// API host override (PrivateLink deployments, tests). Blank/absent = the public
    /// `https://<account>.snowflakecomputing.com`.
    base_url: Option<String>,
    /// Rows per compiled statement (multi-row VALUES). Default 500.
    batch_rows: usize,
    max_retries: u32,
}

/// The SQL API base for this config (`<base>/api/v2/statements`).
fn api_base(c: &SfCfg) -> String {
    match &c.base_url {
        Some(b) => b.trim_end_matches('/').to_string(),
        None => format!("https://{}.snowflakecomputing.com", c.account.to_lowercase()),
    }
}

fn parse_cfg(json: &str) -> Result<SfCfg, String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("config json: {e}"))?;
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).map(String::from);
    let req = |k: &str| s(k).filter(|x| !x.is_empty()).ok_or_else(|| format!("config missing `{k}`"));
    Ok(SfCfg {
        account: req("account")?,
        database: req("database")?,
        schema: req("schema")?,
        warehouse: req("warehouse")?,
        role: s("role"),
        table: s("table").filter(|t| !t.is_empty()),
        query: s("query").filter(|q| !q.is_empty()),
        base_url: s("base_url").filter(|b| !b.is_empty()),
        batch_rows: v.get("batch_rows").and_then(|x| x.as_u64()).unwrap_or(500).max(1) as usize,
        max_retries: v.get("max_retries").and_then(|x| x.as_u64()).unwrap_or(4) as u32,
    })
}

/// Per-sync write state: the inferred column set (from the first record) and counters.
struct Writer {
    c: SfCfg,
    fq: String,
    keys: Vec<String>,
    write_mode: WriteMode,
    /// `Some` once the first record inferred the columns + the table was ensured
    /// (and truncated, for Overwrite).
    cols: Option<Vec<sql::Col>>,
    accepted: u64,
    dead_letters: Vec<DeadLetter>,
}

impl Writer {
    /// Parse rows (bad JSON dead-letters), ensure the table off the first record, then
    /// write chunks per the stream's write mode (INSERT for append, MERGE for upsert).
    fn write_rows(&mut self, rows: &[String]) -> Result<(), String> {
        let mut parsed: Vec<serde_json::Value> = Vec::with_capacity(rows.len());
        for row in rows {
            match serde_json::from_str::<serde_json::Value>(row) {
                Ok(v) if v.is_object() => parsed.push(v),
                Ok(_) => self.dead_letters.push(DeadLetter {
                    record: row.clone(),
                    reason: "record is not a JSON object".to_string(),
                }),
                Err(e) => self.dead_letters.push(DeadLetter {
                    record: row.clone(),
                    reason: format!("record json: {e}"),
                }),
            }
        }
        if parsed.is_empty() {
            return Ok(());
        }
        let cols = self.ensure(&parsed[0])?.to_vec();
        let upsert = !self.keys.is_empty() && matches!(self.write_mode, WriteMode::Upsert { .. });
        for chunk in parsed.chunks(self.c.batch_rows) {
            let refs: Vec<&serde_json::Value> = chunk.iter().collect();
            let n = if upsert {
                let deduped = sql::dedup_by_keys(&refs, &self.keys);
                let stmt = sql::merge_sql(&self.fq, &cols, &self.keys, deduped.len())?;
                let binds = sql::bindings(&deduped, &cols);
                let n = deduped.len();
                exec(&self.c, &stmt, binds)?;
                n
            } else {
                let stmt = sql::insert_sql(&self.fq, &cols, refs.len());
                let binds = sql::bindings(&refs, &cols);
                let n = refs.len();
                exec(&self.c, &stmt, binds)?;
                n
            };
            self.accepted += n as u64;
        }
        Ok(())
    }

    /// Apply a CDC change batch **in order**: consecutive Insert/Update runs MERGE,
    /// consecutive Delete runs DELETE-by-key. Requires upsert business keys.
    fn write_changes(&mut self, changes: &[ChangeRecord]) -> Result<(), String> {
        if self.keys.is_empty() {
            return Err("CDC changes require write_mode Upsert with business_keys".to_string());
        }
        let mut upserts: Vec<String> = Vec::new();
        let mut deletes: Vec<String> = Vec::new();
        for ch in changes {
            match ch.op {
                ChangeOp::Delete => {
                    if !upserts.is_empty() {
                        self.write_rows(&std::mem::take(&mut upserts))?;
                    }
                    deletes.push(ch.data.clone());
                }
                _ => {
                    if !deletes.is_empty() {
                        self.delete_rows(&std::mem::take(&mut deletes))?;
                    }
                    upserts.push(ch.data.clone());
                }
            }
        }
        if !upserts.is_empty() {
            self.write_rows(&upserts)?;
        }
        if !deletes.is_empty() {
            self.delete_rows(&deletes)?;
        }
        Ok(())
    }

    /// Delete by business-key tuple, chunked like writes.
    fn delete_rows(&mut self, rows: &[String]) -> Result<(), String> {
        let mut parsed: Vec<serde_json::Value> = Vec::new();
        for row in rows {
            match serde_json::from_str::<serde_json::Value>(row) {
                Ok(v) if v.is_object() => parsed.push(v),
                _ => self.dead_letters.push(DeadLetter {
                    record: row.clone(),
                    reason: "delete record is not a JSON object".to_string(),
                }),
            }
        }
        if parsed.is_empty() {
            return Ok(());
        }
        let cols = self.ensure(&parsed[0])?.to_vec();
        let key_cols: Vec<sql::Col> = self
            .keys
            .iter()
            .map(|k| {
                cols.iter()
                    .find(|c| c.name.eq_ignore_ascii_case(k))
                    .cloned()
                    .unwrap_or(sql::Col {
                        name: k.clone(),
                        sf_type: "VARCHAR",
                        bind_type: "TEXT",
                    })
            })
            .collect();
        for chunk in parsed.chunks(self.c.batch_rows) {
            let refs: Vec<&serde_json::Value> = chunk.iter().collect();
            let stmt = sql::delete_sql(&self.fq, &key_cols, refs.len());
            let binds = sql::bindings(&refs, &key_cols);
            exec(&self.c, &stmt, binds)?;
            self.accepted += refs.len() as u64;
        }
        Ok(())
    }

    /// Infer columns from the first record and ensure the table exists (once); an
    /// Overwrite stream truncates once right after the ensure.
    fn ensure(&mut self, first: &serde_json::Value) -> Result<&[sql::Col], String> {
        if self.cols.is_none() {
            let cols = sql::infer_cols(first);
            if cols.is_empty() {
                return Err("cannot infer columns from an empty record".to_string());
            }
            exec(
                &self.c,
                &sql::create_table_sql(&self.fq, &cols),
                serde_json::Map::new(),
            )?;
            if matches!(self.write_mode, WriteMode::Overwrite) {
                exec(
                    &self.c,
                    &format!("TRUNCATE TABLE IF EXISTS {}", self.fq),
                    serde_json::Map::new(),
                )?;
            }
            self.cols = Some(cols);
        }
        Ok(self.cols.as_deref().expect("just set"))
    }
}

/// Execute one statement against the SQL API: `POST /api/v2/statements?async=false`
/// with the session context + bindings; `200` = done (the settled result body is
/// returned — the source reads it, writes ignore it), `202` = poll the handle,
/// 429/5xx retry with backoff, anything else is the statement's error.
fn exec(
    c: &SfCfg,
    statement: &str,
    bindings: serde_json::Map<String, serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let url = format!("{}/api/v2/statements?async=false", api_base(c));
    let mut body = serde_json::json!({
        "statement": statement,
        "database": c.database.to_uppercase(),
        "schema": c.schema.to_uppercase(),
        "warehouse": c.warehouse.to_uppercase(),
    });
    if let Some(role) = &c.role {
        body["role"] = serde_json::Value::String(role.to_uppercase());
    }
    if !bindings.is_empty() {
        body["bindings"] = serde_json::Value::Object(bindings);
    }
    let resp = send_with_retry(c, &url, "POST", body.to_string().as_bytes())?;
    match resp.status {
        200..=201 => serde_json::from_str(&resp.text())
            .map_err(|e| format!("SQL API result not JSON: {e}")),
        202 => {
            let v: serde_json::Value =
                serde_json::from_str(&resp.text()).unwrap_or(serde_json::Value::Null);
            let handle = v
                .get("statementHandle")
                .and_then(|h| h.as_str())
                .ok_or("202 without a statementHandle")?
                .to_string();
            poll(c, &handle)
        }
        s => Err(format!(
            "HTTP {s} from the SQL API: {}",
            truncate(&resp.text(), 300)
        )),
    }
}

/// Poll an async statement handle until it settles, returning the settled result body
/// (bounded — a demo-scale statement that hasn't settled in ~2min is failed as
/// transient for the relay to retry).
fn poll(c: &SfCfg, handle: &str) -> Result<serde_json::Value, String> {
    let url = format!("{}/api/v2/statements/{handle}", api_base(c));
    for _ in 0..60 {
        let resp = send_with_retry(c, &url, "GET", &[])?;
        match resp.status {
            200..=201 => {
                return serde_json::from_str(&resp.text())
                    .map_err(|e| format!("SQL API result not JSON: {e}"));
            }
            202 => std::thread::sleep(std::time::Duration::from_millis(2000)),
            s => {
                return Err(format!(
                    "HTTP {s} polling statement {handle}: {}",
                    truncate(&resp.text(), 300)
                ))
            }
        }
    }
    Err(format!("statement {handle} did not settle in time"))
}

/// Read the configured table or query as one statement: incremental mode
/// wraps the base SELECT with a bound cursor lower-bound + ORDER BY; the result's array
/// rows (across **all** result partitions) are zipped into JSON object records; returns
/// the rows + the advanced cursor.
fn fetch_records(
    config: &cloacina_constructor_contract::sync::Config,
    ctx: &ReadContext,
) -> Result<(Vec<String>, Option<String>), String> {
    let c = parse_cfg(&config.json)?;
    let table = c.table.clone().unwrap_or_else(|| ctx.stream.stream.clone());
    let base = c.query.clone().unwrap_or_else(|| {
        format!(
            "SELECT * FROM {}",
            sql::qualified_table(&c.database, &c.schema, &table)
        )
    });
    let cursor_col = ctx.stream.cursor_field.clone().filter(|f| !f.is_empty());
    // The committed cursor is JSON text; the SQL API values it is compared
    // with are strings.
    let mut cursor: Option<String> = ctx.state.cursor.as_deref().map(|c| {
        match serde_json::from_str::<serde_json::Value>(c) {
            Ok(serde_json::Value::String(s)) => s,
            Ok(other) => other.to_string(),
            Err(_) => c.to_string(),
        }
    });
    let (stmt, binds) = match (&cursor_col, &cursor) {
        (Some(col), Some(cur)) => {
            let mut b = serde_json::Map::new();
            b.insert(
                "1".to_string(),
                serde_json::json!({ "type": "TEXT", "value": cur }),
            );
            (sql::incremental_select(&base, col), b)
        }
        (Some(col), None) => (sql::ordered_select(&base, col), serde_json::Map::new()),
        (None, _) => (base, serde_json::Map::new()),
    };
    let result = exec(&c, &stmt, binds)?;

    // Column names from the result metadata; rows from partition 0 (inline) + the rest.
    let names: Vec<String> = result["resultSetMetaData"]["rowType"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if names.is_empty() {
        return Err("SQL API result has no rowType metadata".to_string());
    }
    let mut arrays: Vec<serde_json::Value> =
        result["data"].as_array().cloned().unwrap_or_default();
    let partitions = result["resultSetMetaData"]["partitionInfo"]
        .as_array()
        .map(|p| p.len())
        .unwrap_or(1);
    if partitions > 1 {
        let handle = result["statementHandle"]
            .as_str()
            .ok_or("partitioned result without a statementHandle")?;
        for p in 1..partitions {
            let url = format!(
                "{}/api/v2/statements/{handle}?partition={p}",
                api_base(&c)
            );
            let resp = send_with_retry(&c, &url, "GET", &[])?;
            if !(200..=201).contains(&resp.status) {
                return Err(format!(
                    "HTTP {} fetching result partition {p}: {}",
                    resp.status,
                    truncate(&resp.text(), 300)
                ));
            }
            let body: serde_json::Value = serde_json::from_str(&resp.text())
                .map_err(|e| format!("partition {p} not JSON: {e}"))?;
            arrays.extend(body["data"].as_array().cloned().unwrap_or_default());
        }
    }
    let rows = sql::rows_to_objects(&names, &arrays);

    // Advance the cursor to the largest value seen. The values arrive as
    // strings, so compare numbers as numbers: `"9" > "12"` as text.
    if let Some(col) = &cursor_col {
        let key = col.to_lowercase();
        for row in &rows {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(row)
                && let Some(cv) = v.get(&key).and_then(|x| x.as_str())
                && cursor
                    .as_deref()
                    .is_none_or(|cur| cursor_cmp(cv, cur).is_gt())
            {
                cursor = Some(cv.to_string());
            }
        }
    }
    Ok((rows, cursor.map(|c| serde_json::Value::String(c).to_string())))
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

/// Send a request, retrying transient failures (429 / 5xx / transport) with backoff,
/// honoring `Retry-After`. Auth is injected on the host; the guest sends
/// `Content-Type: application/json` + a plain request.
fn send_with_retry(
    c: &SfCfg,
    url: &str,
    method: &str,
    body: &[u8],
) -> Result<fidius_guest::http::Response, String> {
    let mut attempt: u32 = 0;
    loop {
        let mut req = fidius_guest::http::Request::post(url, body.to_vec());
        req.method = method.to_uppercase();
        req = req.header("content-type", "application/json");
        req = req.header("accept", "application/json");
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
