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

//! DAL for accumulator state persistence (continuous scheduling).

use super::models::{AccumulatorStateRow, NewAccumulatorState};
use super::DAL;
use crate::database::schema::unified::accumulator_state;
use diesel::prelude::*;

/// Data access layer for accumulator state operations.
#[derive(Clone)]
pub struct AccumulatorStateDAL<'a> {
    dal: &'a DAL,
}

impl<'a> AccumulatorStateDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Save or update accumulator state for an edge.
    pub async fn save(&self, state: NewAccumulatorState) -> Result<(), String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.save_postgres(state.clone()).await,
            self.save_sqlite(state).await
        )
    }

    /// Load accumulator state for a specific edge.
    pub async fn load(&self, edge_id: &str) -> Result<Option<AccumulatorStateRow>, String> {
        let edge_id = edge_id.to_string();
        crate::dispatch_backend!(
            self.dal.backend(),
            self.load_postgres(edge_id.clone()).await,
            self.load_sqlite(edge_id).await
        )
    }

    /// Load all persisted accumulator states.
    pub async fn load_all(&self) -> Result<Vec<AccumulatorStateRow>, String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.load_all_postgres().await,
            self.load_all_sqlite().await
        )
    }

    /// Delete accumulator states by edge IDs.
    pub async fn delete_by_ids(&self, edge_ids: Vec<String>) -> Result<usize, String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.delete_postgres(edge_ids.clone()).await,
            self.delete_sqlite(edge_ids).await
        )
    }

    // --- Postgres implementations ---

    #[cfg(feature = "postgres")]
    async fn save_postgres(&self, state: NewAccumulatorState) -> Result<(), String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::insert_into(accumulator_state::table)
                .values(&state)
                .on_conflict(accumulator_state::edge_id)
                .do_update()
                .set((
                    accumulator_state::consumer_watermark.eq(state.consumer_watermark.as_deref()),
                    accumulator_state::drain_metadata.eq(&state.drain_metadata),
                ))
                .execute(conn)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn load_postgres(&self, edge_id: String) -> Result<Option<AccumulatorStateRow>, String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            accumulator_state::table
                .filter(accumulator_state::edge_id.eq(&edge_id))
                .first::<AccumulatorStateRow>(conn)
                .optional()
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn load_all_postgres(&self) -> Result<Vec<AccumulatorStateRow>, String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            accumulator_state::table
                .load::<AccumulatorStateRow>(conn)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn delete_postgres(&self, edge_ids: Vec<String>) -> Result<usize, String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::delete(
                accumulator_state::table.filter(accumulator_state::edge_id.eq_any(&edge_ids)),
            )
            .execute(conn)
            .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    // --- SQLite implementations ---

    #[cfg(feature = "sqlite")]
    async fn save_sqlite(&self, state: NewAccumulatorState) -> Result<(), String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::replace_into(accumulator_state::table)
                .values(&state)
                .execute(conn)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn load_sqlite(&self, edge_id: String) -> Result<Option<AccumulatorStateRow>, String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            accumulator_state::table
                .filter(accumulator_state::edge_id.eq(&edge_id))
                .first::<AccumulatorStateRow>(conn)
                .optional()
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn load_all_sqlite(&self) -> Result<Vec<AccumulatorStateRow>, String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            accumulator_state::table
                .load::<AccumulatorStateRow>(conn)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn delete_sqlite(&self, edge_ids: Vec<String>) -> Result<usize, String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::delete(
                accumulator_state::table.filter(accumulator_state::edge_id.eq_any(&edge_ids)),
            )
            .execute(conn)
            .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
