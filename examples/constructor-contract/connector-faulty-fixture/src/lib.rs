//! The faulty connector (COLLIERY-I-0632 / CLOACINA-T-0973): fails on demand so
//! the host's error paths can be tested. Config:
//! `{"fail": "fatal" | "transient", "dead_letter": true}`. With `fail` set, a
//! read ends in `Fatal` and a write returns `Err`; with `dead_letter`, each
//! carries one dead-letter first.
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

pub struct Faulty {
    cfg: contract::Config,
}

impl Faulty {
    fn configure(cfg: contract::Config) -> Self {
        Self { cfg }
    }

    fn evaluate(&self) -> (Vec<DeadLetter>, Result<(), ConnectorError>) {
        let v: serde_json::Value =
            serde_json::from_str(&self.cfg.json).unwrap_or(serde_json::Value::Null);
        let dead = if v.get("dead_letter").and_then(|d| d.as_bool()) == Some(true) {
            vec![DeadLetter {
                record: "{\"bad\":true}".to_string(),
                reason: "simulated record-level rejection".to_string(),
            }]
        } else {
            Vec::new()
        };
        let outcome = match v.get("fail").and_then(|f| f.as_str()) {
            Some("transient") => Err(ConnectorError::transient("simulated transient failure")),
            Some(_) => Err(ConnectorError::fatal("simulated fatal failure")),
            None => Ok(()),
        };
        (dead, outcome)
    }
}

#[plugin_impl(Connector, crate = "fidius_guest", config = contract::Config)]
impl Connector for Faulty {
    fn spec(&self) -> ConnectorSpec {
        ConnectorSpec {
            name: "faulty".to_string(),
            connector_version: "0.1.0".to_string(),
            contract_version: 1,
            config_schema: "{\"type\":\"object\",\"properties\":{\"fail\":{\"type\":\"string\"},\"dead_letter\":{\"type\":\"boolean\"}}}".to_string(),
            roles: vec![ConnectorRole::Source, ConnectorRole::Destination],
            supported_sync_modes: vec![SyncMode::FullRefresh],
        }
    }

    fn check(&self, _config: Config) -> CheckResult {
        CheckResult { success: true, message: None }
    }

    fn discover(&self, _config: Config) -> DiscoverOutcome {
        DiscoverOutcome::Catalog(Catalog {
            streams: vec![StreamInfo {
                name: "faulty".to_string(),
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

    fn read(&self, _ctx: ReadContext) -> fidius_guest::Stream<ReadMessage> {
        let (dead_letters, outcome) = self.evaluate();
        let mut msgs: Vec<ReadMessage> = dead_letters.into_iter().map(ReadMessage::DeadLettered).collect();
        match outcome {
            Ok(()) => {
                msgs.push(ReadMessage::Records(RecordBatch::Rows(vec!["{\"faulty\":true}".to_string()])));
                msgs.push(ReadMessage::Checkpoint(StreamState { cursor: None, opaque: Vec::new() }));
            }
            Err(e) => msgs.push(ReadMessage::Fatal(e)),
        }
        fidius_guest::Stream::from_iter(msgs)
    }

    fn write(&self, _ctx: WriteContext, mut batches: fidius_guest::Stream<RecordBatch>) -> WriteOutcome {
        while batches.next_item().is_some() {}
        let (dead_letters, outcome) = self.evaluate();
        WriteOutcome {
            state: StreamState { cursor: None, opaque: Vec::new() },
            diagnostics: Vec::new(),
            dead_letters,
            result: match outcome {
                Ok(()) => WriteResult::Ok(WriteReceipt { accepted: 0 }),
                Err(e) => WriteResult::Err(e),
            },
        }
    }
}
