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

//! What a sync definition binds and what one execution carries
//! (COLLIERY-I-0632 / CLOACINA-T-0976).
//!
//! A sync definition is a workflow instance of the built-in Sync workflow; its
//! params arrive as flat context keys when the dispatcher fires an execution.
//! Structured values travel as JSON text, the same way they cross the connector
//! boundary. The dispatcher adds the per-execution [`SegmentInput`]: the
//! committed stream state the read starts from. The three tasks pass the
//! segment along under the `sync.*` keys below.

use cloacina::Context;
use cloacina_constructor_contract::sync::{
    ConfiguredStream, DeadLetter, Partition, RecordBatch, StreamSchema, StreamState,
};
use serde::de::DeserializeOwned;

use crate::credential::CredentialSpec;
use serde_json::Value;

/// Instance params of a sync definition, by key.
pub mod keys {
    /// The sync's name: the workflow instance name. Keys the cursor rows.
    pub const SYNC: &str = "sync";
    /// The source connector provider (package name) and member.
    pub const SOURCE_PROVIDER: &str = "source_provider";
    pub const SOURCE_MEMBER: &str = "source_member";
    /// The source connector's config document, JSON text.
    pub const SOURCE_CONFIG: &str = "source_config";
    /// The source's capability grants, a JSON `GrantSpec`; absent means deny all.
    pub const SOURCE_GRANTS: &str = "source_grants";
    /// The source's credential, a JSON `CredentialSpec`; absent means none.
    /// It names a secret, never holds one.
    pub const SOURCE_CREDENTIAL: &str = "source_credential";
    pub const DESTINATION_PROVIDER: &str = "destination_provider";
    pub const DESTINATION_MEMBER: &str = "destination_member";
    pub const DESTINATION_CONFIG: &str = "destination_config";
    pub const DESTINATION_GRANTS: &str = "destination_grants";
    pub const DESTINATION_CREDENTIAL: &str = "destination_credential";
    /// The configured stream, a JSON `ConfiguredStream`.
    pub const STREAM: &str = "stream";
    /// The partition, a JSON `Partition`; absent means unpartitioned.
    pub const PARTITION: &str = "partition";
    /// The stream's typed schema, a JSON `StreamSchema`; absent means no
    /// enforcement.
    pub const SCHEMA: &str = "schema";
    /// How many connector checkpoints one segment may span. Default 1.
    pub const SEGMENT_CHECKPOINTS: &str = "segment_checkpoints";
    /// Test-only (feature `fault-injection`): a fault the write task injects.
    /// `fail_after_write_once` fails the task once per sync after the
    /// destination accepted the segment, before the completion writes, so the
    /// retry writes the segment again; `slow_write_ms:<n>` holds the write
    /// task for n milliseconds after the write, so a harness can kill the
    /// process running it. Ignored without the feature.
    pub const FAULT: &str = "fault";

    /// Per execution, set by the dispatcher: the committed `StreamState` the
    /// read starts from, JSON text. Absent means a fresh read.
    pub const STATE: &str = "sync.state";

    /// Between the tasks: the segment's batches (JSON `Vec<RecordBatch>`).
    pub const BATCHES: &str = "sync.batches";
    /// Between the tasks: the state after the segment (JSON `StreamState`).
    pub const NEXT_STATE: &str = "sync.next_state";
    /// Between the tasks: the source reported its end in this segment.
    pub const EXHAUSTED: &str = "sync.exhausted";
    /// Between the tasks: dead-letters so far (JSON `Vec<DeadLetter>`).
    pub const DEAD_LETTERS: &str = "sync.dead_letters";
    /// Written by the write task: rows the destination accepted.
    pub const ACCEPTED: &str = "sync.accepted";
}

