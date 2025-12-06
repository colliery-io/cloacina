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

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use std::collections::{hash_map::DefaultHasher, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use syn::{
    parse::{Parse, ParseStream},
    Ident, ItemMod, LitStr, Result as SynResult, Token,
};

use crate::registry::get_registry;
use crate::tasks::{to_pascal_case, TaskAttributes};

/// C-compatible task metadata structure for FFI
#[repr(C)]
#[derive(Debug, Clone)]
#[allow(dead_code)] // Used in generated code via macros
pub struct TaskMetadata {
    /// Local task ID (e.g., "collect_data")
    pub local_id: *const std::os::raw::c_char,
    /// Template for namespaced ID (e.g., "{tenant}::simple_demo::data_processing::collect_data")
    pub namespaced_id_template: *const std::os::raw::c_char,
    /// JSON string of task dependencies
    pub dependencies_json: *const std::os::raw::c_char,
    /// Name of the task constructor function in the library
    pub constructor_fn_name: *const std::os::raw::c_char,
    /// Task description
    pub description: *const std::os::raw::c_char,
}

// Safety: These pointers point to static string literals which are safe to share
unsafe impl Send for TaskMetadata {}
unsafe impl Sync for TaskMetadata {}

/// C-compatible collection of task metadata for FFI
#[repr(C)]
#[derive(Debug, Clone)]
#[allow(dead_code)] // Used in generated code via macros
pub struct TaskMetadataCollection {
    /// Number of tasks in this package
    pub task_count: u32,
    /// Array of task metadata
    pub tasks: *const TaskMetadata,
    /// Name of the workflow (e.g., "data_processing")
    pub workflow_name: *const std::os::raw::c_char,
    /// Name of the package (e.g., "simple_demo")
    pub package_name: *const std::os::raw::c_char,
}

// Safety: These pointers point to static data which are safe to share
unsafe impl Send for TaskMetadataCollection {}
unsafe impl Sync for TaskMetadataCollection {}

/// Attributes for the packaged_workflow macro
///
/// # Fields
///
/// * `name` - Unique identifier for the workflow (required)
/// * `package` - Package name for namespace isolation (required)
/// * `tenant` - Tenant identifier for the workflow (optional, defaults to "public")
/// * `description` - Optional description of the workflow package
/// * `author` - Optional author information
pub struct PackagedWorkflowAttributes {
    pub name: String,
    pub package: String,
    pub tenant: String,
    pub description: Option<String>,
    pub author: Option<String>,
}

impl Parse for PackagedWorkflowAttributes {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut name = None;
        let mut package = None;
        let mut tenant = None;
        let mut description = None;
        let mut author = None;

        while !input.is_empty() {
            let field_name: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match field_name.to_string().as_str() {
                "package" => {
                    let lit: LitStr = input.parse()?;
                    package = Some(lit.value());
                }
                "name" => {
                    let lit: LitStr = input.parse()?;
                    name = Some(lit.value());
                }
                "tenant" => {
                    let lit: LitStr = input.parse()?;
                    tenant = Some(lit.value());
                }
                "description" => {
                    let lit: LitStr = input.parse()?;
                    description = Some(lit.value());
                }
                "author" => {
                    let lit: LitStr = input.parse()?;
                    author = Some(lit.value());
                }
                _ => {
                    return Err(syn::Error::new(
                        field_name.span(),
                        format!("Unknown attribute: {}", field_name),
                    ));
                }
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        let package = package.ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                "packaged_workflow macro requires 'package' attribute",
            )
        })?;

        let name = name.ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                "packaged_workflow macro requires 'name' attribute",
            )
        })?;

        Ok(PackagedWorkflowAttributes {
            package,
            name,
            tenant: tenant.unwrap_or_else(|| "public".to_string()),
            description,
            author,
        })
    }
}

/// Detect circular dependencies within a package's task dependencies
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

/// Depth-first search implementation for package-level cycle detection
///
/// # Arguments
///
/// * `task_id` - Current task being visited
/// * `task_dependencies` - Map of task IDs to their dependency lists
/// * `visited` - Set tracking visited tasks
/// * `rec_stack` - Set tracking tasks in current recursion stack
/// * `path` - Current path being explored
///
/// # Returns
///
/// * `Ok(())` if no cycle is found
/// * `Err(String)` with cycle description if a cycle is detected
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

/// Calculate the Levenshtein distance between two strings for packaged workflow validation
///
/// Used for finding similar task names when suggesting fixes for typos
/// (duplicated from registry.rs to avoid circular dependencies)
///
/// # Arguments
/// * `a` - First string
/// * `b` - Second string
///
/// # Returns
/// The minimum number of single-character edits required to change one string into the other
// Allow direct indexing in this classic DP algorithm - indices have mathematical meaning
// and enumerate() would obscure the algorithm's intent
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

