//! The Postgres connector (COLLIERY-I-0632 / CLOACINA-T-0979), moved from weir:
//! a **sync** Postgres client over `fidius_guest::sockets::tcp` and
//! `postgres-protocol` (no tokio, no libpq). The **source** honours `SyncMode`
//! (FullRefresh, Incremental by `cursor_field`, Cdc through
//! `pg_logical_slot_get_changes`) and key-shard partitions; the **destination**
//! honours `WriteMode` (Append, Overwrite, Upsert). Config: `url`
//! (postgres://user:pw@host:port/db) or `host`/`port`/`user`/`password`/`dbname`,
//! plus `table` (default: the stream name). Capability: `tcp`.
#![allow(clippy::all)]

use std::io::{Read, Write};

use fidius_macro::{plugin_impl, plugin_interface, WitType};
use sync_guest_types::*;

mod cdc;
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

pub struct Postgres { cfg: cloacina_constructor_contract::sync::Config }

#[plugin_impl(Connector, crate = "fidius_guest", config = cloacina_constructor_contract::sync::Config)]
impl Connector for Postgres {
    fn spec(&self) -> ConnectorSpec {
        ConnectorSpec {
            name: "postgres".to_string(),
            connector_version: "0.1.0".to_string(),
            contract_version: 1,
            config_schema: "{\"type\":\"object\",\"properties\":{\
                \"url\":{\"type\":\"string\",\"airbyte_secret\":true,\"description\":\"Connection URL (may embed the password)\"},\"host\":{\"type\":\"string\"},\"port\":{\"type\":\"integer\"},\
                \"user\":{\"type\":\"string\"},\"password\":{\"type\":\"string\",\"format\":\"password\",\"airbyte_secret\":true},\
                \"dbname\":{\"type\":\"string\"},\"table\":{\"type\":\"string\"},\
                \"sslmode\":{\"type\":\"string\",\"enum\":[\"disable\",\"require\",\"verify-full\"],\"default\":\"require\",\
                \"description\":\"TLS: require (encrypt, default) | verify-full (encrypt + verify chain/hostname) | disable\"},\
                \"sslrootcert\":{\"type\":\"string\",\"description\":\"Inline PEM CA bundle for verify-full (defaults to public webpki roots)\"},\
                \"schema\":{\"type\":\"string\",\"default\":\"public\",\"description\":\"Schema discover() introspects\"},\
                \"typed_columns\":{\"type\":\"boolean\",\"default\":true,\
                \"description\":\"Destination lands typed relational columns inferred from the records (false = legacy single data JSONB column)\"}}}".to_string(),
            roles: vec![ConnectorRole::Source, ConnectorRole::Destination],
            supported_sync_modes: vec![SyncMode::FullRefresh, SyncMode::Incremental, SyncMode::Cdc],
        }
    }

    fn check(&self, _config: Config) -> CheckResult {
        match PgConn::connect(&Conn::from_json(&self.cfg.json)) {
            Ok(mut c) => match c.query_rows("SELECT 1") {
                Ok(_) => CheckResult { success: true, message: None },
                Err(e) => CheckResult { success: false, message: Some(e) },
            },
            Err(e) => CheckResult { success: false, message: Some(e) },
        }
    }

    /// Real table introspection (, porting the mssql pattern): one
    /// stream per base table in the configured schema (config `schema`, default
    /// `public`), with source-defined primary keys. Column TYPES are not carried
    /// here — the platform's typed schemas are captured at sync
    /// via `StreamSchema::infer`, and nothing consumes discover's IPC seam.
    /// A connection failure is an honest `DiscoverOutcome::Error`, not an empty
    /// catalog; an empty schema is an empty catalog.
    fn discover(&self, _config: Config) -> DiscoverOutcome {
        let conn = Conn::from_json(&self.cfg.json);
        let result = (|| -> Result<Vec<StreamInfo>, String> {
            let mut c = PgConn::connect(&conn)?;
            let sql = format!(
                "SELECT t.table_name, coalesce(pk.cols, '') \
                 FROM information_schema.tables t \
                 LEFT JOIN (SELECT tc.table_schema, tc.table_name, \
                            string_agg(kcu.column_name, ',' ORDER BY kcu.ordinal_position) AS cols \
                            FROM information_schema.table_constraints tc \
                            JOIN information_schema.key_column_usage kcu \
                              ON kcu.constraint_name = tc.constraint_name \
                             AND kcu.table_schema = tc.table_schema \
                             AND kcu.table_name = tc.table_name \
                            WHERE tc.constraint_type = 'PRIMARY KEY' \
                            GROUP BY tc.table_schema, tc.table_name) pk \
                   ON pk.table_schema = t.table_schema AND pk.table_name = t.table_name \
                 WHERE t.table_type = 'BASE TABLE' AND t.table_schema = {} \
                 ORDER BY t.table_name",
                lit(&conn.schema)
            );
            let rows = c.query_rows(&sql)?;
            Ok(rows
                .into_iter()
                .filter_map(|mut r| {
                    let name = r.first().cloned().flatten()?;
                    let pks: Vec<String> = r
                        .get_mut(1)
                        .and_then(|c| c.take())
                        .map(|s| s.split(',').filter(|p| !p.is_empty()).map(str::to_string).collect())
                        .unwrap_or_default();
                    Some(StreamInfo {
                        name,
                        namespace: Some(conn.schema.clone()),
                        schema: ArrowSchemaIpc { ipc: Vec::new() },
                        supported_sync_modes: vec![
                            SyncMode::FullRefresh,
                            SyncMode::Incremental,
                            SyncMode::Cdc,
                        ],
                        source_defined_cursor: false,
                        default_cursor_field: None,
                        source_defined_primary_key: if pks.is_empty() { None } else { Some(pks) },
                        partitioning: PartitionScheme::Unpartitioned,
                    })
                })
                .collect())
        })();
        match result {
            Ok(streams) => DiscoverOutcome::Catalog(Catalog { streams }),
            Err(e) => DiscoverOutcome::Error(ConnectorError::transient(e)),
        }
    }