/// A param was missing or malformed.
#[derive(Debug, thiserror::Error)]
pub enum ParamError {
    #[error("sync param `{0}` is missing")]
    Missing(&'static str),
    #[error("sync param `{key}` does not parse: {error}")]
    Malformed { key: &'static str, error: String },
}

/// One side of a sync: which connector member, with what config and grants.
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub provider: String,
    pub member: String,
    /// The connector's config document, JSON text.
    pub config: String,
    /// The capability grants, as a JSON `GrantSpec`; `None` means deny all.
    pub grants: Option<Value>,
    /// The credential the host applies to this side's allowed requests.
    pub credential: Option<CredentialSpec>,
}

/// The params of a sync definition as the tasks read them from the context.
#[derive(Debug, Clone)]
pub struct SyncParams {
    pub sync: String,
    pub source: Endpoint,
    pub destination: Endpoint,
    pub stream: ConfiguredStream,
    pub partition: Partition,
    pub schema: Option<StreamSchema>,
    pub segment_checkpoints: u64,
}

/// The per-execution input: where the read starts.
#[derive(Debug, Clone, Default)]
pub struct SegmentInput {
    pub state: StreamState,
}

fn string(ctx: &Context<Value>, key: &'static str) -> Result<String, ParamError> {
    match ctx.get(key) {
        None | Some(Value::Null) => Err(ParamError::Missing(key)),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(other) => Ok(other.to_string()),
    }
}

/// A JSON-text param, or a param already bound as a JSON value.
fn json<T: DeserializeOwned>(
    ctx: &Context<Value>,
    key: &'static str,
) -> Result<Option<T>, ParamError> {
    let Some(v) = ctx.get(key) else {
        return Ok(None);
    };
    let parsed = match v {
        Value::Null => return Ok(None),
        Value::String(s) if s.trim().is_empty() => return Ok(None),
        Value::String(s) => serde_json::from_str::<T>(s),
        other => serde_json::from_value::<T>(other.clone()),
    };
    parsed.map(Some).map_err(|e| ParamError::Malformed {
        key,
        error: e.to_string(),
    })
}

fn endpoint(
    ctx: &Context<Value>,
    provider: &'static str,
    member: &'static str,
    config: &'static str,
    grants: &'static str,
    credential: &'static str,
) -> Result<Endpoint, ParamError> {
    Ok(Endpoint {
        provider: string(ctx, provider)?,
        member: string(ctx, member)?,
        config: string(ctx, config).unwrap_or_else(|_| "{}".to_string()),
        grants: json::<Value>(ctx, grants)?,
        credential: json::<CredentialSpec>(ctx, credential)?,
    })
}

impl SyncParams {
    /// Read the sync definition's params from an execution's context.
    pub fn from_context(ctx: &Context<Value>) -> Result<Self, ParamError> {
        Ok(Self {
            sync: string(ctx, keys::SYNC)?,
            source: endpoint(
                ctx,
                keys::SOURCE_PROVIDER,
                keys::SOURCE_MEMBER,
                keys::SOURCE_CONFIG,
                keys::SOURCE_GRANTS,
                keys::SOURCE_CREDENTIAL,
            )?,
            destination: endpoint(
                ctx,
                keys::DESTINATION_PROVIDER,
                keys::DESTINATION_MEMBER,
                keys::DESTINATION_CONFIG,
                keys::DESTINATION_GRANTS,
                keys::DESTINATION_CREDENTIAL,
            )?,
            stream: json::<ConfiguredStream>(ctx, keys::STREAM)?
                .ok_or(ParamError::Missing(keys::STREAM))?,
            partition: json::<Partition>(ctx, keys::PARTITION)?.unwrap_or(Partition {
                id: String::new(),
                bounds: None,
            }),
            schema: json::<StreamSchema>(ctx, keys::SCHEMA)?,
            segment_checkpoints: match ctx.get(keys::SEGMENT_CHECKPOINTS) {
                None | Some(Value::Null) => 1,
                Some(Value::Number(n)) => n.as_u64().unwrap_or(1).max(1),
                Some(Value::String(s)) => s.trim().parse::<u64>().unwrap_or(1).max(1),
                Some(_) => 1,
            },
        })
    }
}

impl SegmentInput {
    /// The committed state the dispatcher put on the execution, or a fresh read.
    pub fn from_context(ctx: &Context<Value>) -> Result<Self, ParamError> {
        Ok(Self {
            state: json::<StreamState>(ctx, keys::STATE)?.unwrap_or_default(),
        })
    }
}

/// Read a `sync.*` hand-off value written by an earlier task.
pub fn handoff<T: DeserializeOwned>(
    ctx: &Context<Value>,
    key: &'static str,
) -> Result<Option<T>, ParamError> {
    json::<T>(ctx, key)
}

/// Write a `sync.*` hand-off value for a later task, replacing any earlier one.
pub fn set_handoff<T: serde::Serialize>(
    ctx: &mut Context<Value>,
    key: &str,
    value: &T,
) -> Result<(), cloacina_workflow::ContextError> {
    let json = serde_json::to_string(value)?;
    ctx.remove(key);
    ctx.insert(key, Value::String(json))
}

/// Convenience: the batches hand-off.
pub fn batches(ctx: &Context<Value>) -> Result<Vec<RecordBatch>, ParamError> {
    Ok(handoff::<Vec<RecordBatch>>(ctx, keys::BATCHES)?.unwrap_or_default())
}

/// Convenience: the dead-letters hand-off.
pub fn dead_letters(ctx: &Context<Value>) -> Result<Vec<DeadLetter>, ParamError> {
    Ok(handoff::<Vec<DeadLetter>>(ctx, keys::DEAD_LETTERS)?.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloacina_constructor_contract::sync::{MappingSpec, SyncMode, WriteMode};

    fn ctx() -> Context<Value> {
        let mut c = Context::new();
        c.insert(keys::SYNC, Value::String("orders".into()))
            .unwrap();
        c.insert(keys::SOURCE_PROVIDER, Value::String("pg".into()))
            .unwrap();
        c.insert(keys::SOURCE_MEMBER, Value::String("source".into()))
            .unwrap();
        c.insert(keys::SOURCE_CONFIG, Value::String(r#"{"dsn":"x"}"#.into()))
            .unwrap();
        c.insert(keys::DESTINATION_PROVIDER, Value::String("pg".into()))
            .unwrap();
        c.insert(
            keys::DESTINATION_MEMBER,
            Value::String("destination".into()),
        )
        .unwrap();
        let stream = ConfiguredStream {
            stream: "public.orders".into(),
            sync_mode: SyncMode::Incremental,
            cursor_field: Some("updated_at".into()),
            primary_key: Some(vec!["id".into()]),
            write_mode: WriteMode::Upsert {
                business_keys: vec!["id".into()],
            },
            mapping: MappingSpec::default(),
        };
        c.insert(
            keys::STREAM,
            Value::String(serde_json::to_string(&stream).unwrap()),
        )
        .unwrap();
        c.insert(keys::SEGMENT_CHECKPOINTS, Value::from(3)).unwrap();
        c
    }

    #[test]
    fn params_read_with_defaults() {
        let p = SyncParams::from_context(&ctx()).unwrap();
        assert_eq!(p.sync, "orders");
        assert_eq!(p.source.config, r#"{"dsn":"x"}"#);
        assert_eq!(
            p.destination.config, "{}",
            "config defaults to an empty document"
        );
        assert!(p.source.grants.is_none());
        assert_eq!(p.stream.stream, "public.orders");
        assert_eq!(p.partition.id, "");
        assert!(p.schema.is_none());
        assert_eq!(p.segment_checkpoints, 3);
        assert_eq!(
            SegmentInput::from_context(&ctx()).unwrap().state,
            StreamState::default()
        );
    }

    #[test]
    fn a_missing_stream_is_an_error_and_a_bad_one_names_the_key() {
        let mut c = ctx();
        c.remove(keys::STREAM);
        assert!(matches!(
            SyncParams::from_context(&c),
            Err(ParamError::Missing(keys::STREAM))
        ));
        c.insert(keys::STREAM, Value::String("not json".into()))
            .unwrap();
        match SyncParams::from_context(&c) {
            Err(ParamError::Malformed { key, .. }) => assert_eq!(key, keys::STREAM),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn handoffs_round_trip_and_replace() {
        let mut c = Context::new();
        let state = StreamState {
            cursor: Some("\"1\"".into()),
            opaque: vec![9],
        };
        set_handoff(&mut c, keys::NEXT_STATE, &state).unwrap();
        set_handoff(&mut c, keys::NEXT_STATE, &state).unwrap();
        assert_eq!(
            handoff::<StreamState>(&c, keys::NEXT_STATE).unwrap(),
            Some(state)
        );
        assert!(batches(&c).unwrap().is_empty());
    }
}
