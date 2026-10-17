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

//! Completion writes: rows a task asks the host to write inside the same
//! transaction that marks the task completed (COLLIERY-I-0632, CLOACINA-A-0013
//! amendment 2).
//!
//! A task returns only a [`Context`]. When a task must leave a durable mark that
//! is only true once its work is done, and must never be visible if the task is
//! not marked completed, it attaches a [`CompletionWrites`] under the reserved
//! key [`COMPLETION_WRITES_KEY`]. The result handler strips the key before the
//! context is saved and applies the writes in the completion transaction: if a
//! write fails, the task is not completed, and if the claim was lost, nothing is
//! written. One mechanism serves the in-process executor, the FFI and the agent,
//! because all three carry the context as JSON.
//!
//! The first user is Sync's write task: after the destination has durably
//! written a segment, the task returns the stream's new cursor and the segment's
//! dead-letters here, so the cursor never advances past an unwritten record.
//!
//! The key never reaches a saved context, the fires log or execution history.

use crate::context::Context;
use crate::error::ContextError;
use serde::{Deserialize, Serialize};

/// Reserved [`Context`] data key under which a task attaches its
/// [`CompletionWrites`]. Stripped by the result handler before the context is
/// saved.
pub const COMPLETION_WRITES_KEY: &str = "__cloacina_completion_writes__";

/// The rows to write in the task's completion transaction.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CompletionWrites {
    /// Stream state upserts, one per (sync, stream, partition).
    #[serde(default)]
    pub stream_state: Vec<StreamStateWrite>,
    /// Dead-letter inserts.
    #[serde(default)]
    pub dead_letters: Vec<DeadLetterWrite>,
}

/// Upsert of a sync stream's state: the committed cursor and the connector's
/// opaque resume bytes, and whether the source reported its end at this state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamStateWrite {
    /// The sync's name (its workflow instance name).
    pub sync: String,
    /// The stream within the source.
    pub stream: String,
    /// The partition within the stream; empty for an unpartitioned stream.
    #[serde(default)]
    pub partition: String,
    /// The cursor as JSON text; `None` for a fresh stream.
    #[serde(default)]
    pub cursor: Option<String>,
    /// Connector-owned resume bytes.
    #[serde(default)]
    pub opaque: Vec<u8>,
    /// The source reported end of stream at this state: the pass is done.
    #[serde(default)]
    pub exhausted: bool,
}

/// One record a sync could not process, with the reason. The host stamps the
/// task and workflow execution it came from.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DeadLetterWrite {
    pub sync: String,
    pub stream: String,
    /// The offending record, raw row text or an identifier for it.
    pub record: String,
    pub reason: String,
}

impl CompletionWrites {
    /// No rows to write.
    pub fn is_empty(&self) -> bool {
        self.stream_state.is_empty() && self.dead_letters.is_empty()
    }

    /// Attach these writes to `context` under [`COMPLETION_WRITES_KEY`],
    /// replacing any writes already attached.
    pub fn attach(&self, context: &mut Context<serde_json::Value>) -> Result<(), ContextError> {
        let value = serde_json::to_value(self)?;
        context.remove(COMPLETION_WRITES_KEY);
        context.insert(COMPLETION_WRITES_KEY, value)
    }

    /// Remove the writes from `context`, if any. The host calls this before the
    /// context is saved. A payload that does not parse is an error: the task
    /// asked for writes the host cannot make, so it must not complete.
    pub fn take_from(
        context: &mut Context<serde_json::Value>,
    ) -> Result<Option<Self>, ContextError> {
        match context.remove(COMPLETION_WRITES_KEY) {
            None => Ok(None),
            Some(value) => Ok(Some(serde_json::from_value(value)?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attach_then_take_round_trips_and_strips_the_key() {
        let writes = CompletionWrites {
            stream_state: vec![StreamStateWrite {
                sync: "orders".into(),
                stream: "public.orders".into(),
                partition: String::new(),
                cursor: Some("\"2026-10-16\"".into()),
                opaque: vec![1, 2],
                exhausted: false,
            }],
            dead_letters: vec![DeadLetterWrite {
                sync: "orders".into(),
                stream: "public.orders".into(),
                record: "{}".into(),
                reason: "empty".into(),
            }],
        };
        let mut ctx = Context::new();
        ctx.insert("kept", serde_json::json!(1)).unwrap();
        writes.attach(&mut ctx).unwrap();
        assert!(ctx.get(COMPLETION_WRITES_KEY).is_some());

        let taken = CompletionWrites::take_from(&mut ctx).unwrap().unwrap();
        assert_eq!(taken, writes);
        assert!(ctx.get(COMPLETION_WRITES_KEY).is_none());
        assert_eq!(ctx.get("kept"), Some(&serde_json::json!(1)));
        assert!(CompletionWrites::take_from(&mut ctx).unwrap().is_none());
    }

    #[test]
    fn attach_twice_replaces() {
        let mut ctx = Context::new();
        CompletionWrites::default().attach(&mut ctx).unwrap();
        let second = CompletionWrites {
            dead_letters: vec![DeadLetterWrite::default()],
            ..Default::default()
        };
        second.attach(&mut ctx).unwrap();
        assert_eq!(
            CompletionWrites::take_from(&mut ctx).unwrap().unwrap(),
            second
        );
    }

    #[test]
    fn a_payload_that_does_not_parse_is_an_error() {
        let mut ctx = Context::new();
        ctx.insert(COMPLETION_WRITES_KEY, serde_json::json!("not an object"))
            .unwrap();
        assert!(CompletionWrites::take_from(&mut ctx).is_err());
    }
}