    fn read(&self, ctx: ReadContext) -> fidius_guest::Stream<ReadMessage> {
        // CDC emits structured Changes (op + row); other modes emit Rows.
        let msgs = if ctx.stream.sync_mode == SyncMode::Cdc {
            match self.read_cdc(&ctx) {
                Ok((changes, cursor, opaque)) => vec![
                    ReadMessage::Records(RecordBatch::Changes(changes)),
                    ReadMessage::Checkpoint(StreamState { cursor, opaque }),
                ],
                Err(e) => vec![ReadMessage::Fatal(ConnectorError::transient(e))],
            }
        } else {
            match self.read_inner(&ctx) {
                Ok((rows, cursor, opaque)) => vec![
                    ReadMessage::Records(RecordBatch::Rows(rows)),
                    ReadMessage::Checkpoint(StreamState { cursor, opaque }),
                ],
                Err(e) => vec![ReadMessage::Fatal(ConnectorError::transient(e))],
            }
        };
        fidius_guest::Stream::from_iter(msgs)
    }

    fn write(&self, ctx: WriteContext, mut batches: fidius_guest::Stream<RecordBatch>) -> WriteOutcome {
        let mut rows: Vec<String> = Vec::new();
        let mut changes: Vec<ChangeRecord> = Vec::new();
        while let Some(batch) = batches.next_item() {
            match batch {
                RecordBatch::Rows(mut r) => rows.append(&mut r),
                RecordBatch::Changes(mut c) => changes.append(&mut c),
                RecordBatch::Arrow(_) => {
                    return write_err(ConnectorError::fatal(
                        "postgres dest takes Rows/Changes (JSON text), not Arrow batches",
                    ));
                }
            }
        }
        // CDC changes apply op-aware (upsert / delete / tombstone); plain Rows use
        // the write-mode path (Append / Upsert / Overwrite).
        let result = if changes.is_empty() {
            self.write_inner(&ctx.stream, &rows)
        } else {
            self.apply_changes(&ctx.stream, &changes)
        };
        match result {
            Ok((accepted, dead_letters)) => WriteOutcome {
                state: StreamState { cursor: None, opaque: Vec::new() },
                diagnostics: Vec::new(),
                dead_letters,
                result: WriteResult::Ok(WriteReceipt { accepted }),
            },
            Err(e) => write_err(ConnectorError::transient(e)),
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

impl Postgres {
    fn configure(cfg: cloacina_constructor_contract::sync::Config) -> Self { Self { cfg } }

    fn table(&self, stream: &str) -> String {
        serde_json::from_str::<serde_json::Value>(&self.cfg.json)
            .ok()
            .and_then(|v| v.get("table").and_then(|t| t.as_str()).map(str::to_string))
            .unwrap_or_else(|| stream.to_string())
    }

    /// CDC source: returns `(changes, next cursor = last LSN, opaque = slot name)`.
    fn read_cdc(
        &self,
        ctx: &ReadContext,
    ) -> Result<(Vec<ChangeRecord>, Option<String>, Vec<u8>), String> {
        let conn = Conn::from_json(&self.cfg.json);
        let mut c = PgConn::connect(&conn)?;
        let slot = if ctx.state.opaque.is_empty() {
            cdc_slot_name(&ctx.stream.stream)
        } else {
            String::from_utf8_lossy(&ctx.state.opaque).into_owned()
        };
        let (changes, last_lsn) = cdc_read(&mut c, &slot, &self.table(&ctx.stream.stream))?;
        Ok((changes, last_lsn.or_else(|| ctx.state.cursor.clone()), slot.into_bytes()))
    }

    /// Source (FullRefresh / Incremental): returns `(rows-as-json-text, next cursor, opaque)`.
    fn read_inner(&self, ctx: &ReadContext) -> Result<(Vec<String>, Option<String>, Vec<u8>), String> {
        let conn = Conn::from_json(&self.cfg.json);
        let mut c = PgConn::connect(&conn)?;
        let table = self.table(&ctx.stream.stream);
        let t = quote_ident(&table);

        let shard = shard_predicate(ctx.partition.bounds.as_deref())?;
        match ctx.stream.sync_mode {
            SyncMode::FullRefresh => {
                let where_clause = shard.map(|s| format!(" WHERE {s}")).unwrap_or_default();
                let sql = format!("SELECT row_to_json(t)::text FROM {t} t{where_clause}");
                let cols = c.query_rows(&sql)?;
                Ok((cols.into_iter().map(|mut r| r.remove(0).unwrap_or_default()).collect(), None, Vec::new()))
            }
            SyncMode::Incremental => {
                let cf = ctx.stream.cursor_field.as_deref().ok_or("incremental requires cursor_field")?;
                let cq = quote_ident(cf);
                let mut conds: Vec<String> = Vec::new();
                if let Some(s) = &shard {
                    conds.push(s.clone());
                }
                let cursor = ctx.state.cursor.clone();
                if let Some(cur) = &cursor {
                    // UNTYPED literal on purpose: Postgres casts it to the
                    // column's native type, so bigint/timestamp cursors compare
                    // numerically/temporally — `::text` here compared '2' > '12'
                    // lexicographically and re-delivered rows on typed columns
                    // ( exposed it; text columns are unchanged).
                    conds.push(format!("t.{cq} > {}", lit(cur)));
                }
                let where_clause = if conds.is_empty() {
                    String::new()
                } else {
                    format!(" WHERE {}", conds.join(" AND "))
                };
                let sql = format!(
                    "SELECT row_to_json(t)::text, t.{cq}::text FROM {t} t{where_clause} ORDER BY t.{cq}"
                );
                let cols = c.query_rows(&sql)?;
                let mut next = cursor;
                let mut rows = Vec::with_capacity(cols.len());
                for mut r in cols {
                    let row = r.remove(0).unwrap_or_default();
                    if let Some(cur) = r.into_iter().next().flatten() {
                        next = Some(cur); // ASC → last wins
                    }
                    rows.push(row);
                }
                Ok((rows, next, Vec::new()))
            }
            SyncMode::Cdc => unreachable!(),
        }
    }

    /// Destination: write `rows` per the stream's `WriteMode`. Returns `(accepted,
    /// dead_letters)`.
    ///
    /// The whole write runs in **one transaction** (one fsync at COMMIT, not one per row)
    /// as **batched multi-row INSERTs**. Each chunk is **bisected**: if it fails (a bad
    /// row — constraint violation, bad json), the batch is binary-split and retried in
    /// SAVEPOINTs down to the single offending rows, which are routed to the **dead-letter
    /// queue** while every good row still commits. Happy path: one INSERT per chunk, no
    /// splitting. (`COPY` is the next step for the bulk path's raw throughput.)
    /// `(tombstone?, tombstone_column)` from config — `on_delete = hard|tombstone`.
    fn delete_config(&self) -> (bool, String) {
        let v: serde_json::Value =
            serde_json::from_str(&self.cfg.json).unwrap_or(serde_json::Value::Null);
        let tombstone = v.get("on_delete").and_then(|x| x.as_str()) == Some("tombstone");
        let col = v
            .get("tombstone_column")
            .and_then(|x| x.as_str())
            .unwrap_or("_deleted_at")
            .to_string();
        (tombstone, col)
    }

    /// Apply a CDC change batch **in order**: Insert/Update → upsert by key;
    /// Delete → hard `DELETE` or soft tombstone, per config. Requires `WriteMode::Upsert` keys.
    fn apply_changes(
        &self,
        stream: &ConfiguredStream,
        changes: &[ChangeRecord],
    ) -> Result<(u64, Vec<DeadLetter>), String> {
        let business_keys = match &stream.write_mode {
            WriteMode::Upsert { business_keys } if !business_keys.is_empty() => business_keys.clone(),
            _ => return Err("CDC changes require write_mode Upsert with business_keys".to_string()),
        };
        let (tombstone, tomb_col) = self.delete_config();
        let conn = Conn::from_json(&self.cfg.json);
        let mut c = PgConn::connect(&conn)?;
        let t = quote_ident(&self.table(&stream.stream));
        let keycols: Vec<String> = business_keys.iter().map(|k| quote_ident(k)).collect();
        let coldefs = keycols.iter().map(|c| format!("{c} TEXT NOT NULL")).collect::<Vec<_>>().join(", ");
        let pk = keycols.join(", ");
        let mut dead: Vec<DeadLetter> = Vec::new();
        let mut accepted = 0u64;

        let typed = typed_enabled(&self.cfg.json);
        let txn = (|| -> Result<(), String> {
            c.query_rows("BEGIN")?;
            // Typed CDC: infer columns from the batch's
            // Insert/Update rows; deletes only need the key columns.
            let typed_cols = if typed {
                let data_rows: Vec<String> = changes
                    .iter()
                    .filter(|ch| matches!(ch.op, ChangeOp::Insert | ChangeOp::Update))
                    .map(|ch| ch.data.clone())
                    .collect();
                let fields = typed_fields(&data_rows, &business_keys);
                ensure_typed_table(
                    &mut c,
                    &t,
                    &fields,
                    &business_keys,
                    tombstone.then_some(tomb_col.as_str()),
                )?;
                Some(fields)
            } else {
                let tomb_def = if tombstone {
                    format!(", {} TEXT", quote_ident(&tomb_col))
                } else {
                    String::new()
                };
                c.query_rows(&format!(
                    "CREATE TABLE IF NOT EXISTS {t} ({coldefs}, data JSONB{tomb_def}, PRIMARY KEY ({pk}))"
                ))?;
                if tombstone {
                    c.query_rows(&format!(
                        "ALTER TABLE {t} ADD COLUMN IF NOT EXISTS {} TEXT",
                        quote_ident(&tomb_col)
                    ))?;
                }
                None
            };
            for ch in changes {
                let v: serde_json::Value = match serde_json::from_str(&ch.data) {
                    Ok(v) => v,
                    Err(e) => {
                        dead.push(DeadLetter { record: ch.data.clone(), reason: format!("row json: {e}") });
                        continue;
                    }
                };
                let key_lits = business_keys
                    .iter()
                    .map(|k| lit(&key_text(&v, k)))
                    .collect::<Vec<_>>()
                    .join(", ");
                let where_clause = keycols
                    .iter()
                    .zip(business_keys.iter())
                    .map(|(kc, k)| format!("{kc} = {}", lit(&key_text(&v, k))))
                    .collect::<Vec<_>>()
                    .join(" AND ");
                let sql = match (&ch.op, &typed_cols) {
                    (ChangeOp::Insert | ChangeOp::Update, Some(fields)) => {
                        match typed_tuple(&ch.data, fields) {
                            Ok(tup) => typed_insert_sql(&t, fields, &business_keys, &tup),
                            Err(e) => {
                                dead.push(DeadLetter { record: ch.data.clone(), reason: e });
                                continue;
                            }
                        }
                    }
                    (ChangeOp::Insert | ChangeOp::Update, None) => {
                        cdc::pg_cdc::upsert(&t, &pk, &key_lits, &lit(&ch.data))
                    }
                    (ChangeOp::Delete, _) if tombstone => {
                        cdc::pg_cdc::tombstone(&t, &quote_ident(&tomb_col), &where_clause)
                    }
                    (ChangeOp::Delete, _) => cdc::pg_cdc::delete(&t, &where_clause),
                };
                match c.query_rows(&sql) {
                    Ok(_) => accepted += 1,
                    Err(e) => dead.push(DeadLetter { record: ch.data.clone(), reason: e }),
                }
            }
            c.query_rows("COMMIT")?;
            Ok(())
        })();

        if let Err(e) = txn {
            let _ = c.query_rows("ROLLBACK");
            return Err(e);
        }
        Ok((accepted, dead))
    }

    fn write_inner(
        &self,
        stream: &ConfiguredStream,
        rows: &[String],
    ) -> Result<(u64, Vec<DeadLetter>), String> {
        /// Rows per INSERT statement — bounds statement size while killing round-trips.
        const BATCH: usize = 500;
        let conn = Conn::from_json(&self.cfg.json);
        let mut c = PgConn::connect(&conn)?;
        let t = quote_ident(&self.table(&stream.stream));
        let mut dead: Vec<DeadLetter> = Vec::new();

        let typed = typed_enabled(&self.cfg.json);
        let txn = (|| -> Result<(), String> {
            c.query_rows("BEGIN")?;
            match &stream.write_mode {
                WriteMode::Append | WriteMode::Overwrite if typed => {
                    // Typed columns: infer once over the whole
                    // write so every chunk shares one column set.
                    let fields = typed_fields(rows, &[]);
                    if !fields.is_empty() {
                        ensure_typed_table(&mut c, &t, &fields, &[], None)?;
                        if matches!(stream.write_mode, WriteMode::Overwrite) {
                            c.query_rows(&format!("TRUNCATE {t}"))?;
                        }
                        let attempt =
                            |c: &mut PgConn, sub: &[String]| -> Result<Result<(), String>, String> {
                                let mut tuples = Vec::with_capacity(sub.len());
                                for r in sub {
                                    match typed_tuple(r, &fields) {
                                        Ok(tup) => tuples.push(tup),
                                        Err(e) => return Ok(Err(e)),
                                    }
                                }
                                c.try_savepoint(&typed_insert_sql(&t, &fields, &[], &tuples.join(",")))
                            };
                        for chunk in rows.chunks(BATCH) {
                            bisect(&mut c, chunk, &attempt, &mut dead)?;
                        }
                    }
                }
                WriteMode::Append | WriteMode::Overwrite => {
                    c.query_rows(&format!("CREATE TABLE IF NOT EXISTS {t} (data JSONB)"))?;
                    if matches!(stream.write_mode, WriteMode::Overwrite) {
                        c.query_rows(&format!("TRUNCATE {t}"))?;
                    }
                    let attempt = |c: &mut PgConn, sub: &[String]| -> Result<Result<(), String>, String> {
                        let values = sub
                            .iter()
                            .map(|r| format!("({}::jsonb)", lit(r)))
                            .collect::<Vec<_>>()
                            .join(",");
                        c.try_savepoint(&format!("INSERT INTO {t} (data) VALUES {values}"))
                    };
                    for chunk in rows.chunks(BATCH) {
                        bisect(&mut c, chunk, &attempt, &mut dead)?;
                    }
                }
                WriteMode::Upsert { business_keys } if typed => {
                    if business_keys.is_empty() {
                        return Err("upsert requires non-empty business_keys".to_string());
                    }
                    let fields = typed_fields(rows, business_keys);
                    ensure_typed_table(&mut c, &t, &fields, business_keys, None)?;
                    let attempt =
                        |c: &mut PgConn, sub: &[String]| -> Result<Result<(), String>, String> {
                            let mut tuples = Vec::with_capacity(sub.len());
                            for r in sub {
                                match typed_tuple(r, &fields) {
                                    Ok(tup) => tuples.push(tup),
                                    Err(e) => return Ok(Err(e)),
                                }
                            }
                            c.try_savepoint(&typed_insert_sql(
                                &t,
                                &fields,
                                business_keys,
                                &tuples.join(","),
                            ))
                        };
                    for chunk in rows.chunks(BATCH) {
                        bisect(&mut c, chunk, &attempt, &mut dead)?;
                    }
                }
                WriteMode::Upsert { business_keys } => {
                    if business_keys.is_empty() {
                        return Err("upsert requires non-empty business_keys".to_string());
                    }
                    let keycols: Vec<String> = business_keys.iter().map(|k| quote_ident(k)).collect();
                    let coldefs = keycols.iter().map(|c| format!("{c} TEXT NOT NULL")).collect::<Vec<_>>().join(", ");
                    let pk = keycols.join(", ");
                    c.query_rows(&format!(
                        "CREATE TABLE IF NOT EXISTS {t} ({coldefs}, data JSONB, PRIMARY KEY ({pk}))"
                    ))?;
                    let attempt = |c: &mut PgConn, sub: &[String]| -> Result<Result<(), String>, String> {
                        let mut tuples = Vec::with_capacity(sub.len());
                        for r in sub {
                            let v: serde_json::Value = match serde_json::from_str(r) {
                                Ok(v) => v,
                                Err(e) => return Ok(Err(format!("row json: {e}"))),
                            };
                            let key_lits = business_keys
                                .iter()
                                .map(|k| lit(&key_text(&v, k)))
                                .collect::<Vec<_>>()
                                .join(", ");
                            tuples.push(format!("({key_lits}, {}::jsonb)", lit(r)));
                        }
                        c.try_savepoint(&format!(
                            "INSERT INTO {t} ({pk}, data) VALUES {} \
                             ON CONFLICT ({pk}) DO UPDATE SET data = EXCLUDED.data",
                            tuples.join(",")
                        ))
                    };
                    for chunk in rows.chunks(BATCH) {
                        bisect(&mut c, chunk, &attempt, &mut dead)?;
                    }
                }
            }
            c.query_rows("COMMIT")?;
            Ok(())
        })();

        if let Err(e) = txn {
            let _ = c.query_rows("ROLLBACK");
            return Err(e);
        }
        let accepted = rows.len() as u64 - dead.len() as u64;
        Ok((accepted, dead))
    }
}

// ---- typed relational columns ----

/// `typed_columns` config flag (default TRUE — the warehouse expectation: a
/// synced `orders` stream lands as `id bigint, total double precision, …`, not
/// one `data JSONB` blob; `false` restores the legacy JSONB layout).
fn typed_enabled(cfg_json: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(cfg_json)
        .ok()
        .and_then(|v| v.get("typed_columns").and_then(|x| x.as_bool()))
        .unwrap_or(true)
}

fn pg_type(ft: cloacina_constructor_contract::sync::FieldType) -> &'static str {
    use cloacina_constructor_contract::sync::FieldType::*;
    match ft {
        Integer => "bigint",
        Float => "double precision",
        Boolean => "boolean",
        Timestamp => "timestamptz",
        Str => "text",
        Json => "jsonb",
    }
}

/// Fields inferred from the batch ( `StreamSchema::infer` — the
/// same inference the host's schema capture uses), with the business keys
/// guaranteed present (a key absent from every row lands as text).
fn typed_fields(rows: &[String], keys: &[String]) -> Vec<cloacina_constructor_contract::sync::Field> {
    let mut fields = cloacina_constructor_contract::sync::StreamSchema::infer(rows).fields;
    for k in keys {
        if !fields.iter().any(|f| &f.name == k) {
            fields.push(cloacina_constructor_contract::sync::Field {
                name: k.clone(),
                field_type: cloacina_constructor_contract::sync::FieldType::Str,
                nullable: false,
            });
        }
    }
    fields
}

/// CREATE the typed table if missing (+ PK when keyed), then additively ALTER
/// in any new columns — the  additive path. A concrete type
/// change on an existing column is deliberately NOT patched: the insert errors
/// and those rows dead-letter (the host-side evolution policy is the breaking-
/// change gate).
fn ensure_typed_table(
    c: &mut PgConn,
    t: &str,
    fields: &[cloacina_constructor_contract::sync::Field],
    keys: &[String],
    tombstone_col: Option<&str>,
) -> Result<(), String> {
    let mut defs: Vec<String> = fields
        .iter()
        .map(|f| format!("{} {}", quote_ident(&f.name), pg_type(f.field_type)))
        .collect();
    if let Some(tc) = tombstone_col {
        defs.push(format!("{} TEXT", quote_ident(tc)));
    }
    let pk_clause = if keys.is_empty() {
        String::new()
    } else {
        format!(
            ", PRIMARY KEY ({})",
            keys.iter().map(|k| quote_ident(k)).collect::<Vec<_>>().join(", ")
        )
    };
    c.query_rows(&format!(
        "CREATE TABLE IF NOT EXISTS {t} ({}{pk_clause})",
        defs.join(", ")
    ))?;
    for f in fields {
        c.query_rows(&format!(
            "ALTER TABLE {t} ADD COLUMN IF NOT EXISTS {} {}",
            quote_ident(&f.name),
            pg_type(f.field_type)
        ))?;
    }
    if let Some(tc) = tombstone_col {
        c.query_rows(&format!(
            "ALTER TABLE {t} ADD COLUMN IF NOT EXISTS {} TEXT",
            quote_ident(tc)
        ))?;
    }
    Ok(())
}

/// SQL literal for a JSON value landing in a typed column. Kind mismatches are
/// left to Postgres — the offending row dead-letters via the bisection
/// machinery instead of being silently mangled.
fn typed_value(v: Option<&serde_json::Value>) -> String {
    match v {
        None | Some(serde_json::Value::Null) => "NULL".to_string(),
        Some(serde_json::Value::Bool(b)) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::String(s)) => lit(s),
        Some(other) => format!("{}::jsonb", lit(&other.to_string())),
    }
}

/// One `(v1, v2, …)` tuple for `row` over `fields`; Err = unparseable row.
fn typed_tuple(row: &str, fields: &[cloacina_constructor_contract::sync::Field]) -> Result<String, String> {
    let v: serde_json::Value = serde_json::from_str(row).map_err(|e| format!("row json: {e}"))?;
    let vals: Vec<String> = fields.iter().map(|f| typed_value(v.get(&f.name))).collect();
    Ok(format!("({})", vals.join(", ")))
}

/// `INSERT INTO t (cols) VALUES <tuples>`, with the keyed upsert clause.
fn typed_insert_sql(
    t: &str,
    fields: &[cloacina_constructor_contract::sync::Field],
    keys: &[String],
    tuples: &str,
) -> String {
    let cols: Vec<String> = fields.iter().map(|f| quote_ident(&f.name)).collect();
    let base = format!("INSERT INTO {t} ({}) VALUES {tuples}", cols.join(", "));
    if keys.is_empty() {
        return base;
    }
    let pk = keys.iter().map(|k| quote_ident(k)).collect::<Vec<_>>().join(", ");
    let sets: Vec<String> = fields
        .iter()
        .filter(|f| !keys.contains(&f.name))
        .map(|f| format!("{c} = EXCLUDED.{c}", c = quote_ident(&f.name)))
        .collect();
    if sets.is_empty() {
        format!("{base} ON CONFLICT ({pk}) DO NOTHING")
    } else {
        format!("{base} ON CONFLICT ({pk}) DO UPDATE SET {}", sets.join(", "))
    }
}

fn quote_ident(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// A SQL string literal, single-quote-escaped (with `standard_conforming_strings`
/// on — the default — backslashes are literal, so this is injection-safe).
fn lit(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

fn key_text(row: &serde_json::Value, key: &str) -> String {
    match row.get(key) {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

fn shard_predicate(bounds: Option<&str>) -> Result<Option<String>, String> {
    let Some(bounds) = bounds else { return Ok(None) };
    let v: serde_json::Value = serde_json::from_str(bounds).map_err(|e| format!("partition bounds: {e}"))?;
    let (Some(key), Some(shard), Some(of)) = (
        v.get("key").and_then(|x| x.as_str()),
        v.get("shard").and_then(|x| x.as_u64()),
        v.get("of").and_then(|x| x.as_u64()),
    ) else {
        return Ok(None);
    };
    if of == 0 {
        return Ok(None);
    }
    let kq = quote_ident(key);
    Ok(Some(format!("(((hashtext((t.{kq})::text) % {of}) + {of}) % {of}) = {shard}")))
}

fn cdc_slot_name(stream: &str) -> String {
    let sanitized: String = stream
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    format!("cloacina_{sanitized}").chars().take(63).collect()
}

/// CDC via `pg_logical_slot_get_changes` (test_decoding) — a plain query, slot name
/// is sanitized so it inlines safely. Requires `wal_level=logical`.
fn cdc_read(
    c: &mut PgConn,
    slot: &str,
    target_table: &str,
) -> Result<(Vec<ChangeRecord>, Option<String>), String> {
    let slot_lit = lit(slot);
    let exists = c.query_rows(
        &format!("SELECT EXISTS(SELECT 1 FROM pg_replication_slots WHERE slot_name = {slot_lit})"),
    )?;
    let is = exists.first().and_then(|r| r.first().cloned().flatten()).unwrap_or_default();
    if is != "t" {
        c.query_rows(&format!("SELECT pg_create_logical_replication_slot({slot_lit}, 'test_decoding')"))?;
    }
    let rows = c.query_rows(
        &format!("SELECT lsn::text, data FROM pg_logical_slot_get_changes({slot_lit}, NULL, NULL)"),
    )?;
    let mut out = Vec::new();
    let mut last = None;
    for r in &rows {
        let lsn = r.first().cloned().flatten().unwrap_or_default();
        let data = r.get(1).cloned().flatten().unwrap_or_default();
        // Parse the `test_decoding` line → a structured change; BEGIN/COMMIT framing yields None.
        // The logical slot captures the whole DB, so filter to this stream's table —
        // otherwise a Postgres *destination*'s own writes would feed back into the source.
        if let Some((table, op, row)) = cdc::parse_test_decoding(&data) {
            if table == target_table {
                out.push(ChangeRecord { op: op_to_guest(op), data: row });
            }
        }
        last = Some(lsn); // advance past every consumed row so replay doesn't re-deliver
    }
    Ok((out, last))
}

/// Map the host-side parsed op to this guest's WIT `ChangeOp`.
fn op_to_guest(op: cloacina_constructor_contract::sync::ChangeOp) -> ChangeOp {
    match op {
        cloacina_constructor_contract::sync::ChangeOp::Insert => ChangeOp::Insert,
        cloacina_constructor_contract::sync::ChangeOp::Update => ChangeOp::Update,
        cloacina_constructor_contract::sync::ChangeOp::Delete => ChangeOp::Delete,
    }
}

// ---- connection params + the sync wire client (postgres-protocol over sockets::tcp) ----

use fallible_iterator::FallibleIterator;
use postgres_protocol::authentication::md5_hash;
use postgres_protocol::authentication::sasl::{ChannelBinding, ScramSha256};
use postgres_protocol::message::{backend, frontend};

struct Conn {
    host: String,
    port: u16,
    user: String,
    password: String,
    dbname: String,
    /// `disable | require | verify-full` (; default **require** — no
    /// `prefer`, a silent plaintext fallback is dishonest). Validated at connect.
    sslmode: String,
    /// Inline PEM CA bundle for `verify-full` (: consumed per
    /// connect, never cached). Absent → compiled-in webpki roots.
    sslrootcert: Option<String>,
    /// The schema `discover()` introspects (default `public`).
    schema: String,
}

impl Conn {
    fn from_json(s: &str) -> Self {
        let v: serde_json::Value = serde_json::from_str(s).unwrap_or(serde_json::Value::Null);
        let get = |k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_string);
        // `url` (postgres://user:pw@host:port/db?sslmode=…) wins; else discrete fields.
        let mut conn = if let Some(c) = v.get("url").and_then(|x| x.as_str()).and_then(parse_url) {
            c
        } else {
            let user = get("user").unwrap_or_else(|| "postgres".to_string());
            Conn {
                host: get("host").unwrap_or_else(|| "127.0.0.1".to_string()),
                port: v.get("port").and_then(|x| x.as_u64()).unwrap_or(5432) as u16,
                dbname: get("dbname").unwrap_or_else(|| user.clone()),
                user,
                password: get("password").unwrap_or_default(),
                sslmode: "require".to_string(),
                sslrootcert: None,
                schema: "public".to_string(),
            }
        };
        // Explicit JSON fields override the URL's query (and the default).
        if let Some(m) = get("sslmode") {
            conn.sslmode = m;
        }
        if let Some(pem) = get("sslrootcert") {
            conn.sslrootcert = Some(pem);
        }
        if let Some(s) = get("schema").filter(|s| !s.is_empty()) {
            conn.schema = s;
        }
        conn
    }
}

fn parse_url(url: &str) -> Option<Conn> {
    let rest = url.strip_prefix("postgres://").or_else(|| url.strip_prefix("postgresql://"))?;
    let (creds, hostpart) = rest.split_once('@')?;
    let (user, password) = creds.split_once(':').unwrap_or((creds, ""));
    let (hostport, path) = hostpart.split_once('/').unwrap_or((hostpart, ""));
    let (host, port) = hostport.split_once(':').unwrap_or((hostport, "5432"));
    // `?sslmode=…` query on the db path (libpq-style).
    let (dbname, query) = path.split_once('?').unwrap_or((path, ""));
    let sslmode = query
        .split('&')
        .find_map(|kv| kv.strip_prefix("sslmode="))
        .unwrap_or("require");
    Some(Conn {
        host: host.to_string(),
        port: port.parse().unwrap_or(5432),
        user: user.to_string(),
        password: password.to_string(),
        dbname: if dbname.is_empty() { user.to_string() } else { dbname.to_string() },
        sslmode: sslmode.to_string(),
        sslrootcert: None,
        schema: "public".to_string(),
    })
}

/// The wire under the protocol: plaintext, or rustls layered over the same
/// brokered TCP stream ( guest-side TLS).
enum PgStream {
    Plain(fidius_guest::sockets::tcp::TcpStream),
    Tls(Box<rustls::StreamOwned<rustls::ClientConnection, fidius_guest::sockets::tcp::TcpStream>>),
}

impl Read for PgStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            PgStream::Plain(s) => s.read(buf),
            PgStream::Tls(s) => s.read(buf),
        }
    }
}

impl Write for PgStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            PgStream::Plain(s) => s.write(buf),
            PgStream::Tls(s) => s.write(buf),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            PgStream::Plain(s) => s.flush(),
            PgStream::Tls(s) => s.flush(),
        }
    }
}

