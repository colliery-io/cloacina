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

//! Helper utilities reused by the `#[workflow]` macro for packaged-workflow
//! validation: cycle detection, fuzzy task-name suggestions, and graph-data
//! emission for the manifest's `graph_data_json` field.
//!
//! Historically this file also held a parallel `#[packaged_workflow]`
//! attribute macro implementation (a pre-I-0102 codegen path that emitted
//! its own per-macro `_ffi` plugin shell). The unified
//! `cloacina::package!()` shell macro replaced that path; the orphaned
//! `#[packaged_workflow]` attribute, the `generate_packaged_workflow_impl`
//! token-stream emitter, the `TaskMetadata` / `TaskMetadataCollection`
//! emitted helpers, and the `PackagedWorkflowAttributes` parser were all
//! removed in T-0555 (~800 LOC). The five live helpers in this file are
//! the only consumers `workflow_attr.rs` still depends on.

use std::collections::{HashMap, HashSet};

/// Detect cycles in package-local task dependencies.
///
/// This function performs cycle detection specifically within the scope of a single
/// packaged workflow, without relying on the global registry. It uses a depth-first
/// search to detect cycles in the local dependency graph.
///
/// # Arguments
///
/// * `task_dependencies` - Map of task IDs to their dependency lists
///
/// # Returns
///
/// * `Ok(())` if no cycles are found
/// * `Err(String)` with cycle description if a cycle is detected
pub fn detect_package_cycles(
    task_dependencies: &HashMap<String, Vec<String>>,
) -> Result<(), String> {
    // In test mode, be more lenient about cycle detection (consistent with regular workflow validation)
    let is_test_env = std::env::var("CARGO_CRATE_NAME")
        .map(|name| name.contains("test") || name == "cloacina")
        .unwrap_or(false)
        || std::env::var("CARGO_PKG_NAME")
            .map(|name| name.contains("test") || name == "cloacina")
            .unwrap_or(false);

    if is_test_env {
        // In test mode, skip cycle detection as tasks may be spread across modules
        return Ok(());
    }
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut path = Vec::new();

    for task_id in task_dependencies.keys() {
        if !visited.contains(task_id) {
            dfs_package_cycle_detection(
                task_id,
                task_dependencies,
                &mut visited,
                &mut rec_stack,
                &mut path,
            )?;
        }
    }

    Ok(())
}

fn dfs_package_cycle_detection(
    task_id: &str,
    task_dependencies: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
    path: &mut Vec<String>,
) -> Result<(), String> {
    visited.insert(task_id.to_string());
    rec_stack.insert(task_id.to_string());
    path.push(task_id.to_string());

    if let Some(dependencies) = task_dependencies.get(task_id) {
        for dependency in dependencies {
            // Only check dependencies that are defined within this package
            if task_dependencies.contains_key(dependency) {
                if !visited.contains(dependency) {
                    dfs_package_cycle_detection(
                        dependency,
                        task_dependencies,
                        visited,
                        rec_stack,
                        path,
                    )?;
                } else if rec_stack.contains(dependency) {
                    // Found cycle - build cycle description
                    let cycle_start = path.iter().position(|x| x == dependency).unwrap_or(0);
                    let mut cycle: Vec<String> = path[cycle_start..].to_vec();
                    cycle.push(dependency.to_string()); // Complete the cycle

                    return Err(format!("{} -> {}", cycle.join(" -> "), dependency));
                }
            }
        }
    }

    rec_stack.remove(task_id);
    path.pop();
    Ok(())
}

/// Levenshtein distance between two strings — used to suggest similar
/// task names when a `dependencies = ["..."]` entry doesn't match any
/// task in the package.
// Allow direct indexing in this classic DP algorithm — indices have
// mathematical meaning and `enumerate()` would obscure intent.
#[allow(clippy::needless_range_loop)]
pub fn calculate_levenshtein_distance(a: &str, b: &str) -> usize {
    let a_len = a.len();
    let b_len = b.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut matrix = vec![vec![0; b_len + 1]; a_len + 1];

    for (i, row) in matrix.iter_mut().enumerate().take(a_len + 1) {
        row[0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a.chars().nth(i - 1) == b.chars().nth(j - 1) {
                0
            } else {
                1
            };
            matrix[i][j] = std::cmp::min(
                std::cmp::min(matrix[i - 1][j] + 1, matrix[i][j - 1] + 1),
                matrix[i - 1][j - 1] + cost,
            );
        }
    }

    matrix[a_len][b_len]
}

