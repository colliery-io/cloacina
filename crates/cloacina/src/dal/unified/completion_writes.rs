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

//! Applying a task's [`CompletionWrites`] inside its completion transaction
//! (COLLIERY-I-0632 / CLOACINA-T-0974, CLOACINA-A-0013 amendment 2).
//!
//! The body is a macro rather than a function so that it expands once per
//! backend arm inside `interact_on_backend!`, where `conn` is the concrete
//! connection type; a generic function would need every diesel bound spelled
//! out for both backends. It runs after the status update and only when that
//! update applied, so a lost claim writes nothing, and a failed write rolls
//! the status update back with it.

use cloacina_workflow::completion_writes::CompletionWrites;

/// Validate a payload before any statement runs: a row with no sync or stream
/// name has no home and must fail the whole transaction, not land as an
/// orphan.
pub fn validate(writes: &CompletionWrites) -> Result<(), String> {
    for s in &writes.stream_state {
        if s.sync.is_empty() || s.stream.is_empty() {
            return Err("stream state write with an empty sync or stream name".into());
        }
    }
    for d in &writes.dead_letters {
        if d.sync.is_empty() || d.stream.is_empty() {
            return Err("dead-letter write with an empty sync or stream name".into());
        }
    }
    for s in &writes.stream_schemas {
        if s.sync.is_empty() || s.stream.is_empty() {
            return Err("stream schema write with an empty sync or stream name".into());
        }
    }
    Ok(())
}

/// Apply `writes` on `conn` inside the caller's transaction. Expands to the
/// stream state upserts and the dead-letter inserts, stamped with the task
/// and workflow execution ids and `now`.
#[macro_export]
#[doc(hidden)]
macro_rules! apply_completion_writes {
    ($conn:expr, $writes:expr, $task_id:expr, $workflow_id:expr, $now:expr) => {{
        use diesel::prelude::*;
        use $crate::database::schema::unified::{
            sync_dead_letters, sync_stream_schemas, sync_stream_state,
        };
        use $crate::database::universal_types::{UniversalBinary, UniversalBool, UniversalUuid};

        if let Err(msg) = $crate::dal::unified::completion_writes::validate(&$writes) {
            return Err(diesel::result::Error::QueryBuilderError(msg.into()));
        }
        for s in &$writes.stream_state {
            let row = $crate::dal::unified::models::NewUnifiedSyncStreamState {
                id: UniversalUuid::new_v4(),
                sync_name: s.sync.clone(),
                stream: s.stream.clone(),
                partition_id: s.partition.clone(),
                cursor: s.cursor.clone(),
                opaque: UniversalBinary(s.opaque.clone()),
                exhausted: UniversalBool(s.exhausted),
                created_at: $now,
                updated_at: $now,
            };
            diesel::insert_into(sync_stream_state::table)
                .values(&row)
                .on_conflict((
                    sync_stream_state::sync_name,
                    sync_stream_state::stream,
                    sync_stream_state::partition_id,
                ))
                .do_update()
                .set((
                    sync_stream_state::cursor.eq(s.cursor.clone()),
                    sync_stream_state::opaque.eq(UniversalBinary(s.opaque.clone())),
                    sync_stream_state::exhausted.eq(UniversalBool(s.exhausted)),
                    sync_stream_state::updated_at.eq($now),
                ))
                .execute($conn)?;
        }
        for d in &$writes.dead_letters {
            let row = $crate::dal::unified::models::NewUnifiedSyncDeadLetter {
                id: UniversalUuid::new_v4(),
                sync_name: d.sync.clone(),
                stream: d.stream.clone(),
                task_execution_id: Some($task_id),
                workflow_execution_id: Some($workflow_id),
                record: d.record.clone(),
                reason: d.reason.clone(),
                created_at: $now,
            };
            diesel::insert_into(sync_dead_letters::table)
                .values(&row)
                .execute($conn)?;
        }
        for s in &$writes.stream_schemas {
            let row = $crate::dal::unified::models::NewUnifiedSyncStreamSchema {
                id: UniversalUuid::new_v4(),
                sync_name: s.sync.clone(),
                stream: s.stream.clone(),
                schema: s.schema.clone(),
                broken: s.broken.clone(),
                created_at: $now,
                updated_at: $now,
            };
            diesel::insert_into(sync_stream_schemas::table)
                .values(&row)
                .on_conflict((sync_stream_schemas::sync_name, sync_stream_schemas::stream))
                .do_update()
                .set((
                    sync_stream_schemas::schema.eq(s.schema.clone()),
                    sync_stream_schemas::broken.eq(s.broken.clone()),
                    sync_stream_schemas::updated_at.eq($now),
                ))
                .execute($conn)?;
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloacina_workflow::completion_writes::{DeadLetterWrite, StreamStateWrite};

    #[test]
    fn empty_names_are_rejected() {
        let mut w = CompletionWrites::default();
        assert!(validate(&w).is_ok());
        w.stream_state.push(StreamStateWrite {
            sync: "s".into(),
            stream: String::new(),
            ..Default::default()
        });
        assert!(validate(&w).is_err());
        let mut w = CompletionWrites::default();
        w.dead_letters.push(DeadLetterWrite {
            sync: String::new(),
            stream: "t".into(),
            record: "r".into(),
            reason: "x".into(),
        });
        assert!(validate(&w).is_err());
    }
}
