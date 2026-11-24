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

//! # The connector contract of Sync (COLLIERY-I-0632, CLOACINA-A-0013)
//!
//! A **connector** is a provider whose members are **sources** and
//! **destinations** ([`crate::PrimitiveKind::Source`],
//! [`crate::PrimitiveKind::Destination`]). Both implement one fidius interface,
//! `connector`, with five methods in this vtable order:
//!
//! | index | method | shape | capability |
//! |---|---|---|---|
//! | 0 | `spec() -> ConnectorSpec` | typed | always |
//! | 1 | `check(Config) -> CheckResult` | typed | always |
//! | 2 | `discover(Config) -> DiscoverOutcome` | typed, optional | source |
//! | 3 | `read(ReadContext) -> Stream<ReadMessage>` | server streaming, optional | source |
//! | 4 | `write(WriteContext, Stream<RecordBatch>) -> WriteOutcome` | client streaming, optional | destination |
//!
//! `config` is bound once at construction, like every other constructor: the
//! member's single `#[config]` field is the JSON document ([`Config::json`]).
//! `check` and `discover` take a `Config` explicitly so a catalog can probe a
//! connector before any instance is configured.
//!
//! The `#[plugin_interface]` declaration lives where a fidius `crate =` target is
//! bound (the host loader, a guest crate), as it does for the other primitives;
//! this module stays fidius-free so it builds for `wasm32-wasip2` from serde
//! alone. The `wit` feature adds `WitType` derives for guests that project the
//! types to WIT.
//!
//! The wire is bincode 1.3 with default options ([`codec`]). Two constraints
//! follow: no `serde_json::Value` (free-form JSON travels as text,
//! [`JsonText`]) and no field-level serde attributes on a boundary type (bincode
//! is positional). The types came from weir unchanged in shape, so a guest built
//! on weir's contract speaks this wire byte for byte.
//!
//! What a sync does with these: the read task opens `read` with the stored
//! [`StreamState`] and reads until a [`ReadMessage::Checkpoint`], which closes a
//! segment; the write task streams the segment's [`RecordBatch`]es into `write`
//! and returns the new state and the dead-letters for the completion commit.
//! The cursor advances only after the durable write.

pub mod codec;
pub mod credential;
pub mod cursor;
pub mod error;
pub mod rest;
pub mod schema;
pub mod types;

pub use codec::{CodecError, decode, encode};
pub use cursor::cursor_cmp;
pub use error::{ConnectorError, ContextPair, ErrorKind};
pub use schema::{Field, FieldType, SchemaDiff, StreamSchema};
pub use types::*;

use serde::{Deserialize, Serialize};

/// Outcome of `discover`. An explicit enum rather than `Result<Catalog,
/// ConnectorError>` so the typed return crosses WIT and needs no host-side
/// `Result` special-casing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wit", derive(fidius_macro::WitType))]
pub enum DiscoverOutcome {
    Catalog(Catalog),
    Error(ConnectorError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_outcome_round_trips() {
        let out = DiscoverOutcome::Catalog(Catalog {
            streams: vec![StreamInfo {
                name: "orders".into(),
                namespace: Some("public".into()),
                schema: ArrowSchemaIpc { ipc: vec![1, 2] },
                supported_sync_modes: vec![SyncMode::FullRefresh, SyncMode::Incremental],
                source_defined_cursor: true,
                default_cursor_field: Some("updated_at".into()),
                source_defined_primary_key: Some(vec!["id".into()]),
                partitioning: PartitionScheme::Unpartitioned,
            }],
        });
        let bytes = encode(&out).unwrap();
        assert_eq!(decode::<DiscoverOutcome>(&bytes).unwrap(), out);

        let err = DiscoverOutcome::Error(ConnectorError::config("no host"));
        let bytes = encode(&err).unwrap();
        assert_eq!(decode::<DiscoverOutcome>(&bytes).unwrap(), err);
    }

    #[test]
    fn read_context_and_outcome_round_trip() {
        let ctx = ReadContext {
            stream: ConfiguredStream {
                stream: "orders".into(),
                sync_mode: SyncMode::Incremental,
                cursor_field: Some("updated_at".into()),
                primary_key: Some(vec!["id".into()]),
                write_mode: WriteMode::Upsert {
                    business_keys: vec!["id".into()],
                },
                mapping: MappingSpec {
                    ops: vec![MappingOp::Rename {
                        from: "a".into(),
                        to: "b".into(),
                    }],
                },
            },
            partition: Partition {
                id: "0".into(),
                bounds: None,
            },
            state: StreamState::default(),
        };
        let bytes = encode(&ctx).unwrap();
        assert_eq!(decode::<ReadContext>(&bytes).unwrap(), ctx);

        let out = WriteOutcome {
            state: StreamState {
                cursor: Some("\"2026-10-16\"".into()),
                opaque: vec![],
            },
            diagnostics: vec![LogEntry {
                level: LogLevel::Info,
                message: "ok".into(),
            }],
            dead_letters: vec![DeadLetter {
                record: "{}".into(),
                reason: "empty".into(),
            }],
            result: WriteResult::Ok(WriteReceipt { accepted: 9 }),
        };
        let bytes = encode(&out).unwrap();
        assert_eq!(decode::<WriteOutcome>(&bytes).unwrap(), out);
    }
}