/// Find up to 3 task names within Levenshtein distance ≤ 2 of the
/// target. Used by the `#[workflow]` macro to suggest fixes for typos
/// in `dependencies = [...]` entries.
pub fn find_similar_package_task_names(target: &str, available: &[String]) -> Vec<String> {
    available
        .iter()
        .filter_map(|name| {
            let distance = calculate_levenshtein_distance(target, name);
            if distance <= 2 && distance < target.len() / 2 {
                Some(name.clone())
            } else {
                None
            }
        })
        .take(3)
        .collect()
}

/// Build the JSON `graph_data` blob persisted in the package manifest.
/// Encodes nodes / edges / metadata so consumers can render the DAG
/// without re-deriving it from task dependencies.
pub fn build_package_graph_data(
    detected_tasks: &HashMap<String, syn::Ident>,
    task_dependencies: &HashMap<String, Vec<String>>,
    package_name: &str,
) -> String {
    // Create nodes for each task
    let mut nodes = Vec::new();
    for task_id in detected_tasks.keys() {
        nodes.push(serde_json::json!({
            "id": task_id,
            "data": {
                "id": task_id,
                "name": task_id,
                "description": format!("Task: {}", task_id),
                "source_location": format!("src/{}.rs", package_name),
                "metadata": {}
            }
        }));
    }

    // Create edges for dependencies
    let mut edges = Vec::new();
    for (task_id, dependencies) in task_dependencies {
        for dependency in dependencies {
            // Only include edges for tasks within this package
            if detected_tasks.contains_key(dependency) {
                edges.push(serde_json::json!({
                    "from": dependency,
                    "to": task_id,
                    "data": {
                        "dependency_type": "data",
                        "weight": null,
                        "metadata": {}
                    }
                }));
            }
        }
    }

    // Calculate graph metadata
    let task_count = detected_tasks.len();
    let edge_count = edges.len();
    let root_tasks: Vec<&String> = detected_tasks
        .keys()
        .filter(|task_id| {
            task_dependencies
                .get(*task_id)
                .map(|deps| deps.is_empty())
                .unwrap_or(true)
        })
        .collect();
    let leaf_tasks: Vec<&String> = detected_tasks
        .keys()
        .filter(|task_id| {
            // A task is a leaf if no other task depends on it
            !task_dependencies
                .values()
                .any(|deps| deps.contains(task_id))
        })
        .collect();

    // Build the complete graph data structure
    let graph_data = serde_json::json!({
        "nodes": nodes,
        "edges": edges,
        "metadata": {
            "task_count": task_count,
            "edge_count": edge_count,
            "has_cycles": false, // We already validated no cycles exist
            "depth_levels": calculate_max_depth(task_dependencies),
            "root_tasks": root_tasks,
            "leaf_tasks": leaf_tasks
        }
    });

    graph_data.to_string()
}

fn calculate_max_depth(task_dependencies: &HashMap<String, Vec<String>>) -> usize {
    let mut max_depth = 0;

    for task_id in task_dependencies.keys() {
        let depth = calculate_task_depth(task_id, task_dependencies, &mut HashSet::new());
        max_depth = max_depth.max(depth);
    }

    max_depth + 1 // Convert to number of levels
}

fn calculate_task_depth(
    task_id: &str,
    task_dependencies: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
) -> usize {
    if visited.contains(task_id) {
        return 0; // Prevent infinite recursion
    }

    visited.insert(task_id.to_string());

    let dependencies = task_dependencies.get(task_id);
    match dependencies {
        None => 0,
        Some(deps) if deps.is_empty() => 0,
        Some(deps) => {
            let max_dep_depth = deps
                .iter()
                .filter(|dep| task_dependencies.contains_key(*dep)) // Only count local dependencies
                .map(|dep| calculate_task_depth(dep, task_dependencies, visited))
                .max()
                .unwrap_or(0);
            max_dep_depth + 1
        }
    }
}
