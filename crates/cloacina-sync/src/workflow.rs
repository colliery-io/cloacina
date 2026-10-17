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

//! The built-in Sync workflow (COLLIERY-I-0632 / CLOACINA-T-0976,
//! CLOACINA-A-0013 amendment 2): three tasks that move one segment.
//!
//! 1. **read_segment** opens the source with the committed state the dispatcher
//!    put on the execution, reads until the connector's checkpoint (or the
//!    segment budget, at a checkpoint boundary), and hands the batches, the new
//!    state and the dead-letters on. The source ending without a trailing
//!    checkpoint flushes the records with the previous state; they are read
//!    again next pass and absorbed by upsert.
//! 2. **map_enforce** applies the stream's mapping, then its typed schema;
//!    records that fail either become dead-letters and the segment goes on.
//! 3. **write** streams the batches into the destination and, once accepted,
//!    returns the new cursor and the dead-letters as completion writes: the
//!    server writes them in the transaction that marks this task completed.
//!    The cursor never passes an unwritten record.
//!
//! Every connector call runs on a blocking thread: a wasmtime call blocks the
//! thread it runs on. Connectors are found under the process's provider search
//! path (`cloacina::registry::loader::provider_search_path`).

use chrono::Utc;
use cloacina::registry::loader::connector_host::{
    ConnectorHandle, ConnectorHostError, load_connector,
};
use cloacina::registry::loader::grants::{GrantSpec, ResolvedGrants, translate};
use cloacina::registry::loader::provider_search_path;
use cloacina::{Context, TaskError, task, workflow};
use cloacina_constructor_contract::sync::{
    Config, ConnectorError, DeadLetter, ErrorKind, LogLevel, ReadContext, ReadMessage, RecordBatch,
    StreamState, WriteContext, WriteResult,
};
use cloacina_workflow::completion_writes::{CompletionWrites, DeadLetterWrite, StreamStateWrite};
use serde_json::Value;

use crate::fault;
use crate::mapping::{self, Mapped};
use crate::params::{self, Endpoint, SegmentInput, SyncParams, keys};