/// `require`-mode verifier: encrypt without certificate verification (libpq
/// semantics). Deliberate and explicit — `verify-full` is the verified mode.
#[derive(Debug)]
struct AcceptAnyCert(std::sync::Arc<rustls::crypto::CryptoProvider>);

impl rustls::client::danger::ServerCertVerifier for AcceptAnyCert {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls_pki_types::CertificateDer<'_>,
        _intermediates: &[rustls_pki_types::CertificateDer<'_>],
        _server_name: &rustls_pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls_pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls_pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls_pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

/// Build the rustls client for `require` (encrypt, no verification) or
/// `verify-full` (hostname + chain against `sslrootcert` inline PEM, else the
/// compiled-in webpki roots). Roots are parsed per connect — never cached
///.
fn tls_connection(c: &Conn, verify: bool) -> Result<rustls::ClientConnection, String> {
    let provider = std::sync::Arc::new(rustls_rustcrypto::provider());
    let builder = rustls::ClientConfig::builder_with_provider(provider.clone())
        .with_protocol_versions(rustls::DEFAULT_VERSIONS)
        .map_err(|e| format!("tls versions: {e}"))?;
    let config = if verify {
        let mut roots = rustls::RootCertStore::empty();
        match &c.sslrootcert {
            Some(pem) => {
                use rustls_pki_types::pem::PemObject;
                let mut added = 0usize;
                for cert in rustls_pki_types::CertificateDer::pem_slice_iter(pem.as_bytes()) {
                    let cert = cert.map_err(|e| format!("sslrootcert pem: {e:?}"))?;
                    roots.add(cert).map_err(|e| format!("sslrootcert: {e}"))?;
                    added += 1;
                }
                if added == 0 {
                    return Err("sslrootcert contains no certificates".to_string());
                }
            }
            None => {
                roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            }
        }
        builder.with_root_certificates(roots).with_no_client_auth()
    } else {
        builder
            .dangerous()
            .with_custom_certificate_verifier(std::sync::Arc::new(AcceptAnyCert(provider)))
            .with_no_client_auth()
    };
    let server_name = rustls_pki_types::ServerName::try_from(c.host.clone())
        .map_err(|e| format!("tls server name `{}`: {e}", c.host))?;
    rustls::ClientConnection::new(std::sync::Arc::new(config), server_name)
        .map_err(|e| format!("tls client: {e}"))
}

struct PgConn {
    stream: PgStream,
    buf: bytes::BytesMut,
}

impl PgConn {
    fn connect(c: &Conn) -> Result<Self, String> {
        let verify = match c.sslmode.as_str() {
            "disable" => None,
            "require" => Some(false),
            "verify-full" => Some(true),
            other => {
                return Err(format!(
                    "unknown sslmode `{other}` (expected disable | require | verify-full)"
                ));
            }
        };
        let mut tcp = fidius_guest::sockets::tcp::connect(&c.host, c.port)
            .map_err(|e| format!("connect {}:{}: {e}", c.host, c.port))?;
        let stream = match verify {
            None => PgStream::Plain(tcp),
            Some(verify) => {
                // SSLRequest (len=8, code 80877103); the server answers ONE byte:
                // 'S' = proceed with TLS, 'N' = refused. No silent fallback.
                tcp.write_all(&[0, 0, 0, 8, 0x04, 0xd2, 0x16, 0x2f])
                    .and_then(|_| tcp.flush())
                    .map_err(|e| format!("sslrequest: {e}"))?;
                let mut answer = [0u8; 1];
                tcp.read_exact(&mut answer)
                    .map_err(|e| format!("sslrequest answer: {e}"))?;
                if answer[0] != b'S' {
                    return Err(format!(
                        "server refused TLS (sslmode={}); set sslmode=disable for a \
                         plaintext-only server",
                        c.sslmode
                    ));
                }
                let tls = tls_connection(c, verify)?;
                PgStream::Tls(Box::new(rustls::StreamOwned::new(tls, tcp)))
            }
        };
        let mut conn = PgConn { stream, buf: bytes::BytesMut::new() };
        let mut out = bytes::BytesMut::new();
        frontend::startup_message(
            [("user", c.user.as_str()), ("database", c.dbname.as_str())],
            &mut out,
        )
        .map_err(|e| format!("startup encode: {e}"))?;
        conn.send(&out)?;
        conn.authenticate(c)?;
        conn.wait_ready()?;
        Ok(conn)
    }

    fn send(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.stream.write_all(bytes).map_err(|e| format!("write: {e}"))?;
        self.stream.flush().map_err(|e| format!("flush: {e}"))
    }

    fn recv(&mut self) -> Result<backend::Message, String> {
        loop {
            if let Some(msg) = backend::Message::parse(&mut self.buf).map_err(|e| format!("parse: {e}"))? {
                return Ok(msg);
            }
            let mut chunk = [0u8; 8192];
            let n = self.stream.read(&mut chunk).map_err(|e| format!("read: {e}"))?;
            if n == 0 {
                return Err("connection closed by server".to_string());
            }
            self.buf.extend_from_slice(&chunk[..n]);
        }
    }

    fn authenticate(&mut self, c: &Conn) -> Result<(), String> {
        let mut scram: Option<ScramSha256> = None;
        loop {
            match self.recv()? {
                backend::Message::AuthenticationOk => return Ok(()),
                backend::Message::AuthenticationCleartextPassword => {
                    let mut out = bytes::BytesMut::new();
                    frontend::password_message(c.password.as_bytes(), &mut out)
                        .map_err(|e| format!("password encode: {e}"))?;
                    self.send(&out)?;
                }
                backend::Message::AuthenticationMd5Password(body) => {
                    let hash = md5_hash(c.user.as_bytes(), c.password.as_bytes(), body.salt());
                    let mut out = bytes::BytesMut::new();
                    frontend::password_message(hash.as_bytes(), &mut out)
                        .map_err(|e| format!("md5 encode: {e}"))?;
                    self.send(&out)?;
                }
                backend::Message::AuthenticationSasl(body) => {
                    let mut mechs = body.mechanisms();
                    let mut chose = false;
                    while let Some(m) = mechs.next().map_err(|e| format!("sasl mechs: {e}"))? {
                        if m == "SCRAM-SHA-256" {
                            chose = true;
                            break;
                        }
                    }
                    if !chose {
                        return Err("server offered no SCRAM-SHA-256 mechanism".to_string());
                    }
                    // GS2 flag "n" (client does no channel binding) — honest in both
                    // modes. The former `unrequested()` sent "y", which a server
                    // advertising SCRAM-SHA-256-PLUS (i.e. any TLS server) MUST
                    // reject as downgrade protection.
                    let s = ScramSha256::new(c.password.as_bytes(), ChannelBinding::unsupported());
                    let mut out = bytes::BytesMut::new();
                    frontend::sasl_initial_response("SCRAM-SHA-256", s.message(), &mut out)
                        .map_err(|e| format!("sasl init encode: {e}"))?;
                    self.send(&out)?;
                    scram = Some(s);
                }
                backend::Message::AuthenticationSaslContinue(body) => {
                    let s = scram.as_mut().ok_or("unexpected SASL continue")?;
                    s.update(body.data()).map_err(|e| format!("scram update: {e}"))?;
                    let mut out = bytes::BytesMut::new();
                    frontend::sasl_response(s.message(), &mut out)
                        .map_err(|e| format!("sasl resp encode: {e}"))?;
                    self.send(&out)?;
                }
                backend::Message::AuthenticationSaslFinal(body) => {
                    let s = scram.as_mut().ok_or("unexpected SASL final")?;
                    s.finish(body.data()).map_err(|e| format!("scram finish: {e}"))?;
                }
                backend::Message::ErrorResponse(body) => return Err(pg_error(body)),
                _ => return Err("unexpected message during authentication".to_string()),
            }
        }
    }

    fn wait_ready(&mut self) -> Result<(), String> {
        loop {
            match self.recv()? {
                backend::Message::ReadyForQuery(_) => return Ok(()),
                backend::Message::ErrorResponse(body) => return Err(pg_error(body)),
                _ => {}
            }
        }
    }

    /// Run one **simple Query** (the proven path) and return rows as raw text
    /// columns (`None` = SQL NULL). DDL/INSERT return no rows. Values are inlined
    /// by the caller via `lit`/`jsonb_lit` (single-quote-escaped, fail-closed).
    fn query_rows(&mut self, sql: &str) -> Result<Vec<Vec<Option<String>>>, String> {
        let mut out = bytes::BytesMut::new();
        frontend::query(sql, &mut out).map_err(|e| format!("query encode: {e}"))?;
        self.send(&out)?;
        let mut rows: Vec<Vec<Option<String>>> = Vec::new();
        // Capture an error but keep reading to `ReadyForQuery`: a simple-query error is
        // followed by ReadyForQuery, and leaving it unread desyncs the next statement —
        // which the SAVEPOINT/ROLLBACK bisection (below) depends on.
        let mut err: Option<String> = None;
        loop {
            match self.recv()? {
                backend::Message::DataRow(body) if err.is_none() => {
                    let data = body.buffer();
                    let mut row = Vec::new();
                    let mut ranges = body.ranges();
                    while let Some(range) = ranges.next().map_err(|e| format!("ranges: {e}"))? {
                        row.push(range.map(|r| String::from_utf8_lossy(&data[r]).into_owned()));
                    }
                    rows.push(row);
                }
                backend::Message::ReadyForQuery(_) => {
                    return match err {
                        Some(e) => Err(e),
                        None => Ok(rows),
                    };
                }
                backend::Message::ErrorResponse(body) => err = Some(pg_error(body)),
                _ => {} // DataRow-after-error / RowDescription / CommandComplete / …
            }
        }
    }

    /// Run `sql` inside a SAVEPOINT: RELEASE on success, ROLLBACK-to + RELEASE on failure
    /// (so the surrounding transaction survives and stays usable). The building block for
    /// fault-isolating bisection. Outer `Err` = a real connection/protocol failure (abort
    /// everything); inner `Ok(())` = landed, inner `Err(reason)` = `sql` failed but the
    /// transaction recovered cleanly.
    fn try_savepoint(&mut self, sql: &str) -> Result<Result<(), String>, String> {
        self.query_rows("SAVEPOINT _w")?;
        match self.query_rows(sql) {
            Ok(_) => {
                self.query_rows("RELEASE SAVEPOINT _w")?;
                Ok(Ok(()))
            }
            Err(e) => {
                self.query_rows("ROLLBACK TO SAVEPOINT _w")?;
                self.query_rows("RELEASE SAVEPOINT _w")?;
                Ok(Err(e))
            }
        }
    }
}

/// Write `rows` via `attempt`; on failure, **binary-split and retry** so only the rows
/// that fail *alone* are dead-lettered and everything else still commits. `attempt` runs a
/// sub-batch (Ok = landed, Err = rolled back with a reason). Generic over the executor so
/// the splitting logic is unit-testable without a database. Returns Err only on a real
/// connection/transaction failure (propagated by `attempt`'s `?`).
fn bisect<C>(
    c: &mut C,
    rows: &[String],
    attempt: &dyn Fn(&mut C, &[String]) -> Result<Result<(), String>, String>,
    dead: &mut Vec<DeadLetter>,
) -> Result<(), String> {
    if rows.is_empty() {
        return Ok(());
    }
    match attempt(c, rows)? {
        Ok(()) => Ok(()),
        Err(reason) if rows.len() == 1 => {
            dead.push(DeadLetter { record: rows[0].clone(), reason });
            Ok(())
        }
        Err(_) => {
            let mid = rows.len() / 2;
            bisect(c, &rows[..mid], attempt, dead)?;
            bisect(c, &rows[mid..], attempt, dead)
        }
    }
}

fn pg_error(body: backend::ErrorResponseBody) -> String {
    let mut fields = body.fields();
    let mut msg = String::from("postgres error");
    while let Ok(Some(f)) = fields.next() {
        if f.type_() == b'M' {
            msg = format!("postgres error: {}", String::from_utf8_lossy(f.value_bytes()));
        }
    }
    msg
}
