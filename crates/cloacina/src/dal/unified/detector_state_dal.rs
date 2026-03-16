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

//! DAL for detector state persistence (continuous scheduling).

use super::models::{DetectorStateRow, NewDetectorState};
use super::DAL;
use crate::database::schema::unified::detector_state;
use diesel::prelude::*;

/// Data access layer for detector state operations.
#[derive(Clone)]
pub struct DetectorStateDAL<'a> {
    dal: &'a DAL,
}

impl<'a> DetectorStateDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Save or update detector state for a source.
    pub async fn save(&self, state: NewDetectorState) -> Result<(), String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.save_postgres(state.clone()).await,
            self.save_sqlite(state).await
        )
    }

    /// Load detector state for a specific source.
    pub async fn load(&self, source_name: &str) -> Result<Option<DetectorStateRow>, String> {
        let source_name = source_name.to_string();
        crate::dispatch_backend!(
            self.dal.backend(),
            self.load_postgres(source_name.clone()).await,
            self.load_sqlite(source_name).await
        )
    }

    /// Load all persisted detector states.
    pub async fn load_all(&self) -> Result<Vec<DetectorStateRow>, String> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.load_all_postgres().await,
            self.load_all_sqlite().await
        )
    }

    // --- Postgres implementations ---

    #[cfg(feature = "postgres")]
    async fn save_postgres(&self, state: NewDetectorState) -> Result<(), String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::insert_into(detector_state::table)
                .values(&state)
                .on_conflict(detector_state::source_name)
                .do_update()
                .set(detector_state::committed_state.eq(state.committed_state.as_deref()))
                .execute(conn)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn load_postgres(&self, source_name: String) -> Result<Option<DetectorStateRow>, String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            detector_state::table
                .filter(detector_state::source_name.eq(&source_name))
                .first::<DetectorStateRow>(conn)
                .optional()
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "postgres")]
    async fn load_all_postgres(&self) -> Result<Vec<DetectorStateRow>, String> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            detector_state::table
                .load::<DetectorStateRow>(conn)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    // --- SQLite implementations ---

    #[cfg(feature = "sqlite")]
    async fn save_sqlite(&self, state: NewDetectorState) -> Result<(), String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            diesel::replace_into(detector_state::table)
                .values(&state)
                .execute(conn)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn load_sqlite(&self, source_name: String) -> Result<Option<DetectorStateRow>, String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            detector_state::table
                .filter(detector_state::source_name.eq(&source_name))
                .first::<DetectorStateRow>(conn)
                .optional()
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(feature = "sqlite")]
    async fn load_all_sqlite(&self) -> Result<Vec<DetectorStateRow>, String> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| e.to_string())?;

        conn.interact(move |conn| {
            detector_state::table
                .load::<DetectorStateRow>(conn)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
