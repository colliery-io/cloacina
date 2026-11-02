//! The MSSQL connector (COLLIERY-I-0633 / CLOACINA-T-0984), moved from weir: a
//! **sync** TDS client over `fidius_guest::sockets::tcp` (TDS is raw TCP, not
//! HTTP). The TDS codec and row typing are `tiberius` (the `tiberius-wasm` fork
//! in `vendor/`), driven without a runtime: its async client runs over a
//! poll-always-`Ready` adapter around the blocking socket, through
//! `futures::executor::block_on`. Reads use `FOR JSON PATH`, so the server
//! produces the JSON rows. A **source** only: FullRefresh, and Incremental by
//! `cursor_field`; no CDC. Config: `host`, `port`, `database`, `user`,
//! `password`, `table` (default: the stream name), `tls_mode`, `tls_ca_pem`,
//! `tls_insecure_skip_verify`. Capability: `tcp`.
#![allow(clippy::all)]

use std::io::{Read, Write};
use std::pin::Pin;
use std::task::{Context, Poll};

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

pub struct Mssql {
    cfg: cloacina_constructor_contract::sync::Config,
}

impl Mssql {
    fn configure(cfg: cloacina_constructor_contract::sync::Config) -> Self {
        Self { cfg }
    }

    /// The stream's table: config `table`, else the configured stream name.
    fn table(&self, stream: &str) -> String {
        serde_json::from_str::<serde_json::Value>(&self.cfg.json)
            .ok()
            .and_then(|v| v.get("table").and_then(|t| t.as_str()).map(str::to_string))
            .unwrap_or_else(|| stream.to_string())
    }

    fn read_inner(&self, ctx: &ReadContext) -> Result<(Vec<String>, Option<String>), String> {
        let conn = Conn::from_json(&self.cfg.json)?;
        let mut c = TdsConn::connect(&conn)?;
        let table = self.table(&ctx.stream.stream);
        match ctx.stream.sync_mode {
            SyncMode::FullRefresh => {
                let arr = c.query_json(&sql::full_select_json_sql(&table), None)?;
                Ok((arr.iter().map(|v| v.to_string()).collect(), None))
            }
            SyncMode::Incremental => {
                let cf = ctx
                    .stream
                    .cursor_field
                    .as_deref()
                    .ok_or("incremental requires cursor_field")?;
                // The cursor is JSON text; a string cursor binds unquoted.
                let cursor = ctx.state.cursor.clone();
                let bind = cursor.as_deref().map(|c| {
                    serde_json::from_str::<String>(c).unwrap_or_else(|_| c.to_string())
                });
                let stmt = sql::incremental_select_json_sql(&table, cf, bind.is_some());
                let arr = c.query_json(&stmt, bind.as_deref())?;
                let mut next = cursor;
                let mut rows = Vec::with_capacity(arr.len());
                for v in &arr {
                    // The rows arrive ordered by the cursor; the next cursor is the
                    // largest value seen, numbers compared as numbers.
                    if let Some(cv) = v.get(cf) {
                        let cs = cv.to_string();
                        if next.as_deref().map_or(true, |cur| cursor_cmp(&cs, cur).is_gt()) {
                            next = Some(cs);
                        }
                    }
                    rows.push(v.to_string());
                }
                Ok((rows, next))
            }
            SyncMode::Cdc => Err("the mssql source has no CDC".into()),
        }
    }
}

#[plugin_impl(Connector, crate = "fidius_guest", config = cloacina_constructor_contract::sync::Config)]
impl Connector for Mssql {
    fn spec(&self) -> ConnectorSpec {
        ConnectorSpec {
            name: "mssql".to_string(),
            connector_version: "0.1.0".to_string(),
            contract_version: 1,
            config_schema: "{\"type\":\"object\",\"required\":[\"host\",\"database\",\"user\",\"password\"],\"properties\":{\
                \"host\":{\"type\":\"string\",\"title\":\"Server host\"},\
                \"port\":{\"type\":\"integer\",\"title\":\"Port (default 1433)\"},\
                \"database\":{\"type\":\"string\",\"title\":\"Database\"},\
                \"user\":{\"type\":\"string\",\"title\":\"Login\"},\
                \"password\":{\"type\":\"string\",\"title\":\"Password\",\"format\":\"password\",\"airbyte_secret\":true},\
                \"table\":{\"type\":\"string\",\"title\":\"Table (blank = stream name)\"},\
                \"tls_mode\":{\"type\":\"string\",\"enum\":[\"disable\",\"prefer\",\"require\"],\"default\":\"require\",\
                \"title\":\"TLS: require (default) | prefer | disable\"},\
                \"tls_ca_pem\":{\"type\":\"string\",\"title\":\"Inline PEM CA bundle (default: public webpki roots)\"},\
                \"tls_insecure_skip_verify\":{\"type\":\"boolean\",\"default\":false,\
                \"title\":\"Encrypt WITHOUT certificate verification (dangerous)\"}}}".to_string(),
            roles: vec![ConnectorRole::Source],
            supported_sync_modes: vec![SyncMode::FullRefresh, SyncMode::Incremental],
        }
    }

