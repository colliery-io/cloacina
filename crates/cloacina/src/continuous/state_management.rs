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

//! Administrative state management for continuous scheduling.
//!
//! Provides orphaned state detection and cleanup utilities.

use super::graph::DataSourceGraph;
use crate::dal::unified::AccumulatorStateDAL;
use crate::dal::DAL;
use std::collections::HashSet;

/// List orphaned accumulator state edge IDs.
///
/// Orphaned = persisted state for edges not present in the current graph.
pub async fn list_orphaned_states(
    graph: &DataSourceGraph,
    dal: &DAL,
) -> Result<Vec<String>, String> {
    let current_edge_ids: HashSet<String> = graph
        .edges
        .iter()
        .map(|e| format!("{}:{}", e.source, e.task))
        .collect();

    let acc_dal = AccumulatorStateDAL::new(dal);
    let persisted = acc_dal.load_all().await?;

    let orphaned: Vec<String> = persisted
        .into_iter()
        .filter(|s| !current_edge_ids.contains(&s.edge_id))
        .map(|s| s.edge_id)
        .collect();

    Ok(orphaned)
}

/// Prune (delete) orphaned accumulator states.
///
/// Returns the number of states deleted.
pub async fn prune_orphaned_states(graph: &DataSourceGraph, dal: &DAL) -> Result<usize, String> {
    let orphaned = list_orphaned_states(graph, dal).await?;
    if orphaned.is_empty() {
        return Ok(0);
    }

    let acc_dal = AccumulatorStateDAL::new(dal);
    acc_dal.delete_by_ids(orphaned).await
}
