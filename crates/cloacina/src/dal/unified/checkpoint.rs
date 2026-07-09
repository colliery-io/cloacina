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

//! Unified Checkpoint DAL for computation graph state persistence
//!
//! Provides save/load operations for accumulator checkpoints, boundaries,
//! reactor state, and state accumulator buffers. All operations use upsert
//! semantics keyed by (graph_name, accumulator_name) or (graph_name).

use super::models::{
    NewUnifiedAccumulatorBoundary, NewUnifiedAccumulatorCheckpoint, NewUnifiedReactorState,
    NewUnifiedStateAccumulatorBuffer, UnifiedAccumulatorBoundary, UnifiedAccumulatorCheckpoint,
    UnifiedReactorState, UnifiedStateAccumulatorBuffer,
};
use super::DAL;
use crate::database::schema::unified::{
    accumulator_boundaries, accumulator_checkpoints, reactor_state, state_accumulator_buffers,
};
use crate::database::universal_types::{UniversalBinary, UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use diesel::prelude::*;

/// Data access layer for computation graph checkpoint operations.
#[derive(Clone)]
pub struct CheckpointDAL<'a> {
    dal: &'a DAL,
}

impl<'a> CheckpointDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    // ========================================================================
    // Accumulator Checkpoints
    // ========================================================================

    /// Save (upsert) an accumulator checkpoint.
    pub async fn save_checkpoint(
        &self,
        graph_name: &str,
        accumulator_name: &str,
        data: Vec<u8>,
    ) -> Result<(), ValidationError> {
        let graph_name = graph_name.to_string();
        let accumulator_name = accumulator_name.to_string();
        let now = UniversalTimestamp::now();
        let id = UniversalUuid::new_v4();
        let data = UniversalBinary::from(data);

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(accumulator_checkpoints::table)
                .values(&NewUnifiedAccumulatorCheckpoint {
                    id,
                    graph_name: graph_name.clone(),
                    accumulator_name: accumulator_name.clone(),
                    checkpoint_data: data.clone(),
                    created_at: now,
                    updated_at: now,
                })
                .on_conflict((
                    accumulator_checkpoints::graph_name,
                    accumulator_checkpoints::accumulator_name,
                ))
                .do_update()
                .set((
                    accumulator_checkpoints::checkpoint_data.eq(data),
                    accumulator_checkpoints::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }

    /// Load an accumulator checkpoint.
    pub async fn load_checkpoint(
        &self,
        graph_name: &str,
        accumulator_name: &str,
    ) -> Result<Option<Vec<u8>>, ValidationError> {
        let graph_name = graph_name.to_string();
        let accumulator_name = accumulator_name.to_string();

        let result: Option<UnifiedAccumulatorCheckpoint> =
            crate::interact_on_backend!(self.dal, |conn| {
                accumulator_checkpoints::table
                    .filter(accumulator_checkpoints::graph_name.eq(&graph_name))
                    .filter(accumulator_checkpoints::accumulator_name.eq(&accumulator_name))
                    .first(conn)
                    .optional()
            })?;

        Ok(result.map(|r| r.checkpoint_data.into_inner()))
    }

    // ========================================================================
    // Accumulator Boundaries
    // ========================================================================

    /// Save (upsert) a boundary with sequence number.
    pub async fn save_boundary(
        &self,
        graph_name: &str,
        accumulator_name: &str,
        data: Vec<u8>,
        sequence_number: i64,
    ) -> Result<(), ValidationError> {
        let graph_name = graph_name.to_string();
        let accumulator_name = accumulator_name.to_string();
        let now = UniversalTimestamp::now();
        let id = UniversalUuid::new_v4();
        let data = UniversalBinary::from(data);

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(accumulator_boundaries::table)
                .values(&NewUnifiedAccumulatorBoundary {
                    id,
                    graph_name: graph_name.clone(),
                    accumulator_name: accumulator_name.clone(),
                    boundary_data: data.clone(),
                    sequence_number,
                    created_at: now,
                    updated_at: now,
                })
                .on_conflict((
                    accumulator_boundaries::graph_name,
                    accumulator_boundaries::accumulator_name,
                ))
                .do_update()
                .set((
                    accumulator_boundaries::boundary_data.eq(data),
                    accumulator_boundaries::sequence_number.eq(sequence_number),
                    accumulator_boundaries::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }

    /// Load a boundary and its sequence number.
    pub async fn load_boundary(
        &self,
        graph_name: &str,
        accumulator_name: &str,
    ) -> Result<Option<(Vec<u8>, i64)>, ValidationError> {
        let graph_name = graph_name.to_string();
        let accumulator_name = accumulator_name.to_string();

        let result: Option<UnifiedAccumulatorBoundary> =
            crate::interact_on_backend!(self.dal, |conn| {
                accumulator_boundaries::table
                    .filter(accumulator_boundaries::graph_name.eq(&graph_name))
                    .filter(accumulator_boundaries::accumulator_name.eq(&accumulator_name))
                    .first(conn)
                    .optional()
            })?;

        Ok(result.map(|r| (r.boundary_data.into_inner(), r.sequence_number)))
    }

    // ========================================================================
    // Reactor State
    // ========================================================================

    /// Save (upsert) reactor state.
    pub async fn save_reactor_state(
        &self,
        graph_name: &str,
        cache_data: Vec<u8>,
        dirty_flags: Vec<u8>,
        sequential_queue: Option<Vec<u8>>,
    ) -> Result<(), ValidationError> {
        let graph_name = graph_name.to_string();
        let now = UniversalTimestamp::now();
        let id = UniversalUuid::new_v4();
        let cache_data = UniversalBinary::from(cache_data);
        let dirty_flags = UniversalBinary::from(dirty_flags);
        let sequential_queue = sequential_queue.map(UniversalBinary::from);

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(reactor_state::table)
                .values(&NewUnifiedReactorState {
                    id,
                    graph_name: graph_name.clone(),
                    cache_data: cache_data.clone(),
                    dirty_flags: dirty_flags.clone(),
                    sequential_queue: sequential_queue.clone(),
                    created_at: now,
                    updated_at: now,
                })
                .on_conflict(reactor_state::graph_name)
                .do_update()
                .set((
                    reactor_state::cache_data.eq(cache_data),
                    reactor_state::dirty_flags.eq(dirty_flags),
                    reactor_state::sequential_queue.eq(sequential_queue),
                    reactor_state::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }

    /// Load reactor state.
    pub async fn load_reactor_state(
        &self,
        graph_name: &str,
    ) -> Result<Option<(Vec<u8>, Vec<u8>, Option<Vec<u8>>)>, ValidationError> {
        let graph_name = graph_name.to_string();

        let result: Option<UnifiedReactorState> = crate::interact_on_backend!(self.dal, |conn| {
            reactor_state::table
                .filter(reactor_state::graph_name.eq(&graph_name))
                .first(conn)
                .optional()
        })?;

        Ok(result.map(|r| {
            (
                r.cache_data.into_inner(),
                r.dirty_flags.into_inner(),
                r.sequential_queue.map(|q| q.into_inner()),
            )
        }))
    }

    // ========================================================================
    // State Accumulator Buffers
    // ========================================================================

    /// Save (upsert) a state accumulator buffer.
    pub async fn save_state_buffer(
        &self,
        graph_name: &str,
        accumulator_name: &str,
        data: Vec<u8>,
        capacity: i32,
    ) -> Result<(), ValidationError> {
        let graph_name = graph_name.to_string();
        let accumulator_name = accumulator_name.to_string();
        let now = UniversalTimestamp::now();
        let id = UniversalUuid::new_v4();
        let data = UniversalBinary::from(data);

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(state_accumulator_buffers::table)
                .values(&NewUnifiedStateAccumulatorBuffer {
                    id,
                    graph_name: graph_name.clone(),
                    accumulator_name: accumulator_name.clone(),
                    buffer_data: data.clone(),
                    capacity,
                    created_at: now,
                    updated_at: now,
                })
                .on_conflict((
                    state_accumulator_buffers::graph_name,
                    state_accumulator_buffers::accumulator_name,
                ))
                .do_update()
                .set((
                    state_accumulator_buffers::buffer_data.eq(data),
                    state_accumulator_buffers::capacity.eq(capacity),
                    state_accumulator_buffers::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }

    /// Load a state accumulator buffer.
    pub async fn load_state_buffer(
        &self,
        graph_name: &str,
        accumulator_name: &str,
    ) -> Result<Option<(Vec<u8>, i32)>, ValidationError> {
        let graph_name = graph_name.to_string();
        let accumulator_name = accumulator_name.to_string();

        let result: Option<UnifiedStateAccumulatorBuffer> =
            crate::interact_on_backend!(self.dal, |conn| {
                state_accumulator_buffers::table
                    .filter(state_accumulator_buffers::graph_name.eq(&graph_name))
                    .filter(state_accumulator_buffers::accumulator_name.eq(&accumulator_name))
                    .first(conn)
                    .optional()
            })?;

        Ok(result.map(|r| (r.buffer_data.into_inner(), r.capacity)))
    }

    // ========================================================================
    // Cleanup
    // ========================================================================

    /// Delete all state for a graph (used on graph unload/removal).
    pub async fn delete_graph_state(&self, graph_name: &str) -> Result<(), ValidationError> {
        let graph_name = graph_name.to_string();

        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction(|conn| {
                diesel::delete(
                    accumulator_checkpoints::table
                        .filter(accumulator_checkpoints::graph_name.eq(&graph_name)),
                )
                .execute(conn)?;
                diesel::delete(
                    accumulator_boundaries::table
                        .filter(accumulator_boundaries::graph_name.eq(&graph_name)),
                )
                .execute(conn)?;
                diesel::delete(
                    reactor_state::table.filter(reactor_state::graph_name.eq(&graph_name)),
                )
                .execute(conn)?;
                diesel::delete(
                    state_accumulator_buffers::table
                        .filter(state_accumulator_buffers::graph_name.eq(&graph_name)),
                )
                .execute(conn)
            })
        })?;

        Ok(())
    }
}
