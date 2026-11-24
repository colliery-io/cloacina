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

//! DAL for the committed state of sync streams (COLLIERY-I-0632 /
//! CLOACINA-T-0974).
//!
//! Reads only. The rows are written by the task completion transaction
//! ([`super::completion_writes::apply`] inside
//! `TaskExecutionDAL::mark_completed_with`), never by a standalone upsert, so a
//! cursor can only advance together with the completion of the task that
//! wrote the segment. The dispatcher reads the state here to open the next
//! segment and to learn that a pass is done (`exhausted`).

use super::DAL;
use super::models::UnifiedSyncStreamState;
use crate::database::schema::unified::sync_stream_state;
use crate::error::ValidationError;
use diesel::prelude::*;

/// DAL accessor for `sync_stream_state`.
pub struct SyncStreamStateDAL<'a> {
    dal: &'a DAL,
}

impl<'a> SyncStreamStateDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// The committed state of one (sync, stream, partition), or `None` for a
    /// stream that has never completed a segment.
    pub async fn get(
        &self,
        sync_name: &str,
        stream: &str,
        partition_id: &str,
    ) -> Result<Option<UnifiedSyncStreamState>, ValidationError> {
        let sync_owned = sync_name.to_string();
        let stream_owned = stream.to_string();
        let partition_owned = partition_id.to_string();
        let row = crate::interact_on_backend!(self.dal, |conn| {
            sync_stream_state::table
                .filter(sync_stream_state::sync_name.eq(&sync_owned))
                .filter(sync_stream_state::stream.eq(&stream_owned))
                .filter(sync_stream_state::partition_id.eq(&partition_owned))
                .select(UnifiedSyncStreamState::as_select())
                .first(conn)
                .optional()
        })?;
        Ok(row)
    }

    /// Every stream state of one sync, ordered by stream then partition.
    pub async fn list_for_sync(
        &self,
        sync_name: &str,
    ) -> Result<Vec<UnifiedSyncStreamState>, ValidationError> {
        let sync_owned = sync_name.to_string();
        let rows = crate::interact_on_backend!(self.dal, |conn| {
            sync_stream_state::table
                .filter(sync_stream_state::sync_name.eq(&sync_owned))
                .order((
                    sync_stream_state::stream.asc(),
                    sync_stream_state::partition_id.asc(),
                ))
                .select(UnifiedSyncStreamState::as_select())
                .load(conn)
        })?;
        Ok(rows)
    }

    /// Forget a sync's state, every stream and partition: the next pass starts
    /// from a fresh read. Returns the number of rows removed.
    pub async fn reset_sync(&self, sync_name: &str) -> Result<usize, ValidationError> {
        let sync_owned = sync_name.to_string();
        let n = crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(
                sync_stream_state::table.filter(sync_stream_state::sync_name.eq(&sync_owned)),
            )
            .execute(conn)
        })?;
        Ok(n)
    }
}
