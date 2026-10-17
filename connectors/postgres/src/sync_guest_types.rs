// The Sync connector contract as a WASM guest sees it (COLLIERY-I-0632 /
// CLOACINA-T-0973). One canonical copy, carried by every connector guest:
// `fidius_build::emit_wit` reads only the guest's own source tree, so the
// `WitType` types cannot come from the contract crate. They mirror
// `cloacina_constructor_contract::sync` field for field, so the load-time
// interface hash and the `Value` shapes match the host. A drift test keeps the
// copies identical (Sync 1.9). Do not edit by hand in a guest.
use super::*;

#[derive(WitType, Clone)]
pub struct Config { pub json: String }
#[derive(WitType, Clone)]
pub struct ArrowSchemaIpc { pub ipc: Vec<u8> }
// RecordBatch and ArrowIpc also derive serde: client-streamed `write` items
// cross as bincode, so the consumed `Stream<RecordBatch>` needs Deserialize.
#[derive(WitType, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArrowIpc { pub ipc: Vec<u8> }
#[derive(WitType, Clone, PartialEq)]
pub enum SyncMode { FullRefresh, Incremental, Cdc }
#[derive(WitType, Clone, PartialEq)]
pub enum ConnectorRole { Source, Destination, ReverseEtl }
#[derive(WitType, Clone)]
pub struct ConnectorSpec {
    pub name: String,
    pub connector_version: String,
    pub contract_version: u32,
    pub config_schema: String,
    pub roles: Vec<ConnectorRole>,
    pub supported_sync_modes: Vec<SyncMode>,
}
#[derive(WitType, Clone)]
pub struct CheckResult { pub success: bool, pub message: Option<String> }
#[derive(WitType, Clone)]
pub enum PartitionScheme {
    Unpartitioned,
    ByCursorRange { granularity: String },
    ByKeyShards { key: String, count: u32 },
    ByParent { parent_stream: String, key: String },
}
#[derive(WitType, Clone)]
pub struct StreamInfo {
    pub name: String,
    pub namespace: Option<String>,
    pub schema: ArrowSchemaIpc,
    pub supported_sync_modes: Vec<SyncMode>,
    pub source_defined_cursor: bool,
    pub default_cursor_field: Option<String>,
    pub source_defined_primary_key: Option<Vec<String>>,
    pub partitioning: PartitionScheme,
}
#[derive(WitType, Clone)]
pub struct Catalog { pub streams: Vec<StreamInfo> }
#[derive(WitType, Clone)]
pub enum ErrorKind { Config, Transient, RecordLevel, Fatal }
#[derive(WitType, Clone)]
pub struct ContextPair { pub key: String, pub value: String }
#[derive(WitType, Clone)]
pub struct ConnectorError {
    pub kind: ErrorKind,
    pub message: String,
    pub retryable: bool,
    pub context: Vec<ContextPair>,
}
impl ConnectorError {
    pub fn config(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Config, message: message.into(), retryable: false, context: Vec::new() }
    }
    pub fn transient(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Transient, message: message.into(), retryable: true, context: Vec::new() }
    }
    pub fn fatal(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Fatal, message: message.into(), retryable: false, context: Vec::new() }
    }
}
#[derive(WitType, Clone)]
pub enum DiscoverOutcome { Catalog(Catalog), Error(ConnectorError) }
#[derive(WitType, Clone)]
pub enum WriteMode { Append, Upsert { business_keys: Vec<String> }, Overwrite }
#[derive(WitType, Clone)]
pub enum MappingOp {
    Select { fields: Vec<String> },
    Drop { fields: Vec<String> },
    Rename { from: String, to: String },
    Cast { field: String, to: CastType },
    Filter { field: String, op: CompareOp, value: String },
    Compute { field: String, value: ComputeExpr },
}
#[derive(WitType, Clone)]
pub enum CastType { Str, Integer, Float, Boolean }
#[derive(WitType, Clone)]
pub enum CompareOp { Eq, Ne, Lt, Le, Gt, Ge }
#[derive(WitType, Clone)]
pub enum ComputeExpr {
    Const(String),
    Field(String),
    Concat(Vec<String>),
    Lower(String),
    Upper(String),
}
#[derive(WitType, Clone)]
pub struct MappingSpec { pub ops: Vec<MappingOp> }
#[derive(WitType, Clone)]
pub struct ConfiguredStream {
    pub stream: String,
    pub sync_mode: SyncMode,
    pub cursor_field: Option<String>,
    pub primary_key: Option<Vec<String>>,
    pub write_mode: WriteMode,
    pub mapping: MappingSpec,
}
#[derive(WitType, Clone)]
pub struct Partition { pub id: String, pub bounds: Option<String> }
#[derive(WitType, Clone)]
pub struct StreamState { pub cursor: Option<String>, pub opaque: Vec<u8> }
#[derive(WitType, Clone, serde::Serialize, serde::Deserialize)]
pub enum RecordBatch { Rows(Vec<String>), Arrow(ArrowIpc), Changes(Vec<ChangeRecord>) }
#[derive(WitType, Clone, serde::Serialize, serde::Deserialize)]
pub enum ChangeOp { Insert, Update, Delete }
#[derive(WitType, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChangeRecord { pub op: ChangeOp, pub data: String }
#[derive(WitType, Clone)]
pub enum LogLevel { Debug, Info, Warn, Error }
#[derive(WitType, Clone)]
pub struct LogEntry { pub level: LogLevel, pub message: String }
#[derive(WitType, Clone)]
pub struct DeadLetter { pub record: String, pub reason: String }
#[derive(WitType, Clone)]
pub struct ReadContext {
    pub stream: ConfiguredStream,
    pub partition: Partition,
    pub state: StreamState,
}
#[derive(WitType, Clone)]
pub enum ReadMessage {
    Records(RecordBatch),
    Checkpoint(StreamState),
    Log(LogEntry),
    DeadLettered(DeadLetter),
    Fatal(ConnectorError),
}
#[derive(WitType, Clone)]
pub struct WriteContext { pub stream: ConfiguredStream }
#[derive(WitType, Clone)]
pub struct WriteReceipt { pub accepted: u64 }
#[derive(WitType, Clone)]
pub enum WriteResult { Ok(WriteReceipt), Err(ConnectorError) }
#[derive(WitType, Clone)]
pub struct WriteOutcome {
    pub state: StreamState,
    pub diagnostics: Vec<LogEntry>,
    pub dead_letters: Vec<DeadLetter>,
    pub result: WriteResult,
}