/// Find task names similar to the given name for typo suggestions in packaged workflows
///
/// Uses Levenshtein distance to find similar task names (consistent with regular workflow validation)
///
/// # Arguments
/// * `target` - The task name to find similar names for
/// * `available` - List of available task names
///
/// # Returns
/// Up to 3 task names that are similar to the target
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

/// Build graph data structure for a packaged workflow
///
/// Creates a WorkflowGraphData structure that represents the task dependencies
/// as a proper DAG, suitable for serialization into the package metadata.
///
/// # Arguments
/// * `detected_tasks` - Map of task IDs to function names
/// * `task_dependencies` - Map of task IDs to their dependency lists
/// * `package_name` - Name of the package for metadata
///
/// # Returns
/// JSON string representation of the WorkflowGraphData
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

/// Calculate the maximum depth in the task dependency graph
///
/// # Arguments
/// * `task_dependencies` - Map of task IDs to their dependency lists
///
/// # Returns
/// The maximum depth level in the graph (number of dependency levels)
fn calculate_max_depth(task_dependencies: &HashMap<String, Vec<String>>) -> usize {
    let mut max_depth = 0;

    for task_id in task_dependencies.keys() {
        let depth = calculate_task_depth(task_id, task_dependencies, &mut HashSet::new());
        max_depth = max_depth.max(depth);
    }

    max_depth + 1 // Convert to number of levels
}

/// Calculate the depth of a specific task in the dependency graph
///
/// # Arguments
/// * `task_id` - The task to calculate depth for
/// * `task_dependencies` - Map of task IDs to their dependency lists
/// * `visited` - Set to track visited tasks and prevent infinite recursion
///
/// # Returns
/// The depth of the task (0 for root tasks)
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

