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

//! DAL for pending boundary WAL and edge drain cursors (continuous scheduling).

use super::models::{
    EdgeDrainCursorRow, NewEdgeDrainCursor, NewPendingBoundary, PendingBoundaryRow,
};
use super::DAL;
use crate::database::schema::unified::{edge_drain_cursors, pending_boundaries};
use diesel::prelude::*;

/// Data access layer for pending boundary and edge drain cursor operations.
#[derive(Clone)]
pub struct PendingBoundaryDAL<'a> {
    dal: &'a DAL,
}

impl<'a> PendingBoundaryDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Append a boundary to the WAL, returning the new row ID.
    pub async fn append(&self, source_name: String, boundary_json: String) -> Result<i64, String> {
        let boundary = NewPendingBoundary {
            source_name,
            boundary_json,
        };
        crate::dispatch_backend!(
            self.dal.backend(),
            self.append_postgres(boundary.clone()).await,
            self.append_sqlite(boundary).await
        )
    }

    /// Load all boundaries after a cursor position for a source.
    pub async fn load_after_cursor(
        &self,
        source_name: String,
        cursor_id: i64,
    ) -> Result<Vec<PendingBoundaryRow>, String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.load_after_cursor_postgres(source_name.clone(), cursor_id)
                .await,
            self.load_after_cursor_sqlite(source_name, cursor_id).await
        )
    }

    /// Advance the drain cursor for an edge.
    pub async fn advance_cursor(&self, edge_id: String, drain_id: i64) -> Result<(), String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.advance_cursor_postgres(edge_id.clone(), drain_id)
                .await,
            self.advance_cursor_sqlite(edge_id, drain_id).await
        )
    }

    /// Load the last drain ID for an edge cursor.
    pub async fn load_cursor(&self, edge_id: String) -> Result<i64, String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.load_cursor_postgres(edge_id.clone()).await,
            self.load_cursor_sqlite(edge_id).await
        )
    }

    /// Initialize a drain cursor for an edge (no-op if already exists).
    pub async fn init_cursor(&self, edge_id: String, source_name: String) -> Result<(), String> {
        let cursor = NewEdgeDrainCursor {
            edge_id,
            source_name,
            last_drain_id: 0,
        };
        crate::dispatch_backend!(
            self.dal.backend(),
            self.init_cursor_postgres(cursor.clone()).await,
            self.init_cursor_sqlite(cursor).await
        )
    }

    /// Get the minimum cursor position across all edges for a source.
    pub async fn min_cursor_for_source(&self, source_name: String) -> Result<i64, String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.min_cursor_for_source_postgres(source_name.clone())
                .await,
            self.min_cursor_for_source_sqlite(source_name).await
        )
    }

    /// Get the maximum boundary ID for a source (most recent boundary).
    pub async fn max_id_for_source(&self, source_name: String) -> Result<Option<i64>, String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.max_id_for_source_postgres(source_name.clone()).await,
            self.max_id_for_source_sqlite(source_name).await
        )
    }

    /// Delete consumed boundaries up to (and including) a given ID for a source.
    pub async fn cleanup(&self, source_name: String, up_to_id: i64) -> Result<usize, String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.cleanup_postgres(source_name.clone(), up_to_id).await,
            self.cleanup_sqlite(source_name, up_to_id).await
        )
    }

    /// Load all edge drain cursors.
    pub async fn load_all_cursors(&self) -> Result<Vec<EdgeDrainCursorRow>, String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.load_all_cursors_postgres().await,
            self.load_all_cursors_sqlite().await
        )
    }

    // =========================================================================
    // Postgres implementations
    // =========================================================================

    #[cfg(feature = "postgres")]
    async fn append_postgres(&self, boundary: NewPendingBoundary) -> Result<i64, String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::insert_into(pending_boundaries::table)
                .values(&boundary)
                .returning(pending_boundaries::id)
                .get_result::<i64>(conn)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn load_after_cursor_postgres(
        &self,
        source_name: String,
        cursor_id: i64,
    ) -> Result<Vec<PendingBoundaryRow>, String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            pending_boundaries::table
                .filter(pending_boundaries::source_name.eq(&source_name))
                .filter(pending_boundaries::id.gt(cursor_id))
                .order(pending_boundaries::id.asc())
                .load::<PendingBoundaryRow>(conn)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn advance_cursor_postgres(&self, edge_id: String, drain_id: i64) -> Result<(), String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::update(
                edge_drain_cursors::table.filter(edge_drain_cursors::edge_id.eq(&edge_id)),
            )
            .set(edge_drain_cursors::last_drain_id.eq(drain_id))
            .execute(conn)
            .map(|_| ())
            .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn load_cursor_postgres(&self, edge_id: String) -> Result<i64, String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            edge_drain_cursors::table
                .filter(edge_drain_cursors::edge_id.eq(&edge_id))
                .select(edge_drain_cursors::last_drain_id)
                .first::<i64>(conn)
                .optional()
                .map(|opt| opt.unwrap_or(0))
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn init_cursor_postgres(&self, cursor: NewEdgeDrainCursor) -> Result<(), String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::insert_into(edge_drain_cursors::table)
                .values(&cursor)
                .on_conflict(edge_drain_cursors::edge_id)
                .do_nothing()
                .execute(conn)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn min_cursor_for_source_postgres(&self, source_name: String) -> Result<i64, String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            use diesel::dsl::min;
            edge_drain_cursors::table
                .filter(edge_drain_cursors::source_name.eq(&source_name))
                .select(min(edge_drain_cursors::last_drain_id))
                .first::<Option<i64>>(conn)
                .map(|opt| opt.unwrap_or(0))
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn max_id_for_source_postgres(&self, source_name: String) -> Result<Option<i64>, String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            use diesel::dsl::max;
            pending_boundaries::table
                .filter(pending_boundaries::source_name.eq(&source_name))
                .select(max(pending_boundaries::id))
                .first::<Option<i64>>(conn)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn cleanup_postgres(&self, source_name: String, up_to_id: i64) -> Result<usize, String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::delete(
                pending_boundaries::table
                    .filter(pending_boundaries::source_name.eq(&source_name))
                    .filter(pending_boundaries::id.le(up_to_id)),
            )
            .execute(conn)
            .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn load_all_cursors_postgres(&self) -> Result<Vec<EdgeDrainCursorRow>, String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            edge_drain_cursors::table
                .load::<EdgeDrainCursorRow>(conn)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    // =========================================================================
    // SQLite implementations
    // =========================================================================

    #[cfg(feature = "sqlite")]
    async fn append_sqlite(&self, boundary: NewPendingBoundary) -> Result<i64, String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::insert_into(pending_boundaries::table)
                .values(&boundary)
                .execute(conn)
                .map_err(|e| e.to_string())?;

            // Get the last inserted rowid
            diesel::select(diesel::dsl::sql::<diesel::sql_types::BigInt>(
                "last_insert_rowid()",
            ))
            .get_result::<i64>(conn)
            .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn load_after_cursor_sqlite(
        &self,
        source_name: String,
        cursor_id: i64,
    ) -> Result<Vec<PendingBoundaryRow>, String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            pending_boundaries::table
                .filter(pending_boundaries::source_name.eq(&source_name))
                .filter(pending_boundaries::id.gt(cursor_id))
                .order(pending_boundaries::id.asc())
                .load::<PendingBoundaryRow>(conn)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn advance_cursor_sqlite(&self, edge_id: String, drain_id: i64) -> Result<(), String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::update(
                edge_drain_cursors::table.filter(edge_drain_cursors::edge_id.eq(&edge_id)),
            )
            .set(edge_drain_cursors::last_drain_id.eq(drain_id))
            .execute(conn)
            .map(|_| ())
            .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn load_cursor_sqlite(&self, edge_id: String) -> Result<i64, String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            edge_drain_cursors::table
                .filter(edge_drain_cursors::edge_id.eq(&edge_id))
                .select(edge_drain_cursors::last_drain_id)
                .first::<i64>(conn)
                .optional()
                .map(|opt| opt.unwrap_or(0))
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn init_cursor_sqlite(&self, cursor: NewEdgeDrainCursor) -> Result<(), String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::insert_or_ignore_into(edge_drain_cursors::table)
                .values(&cursor)
                .execute(conn)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn min_cursor_for_source_sqlite(&self, source_name: String) -> Result<i64, String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            use diesel::dsl::min;
            edge_drain_cursors::table
                .filter(edge_drain_cursors::source_name.eq(&source_name))
                .select(min(edge_drain_cursors::last_drain_id))
                .first::<Option<i64>>(conn)
                .map(|opt| opt.unwrap_or(0))
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn max_id_for_source_sqlite(&self, source_name: String) -> Result<Option<i64>, String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            use diesel::dsl::max;
            pending_boundaries::table
                .filter(pending_boundaries::source_name.eq(&source_name))
                .select(max(pending_boundaries::id))
                .first::<Option<i64>>(conn)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn cleanup_sqlite(&self, source_name: String, up_to_id: i64) -> Result<usize, String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::delete(
                pending_boundaries::table
                    .filter(pending_boundaries::source_name.eq(&source_name))
                    .filter(pending_boundaries::id.le(up_to_id)),
            )
            .execute(conn)
            .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn load_all_cursors_sqlite(&self) -> Result<Vec<EdgeDrainCursorRow>, String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            edge_drain_cursors::table
                .load::<EdgeDrainCursorRow>(conn)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
