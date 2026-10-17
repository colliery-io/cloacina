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

//! Boundary types of the connector contract: what crosses between the host and a
//! source or destination connector. Moved from weir with Sync (COLLIERY-I-0632).
//!
//! All types are `serde`-serializable so they can cross the fidius boundary.
//! **Two bincode constraints shape these types** (fidius's typed wire is
//! `bincode`, and our raw envelopes use it too):
//! 1. bincode is *not self-describing*, so `serde_json::Value` (which
//!    deserializes via `deserialize_any`) cannot cross it — free-form JSON is
//!    carried as **JSON text (`String`)** ([`JsonText`]), which is also cleanly
//!    projectable to a WIT `string` for the WASM path.
//! 2. bincode reads fields positionally, so `#[serde(skip_serializing_if)]` /
//!    `#[serde(default)]` would desync the stream — **every field always
//!    serializes** (no field-level serde attributes here).

use serde::{Deserialize, Serialize};

use super::error::ConnectorError;

/// A JSON document carried as text. Used wherever the contract needs free-form
/// JSON that must survive bincode and project to WIT.
pub type JsonText = String;

/// Free-form, connector-specific configuration (a JSON document validated
/// against [`ConnectorSpec::config_schema`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub struct Config {
    /// The JSON config document, as text.
    pub json: JsonText,
}

/// Canonical Arrow schema, carried as Arrow IPC **schema-message bytes**.
///
/// Arrow's type system is canonical; its IPC serialization is the
/// wire form. The contract treats it as opaque bytes; the SDK/runtime decode it
/// with the `arrow` crate (wired in /0004).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub struct ArrowSchemaIpc {
    /// Arrow IPC schema-message bytes.
    pub ipc: Vec<u8>,
}

/// A record batch serialized as Arrow IPC **stream bytes**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub struct ArrowIpc {
    /// Arrow IPC stream bytes.
    pub ipc: Vec<u8>,
}

/// Supported sync modes for a stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub enum SyncMode {
    FullRefresh,
    Incremental,
    /// Log-based change data capture.
    Cdc,
}

/// The role(s) a connector fulfils. Authoritative dispatch capabilities come
/// from fidius `#[optional]` capability bits on the descriptor; this is the
/// human/UX-facing declaration in [`ConnectorSpec`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub enum ConnectorRole {
    Source,
    Destination,
    /// A destination that writes to operational systems (reverse-ETL).
    ReverseEtl,
}

/// Static descriptor returned by `Connector::spec`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub struct ConnectorSpec {
    pub name: String,
    pub connector_version: String,
    /// The contract version this connector targets (see ).
    pub contract_version: u32,
    /// JSON Schema (as JSON text) describing valid [`Config`].
    pub config_schema: JsonText,
    pub roles: Vec<ConnectorRole>,
    pub supported_sync_modes: Vec<SyncMode>,
}

/// Result of `Connector::check` — config + connectivity validation; moves no data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub struct CheckResult {
    pub success: bool,
    pub message: Option<String>,
}

/// How a source stream may be sliced into independent work units. The connector
/// **declares** the scheme in `discover`; the host **plans** the concrete
/// partitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub enum PartitionScheme {
    /// Not partitionable; one work unit per stream.
    /// (Named `Unpartitioned`, not `None`, because `none` is a reserved WIT keyword.)
    Unpartitioned,
    /// Sliceable along the cursor (e.g. by day).
    ByCursorRange { granularity: String },
    /// Sliceable by hash/shard of a key.
    ByKeyShards { key: String, count: u32 },
    /// Sliceable by parent-stream id (substreams).
    ByParent { parent_stream: String, key: String },
}

/// A stream as discovered from the source.
/// (Named `StreamInfo`, not `Stream`, because `stream` is a reserved WIT keyword.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub struct StreamInfo {
    pub name: String,
    pub namespace: Option<String>,
    /// Canonical Arrow schema for the stream's records.
    pub schema: ArrowSchemaIpc,
    pub supported_sync_modes: Vec<SyncMode>,
    pub source_defined_cursor: bool,
    pub default_cursor_field: Option<String>,
    pub source_defined_primary_key: Option<Vec<String>>,
    pub partitioning: PartitionScheme,
}

/// The catalog returned by `Connector::discover`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub struct Catalog {
    pub streams: Vec<StreamInfo>,
}

/// Destination write semantics for a configured stream. Reverse-ETL is
/// [`WriteMode::Upsert`] to an operational destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub enum WriteMode {
    Append,
    Upsert { business_keys: Vec<String> },
    Overwrite,
}

/// Light in-flight mapping applied by the host between read and write
///. v0 placeholder: an ordered, deliberately bounded op list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub struct MappingSpec {
    pub ops: Vec<MappingOp>,
}

/// A single bounded mapping operation (no joins/aggregates/UDFs — ).
/// Every variant is **flat / non-recursive** so the spec stays `WitType`-safe
/// across the wasm boundary (it rides `ConfiguredStream` → `ReadContext`); richer
/// nested expressions are a follow-on (host-side parse).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub enum MappingOp {
    Select {
        fields: Vec<String>,
    },
    Drop {
        fields: Vec<String>,
    },
    Rename {
        from: String,
        to: String,
    },
    /// Coerce `field` to a scalar type; a failed coercion dead-letters the record.
    Cast {
        field: String,
        to: CastType,
    },
    /// Keep only records where `field` `op` `value` (numeric compare when both
    /// parse as numbers, else string). Non-matching records are dropped + counted.
    Filter {
        field: String,
        op: CompareOp,
        value: String,
    },
    /// Set/overwrite `field` from a bounded expression over the record.
    Compute {
        field: String,
        value: ComputeExpr,
    },
}

