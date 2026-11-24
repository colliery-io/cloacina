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

//! The HTTP connector fixture (COLLIERY-I-0633 / CLOACINA-T-0982): a source
//! whose read GETs the `fetch` URL in its config and yields one row
//! `{"status": <code>, "body": "<text>"}` (or `{"error": "<text>"}`), then a
//! checkpoint. The request leaves unsigned; the host's egress policy adds the
//! credential, so the guest never holds it.
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

/// Config: `{"fetch": "<url>"}`.
pub struct Http {
    cfg: contract::Config,
}

impl Http {
    fn configure(cfg: contract::Config) -> Self {
        Self { cfg }
    }

    fn url(&self) -> Option<String> {
        serde_json::from_str::<serde_json::Value>(&self.cfg.json)
            .ok()
            .and_then(|v| v.get("fetch").and_then(|u| u.as_str()).map(str::to_string))
    }
}

#[plugin_impl(Connector, crate = "fidius_guest", config = contract::Config)]
impl Connector for Http {
    fn spec(&self) -> ConnectorSpec {
        ConnectorSpec {
            name: "http".to_string(),
            connector_version: "0.1.0".to_string(),
            contract_version: 1,
            config_schema: "{\"type\":\"object\"}".to_string(),
            roles: vec![ConnectorRole::Source],
            supported_sync_modes: vec![SyncMode::FullRefresh],
        }
    }

    fn check(&self, _config: Config) -> CheckResult {
        let ok = self.url().is_some();
        CheckResult {
            success: ok,
            message: if ok {
                None
            } else {
                Some("config has no `fetch` URL".to_string())
            },
        }
    }

    fn discover(&self, _config: Config) -> DiscoverOutcome {
        DiscoverOutcome::Catalog(Catalog {
            streams: vec![StreamInfo {
                name: "http".to_string(),
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

    /// One GET, one row, one checkpoint.
    fn read(&self, _ctx: ReadContext) -> fidius_guest::Stream<ReadMessage> {
        let row = match self.url() {
            None => serde_json::json!({ "error": "config has no `fetch` URL" }),
            Some(url) => match fidius_guest::http::get(&url) {
                Ok(r) => serde_json::json!({ "status": r.status, "body": r.text() }),
                Err(e) => serde_json::json!({ "error": e.to_string() }),
            },
        };
        fidius_guest::Stream::from_iter(vec![
            ReadMessage::Records(RecordBatch::Rows(vec![row.to_string()])),
            ReadMessage::Checkpoint(StreamState {
                cursor: None,
                opaque: Vec::new(),
            }),
        ])
    }

    /// Not a destination: accept nothing.
    fn write(
        &self,
        _ctx: WriteContext,
        _batches: fidius_guest::Stream<RecordBatch>,
    ) -> WriteOutcome {
        WriteOutcome {
            state: StreamState {
                cursor: None,
                opaque: Vec::new(),
            },
            diagnostics: Vec::new(),
            dead_letters: Vec::new(),
            result: WriteResult::Err(ConnectorError {
                kind: ErrorKind::Config,
                message: "the http fixture is a source only".to_string(),
                retryable: false,
                context: Vec::new(),
            }),
        }
    }
}