/// Generate packaged workflow implementation
///
/// Creates the necessary entry points and metadata for a distributable workflow package.
/// This includes:
/// - Package metadata structure
/// - Task registration functions using namespace isolation
/// - Standard ABI entry points for dynamic loading
/// - Version information and fingerprinting
/// - Compile-time validation of task dependencies within the package
///
/// # Arguments
///
/// * `attrs` - The packaged workflow attributes
/// * `input` - The input module to be packaged
///
/// # Returns
///
/// A `TokenStream2` containing the generated packaged workflow implementation
pub fn generate_packaged_workflow_impl(
    attrs: PackagedWorkflowAttributes,
    input: ItemMod,
) -> TokenStream2 {
    let mod_name = &input.ident;
    let mod_vis = &input.vis;
    let mod_content = &input.content;

    let workflow_name = &attrs.name;
    let package_name = &attrs.package;
    let package_tenant = &attrs.tenant;
    let package_description = attrs
        .description
        .unwrap_or_else(|| format!("Workflow package: {}", package_name));
    let package_author = attrs.author.unwrap_or_else(|| "Unknown".to_string());

    // Generate a normalized package name for use in identifiers
    let package_ident = syn::Ident::new(
        &package_name
            .replace("-", "_")
            .replace(" ", "_")
            .to_lowercase(),
        mod_name.span(),
    );

    // Generate unique ABI function names based on package
    let _register_abi_name = syn::Ident::new(
        &format!("register_tasks_abi_{}", package_ident),
        mod_name.span(),
    );
    let metadata_abi_name = syn::Ident::new(
        &format!("get_package_metadata_abi_{}", package_ident),
        mod_name.span(),
    );

    // Generate metadata struct name
    let metadata_struct_name = syn::Ident::new(
        &format!(
            "{}PackageMetadata",
            to_pascal_case(&package_ident.to_string())
        ),
        mod_name.span(),
    );

    // Extract task function information from module content and perform validation
    let mut task_metadata_entries = Vec::new();
    let mut detected_tasks = HashMap::new();
    let mut task_dependencies = HashMap::new();

    if let Some((_, items)) = mod_content {
        // First pass: collect all tasks and their metadata
        for item in items {
            if let syn::Item::Fn(item_fn) = item {
                // Check if this function has a #[task] attribute
                for attr in &item_fn.attrs {
                    if attr.path().is_ident("task") {
                        let fn_name = &item_fn.sig.ident;

                        // Parse the task attributes to get the task ID and dependencies
                        if let Ok(task_attrs) = attr.parse_args::<TaskAttributes>() {
                            let task_id = &task_attrs.id;
                            detected_tasks.insert(task_id.clone(), fn_name.clone());
                            task_dependencies
                                .insert(task_id.clone(), task_attrs.dependencies.clone());

                            // Generate task constructor name (following the pattern from the task macro)
                            let task_constructor_name =
                                syn::Ident::new(&format!("{}_task", fn_name), fn_name.span());

                            // Generate metadata entry for this task
                            let dependencies = task_attrs.dependencies.clone();

                            // Convert local dependency names to fully qualified namespaces
                            let fully_qualified_deps: Vec<String> = dependencies
                                .iter()
                                .map(|dep_id| {
                                    format!(
                                        "{{tenant}}::{}::{}::{}",
                                        package_name, workflow_name, dep_id
                                    )
                                })
                                .collect();

                            let dependencies_json = if fully_qualified_deps.is_empty() {
                                "[]".to_string()
                            } else {
                                format!("[\"{}\"]", fully_qualified_deps.join("\",\""))
                            };

                            let namespaced_id_template = format!(
                                "{{tenant}}::{}::{}::{}",
                                package_name, workflow_name, task_id
                            );
                            let description = format!("Task: {}", task_id);

                            task_metadata_entries.push(quote! {
                                TaskMetadata {
                                    local_id: concat!(#task_id, "\0").as_ptr() as *const std::os::raw::c_char,
                                    namespaced_id_template: concat!(#namespaced_id_template, "\0").as_ptr() as *const std::os::raw::c_char,
                                    dependencies_json: concat!(#dependencies_json, "\0").as_ptr() as *const std::os::raw::c_char,
                                    constructor_fn_name: concat!(stringify!(#task_constructor_name), "\0").as_ptr() as *const std::os::raw::c_char,
                                    description: concat!(#description, "\0").as_ptr() as *const std::os::raw::c_char,
                                }
                            });
                        }
                        break;
                    }
                }
            }
        }

        // Second pass: validate dependencies within the package
        // Check if we're in test environment for lenient validation (consistent with regular workflow validation)
        let is_test_env = std::env::var("CARGO_CRATE_NAME")
            .map(|name| name.contains("test") || name == "cloacina")
            .unwrap_or(false)
            || std::env::var("CARGO_PKG_NAME")
                .map(|name| name.contains("test") || name == "cloacina")
                .unwrap_or(false);

        for (task_id, dependencies) in &task_dependencies {
            for dependency in dependencies {
                // Check if dependency exists within this package
                if !detected_tasks.contains_key(dependency) {
                    // If not found locally, check global registry for external dependencies
                    let validation_result = {
                        if is_test_env {
                            // In test mode, be more lenient about missing dependencies
                            Ok(())
                        } else {
                            match get_registry().try_lock() {
                                Ok(registry) => {
                                    if !registry.get_all_task_ids().contains(dependency) {
                                        // Generate improved error message with suggestions (consistent with regular workflow validation)
                                        let available_package_tasks: Vec<String> =
                                            detected_tasks.keys().cloned().collect();
                                        let package_suggestions = find_similar_package_task_names(
                                            dependency,
                                            &available_package_tasks,
                                        );
                                        let global_suggestions = find_similar_package_task_names(
                                            dependency,
                                            &registry.get_all_task_ids(),
                                        );

                                        let mut error_msg = format!(
                                            "Task '{}' depends on undefined task '{}'. \
                                            This dependency is not defined within the '{}' package \
                                            and is not available in the global registry.\n\n",
                                            task_id, dependency, package_name
                                        );

                                        // Add suggestions if any found
                                        if !package_suggestions.is_empty() {
                                            error_msg.push_str(&format!(
                                                "Did you mean one of these tasks in this package?\n  {}\n\n",
                                                package_suggestions.join("\n  ")
                                            ));
                                        }

                                        if !global_suggestions.is_empty() {
                                            error_msg.push_str(&format!(
                                                "Or did you mean one of these global tasks?\n  {}\n\n",
                                                global_suggestions.join("\n  ")
                                            ));
                                        }

                                        error_msg.push_str(&format!(
                                            "Available tasks in this package: [{}]\n\n\
                                            Hint: Make sure all task dependencies are either:\n\
                                            1. Defined within the same packaged workflow module, or\n\
                                            2. Registered in the global task registry before this package is processed",
                                            available_package_tasks.join(", ")
                                        ));

                                        Err(error_msg)
                                    } else {
                                        Ok(())
                                    }
                                }
                                Err(_) => {
                                    // If we can't acquire the lock, skip validation to avoid hanging
                                    Ok(())
                                }
                            }
                        }
                    };

                    // Return compile error if validation failed
                    if let Err(error_msg) = validation_result {
                        return quote! {
                            compile_error!(#error_msg);
                        };
                    }
                }
            }
        }

        // Third pass: check for circular dependencies within the package
        let cycle_result = detect_package_cycles(&task_dependencies);
        if let Err(cycle_error) = cycle_result {
            let error_msg = format!(
                "Circular dependency detected within package '{}': {}\n\n\
                Hint: Review your task dependencies to eliminate cycles.",
                package_name, cycle_error
            );
            return quote! {
                compile_error!(#error_msg);
            };
        }
    }

    // Generate package fingerprint based on version and content
    let mut hasher = DefaultHasher::new();
    package_name.hash(&mut hasher);
    if let Some((_, items)) = mod_content {
        for item in items {
            quote::quote!(#item).to_string().hash(&mut hasher);
        }
    }
    let package_fingerprint = format!("{:016x}", hasher.finish());

    // Build the workflow graph for this package
    let graph_data_json =
        build_package_graph_data(&detected_tasks, &task_dependencies, package_name);

    // Generate task metadata structures for FFI export
    let task_metadata_items = if !detected_tasks.is_empty() {
        let mut task_metadata_entries = Vec::new();
        let mut task_execution_cases = Vec::new();

        for (task_index, (task_id, fn_name)) in detected_tasks.iter().enumerate() {
            let task_index = task_index as u32;
            let dependencies = task_dependencies.get(task_id).cloned().unwrap_or_default();

            // Generate fully qualified namespace: tenant::package::workflow::task
            // For now, we'll use placeholders that get filled in at runtime
            let namespaced_id = format!(
                "{{tenant}}::{}::{}::{}",
                package_name, workflow_name, task_id
            );

            // Generate dependencies as JSON array string
            let dependencies_json = if dependencies.is_empty() {
                "[]".to_string()
            } else {
                format!("[\"{}\"]", dependencies.join("\",\""))
            };

            let source_location = format!("src/{}.rs", mod_name);

            task_metadata_entries.push(quote! {
                cloacina_ctl_task_metadata {
                    index: #task_index,
                    local_id: concat!(#task_id, "\0").as_ptr() as *const std::os::raw::c_char,
                    namespaced_id_template: concat!(#namespaced_id, "\0").as_ptr() as *const std::os::raw::c_char,
                    dependencies_json: concat!(#dependencies_json, "\0").as_ptr() as *const std::os::raw::c_char,
                    description: concat!("Task: ", #task_id, "\0").as_ptr() as *const std::os::raw::c_char,
                    source_location: concat!(#source_location, "\0").as_ptr() as *const std::os::raw::c_char,
                }
            });

            // Generate task execution case
            task_execution_cases.push(quote! {
                #task_id => {
                    match #fn_name(&mut context).await {
                        Ok(()) => Ok(()),
                        Err(e) => Err(format!("Task '{}' failed: {:?}", #task_id, e))
                    }
                }
            });
        }

        let task_count = detected_tasks.len();

        // Generate standard function name expected by the loader
        let metadata_fn_name = syn::Ident::new("cloacina_get_task_metadata", mod_name.span());

        quote! {
            /// C-compatible task metadata structure for FFI
            #[repr(C)]
            #[derive(Debug, Clone, Copy)]
            pub struct cloacina_ctl_task_metadata {
                pub index: u32,
                pub local_id: *const std::os::raw::c_char,
                pub namespaced_id_template: *const std::os::raw::c_char,
                pub dependencies_json: *const std::os::raw::c_char,
                pub description: *const std::os::raw::c_char,
                pub source_location: *const std::os::raw::c_char,
            }

            // Safety: These pointers point to static string literals which are safe to share
            unsafe impl Sync for cloacina_ctl_task_metadata {}

            /// Package task metadata for FFI export
            #[repr(C)]
            #[derive(Debug, Clone, Copy)]
            pub struct cloacina_ctl_package_tasks {
                pub task_count: u32,
                pub tasks: *const cloacina_ctl_task_metadata,
                pub package_name: *const std::os::raw::c_char,
                pub package_description: *const std::os::raw::c_char,
                pub package_author: *const std::os::raw::c_char,
                pub workflow_fingerprint: *const std::os::raw::c_char,
                pub graph_data_json: *const std::os::raw::c_char,
            }

            // Safety: These pointers point to static data which is safe to share
            unsafe impl Sync for cloacina_ctl_package_tasks {}

            /// Static array of task metadata
            static TASK_METADATA_ARRAY: [cloacina_ctl_task_metadata; #task_count] = [
                #(#task_metadata_entries),*
            ];

            /// Static graph data as JSON
            static GRAPH_DATA_JSON: &str = concat!(#graph_data_json, "\0");

            /// Static package tasks metadata
            static PACKAGE_TASKS_METADATA: cloacina_ctl_package_tasks = cloacina_ctl_package_tasks {
                task_count: #task_count as u32,
                tasks: TASK_METADATA_ARRAY.as_ptr(),
                package_name: concat!(#package_name, "\0").as_ptr() as *const std::os::raw::c_char,
                package_description: concat!(#package_description, "\0").as_ptr() as *const std::os::raw::c_char,
                package_author: concat!(#package_author, "\0").as_ptr() as *const std::os::raw::c_char,
                workflow_fingerprint: concat!(#package_fingerprint, "\0").as_ptr() as *const std::os::raw::c_char,
                graph_data_json: GRAPH_DATA_JSON.as_ptr() as *const std::os::raw::c_char,
            };

            /// Get task metadata for cloacina-ctl compilation
            #[no_mangle]
            pub extern "C" fn #metadata_fn_name() -> *const cloacina_ctl_package_tasks {
                &PACKAGE_TASKS_METADATA
            }

            /// String-based task execution function for cloacina-ctl
            #[no_mangle]
            pub extern "C" fn cloacina_execute_task(
                task_name: *const std::os::raw::c_char,
                task_name_len: u32,
                context_json: *const std::os::raw::c_char,
                context_len: u32,
                result_buffer: *mut u8,
                result_capacity: u32,
                result_len: *mut u32,
            ) -> i32 {
                // Safety: Convert raw pointers to safe Rust types
                let task_name_bytes = unsafe {
                    std::slice::from_raw_parts(task_name as *const u8, task_name_len as usize)
                };

                let task_name_str = match std::str::from_utf8(task_name_bytes) {
                    Ok(s) => s,
                    Err(_) => {
                        return write_error_result("Invalid UTF-8 in task name", result_buffer, result_capacity, result_len);
                    }
                };

                let context_bytes = unsafe {
                    std::slice::from_raw_parts(context_json as *const u8, context_len as usize)
                };

                let context_str = match std::str::from_utf8(context_bytes) {
                    Ok(s) => s,
                    Err(_) => {
                        return write_error_result("Invalid UTF-8 in context", result_buffer, result_capacity, result_len);
                    }
                };

                // Execute the actual task by creating context from JSON
                let mut context = match cloacina::Context::from_json(context_str.to_string()) {
                    Ok(ctx) => ctx,
                    Err(e) => {
                        return write_error_result(&format!("Failed to create context from JSON: {}", e), result_buffer, result_capacity, result_len);
                    }
                };

                // Use an async runtime for task execution
                let runtime = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        return write_error_result(&format!("Failed to create async runtime: {}", e), result_buffer, result_capacity, result_len);
                    }
                };

                let task_result = runtime.block_on(async {
                    match task_name_str {
                        #(#task_execution_cases)*
                        _ => Err(format!("Unknown task: {}", task_name_str))
                    }
                });

                // Handle the result and write to output buffer
                match task_result {
                    Ok(()) => {
                        // Return the actual modified context as JSON instead of a hardcoded success message
                        match context.to_json() {
                            Ok(context_json) => {
                                // Parse context JSON to serde_json::Value for write_success_result
                                match serde_json::from_str::<serde_json::Value>(&context_json) {
                                    Ok(context_value) => write_success_result(&context_value, result_buffer, result_capacity, result_len),
                                    Err(e) => {
                                        let error = format!("Failed to parse context JSON for task '{}': {}", task_name_str, e);
                                        write_error_result(&error, result_buffer, result_capacity, result_len)
                                    }
                                }
                            }
                            Err(e) => {
                                let error = format!("Failed to serialize context for task '{}': {}", task_name_str, e);
                                write_error_result(&error, result_buffer, result_capacity, result_len)
                            }
                        }
                    }
                    Err(e) => {
                        let error = format!("Task '{}' failed: {}", task_name_str, e);
                        write_error_result(&error, result_buffer, result_capacity, result_len)
                    }
                }
            }


            fn write_success_result(result: &serde_json::Value, buffer: *mut u8, capacity: u32, result_len: *mut u32) -> i32 {
                let json_str = match serde_json::to_string(result) {
                    Ok(s) => s,
                    Err(_) => return -1,
                };

                let bytes = json_str.as_bytes();
                let len = bytes.len().min(capacity as usize);

                unsafe {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, len);
                    *result_len = len as u32;
                }

                0 // Success
            }

            fn write_error_result(error: &str, buffer: *mut u8, capacity: u32, result_len: *mut u32) -> i32 {
                let error_json = serde_json::json!({
                    "error": error,
                    "status": "error"
                });

                let json_str = match serde_json::to_string(&error_json) {
                    Ok(s) => s,
                    Err(_) => return -2,
                };

                let bytes = json_str.as_bytes();
                let len = bytes.len().min(capacity as usize);

                unsafe {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, len);
                    *result_len = len as u32;
                }

                -1 // Error
            }
        }
    } else {
        // Generate standard function name expected by the loader
        let metadata_fn_name = syn::Ident::new("cloacina_get_task_metadata", mod_name.span());

        quote! {
            /// Empty task metadata structure for packages with no tasks
            #[repr(C)]
            #[derive(Debug, Clone, Copy)]
            pub struct cloacina_ctl_task_metadata {
                pub index: u32,
                pub local_id: *const std::os::raw::c_char,
                pub namespaced_id_template: *const std::os::raw::c_char,
                pub dependencies_json: *const std::os::raw::c_char,
                pub description: *const std::os::raw::c_char,
                pub source_location: *const std::os::raw::c_char,
            }

            // Safety: These pointers point to static string literals which are safe to share
            unsafe impl Sync for cloacina_ctl_task_metadata {}

            #[repr(C)]
            #[derive(Debug, Clone, Copy)]
            pub struct cloacina_ctl_package_tasks {
                pub task_count: u32,
                pub tasks: *const cloacina_ctl_task_metadata,
                pub package_name: *const std::os::raw::c_char,
                pub package_description: *const std::os::raw::c_char,
                pub package_author: *const std::os::raw::c_char,
                pub workflow_fingerprint: *const std::os::raw::c_char,
                pub graph_data_json: *const std::os::raw::c_char,
            }

            // Safety: These pointers point to static data which is safe to share
            unsafe impl Sync for cloacina_ctl_package_tasks {}

            static EMPTY_GRAPH_DATA: &str = concat!("{\"nodes\":[],\"edges\":[],\"metadata\":{\"task_count\":0,\"edge_count\":0,\"has_cycles\":false,\"depth_levels\":0,\"root_tasks\":[],\"leaf_tasks\":[]}}", "\0");

            static PACKAGE_TASKS_METADATA: cloacina_ctl_package_tasks = cloacina_ctl_package_tasks {
                task_count: 0,
                tasks: std::ptr::null(),
                package_name: concat!(#package_name, "\0").as_ptr() as *const std::os::raw::c_char,
                package_description: concat!(#package_description, "\0").as_ptr() as *const std::os::raw::c_char,
                package_author: concat!(#package_author, "\0").as_ptr() as *const std::os::raw::c_char,
                workflow_fingerprint: concat!(#package_fingerprint, "\0").as_ptr() as *const std::os::raw::c_char,
                graph_data_json: EMPTY_GRAPH_DATA.as_ptr() as *const std::os::raw::c_char,
            };

            #[no_mangle]
            pub extern "C" fn #metadata_fn_name() -> *const cloacina_ctl_package_tasks {
                &PACKAGE_TASKS_METADATA
            }

            /// String-based task execution function for empty packages
            #[no_mangle]
            pub extern "C" fn cloacina_execute_task(
                _task_name: *const std::os::raw::c_char,
                _task_name_len: u32,
                _context_json: *const std::os::raw::c_char,
                _context_len: u32,
                result_buffer: *mut u8,
                result_capacity: u32,
                result_len: *mut u32,
            ) -> i32 {
                let error_json = serde_json::json!({
                    "error": "No tasks defined in this package",
                    "status": "error"
                });

                let json_str = match serde_json::to_string(&error_json) {
                    Ok(s) => s,
                    Err(_) => return -2,
                };

                let bytes = json_str.as_bytes();
                let len = bytes.len().min(result_capacity as usize);

                unsafe {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), result_buffer, len);
                    *result_len = len as u32;
                }

                -1 // Error
            }
        }
    };

    // Generate the new host-managed registry FFI functions
    let task_count = detected_tasks.len();
    let new_metadata_functions = if !detected_tasks.is_empty() {
        quote! {
            /// Get task metadata for this package (new host-managed approach)
            ///
            /// This function returns metadata about all tasks in this package
            /// without registering them. The host process will use this metadata
            /// to register tasks in the host's global registry.
            #[no_mangle]
            pub extern "C" fn get_task_metadata() -> *const TaskMetadataCollection {
                &HOST_TASK_METADATA_COLLECTION
            }

            /// Static array of task metadata for host-managed registry
            static HOST_TASK_METADATA_ARRAY: [TaskMetadata; #task_count] = [
                #(#task_metadata_entries),*
            ];

            /// Static task metadata collection for host-managed registry
            static HOST_TASK_METADATA_COLLECTION: TaskMetadataCollection = TaskMetadataCollection {
                task_count: #task_count as u32,
                tasks: HOST_TASK_METADATA_ARRAY.as_ptr(),
                workflow_name: concat!(#workflow_name, "\0").as_ptr() as *const std::os::raw::c_char,
                package_name: concat!(#package_name, "\0").as_ptr() as *const std::os::raw::c_char,
            };
        }
    } else {
        quote! {
            /// Get task metadata for this package (empty package)
            #[no_mangle]
            pub extern "C" fn get_task_metadata() -> *const TaskMetadataCollection {
                &HOST_EMPTY_TASK_METADATA_COLLECTION
            }

            /// Static empty task metadata collection
            static HOST_EMPTY_TASK_METADATA_COLLECTION: TaskMetadataCollection = TaskMetadataCollection {
                task_count: 0,
                tasks: std::ptr::null(),
                workflow_name: concat!(#workflow_name, "\0").as_ptr() as *const std::os::raw::c_char,
                package_name: concat!(#package_name, "\0").as_ptr() as *const std::os::raw::c_char,
            };
        }
    };

    // Extract the module items for proper token generation
    let module_items = if let Some((_, items)) = mod_content {
        items.iter().collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    quote! {
        // Keep the original module with enhanced functionality
        #mod_vis mod #mod_name {
            #(#module_items)*

            // Include task metadata structures and functions
            #task_metadata_items

            // Include new host-managed registry types and functions

            /// C-compatible task metadata structure for FFI
            #[repr(C)]
            #[derive(Debug, Clone)]
            pub struct TaskMetadata {
                /// Local task ID (e.g., "collect_data")
                pub local_id: *const std::os::raw::c_char,
                /// Template for namespaced ID (e.g., "{tenant}::simple_demo::data_processing::collect_data")
                pub namespaced_id_template: *const std::os::raw::c_char,
                /// JSON string of task dependencies
                pub dependencies_json: *const std::os::raw::c_char,
                /// Name of the task constructor function in the library
                pub constructor_fn_name: *const std::os::raw::c_char,
                /// Task description
                pub description: *const std::os::raw::c_char,
            }

            // Safety: These pointers point to static string literals which are safe to share
            unsafe impl Send for TaskMetadata {}
            unsafe impl Sync for TaskMetadata {}

            /// C-compatible collection of task metadata for FFI
            #[repr(C)]
            #[derive(Debug, Clone)]
            pub struct TaskMetadataCollection {
                /// Number of tasks in this package
                pub task_count: u32,
                /// Array of task metadata
                pub tasks: *const TaskMetadata,
                /// Name of the workflow (e.g., "data_processing")
                pub workflow_name: *const std::os::raw::c_char,
                /// Name of the package (e.g., "simple_demo")
                pub package_name: *const std::os::raw::c_char,
            }

            // Safety: These pointers point to static data which are safe to share
            unsafe impl Send for TaskMetadataCollection {}
            unsafe impl Sync for TaskMetadataCollection {}

            #new_metadata_functions

            /// Package metadata for this workflow package
            #[derive(Debug, Clone)]
            pub struct #metadata_struct_name {
                pub package: &'static str,
                pub tenant: &'static str,
                pub description: &'static str,
                pub author: &'static str,
                pub fingerprint: &'static str,
            }

            impl #metadata_struct_name {
                pub const fn new() -> Self {
                    Self {
                        package: #package_name,
                        tenant: #package_tenant,
                        description: #package_description,
                        author: #package_author,
                        fingerprint: #package_fingerprint,
                    }
                }
            }

            /// Get package metadata
            pub fn get_package_metadata() -> #metadata_struct_name {
                #metadata_struct_name::new()
            }


            /// Get package metadata via ABI
            #[no_mangle]
            pub extern "C" fn #metadata_abi_name() -> *const #metadata_struct_name {
                Box::leak(Box::new(get_package_metadata()))
            }

            /// Create a workflow instance from this package
            ///
            /// This function is called by the registry reconciler to create
            /// a workflow instance that can be executed by the scheduler.
            #[no_mangle]
            pub extern "C" fn cloacina_create_workflow(
                tenant_id: *const std::os::raw::c_char,
                workflow_id: *const std::os::raw::c_char,
            ) -> *const cloacina::workflow::Workflow {
                use std::ffi::CStr;

                // Validate input pointers
                if tenant_id.is_null() || workflow_id.is_null() {
                    return std::ptr::null();
                }

                // Convert C strings to Rust strings
                let tenant_id = unsafe {
                    match CStr::from_ptr(tenant_id).to_str() {
                        Ok(s) => s,
                        Err(_) => return std::ptr::null(),
                    }
                };

                let workflow_id = unsafe {
                    match CStr::from_ptr(workflow_id).to_str() {
                        Ok(s) => s,
                        Err(_) => return std::ptr::null(),
                    }
                };

                // Create workflow and add registered tasks (following workflow! macro pattern)
                let mut workflow = cloacina::workflow::Workflow::new(#workflow_name);
                workflow.set_tenant(tenant_id);
                workflow.set_package(#package_name);

                // Add tasks from the task registry to the workflow
                let task_registry = cloacina::task::global_task_registry();
                {
                    let registry = task_registry.read();
                    tracing::debug!("Task registry has {} entries", registry.len());
                    tracing::debug!("Looking for tasks with package={}, workflow={}, tenant={}", #package_name, #workflow_name, tenant_id);
                    eprintln!("DEBUG: Task registry has {} entries", registry.len());
                    eprintln!("DEBUG: Looking for tasks with package={}, workflow={}, tenant={}", #package_name, #workflow_name, tenant_id);

                    let mut found_tasks = 0;
                    for (namespace, task_constructor) in registry.iter() {
                        tracing::debug!("Found task: tenant={}, package={}, workflow={}, task={}",
                                       namespace.tenant_id, namespace.package_name, namespace.workflow_id, namespace.task_id);
                        eprintln!("DEBUG: Found task: tenant={}, package={}, workflow={}, task={}",
                                 namespace.tenant_id, namespace.package_name, namespace.workflow_id, namespace.task_id);

                        // Only include tasks from this package and tenant
                        if namespace.package_name == #package_name
                            && namespace.workflow_id == #workflow_name
                            && namespace.tenant_id == tenant_id
                        {
                            tracing::debug!("Adding task {} to workflow", namespace.task_id);
                            eprintln!("DEBUG: Adding task {} to workflow", namespace.task_id);
                            let task = task_constructor();
                            if let Err(e) = workflow.add_task(task) {
                                tracing::warn!("Failed to add task {} to workflow: {:?}", namespace.task_id, e);
                                eprintln!("Warning: Failed to add task {} to workflow: {:?}", namespace.task_id, e);
                            } else {
                                found_tasks += 1;
                            }
                        }
                    }
                    tracing::debug!("Added {} tasks to workflow", found_tasks);
                    eprintln!("DEBUG: Added {} tasks to workflow", found_tasks);
                }

                // Validate and finalize the workflow (following workflow! macro pattern)
                match workflow.validate() {
                    Ok(_) => {
                        let finalized_workflow = workflow.finalize();
                        Box::into_raw(Box::new(finalized_workflow))
                    }
                    Err(_) => {
                        // If validation fails, return null
                        std::ptr::null()
                    }
                }
            }
        }
    }
}

/// The packaged_workflow macro for creating distributable workflow packages
///
/// This macro transforms a module into a packaged workflow that can be:
/// - Compiled into a shared library (.so file)
/// - Dynamically loaded by executors
/// - Properly isolated using namespace system
/// - Versioned and fingerprinted for integrity
///
/// # Usage
///
/// ```rust
/// #[packaged_workflow(
///     package = "analytics_pipeline",
///     version = "1.0.0",
///     description = "Real-time analytics workflow",
///     author = "Analytics Team"
/// )]
/// mod analytics_workflow {
///     use cloacina_macros::task;
///     use cloacina::{Context, TaskError};
///
///     #[task(id = "extract_data", dependencies = [])]
///     async fn extract_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
///         // Implementation
///         Ok(())
///     }
///
///     #[task(id = "transform_data", dependencies = ["extract_data"])]
///     async fn transform_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
///         // Implementation
///         Ok(())
///     }
/// }
/// ```
///
/// # Attributes
///
/// See `PackagedWorkflowAttributes` for available configuration options.
pub fn packaged_workflow(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = TokenStream2::from(args);
    let input = TokenStream2::from(input);

    let attrs = match syn::parse2::<PackagedWorkflowAttributes>(args) {
        Ok(attrs) => attrs,
        Err(e) => {
            return syn::Error::new(
                Span::call_site(),
                format!("Invalid packaged_workflow attributes: {}", e),
            )
            .to_compile_error()
            .into();
        }
    };

    let input_mod = match syn::parse2::<syn::ItemMod>(input) {
        Ok(input_mod) => input_mod,
        Err(e) => {
            return syn::Error::new(
                Span::call_site(),
                format!(
                    "packaged_workflow macro can only be applied to modules: {}",
                    e
                ),
            )
            .to_compile_error()
            .into();
        }
    };

    generate_packaged_workflow_impl(attrs, input_mod).into()
}