/// Scalar target for [`MappingOp::Cast`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub enum CastType {
    Str,
    Integer,
    Float,
    Boolean,
}

/// Comparison for [`MappingOp::Filter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// A bounded compute expression for [`MappingOp::Compute`] — flat, no I/O /
/// network / arbitrary code (the dbt boundary).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub enum ComputeExpr {
    /// A literal string value.
    Const(String),
    /// Copy another field's value.
    Field(String),
    /// Concatenate the string values of the named fields.
    Concat(Vec<String>),
    /// Lower-case a field's string value.
    Lower(String),
    /// Upper-case a field's string value.
    Upper(String),
}

/// A user-configured stream within a connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub struct ConfiguredStream {
    /// References [`StreamInfo::name`].
    pub stream: String,
    pub sync_mode: SyncMode,
    pub cursor_field: Option<String>,
    pub primary_key: Option<Vec<String>>,
    pub write_mode: WriteMode,
    pub mapping: MappingSpec,
}

/// A configured catalog (the selected streams of a connection).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfiguredCatalog {
    pub streams: Vec<ConfiguredStream>,
}

/// A concrete partition (work-unit slice) the host planned from a
/// [`PartitionScheme`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub struct Partition {
    pub id: String,
    /// Scheme-specific bounds as JSON text (cursor range, shard index, parent id, …).
    pub bounds: Option<JsonText>,
}

/// Opaque, connector-owned resumption state plus an optional cursor (JSON text).
/// The **host** commits this transactionally; the connector
/// never persists durability itself.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub struct StreamState {
    pub cursor: Option<JsonText>,
    pub opaque: Vec<u8>,
}

/// The change operation a CDC record carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub enum ChangeOp {
    Insert,
    Update,
    Delete,
}

/// One change: an op + the row JSON (which includes the key columns, so a delete
/// carries its key). Flows through the host + mapping like a `Rows` entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub struct ChangeRecord {
    pub op: ChangeOp,
    pub data: JsonText,
}

/// A batch of records in one of the canonical encodings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub enum RecordBatch {
    /// Long-tail row encoding; each entry is a JSON-object row (as text)
    /// conforming to the stream's Arrow schema.
    Rows(Vec<JsonText>),
    /// Bulk native path: Arrow IPC stream bytes.
    Arrow(ArrowIpc),
    /// CDC changes: each entry is a row plus its op (insert/update/delete),
    /// so deletes propagate to the destination.
    Changes(Vec<ChangeRecord>),
}

impl RecordBatch {
    /// Number of rows when known cheaply (`Rows`); `None` for opaque Arrow IPC.
    pub fn row_count_hint(&self) -> Option<usize> {
        match self {
            RecordBatch::Rows(r) => Some(r.len()),
            RecordBatch::Changes(c) => Some(c.len()),
            RecordBatch::Arrow(_) => None,
        }
    }
}

/// Severity for a [`LogEntry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Out-of-band diagnostic emitted alongside a chunk/ack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
}

/// One record the connector could **not** process but which is *not* an
/// operation-level failure ( §5 record-level). The operation
/// still succeeds; the host routes these to a dead-letter sink and counts
/// them. Carried in the operational channel, so a run can succeed *and* report
/// dead-letters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub struct DeadLetter {
    /// The offending record (raw row text, or an identifier for it).
    pub record: String,
    /// Why it was rejected.
    pub reason: String,
}

/// One item of the **streaming `read`**: the connector
/// yields records, periodic checkpoints (the host commits on these), logs, and
/// dead-letters as a single typed stream — replacing the v0 chunked-pull read
/// envelope. A terminal `ConnectorError` ends the stream (host-side).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub enum ReadMessage {
    Records(RecordBatch),
    Checkpoint(StreamState),
    Log(LogEntry),
    DeadLettered(DeadLetter),
    /// A terminal connector-domain failure — ends the read; the host fails the
    /// run (the `kind` drives retry, like the v0 read envelope's error arm).
    Fatal(ConnectorError),
}

// --- Raw envelopes for `read` / `write` (`#[wire(raw)]`) ---

/// Input to the streaming `Connector::read` for one `(stream, partition)`
///. `config` is **not** here in v1 — it is bound once at
/// construction (configured instances).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub struct ReadContext {
    pub stream: ConfiguredStream,
    pub partition: Partition,
    /// Resume point; `StreamState::default()` for a fresh read.
    pub state: StreamState,
}

/// The configured-stream context for a `write`. `config` is bound at
/// construction (configured instances); the destination
/// consumes a client-stream of [`RecordBatch`]es for this context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub struct WriteContext {
    pub stream: ConfiguredStream,
}

/// Records accepted by a successful `write`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub struct WriteReceipt {
    pub accepted: u64,
}

/// Data ⊕ error result of a `write` (an enum, not `Result`, so it crosses WIT —
/// mirrors [`DiscoverOutcome`]). `Err` ends the write; its `kind` drives retry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub enum WriteResult {
    Ok(WriteReceipt),
    Err(ConnectorError),
}

/// The outcome of one client-streaming `write` — the same three-channel split as
/// the read path: operational (`state`, `diagnostics`, always present) + the
/// data ⊕ error [`WriteResult`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub struct WriteOutcome {
    pub state: StreamState,
    pub diagnostics: Vec<LogEntry>,
    pub dead_letters: Vec<DeadLetter>,
    pub result: WriteResult,
}