/// What a segment's read produced.
#[derive(Debug, Default)]
pub struct Segment {
    pub batches: Vec<RecordBatch>,
    pub next_state: StreamState,
    pub exhausted: bool,
    pub dead_letters: Vec<DeadLetter>,
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error(transparent)]
    Param(#[from] params::ParamError),
    #[error("grants for {side} do not parse: {error}")]
    Grants { side: &'static str, error: String },
    #[error("load {side} connector `{provider}/{member}`: {error}")]
    Load {
        side: &'static str,
        provider: String,
        member: String,
        error: String,
    },
    #[error(transparent)]
    Host(#[from] ConnectorHostError),
    /// The connector reported a failure. `kind` decides what the host does:
    /// a `Transient` is worth a retry, the others are not.
    #[error("connector {kind:?}: {message}")]
    Connector { kind: ErrorKind, message: String },
    #[error("join: {0}")]
    Join(String),
}

impl From<ConnectorError> for SyncError {
    fn from(e: ConnectorError) -> Self {
        SyncError::Connector {
            kind: e.kind,
            message: e.message,
        }
    }
}

/// A `GrantSpec` from its JSON form: an object whose keys are the grant kinds
/// (`http`, `tcp`, `fs`, `env`, `secrets`) and whose values are lists of
/// patterns, the shape a package.toml `[grants]` table takes.
pub fn grant_spec_from_json(v: &Value) -> Result<GrantSpec, String> {
    let obj = v
        .as_object()
        .ok_or_else(|| "grants must be a JSON object of lists".to_string())?;
    let mut pairs = Vec::with_capacity(obj.len());
    for (kind, list) in obj {
        let items = list
            .as_array()
            .ok_or_else(|| format!("grants.{kind} must be a list of strings"))?
            .iter()
            .map(|x| {
                x.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("grants.{kind} holds a non-string entry"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        pairs.push((kind.clone(), items));
    }
    Ok(GrantSpec::from_pairs(pairs))
}

fn grants_of(side: &'static str, ep: &Endpoint) -> Result<ResolvedGrants, SyncError> {
    match &ep.grants {
        None => Ok(ResolvedGrants::deny_all()),
        Some(v) => {
            let spec =
                grant_spec_from_json(v).map_err(|error| SyncError::Grants { side, error })?;
            translate(&spec).map_err(|e| SyncError::Grants {
                side,
                error: e.to_string(),
            })
        }
    }
}

/// Load one side's connector member from the provider search path.
pub fn load_endpoint(side: &'static str, ep: &Endpoint) -> Result<ConnectorHandle, SyncError> {
    let grants = grants_of(side, ep)?;
    load_connector(
        provider_search_path(),
        &ep.provider,
        &ep.member,
        &Config {
            json: ep.config.clone(),
        },
        &grants,
    )
    .map_err(|e| SyncError::Load {
        side,
        provider: ep.provider.clone(),
        member: ep.member.clone(),
        error: e.to_string(),
    })
}

/// Read one segment from the source. Blocking: call from a blocking thread.
///
/// The segment closes at the `segment_checkpoints`-th checkpoint. The read
/// stream is dropped there, which tears the producer down; the next segment
/// reopens the source at the committed state.
pub fn read_segment_blocking(p: &SyncParams, input: SegmentInput) -> Result<Segment, SyncError> {
    let source = load_endpoint("source", &p.source)?;
    let ctx = ReadContext {
        stream: p.stream.clone(),
        partition: p.partition.clone(),
        state: input.state.clone(),
    };
    let mut seg = Segment {
        next_state: input.state,
        exhausted: true,
        ..Default::default()
    };
    let mut checkpoints = 0u64;
    let mut iter = source.read_blocking(&ctx)?;
    while let Some(item) = iter.next() {
        match item? {
            ReadMessage::Records(batch) => seg.batches.push(batch),
            ReadMessage::DeadLettered(d) => seg.dead_letters.push(d),
            ReadMessage::Log(entry) => match entry.level {
                LogLevel::Debug => tracing::debug!(sync = %p.sync, "{}", entry.message),
                LogLevel::Info => tracing::info!(sync = %p.sync, "{}", entry.message),
                LogLevel::Warn => tracing::warn!(sync = %p.sync, "{}", entry.message),
                LogLevel::Error => tracing::error!(sync = %p.sync, "{}", entry.message),
            },
            ReadMessage::Fatal(e) => return Err(e.into()),
            ReadMessage::Checkpoint(state) => {
                seg.next_state = state;
                checkpoints += 1;
                if checkpoints >= p.segment_checkpoints {
                    // The budget is spent. Look one message ahead: a source that
                    // ends here is exhausted, and the pass must say so, or the
                    // dispatcher would reopen it at this cursor forever. A source
                    // with more to say keeps `exhausted` false; the message we
                    // looked at is read again next segment from this state.
                    seg.exhausted = iter.next().is_none();
                    break;
                }
            }
        }
    }
    Ok(seg)
}

/// Apply the mapping, then the schema, to every record of every batch.
/// Records that fail either are returned as dead-letters.
pub fn map_and_enforce(
    p: &SyncParams,
    batches: Vec<RecordBatch>,
) -> (Vec<RecordBatch>, Vec<DeadLetter>) {
    let mut dead = Vec::new();
    let mut out = Vec::with_capacity(batches.len());
    let one = |row: String, dead: &mut Vec<DeadLetter>| -> Option<String> {
        let rec: Value = match serde_json::from_str(&row) {
            Ok(v) => v,
            Err(e) => {
                dead.push(DeadLetter {
                    record: row,
                    reason: format!("record is not JSON: {e}"),
                });
                return None;
            }
        };
        let mapped = match mapping::apply(&p.stream.mapping, rec) {
            Mapped::Keep(v) => v,
            Mapped::Filtered => return None,
            Mapped::DeadLetter(reason) => {
                dead.push(DeadLetter {
                    record: row,
                    reason,
                });
                return None;
            }
        };
        let enforced = match &p.schema {
            None => mapped,
            Some(schema) => match mapping::enforce(schema, mapped) {
                Mapped::Keep(v) => v,
                Mapped::Filtered => return None,
                Mapped::DeadLetter(reason) => {
                    dead.push(DeadLetter {
                        record: row,
                        reason,
                    });
                    return None;
                }
            },
        };
        Some(enforced.to_string())
    };
    for batch in batches {
        match batch {
            RecordBatch::Rows(rows) => {
                let kept: Vec<String> =
                    rows.into_iter().filter_map(|r| one(r, &mut dead)).collect();
                if !kept.is_empty() {
                    out.push(RecordBatch::Rows(kept));
                }
            }
            RecordBatch::Changes(changes) => {
                let kept: Vec<_> = changes
                    .into_iter()
                    .filter_map(|c| {
                        one(c.data, &mut dead).map(|data| {
                            cloacina_constructor_contract::sync::ChangeRecord { op: c.op, data }
                        })
                    })
                    .collect();
                if !kept.is_empty() {
                    out.push(RecordBatch::Changes(kept));
                }
            }
            arrow @ RecordBatch::Arrow(_) => out.push(arrow),
        }
    }
    (out, dead)
}

/// Write the batches to the destination. Blocking. Returns rows accepted and
/// the destination's dead-letters.
pub fn write_blocking(
    p: &SyncParams,
    batches: Vec<RecordBatch>,
) -> Result<(u64, Vec<DeadLetter>), SyncError> {
    let destination = load_endpoint("destination", &p.destination)?;
    let outcome = destination.write(
        &WriteContext {
            stream: p.stream.clone(),
        },
        batches,
    )?;
    for d in &outcome.diagnostics {
        tracing::info!(sync = %p.sync, "{}", d.message);
    }
    match outcome.result {
        WriteResult::Ok(receipt) => Ok((receipt.accepted, outcome.dead_letters)),
        WriteResult::Err(e) => Err(e.into()),
    }
}

fn task_err(task_id: &str, e: impl std::fmt::Display) -> TaskError {
    TaskError::ExecutionFailed {
        message: e.to_string(),
        task_id: task_id.to_string(),
        timestamp: Utc::now(),
    }
}

/// The built-in Sync workflow. A sync definition is an instance of it; the
/// dispatcher fires one execution per segment with `sync.state` set.
#[workflow(
    name = "sync",
    description = "Move one segment of records from a source to a destination through sandboxed connectors",
    author = "Colliery Software",
    params(
        sync: String,
        source_provider: String,
        source_member: String,
        source_config: String = "{}",
        source_grants: String = "",
        destination_provider: String,
        destination_member: String,
        destination_config: String = "{}",
        destination_grants: String = "",
        stream: String,
        partition: String = "",
        schema: String = "",
        segment_checkpoints: i64 = 1,
        fault: String = ""
    )
)]
pub mod sync {
    use super::*;

    /// Open the source at the committed state and read one segment.
    #[task(id = "read_segment", dependencies = [], retry_attempts = 3)]
    pub async fn read_segment(context: &mut Context<Value>) -> Result<(), TaskError> {
        let p = SyncParams::from_context(context).map_err(|e| task_err("read_segment", e))?;
        let input = SegmentInput::from_context(context).map_err(|e| task_err("read_segment", e))?;
        let seg = tokio::task::spawn_blocking(move || read_segment_blocking(&p, input))
            .await
            .map_err(|e| task_err("read_segment", SyncError::Join(e.to_string())))?
            .map_err(|e| task_err("read_segment", e))?;
        params::set_handoff(context, keys::BATCHES, &seg.batches)?;
        params::set_handoff(context, keys::NEXT_STATE, &seg.next_state)?;
        params::set_handoff(context, keys::DEAD_LETTERS, &seg.dead_letters)?;
        context.remove(keys::EXHAUSTED);
        context.insert(keys::EXHAUSTED, Value::Bool(seg.exhausted))?;
        tracing::info!(
            batches = seg.batches.len(),
            dead_letters = seg.dead_letters.len(),
            exhausted = seg.exhausted,
            "sync: segment read"
        );
        Ok(())
    }

    /// Apply the mapping and the typed schema; failures become dead-letters.
    #[task(id = "map_enforce", dependencies = ["read_segment"])]
    pub async fn map_enforce(context: &mut Context<Value>) -> Result<(), TaskError> {
        let p = SyncParams::from_context(context).map_err(|e| task_err("map_enforce", e))?;
        let batches = params::batches(context).map_err(|e| task_err("map_enforce", e))?;
        let mut dead = params::dead_letters(context).map_err(|e| task_err("map_enforce", e))?;
        let (mapped, more_dead) = map_and_enforce(&p, batches);
        dead.extend(more_dead);
        params::set_handoff(context, keys::BATCHES, &mapped)?;
        params::set_handoff(context, keys::DEAD_LETTERS, &dead)?;
        Ok(())
    }

    /// Write the segment, then hand the cursor and the dead-letters to the
    /// completion transaction.
    #[task(id = "write", dependencies = ["map_enforce"], retry_attempts = 3)]
    pub async fn write(context: &mut Context<Value>) -> Result<(), TaskError> {
        let p = SyncParams::from_context(context).map_err(|e| task_err("write", e))?;
        let batches = params::batches(context).map_err(|e| task_err("write", e))?;
        let mut dead = params::dead_letters(context).map_err(|e| task_err("write", e))?;
        let next_state: StreamState = params::handoff(context, keys::NEXT_STATE)
            .map_err(|e| task_err("write", e))?
            .unwrap_or_default();
        let exhausted = matches!(context.get(keys::EXHAUSTED), Some(Value::Bool(true)));

        let accepted = if batches.is_empty() {
            0
        } else {
            let p2 = p.clone();
            let (accepted, more_dead) =
                tokio::task::spawn_blocking(move || write_blocking(&p2, batches))
                    .await
                    .map_err(|e| task_err("write", SyncError::Join(e.to_string())))?
                    .map_err(|e| task_err("write", e))?;
            dead.extend(more_dead);
            accepted
        };

        // Test-only faults, between the durable write and the completion writes:
        // exactly the window a crash must not lose a record in.
        fault::after_write(context)
            .await
            .map_err(|e| task_err("write", e))?;

        // The durable write is done. The cursor and the dead-letters go to the
        // transaction that marks this task completed.
        let writes = CompletionWrites {
            stream_state: vec![StreamStateWrite {
                sync: p.sync.clone(),
                stream: p.stream.stream.clone(),
                partition: p.partition.id.clone(),
                cursor: next_state.cursor.clone(),
                opaque: next_state.opaque.clone(),
                exhausted,
            }],
            dead_letters: dead
                .iter()
                .map(|d| DeadLetterWrite {
                    sync: p.sync.clone(),
                    stream: p.stream.stream.clone(),
                    record: d.record.clone(),
                    reason: d.reason.clone(),
                })
                .collect(),
        };
        writes.attach(context)?;
        context.remove(keys::ACCEPTED);
        context.insert(keys::ACCEPTED, Value::from(accepted))?;
        // The batches have landed; do not carry them into the saved context.
        context.remove(keys::BATCHES);
        tracing::info!(sync = %p.sync, accepted, dead_letters = dead.len(), exhausted, "sync: segment written");
        Ok(())
    }
}