    fn check(&self, _config: Config) -> CheckResult {
        match Conn::from_json(&self.cfg.json).and_then(|conn| TdsConn::connect(&conn).map(|_| ())) {
            Ok(()) => CheckResult { success: true, message: None },
            Err(e) => CheckResult { success: false, message: Some(e) },
        }
    }

    /// One stream per base table, named `schema.table`. Column types are known
    /// at read: `FOR JSON` yields typed scalars.
    fn discover(&self, _config: Config) -> DiscoverOutcome {
        let streams = (|| -> Result<Vec<StreamInfo>, String> {
            let conn = Conn::from_json(&self.cfg.json)?;
            let mut c = TdsConn::connect(&conn)?;
            let arr = c.query_json(sql::DISCOVER_TABLES_SQL, None)?;
            Ok(arr
                .iter()
                .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
                .map(|name| StreamInfo {
                    name: name.to_string(),
                    namespace: None,
                    schema: ArrowSchemaIpc { ipc: Vec::new() },
                    supported_sync_modes: vec![SyncMode::FullRefresh, SyncMode::Incremental],
                    source_defined_cursor: false,
                    default_cursor_field: None,
                    source_defined_primary_key: None,
                    partitioning: PartitionScheme::Unpartitioned,
                })
                .collect())
        })();
        match streams {
            Ok(streams) => DiscoverOutcome::Catalog(Catalog { streams }),
            Err(e) => DiscoverOutcome::Error(ConnectorError::transient(e)),
        }
    }

    fn read(&self, ctx: ReadContext) -> fidius_guest::Stream<ReadMessage> {
        let msgs: Vec<ReadMessage> = match self.read_inner(&ctx) {
            Ok((rows, cursor)) => vec![
                ReadMessage::Records(RecordBatch::Rows(rows)),
                ReadMessage::Checkpoint(StreamState { cursor, opaque: Vec::new() }),
            ],
            Err(e) => vec![ReadMessage::Fatal(ConnectorError::transient(e))],
        };
        fidius_guest::Stream::from_iter(msgs)
    }

    fn write(&self, _ctx: WriteContext, mut batches: fidius_guest::Stream<RecordBatch>) -> WriteOutcome {
        while batches.next_item().is_some() {}
        WriteOutcome {
            state: StreamState { cursor: None, opaque: Vec::new() },
            diagnostics: Vec::new(),
            dead_letters: Vec::new(),
            result: WriteResult::Err(ConnectorError::fatal("mssql is a source, not a destination")),
        }
    }
}

// ---- connection params and the sync TDS client (tiberius over sockets::tcp) ----

struct Conn {
    host: String,
    port: u16,
    database: String,
    user: String,
    password: String,
    /// `disable | prefer | require`; default `require`: managed SQL Server
    /// wants TLS. `disable` for a plaintext-only dev server.
    tls_mode: String,
    /// Inline PEM CA bundle; absent means the compiled-in webpki roots.
    tls_ca_pem: Option<String>,
    /// Encrypt without verifying the certificate. Dangerous; the driver logs it.
    tls_insecure_skip_verify: bool,
}

