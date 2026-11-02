//! The echo connector (COLLIERY-I-0632 / CLOACINA-T-0973): a source that
//! yields the rows named in its config, then a checkpoint; a destination that
//! counts the rows it is given. The smallest connector that exercises every
//! method of the contract through the connector host.
//!
//! The interface is re-declared here against `fidius_guest` with the same
//! signatures as the host's declaration, so the interface hash matches. The
//! typed-method types are the local `WitType` copies in `sync_guest_types.rs`;
//! `config` is bound at construction as `cloacina_constructor_contract::sync::Config`.
#![allow(clippy::all)]

use cloacina_constructor_contract::sync as contract;
use fidius_macro::{WitType, plugin_impl, plugin_interface};

mod sync_guest_types;
use sync_guest_types::*;

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

/// Config: `{"rows": ["<json row>", ...], "cursor": "<text>"}`. Rows default to one
/// `{"echo":true}`; the cursor defaults to none.
pub struct Echo {
    cfg: contract::Config,
}

impl Echo {
    fn configure(cfg: contract::Config) -> Self {
        Self { cfg }
    }

    fn rows(&self) -> Vec<String> {
        serde_json::from_str::<serde_json::Value>(&self.cfg.json)
            .ok()
            .and_then(|v| {
                v.get("rows").and_then(|r| r.as_array()).map(|a| {
                    a.iter()
                        .map(|x| match x {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        })
                        .collect()
                })
            })
            .unwrap_or_else(|| vec!["{\"echo\":true}".to_string()])
    }

    fn cursor(&self) -> Option<String> {
        serde_json::from_str::<serde_json::Value>(&self.cfg.json)
            .ok()
            .and_then(|v| v.get("cursor").and_then(|c| c.as_str()).map(str::to_string))
    }

    /// `{"segments": N}`: the read yields N groups of (rows, checkpoint), the
    /// checkpoint cursor counting `"1"`, `"2"`, … so a dispatcher test can see
    /// several segments and a cursor that moves. Default 1, which keeps the
    /// single `cursor` config meaningful.
    fn segments(&self) -> u64 {
        serde_json::from_str::<serde_json::Value>(&self.cfg.json)
            .ok()
            .and_then(|v| v.get("segments").and_then(|n| n.as_u64()))
            .unwrap_or(1)
            .max(1)
    }
}

#[plugin_impl(Connector, crate = "fidius_guest", config = contract::Config)]
impl Connector for Echo {
    fn spec(&self) -> ConnectorSpec {
        ConnectorSpec {
            name: "echo".to_string(),
            connector_version: "0.1.0".to_string(),
            contract_version: 1,
            config_schema: "{\"type\":\"object\"}".to_string(),
            roles: vec![ConnectorRole::Source, ConnectorRole::Destination],
            supported_sync_modes: vec![SyncMode::FullRefresh],
        }
    }

    fn check(&self, config: Config) -> CheckResult {
        let ok = serde_json::from_str::<serde_json::Value>(&config.json).is_ok();
        CheckResult {
            success: ok,
            message: if ok { None } else { Some("config is not JSON".to_string()) },
        }
    }

    fn discover(&self, _config: Config) -> DiscoverOutcome {
        DiscoverOutcome::Catalog(Catalog {
            streams: vec![StreamInfo {
                name: "echo".to_string(),
                namespace: None,
                schema: ArrowSchemaIpc { ipc: Vec::new() },
                supported_sync_modes: vec![SyncMode::FullRefresh],
                source_defined_cursor: false,
                default_cursor_field: None,
                source_defined_primary_key: None,
                partitioning: PartitionScheme::Unpartitioned,
            }],
        })
    }

    /// Per segment: the configured rows in one batch, a log line, then a
    /// checkpoint. With `segments` > 1 the checkpoint cursor counts up and a
    /// resume point `"k"` in `ctx.state` skips the first k segments, so a
    /// dispatcher reopening the source at the committed cursor sees the rest.
    fn read(&self, ctx: ReadContext) -> fidius_guest::Stream<ReadMessage> {
        let total = self.segments();
        let resume = ctx
            .state
            .cursor
            .as_deref()
            .and_then(|c| c.trim_matches('"').parse::<u64>().ok())
            .unwrap_or(0);
        let mut msgs = Vec::new();
        for i in (resume + 1)..=total {
            msgs.push(ReadMessage::Records(RecordBatch::Rows(self.rows())));
            msgs.push(ReadMessage::Log(LogEntry {
                level: LogLevel::Info,
                message: format!("echo read segment {i}"),
            }));
            let cursor = if total == 1 { self.cursor() } else { Some(format!("\"{i}\"")) };
            msgs.push(ReadMessage::Checkpoint(StreamState {
                cursor,
                opaque: Vec::new(),
            }));
        }
        fidius_guest::Stream::from_iter(msgs)
    }

    /// Client streaming: pull every batch, counting the rows accepted.
    fn write(&self, _ctx: WriteContext, mut batches: fidius_guest::Stream<RecordBatch>) -> WriteOutcome {
        let mut accepted = 0u64;
        while let Some(batch) = batches.next_item() {
            accepted += match batch {
                RecordBatch::Rows(rows) => rows.len() as u64,
                RecordBatch::Changes(changes) => changes.len() as u64,
                RecordBatch::Arrow(_) => 0,
            };
        }
        WriteOutcome {
            state: StreamState {
                cursor: self.cursor(),
                opaque: Vec::new(),
            },
            diagnostics: Vec::new(),
            dead_letters: Vec::new(),
            result: WriteResult::Ok(WriteReceipt { accepted }),
        }
    }
}
