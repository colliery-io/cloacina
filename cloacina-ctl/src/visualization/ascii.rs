/*
 *  Copyright 2025 Colliery Software
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

use anyhow::{bail, Result};
use std::collections::{HashMap, HashSet};

use crate::cli::Cli;
use crate::manifest::{PackageManifest, TaskInfo};
use crate::utils::{should_print, LogLevel};

pub fn generate_ascii_visualization(
    manifest: &PackageManifest,
    layout: &str,
    details: bool,
    cli: &Cli,
) -> Result<()> {
    if should_print(cli, LogLevel::Info) {
        // Print package header
        println!("{} ({})", manifest.package.name, manifest.package.version);
        if !manifest.package.description.is_empty() {
            println!("{}", manifest.package.description);
        }
        println!();

        match layout {
            "horizontal" => {
                if let Some(ref graph) = manifest.graph {
                    generate_horizontal_ascii_from_graph(graph, details)?
                } else {
                    generate_horizontal_ascii(&manifest.tasks, details)?
                }
            }
            "compact" => {
                if let Some(ref graph) = manifest.graph {
                    generate_compact_ascii_from_graph(graph, details)?
                } else {
                    generate_compact_ascii(&manifest.tasks, details)?
                }
            }
            _ => bail!(
                "Unsupported layout: {}. Use 'horizontal' or 'compact'",
                layout
            ),
        }
    }

    Ok(())
}

pub fn generate_horizontal_ascii(tasks: &[TaskInfo], details: bool) -> Result<()> {
    if tasks.is_empty() {
        println!("No tasks defined in this package.");
        return Ok(());
    }

    // Build dependency graph for topological ordering
    let ordered_tasks = topological_sort_tasks(tasks)?;

    if details {
        // Detailed view with task metadata
        for (i, task) in ordered_tasks.iter().enumerate() {
            let box_width = 45;
            let name_display = if task.id.len() > box_width - 4 {
                format!("{}...", &task.id[..box_width - 7])
            } else {
                task.id.clone()
            };

            // Top border
            println!("┌{:─<width$}┐", "", width = box_width - 2);

            // Task name
            println!("│ {:<width$} │", name_display, width = box_width - 4);

            // Source location
            if !task.source_location.is_empty() {
                let source_display = if task.source_location.len() > box_width - 11 {
                    format!("{}...", &task.source_location[..box_width - 14])
                } else {
                    task.source_location.clone()
                };
                println!(
                    "│ Source: {:<width$} │",
                    source_display,
                    width = box_width - 11
                );
            }

            // Dependencies
            if !task.dependencies.is_empty() {
                let deps_str = task.dependencies.join(", ");
                let deps_display = if deps_str.len() > box_width - 11 {
                    format!("{}...", &deps_str[..box_width - 14])
                } else {
                    deps_str
                };
                println!("│ Deps: {:<width$} │", deps_display, width = box_width - 9);
            } else {
                println!("│ {:<width$} │", "No dependencies", width = box_width - 4);
            }

            // Bottom border
            println!("└{:─<width$}┘", "", width = box_width - 2);

            // Arrow to next task (except for last)
            if i < ordered_tasks.len() - 1 {
                let center = box_width / 2;
                println!("{:>width$}", "│", width = center);
                println!("{:>width$}", "▼", width = center);
                println!();
            }
        }
    } else {
        // Simple horizontal flow - top borders
        for (i, task) in ordered_tasks.iter().enumerate() {
            let box_width = std::cmp::max(task.id.len() + 4, 14);

            print!("┌{:─<width$}┐", "", width = box_width - 2);

            // Arrow between boxes
            if i < ordered_tasks.len() - 1 {
                print!("───▶");
            }
        }
        println!();

        // Task names line
        for (i, task) in ordered_tasks.iter().enumerate() {
            let box_width = std::cmp::max(task.id.len() + 4, 14);

            print!("│{:^width$}│", task.id, width = box_width - 2);

            // Space between boxes
            if i < ordered_tasks.len() - 1 {
                print!("    ");
            }
        }
        println!();

        // Bottom borders line
        for (i, task) in ordered_tasks.iter().enumerate() {
            let box_width = std::cmp::max(task.id.len() + 4, 14);

            print!("└{:─<width$}┘", "", width = box_width - 2);

            if i < ordered_tasks.len() - 1 {
                print!("    ");
            }
        }
        println!();
    }

    Ok(())
}

pub fn generate_compact_ascii(tasks: &[TaskInfo], details: bool) -> Result<()> {
    if tasks.is_empty() {
        println!("No tasks defined in this package.");
        return Ok(());
    }

    let ordered_tasks = topological_sort_tasks(tasks)?;

    if details {
        // Compact with some metadata
        println!("Execution Flow:");
        for (i, task) in ordered_tasks.iter().enumerate() {
            let deps_info = if task.dependencies.is_empty() {
                "".to_string()
            } else {
                format!(" (depends: {})", task.dependencies.join(", "))
            };

            print!("{}. {}{}", i + 1, task.id, deps_info);
            if i < ordered_tasks.len() - 1 {
                print!(" → ");
            }
        }
        println!();

        // Summary line
        println!("\nTask count: {}", ordered_tasks.len());
    } else {
        // Ultra-compact: just task names with arrows
        for (i, task) in ordered_tasks.iter().enumerate() {
            print!("{}", task.id);
            if i < ordered_tasks.len() - 1 {
                print!(" → ");
            }
        }
        println!();
    }

    Ok(())
}

pub fn topological_sort_tasks(tasks: &[TaskInfo]) -> Result<Vec<&TaskInfo>> {
    // Simple topological sort implementation
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    let mut visiting = HashSet::new();

    // Create a map for quick task lookup
    let task_map: HashMap<String, &TaskInfo> = tasks.iter().map(|t| (t.id.clone(), t)).collect();

    fn visit<'a>(
        task: &'a TaskInfo,
        task_map: &HashMap<String, &'a TaskInfo>,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
        result: &mut Vec<&'a TaskInfo>,
    ) -> Result<()> {
        if visiting.contains(&task.id) {
            bail!("Circular dependency detected involving task: {}", task.id);
        }

        if visited.contains(&task.id) {
            return Ok(());
        }

        visiting.insert(task.id.clone());

        // Visit dependencies first
        for dep_id in &task.dependencies {
            if let Some(dep_task) = task_map.get(dep_id) {
                visit(dep_task, task_map, visited, visiting, result)?;
            }
            // Note: we ignore external dependencies not in this package
        }

        visiting.remove(&task.id);
        visited.insert(task.id.clone());
        result.push(task);

        Ok(())
    }

    // Visit all tasks
    for task in tasks {
        if !visited.contains(&task.id) {
            visit(task, &task_map, &mut visited, &mut visiting, &mut result)?;
        }
    }

    Ok(result)
}

pub fn generate_horizontal_ascii_from_graph(
    graph: &cloacina::WorkflowGraphData,
    details: bool,
) -> Result<()> {
    if graph.nodes.is_empty() {
        println!("No tasks defined in this package.");
        return Ok(());
    }

    // Build dependency map from graph edges
    let mut dependencies: HashMap<String, Vec<String>> = HashMap::new();
    for node in &graph.nodes {
        dependencies.insert(node.id.clone(), Vec::new());
    }
    for edge in &graph.edges {
        dependencies
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
    }

    // Calculate execution levels (tasks that can run in parallel)
    let levels = calculate_execution_levels_from_graph(&dependencies)?;

    if details {
        // Detailed view with task metadata, organized by levels
        for (level_idx, level) in levels.iter().enumerate() {
            println!("Level {} ({} parallel tasks):", level_idx, level.len());
            println!();

            // Display tasks in this level side by side (if multiple)
            if level.len() == 1 {
                // Single task - use full width
                let task_id = &level[0];
                if let Some(node) = graph.nodes.iter().find(|n| n.id == *task_id) {
                    display_detailed_task_box(node, &dependencies, 50);
                }
            } else {
                // Multiple parallel tasks - display side by side
                let box_width = std::cmp::min(40, (120 / level.len()).max(20));

                // Top borders
                for (i, _) in level.iter().enumerate() {
                    print!("┌{:─<width$}┐", "", width = box_width - 2);
                    if i < level.len() - 1 {
                        print!("  ");
                    }
                }
                println!();

                // Task names
                for (i, task_id) in level.iter().enumerate() {
                    let display_name = if task_id.len() > box_width - 4 {
                        format!("{}...", &task_id[..box_width - 7])
                    } else {
                        task_id.clone()
                    };
                    print!("│ {:<width$} │", display_name, width = box_width - 4);
                    if i < level.len() - 1 {
                        print!("  ");
                    }
                }
                println!();

                // Dependencies info
                for (i, task_id) in level.iter().enumerate() {
                    let deps = dependencies.get(task_id).cloned().unwrap_or_default();
                    let deps_info = if deps.is_empty() {
                        "No deps".to_string()
                    } else if deps.len() == 1 {
                        format!("← {}", deps[0])
                    } else {
                        format!("← {} deps", deps.len())
                    };
                    let display_deps = if deps_info.len() > box_width - 4 {
                        format!("{}...", &deps_info[..box_width - 7])
                    } else {
                        deps_info
                    };
                    print!("│ {:<width$} │", display_deps, width = box_width - 4);
                    if i < level.len() - 1 {
                        print!("  ");
                    }
                }
                println!();

                // Bottom borders
                for (i, _) in level.iter().enumerate() {
                    print!("└{:─<width$}┘", "", width = box_width - 2);
                    if i < level.len() - 1 {
                        print!("  ");
                    }
                }
                println!();
            }

            // Arrow to next level (except for last)
            if level_idx < levels.len() - 1 {
                println!();
                println!("{}│", " ".repeat(25));
                println!("{}▼", " ".repeat(25));
                println!();
            }
        }
    } else {
        // Simple horizontal view showing levels
        for (level_idx, level) in levels.iter().enumerate() {
            if level.len() == 1 {
                // Single task
                let box_width = std::cmp::max(level[0].len() + 4, 14);
                println!("┌{:─<width$}┐", "", width = box_width - 2);
                println!("│{:^width$}│", level[0], width = box_width - 2);
                println!("└{:─<width$}┘", "", width = box_width - 2);
            } else {
                // Multiple parallel tasks
                println!("Level {} - {} parallel tasks:", level_idx, level.len());
                let max_width = level.iter().map(|t| t.len()).max().unwrap_or(10) + 4;

                // Top borders
                for (i, _) in level.iter().enumerate() {
                    print!("┌{:─<width$}┐", "", width = max_width - 2);
                    if i < level.len() - 1 {
                        print!(" ");
                    }
                }
                println!();

                // Task names
                for (i, task) in level.iter().enumerate() {
                    print!("│{:^width$}│", task, width = max_width - 2);
                    if i < level.len() - 1 {
                        print!(" ");
                    }
                }
                println!();

                // Bottom borders
                for (i, _) in level.iter().enumerate() {
                    print!("└{:─<width$}┘", "", width = max_width - 2);
                    if i < level.len() - 1 {
                        print!(" ");
                    }
                }
                println!();
            }

            // Arrow to next level
            if level_idx < levels.len() - 1 {
                println!("       │");
                println!("       ▼");
            }
        }
    }

    Ok(())
}

pub fn display_detailed_task_box(
    node: &cloacina::GraphNode,
    dependencies: &HashMap<String, Vec<String>>,
    box_width: usize,
) {
    let name_display = if node.id.len() > box_width - 4 {
        format!("{}...", &node.id[..box_width - 7])
    } else {
        node.id.clone()
    };

    // Top border
    println!("┌{:─<width$}┐", "", width = box_width - 2);

    // Task name
    println!("│ {:<width$} │", name_display, width = box_width - 4);

    // Source location
    if let Some(ref source) = node.data.source_location {
        if !source.is_empty() {
            let source_display = if source.len() > box_width - 11 {
                format!("{}...", &source[..box_width - 14])
            } else {
                source.clone()
            };
            println!(
                "│ Source: {:<width$} │",
                source_display,
                width = box_width - 11
            );
        }
    }

    // Dependencies
    let deps = dependencies.get(&node.id).cloned().unwrap_or_default();
    if !deps.is_empty() {
        let deps_str = deps.join(", ");
        let deps_display = if deps_str.len() > box_width - 11 {
            format!("{}...", &deps_str[..box_width - 14])
        } else {
            deps_str
        };
        println!("│ Deps: {:<width$} │", deps_display, width = box_width - 9);
    } else {
        println!("│ {:<width$} │", "No dependencies", width = box_width - 4);
    }

    // Bottom border
    println!("└{:─<width$}┘", "", width = box_width - 2);
}

pub fn calculate_execution_levels_from_graph(
    dependencies: &HashMap<String, Vec<String>>,
) -> Result<Vec<Vec<String>>> {
    let mut levels = Vec::new();
    let mut remaining: HashSet<String> = dependencies.keys().cloned().collect();
    let mut completed = HashSet::new();

    while !remaining.is_empty() {
        let mut current_level = Vec::new();

        // Find tasks with all dependencies completed
        for task_id in &remaining {
            let task_deps = dependencies.get(task_id).cloned().unwrap_or_default();
            let all_deps_done = task_deps.iter().all(|dep| completed.contains(dep));

            if all_deps_done {
                current_level.push(task_id.clone());
            }
        }

        if current_level.is_empty() {
            // This shouldn't happen in a valid DAG, but let's handle it gracefully
            let error_msg = format!(
                "Unable to resolve dependencies for remaining tasks: {:?}",
                remaining
            );
            return Err(anyhow::anyhow!(error_msg));
        }

        // Remove current level tasks from remaining
        for task_id in &current_level {
            remaining.remove(task_id);
            completed.insert(task_id.clone());
        }

        levels.push(current_level);
    }

    Ok(levels)
}

pub fn generate_compact_ascii_from_graph(
    graph: &cloacina::WorkflowGraphData,
    details: bool,
) -> Result<()> {
    if graph.nodes.is_empty() {
        println!("No tasks defined in this package.");
        return Ok(());
    }

    // Build dependency map from graph edges
    let mut dependencies: HashMap<String, Vec<String>> = HashMap::new();
    for node in &graph.nodes {
        dependencies.insert(node.id.clone(), Vec::new());
    }
    for edge in &graph.edges {
        dependencies
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
    }

    // Calculate execution levels (tasks that can run in parallel)
    let levels = calculate_execution_levels_from_graph(&dependencies)?;

    if details {
        // Compact with level and dependency metadata
        println!("Execution Flow ({} levels):", levels.len());
        for (level_idx, level) in levels.iter().enumerate() {
            if level.len() == 1 {
                let task_id = &level[0];
                let deps = dependencies.get(task_id).cloned().unwrap_or_default();
                let deps_info = if deps.is_empty() {
                    "".to_string()
                } else {
                    format!(" (←{})", deps.join(","))
                };
                println!("  L{}: {}{}", level_idx, task_id, deps_info);
            } else {
                println!("  L{}: {} parallel tasks", level_idx, level.len());
                for task_id in level {
                    let deps = dependencies.get(task_id).cloned().unwrap_or_default();
                    let deps_info = if deps.is_empty() {
                        "".to_string()
                    } else {
                        format!(" (←{})", deps.join(","))
                    };
                    println!("    • {}{}", task_id, deps_info);
                }
            }
        }

        // Summary
        let total_tasks = levels.iter().map(|l| l.len()).sum::<usize>();
        let max_parallel = levels.iter().map(|l| l.len()).max().unwrap_or(0);
        println!(
            "\nSummary: {} tasks, {} levels, max {} parallel",
            total_tasks,
            levels.len(),
            max_parallel
        );
    } else {
        // Ultra-compact: show level structure with parallel indicators
        let level_indicators: Vec<String> = levels
            .iter()
            .enumerate()
            .map(|(idx, level)| {
                if level.len() == 1 {
                    level[0].clone()
                } else {
                    format!(
                        "[{}×{}]",
                        level.len(),
                        level
                            .iter()
                            .map(|t| if t.len() > 8 { &t[..6] } else { t })
                            .collect::<Vec<_>>()
                            .join(",")
                            .chars()
                            .take(20)
                            .collect::<String>()
                    )
                }
            })
            .collect();

        for (i, indicator) in level_indicators.iter().enumerate() {
            print!("{}", indicator);
            if i < level_indicators.len() - 1 {
                print!(" → ");
            }
        }
        println!();
    }

    Ok(())
}
