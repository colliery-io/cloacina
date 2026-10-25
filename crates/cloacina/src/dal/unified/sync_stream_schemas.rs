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
//! DAL for the captured schemas of sync streams (COLLIERY-I-0635 /
//! CLOACINA-T-1003).
//!
//! The write task's completion transaction stores each evolved schema
//! ([`super::completion_writes`]), next to the cursor. The dispatcher reads
//! the stored one and hands it to the next segment. An operator's "accept"
//! is the one write outside a completion: it empties the stored schema, so
//! the next write captures the records' shape as new fields.

use super::DAL;
use super::models::{NewUnifiedSyncStreamSchema, UnifiedSyncStreamSchema};
use crate::database::schema::unified::sync_stream_schemas;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use diesel::prelude::*;

/// The schema "accept" leaves: no fields, so every field the next records
/// carry is new, and nothing is enforced until then.
pub const ACCEPTED_SCHEMA: &str = r#"{"fields":[]}"#;

/// DAL accessor for `sync_stream_schemas`.
pub struct SyncStreamSchemasDAL<'a> {
    dal: &'a DAL,
}

impl<'a> SyncStreamSchemasDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// The stored schema of one (sync, stream), or `None` before its first
    /// write.
    pub async fn get(
        &self,
        sync_name: &str,
        stream: &str,
    ) -> Result<Option<UnifiedSyncStreamSchema>, ValidationError> {
        let sync_owned = sync_name.to_string();
        let stream_owned = stream.to_string();
        let row = crate::interact_on_backend!(self.dal, |conn| {
            sync_stream_schemas::table
                .filter(sync_stream_schemas::sync_name.eq(&sync_owned))
                .filter(sync_stream_schemas::stream.eq(&stream_owned))
                .select(UnifiedSyncStreamSchema::as_select())
                .first(conn)
                .optional()
        })?;
        Ok(row)
    }

    /// Every stored schema of one sync, by stream.
    pub async fn list_for_sync(
        &self,
        sync_name: &str,
    ) -> Result<Vec<UnifiedSyncStreamSchema>, ValidationError> {
        let sync_owned = sync_name.to_string();
        let rows = crate::interact_on_backend!(self.dal, |conn| {
            sync_stream_schemas::table
                .filter(sync_stream_schemas::sync_name.eq(&sync_owned))
                .order(sync_stream_schemas::stream.asc())
                .select(UnifiedSyncStreamSchema::as_select())
                .load(conn)
        })?;
        Ok(rows)
    }

    /// Accept the records' new shape: empty the stored schema and clear
    /// `broken`. The next write captures the shape it sees.
    pub async fn accept(&self, sync_name: &str, stream: &str) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        let row = NewUnifiedSyncStreamSchema {
            id: UniversalUuid::new_v4(),
            sync_name: sync_name.to_string(),
            stream: stream.to_string(),
            schema: ACCEPTED_SCHEMA.to_string(),
            broken: None,
            created_at: now,
            updated_at: now,
        };
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(sync_stream_schemas::table)
                .values(&row)
                .on_conflict((sync_stream_schemas::sync_name, sync_stream_schemas::stream))
                .do_update()
                .set((
                    sync_stream_schemas::schema.eq(ACCEPTED_SCHEMA),
                    sync_stream_schemas::broken.eq(None::<String>),
                    sync_stream_schemas::updated_at.eq(now),
                ))
                .execute(conn)
        })?;
        Ok(())
    }
}