impl Conn {
    fn from_json(json: &str) -> Result<Self, String> {
        let v: serde_json::Value =
            serde_json::from_str(json).map_err(|e| format!("config json: {e}"))?;
        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).map(String::from);
        let req = |k: &str| s(k).filter(|x| !x.is_empty()).ok_or_else(|| format!("config missing `{k}`"));
        Ok(Conn {
            host: req("host")?,
            port: v.get("port").and_then(|x| x.as_u64()).unwrap_or(1433) as u16,
            database: req("database")?,
            user: req("user")?,
            password: req("password")?,
            tls_mode: s("tls_mode").unwrap_or_else(|| "require".to_string()),
            tls_ca_pem: s("tls_ca_pem").filter(|p| !p.is_empty()),
            tls_insecure_skip_verify: v
                .get("tls_insecure_skip_verify")
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
        })
    }
}

/// A blocking socket as a `futures` `AsyncRead`/`AsyncWrite`. Every poll makes
/// the blocking call and returns `Ready`, so `block_on` drives tiberius to
/// completion in one poll cycle with a no-op waker.
struct SyncSock(fidius_guest::sockets::tcp::TcpStream);

impl futures::io::AsyncRead for SyncSock {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(self.0.read(buf).map_err(|e| std::io::Error::other(e.to_string())))
    }
}

impl futures::io::AsyncWrite for SyncSock {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(
            self.0
                .write_all(buf)
                .map(|_| buf.len())
                .map_err(|e| std::io::Error::other(e.to_string())),
        )
    }
    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(self.0.flush().map_err(|e| std::io::Error::other(e.to_string())))
    }
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

struct TdsConn {
    client: tiberius::Client<SyncSock>,
}

impl TdsConn {
    fn connect(c: &Conn) -> Result<Self, String> {
        let sock = fidius_guest::sockets::tcp::connect(&c.host, c.port)
            .map_err(|e| format!("connect {}:{}: {e}", c.host, c.port))?;
        let mut cfg = tiberius::Config::new();
        cfg.host(&c.host);
        cfg.port(c.port);
        cfg.database(&c.database);
        cfg.authentication(tiberius::AuthMethod::sql_server(&c.user, &c.password));
        // The TDS 7.x TLS handshake runs inside PRELOGIN over the same socket.
        // Verification defaults to the webpki roots; `tls_ca_pem` supplies an
        // inline CA; `tls_insecure_skip_verify` encrypts without verifying.
        match c.tls_mode.as_str() {
            "disable" => {
                cfg.encryption(tiberius::EncryptionLevel::NotSupported);
                cfg.trust_cert();
            }
            mode @ ("require" | "prefer") => {
                cfg.encryption(if mode == "require" {
                    tiberius::EncryptionLevel::Required
                } else {
                    tiberius::EncryptionLevel::On
                });
                if c.tls_insecure_skip_verify {
                    cfg.trust_cert();
                } else if let Some(pem) = &c.tls_ca_pem {
                    cfg.trust_cert_pem(pem);
                }
            }
            other => {
                return Err(format!(
                    "unknown tls_mode `{other}` (expected disable | prefer | require)"
                ));
            }
        }
        let client = block_on(tiberius::Client::connect(cfg, SyncSock(sock)))
            .map_err(|e| format!("tds connect: {e}"))?;
        Ok(TdsConn { client })
    }

    /// Run `stmt`, with an optional text bind for `@P1`, and return the
    /// `FOR JSON PATH` result as a JSON array. SQL Server splits the JSON
    /// across string cells, so the cells are joined and then parsed; no rows
    /// means `[]`.
    fn query_json(
        &mut self,
        stmt: &str,
        bind: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let text = block_on(async {
            let stream = match bind {
                Some(p) => self.client.query(stmt, &[&p]).await,
                None => self.client.query(stmt, &[]).await,
            }?;
            let rows = stream.into_first_result().await?;
            let mut out = String::new();
            for row in &rows {
                if let Some(Some(cell)) = row.try_get::<&str, _>(0).ok() {
                    out.push_str(cell);
                }
            }
            Ok::<String, tiberius::error::Error>(out)
        })
        .map_err(|e| format!("tds query: {e}"))?;
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }
        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(serde_json::Value::Array(a)) => Ok(a),
            Ok(other) => Ok(vec![other]),
            Err(e) => Err(format!("FOR JSON result not JSON: {e}")),
        }
    }
}

/// Drive a future on the blocking socket: every poll of `SyncSock` is `Ready`.
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    futures::executor::block_on(fut)
}
