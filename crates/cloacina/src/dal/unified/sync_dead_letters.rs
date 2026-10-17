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

//! DAL for sync dead-letters (COLLIERY-I-0632 / CLOACINA-T-0974): the records
//! a sync could not process, with the reason. Reads only; the rows are written
//! by the task completion transaction ([`super::completion_writes::apply`]).

use super::DAL;
use super::models::UnifiedSyncDeadLetter;
use crate::database::schema::unified::sync_dead_letters;
use crate::error::ValidationError;
use diesel::prelude::*;

/// DAL accessor for `sync_dead_letters`.
pub struct SyncDeadLettersDAL<'a> {
    dal: &'a DAL,
}

impl<'a> SyncDeadLettersDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// The newest dead-letters of one sync, at most `limit`, newest first.
    pub async fn list_for_sync(
        &self,
        sync_name: &str,
        limit: i64,
    ) -> Result<Vec<UnifiedSyncDeadLetter>, ValidationError> {
        let sync_owned = sync_name.to_string();
        let rows = crate::interact_on_backend!(self.dal, |conn| {
            sync_dead_letters::table
                .filter(sync_dead_letters::sync_name.eq(&sync_owned))
                .order(sync_dead_letters::created_at.desc())
                .limit(limit)
                .select(UnifiedSyncDeadLetter::as_select())
                .load(conn)
        })?;
        Ok(rows)
    }

    /// How many dead-letters one sync holds.
    pub async fn count_for_sync(&self, sync_name: &str) -> Result<i64, ValidationError> {
        let sync_owned = sync_name.to_string();
        let n = crate::interact_on_backend!(self.dal, |conn| {
            sync_dead_letters::table
                .filter(sync_dead_letters::sync_name.eq(&sync_owned))
                .count()
                .get_result::<i64>(conn)
        })?;
        Ok(n)
    }

    /// Remove every dead-letter of one sync. Returns the number removed.
    pub async fn clear_sync(&self, sync_name: &str) -> Result<usize, ValidationError> {
        let sync_owned = sync_name.to_string();
        let n = crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(
                sync_dead_letters::table.filter(sync_dead_letters::sync_name.eq(&sync_owned)),
            )
            .execute(conn)
        })?;
        Ok(n)
    }
}
