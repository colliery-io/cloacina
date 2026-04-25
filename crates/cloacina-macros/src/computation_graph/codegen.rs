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

//! Code generator for `#[computation_graph]`.
//!
//! Takes a validated `GraphIR` and the module's functions, and produces:
//! 1. The original module (with functions intact)
//! 2. A compiled async function that executes the graph

use std::collections::{HashMap, HashSet};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, ItemFn, ItemMod};

use super::graph_ir::{GraphEdge, GraphIR, GraphNode};
use super::parser::TriggerSpec;

/// Convert a snake_case Ident to PascalCase string for struct naming.
fn pascal_case_ident(ident: &Ident) -> Ident {
    let pascal = ident
        .to_string()
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<String>();
    format_ident!("{}", pascal)
}

/// Validate the graph against the module's functions and generate the compiled output.
pub fn generate(ir: &GraphIR, module: &ItemMod) -> syn::Result<TokenStream> {
    // Extract functions from the module
    let functions = extract_functions(module)?;
    let function_names: HashSet<String> = functions.keys().cloned().collect();
    let node_names: HashSet<String> = ir.nodes.keys().cloned().collect();

    // Validation: every node in the graph must have a function in the module
    for node_name in &node_names {
        if !function_names.contains(node_name) {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "node '{}' is referenced in the graph topology but no function with that name exists in the module",
                    node_name
                ),
            ));
        }
    }

    // Validation: every function in the module must appear in the graph (no orphans)
    for fn_name in &function_names {
        if !node_names.contains(fn_name) {
            if let Some(func) = functions.get(fn_name) {
                return Err(syn::Error::new(
                    func.sig.ident.span(),
                    format!(
                        "function '{}' exists in the module but is not referenced in the graph topology. \
                         All functions in a computation_graph module must appear in the graph declaration.",
                        fn_name
                    ),
                ));
            }
        }
    }

    // Check for #[node(blocking)] attributes
    let blocking_nodes: HashSet<String> = functions
        .iter()
        .filter(|(_, f)| has_blocking_attr(f))
        .map(|(name, _)| name.clone())
        .collect();

    // Determine path prefix for generated code:
    // Inside cloacina crate: use `crate::computation_graph::`
    // External crates: use `cloacina_computation_graph::`
    let is_cloacina_crate_early = std::env::var("CARGO_CRATE_NAME")
        .map(|n| n == "cloacina")
        .unwrap_or(false);

    // Generate the compiled function
    let compiled_fn =
        generate_compiled_function(ir, &functions, &blocking_nodes, is_cloacina_crate_early)?;

    // Get the module name and visibility
    let mod_name = &module.ident;
    let vis = &module.vis;
    let mod_attrs = &module.attrs;

    // Extract module content
    let content = module
        .content
        .as_ref()
        .map(|(_, items)| items.clone())
        .unwrap_or_default();

    // Generate the compiled function name
    let compiled_fn_name = format_ident!("{}_compiled", mod_name);

    // Collect return types from routing nodes so we can `use Type::*` for variant patterns
    let routing_use_stmts = generate_routing_use_stmts(ir, &functions, mod_name);

    // Generate inventory registration for global registry
    let mod_name_str = mod_name.to_string();
    let auto_register_name = format_ident!("_auto_register_graph_{}", mod_name);
    let _ = auto_register_name; // reserved for future diagnostics

    // Path roots that differ between internal (cloacina) and external consumers.
    let cg_path = if is_cloacina_crate_early {
        quote! { ::cloacina_computation_graph }
    } else {
        quote! { ::cloacina_computation_graph }
    };
    // Entry accumulators as both strings (for per-graph const block) and
    // compile-time-visible vec!.
    let entry_acc_strs: Vec<String> = ir.entry_accumulators();
    let entry_accs_vec = {
        let entries = entry_acc_strs.iter();
        quote! { vec![#(#entries.to_string()),*] }
    };

    // Derive per-trigger-form code fragments:
    // - `legacy_acc_names_expr`: expression producing the `accumulator_names`
    //   field of `ComputationGraphRegistration` (legacy, kept for packaging
    //   FFI + reconciler).
    // - `legacy_reaction_mode_expr`: expression producing `reaction_mode`.
    // - `trigger_reactor_expr`: expression producing `Option<String>` for
    //   `trigger_reactor`.
    // - `ffi_accumulator_entries_expr`: expression producing the list of
    //   `AccumulatorDeclarationEntry` values for the packaged FFI metadata.
    // - `ffi_reaction_mode_expr`: expression producing the reaction-mode
    //   string used in packaged FFI metadata.
    // - `type_binding_check`: const block emitted adjacent to the graph that
    //   const-evaluates a subset check between entry accumulators and the
    //   referenced reactor's `ACCUMULATORS` (only non-empty for the split
    //   form).

    let (
        legacy_acc_names_expr,
        legacy_reaction_mode_expr,
        trigger_reactor_expr,
        ffi_accumulator_entries_expr,
        ffi_reaction_mode_expr,
        type_binding_check,
    ) = match &ir.trigger {
        TriggerSpec::ByReactor(type_path) => {
            // Emit a private type alias at the outer scope so the bound
            // reactor type is reachable from inside the generated `_ffi`
            // submodule (where a bare path like `MyReactor` would not
            // resolve). All references to the trigger reactor go through
            // this alias.
            let alias_ident = format_ident!("__CGTriggerReactor_{}", mod_name);
            let trigger_alias = quote! {
                #[doc(hidden)]
                #[allow(non_camel_case_types)]
                type #alias_ident = #type_path;
            };

            // Emit a const-eval subset check between the graph's entry
            // accumulators and the bound reactor's ACCUMULATORS.
            let check_fn_ident = format_ident!("__cloacina_check_reactor_binding_{}", mod_name);
            let panic_msg = format!(
                "computation_graph '{}' has an entry accumulator that is not \
                 in the referenced reactor's ACCUMULATORS set",
                mod_name_str
            );
            let entry_lits = entry_acc_strs.clone();

            let check = quote! {
                #trigger_alias

                #[doc(hidden)]
                const fn #check_fn_ident() {
                    const ENTRY: &[&str] = &[#(#entry_lits),*];
                    let reactor: &[&str] =
                        <#type_path as #cg_path::Reactor>::ACCUMULATORS;
                    let mut i = 0usize;
                    while i < ENTRY.len() {
                        let e = ENTRY[i].as_bytes();
                        let mut j = 0usize;
                        let mut found = false;
                        while j < reactor.len() {
                            let r = reactor[j].as_bytes();
                            if e.len() == r.len() {
                                let mut k = 0usize;
                                let mut eq = true;
                                while k < e.len() {
                                    if e[k] != r[k] {
                                        eq = false;
                                        break;
                                    }
                                    k += 1;
                                }
                                if eq {
                                    found = true;
                                    break;
                                }
                            }
                            j += 1;
                        }
                        if !found {
                            panic!(#panic_msg);
                        }
                        i += 1;
                    }
                }
                const _: () = #check_fn_ident();
            };

            let legacy_accs = quote! {
                <#type_path as #cg_path::Reactor>::ACCUMULATORS
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect::<Vec<String>>()
            };
            let legacy_mode = quote! {
                <#type_path as #cg_path::Reactor>::REACTION_MODE
                    .as_str()
                    .to_string()
            };
            let trigger_reactor = quote! {
                Some(<#type_path as #cg_path::Reactor>::NAME.to_string())
            };
            // FFI fragments live inside `pub mod _ffi { ... }`, so reference
            // the trigger reactor via `super::#alias_ident` rather than the
            // bare user-supplied path.
            let ffi_accs = quote! {
                <super::#alias_ident as #cg_path::Reactor>::ACCUMULATORS
                    .iter()
                    .map(|name| cloacina_workflow_plugin::AccumulatorDeclarationEntry {
                        name: (*name).to_string(),
                        accumulator_type: "passthrough".to_string(),
                        config: std::collections::HashMap::new(),
                    })
                    .collect::<Vec<_>>()
            };
            let ffi_mode = quote! {
                <super::#alias_ident as #cg_path::Reactor>::REACTION_MODE
                    .as_str()
                    .to_string()
            };

            (
                legacy_accs,
                legacy_mode,
                trigger_reactor,
                ffi_accs,
                ffi_mode,
                check,
            )
        }
        TriggerSpec::None => {
            let legacy_accs = quote! { Vec::<String>::new() };
            let legacy_mode = quote! { "none".to_string() };
            let trigger_reactor = quote! { None::<String> };
            let ffi_accs =
                quote! { Vec::<cloacina_workflow_plugin::AccumulatorDeclarationEntry>::new() };
            let ffi_mode = quote! { "none".to_string() };
            (
                legacy_accs,
                legacy_mode,
                trigger_reactor,
                ffi_accs,
                ffi_mode,
                proc_macro2::TokenStream::new(),
            )
        }
    };

    // Generate the packaged FFI module (only when feature = "packaged")
    let ffi_plugin_name = format_ident!("_GraphPlugin{}", pascal_case_ident(mod_name));
    let packaged_ffi = quote! {
        #[cfg(feature = "packaged")]
        pub mod _ffi {
            use cloacina_workflow_plugin::__fidius_CloacinaPlugin;
            use cloacina_workflow_plugin::CloacinaPlugin as _;

            pub struct #ffi_plugin_name;

            #[cloacina_workflow_plugin::plugin_impl(CloacinaPlugin, crate = "cloacina_workflow_plugin")]
            impl cloacina_workflow_plugin::CloacinaPlugin for #ffi_plugin_name {
                fn get_task_metadata(&self) -> Result<cloacina_workflow_plugin::PackageTasksMetadata, cloacina_workflow_plugin::PluginError> {
                    // Computation graph packages don't have workflow tasks
                    Ok(cloacina_workflow_plugin::PackageTasksMetadata {
                        workflow_name: String::new(),
                        package_name: env!("CARGO_PKG_NAME").to_string(),
                        package_description: None,
                        package_author: None,
                        workflow_fingerprint: None,
                        graph_data_json: None,
                        tasks: vec![],
                    })
                }

                fn execute_task(&self, _request: cloacina_workflow_plugin::TaskExecutionRequest) -> Result<cloacina_workflow_plugin::TaskExecutionResult, cloacina_workflow_plugin::PluginError> {
                    Err(cloacina_workflow_plugin::PluginError {
                        code: "NOT_SUPPORTED".to_string(),
                        message: "This is a computation graph package, not a workflow package".to_string(),
                        details: None,
                    })
                }

                fn get_graph_metadata(&self) -> Result<cloacina_workflow_plugin::GraphPackageMetadata, cloacina_workflow_plugin::PluginError> {
                    Ok(cloacina_workflow_plugin::GraphPackageMetadata {
                        graph_name: #mod_name_str.to_string(),
                        package_name: env!("CARGO_PKG_NAME").to_string(),
                        reaction_mode: #ffi_reaction_mode_expr,
                        input_strategy: "latest".to_string(),
                        accumulators: #ffi_accumulator_entries_expr,
                    })
                }

                fn execute_graph(&self, request: cloacina_workflow_plugin::GraphExecutionRequest) -> Result<cloacina_workflow_plugin::GraphExecutionResult, cloacina_workflow_plugin::PluginError> {
                    static CDYLIB_RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

                    let rt = CDYLIB_RUNTIME.get_or_init(|| {
                        tokio::runtime::Builder::new_multi_thread()
                            .enable_all()
                            .worker_threads(2)
                            .thread_name("cg-cdylib-worker")
                            .build()
                            .expect("Failed to create cdylib tokio runtime for computation graph")
                    });

                    // Build InputCache from the JSON request.
                    // The FFI boundary always uses JSON strings. We parse each
                    // into serde_json::Value and re-serialize using the
                    // computation graph's serialize() (JSON in debug, bincode in release).
                    let mut cache = cloacina_computation_graph::InputCache::new();
                    for (source_name, json_str) in &request.cache {
                        let value: serde_json::Value = serde_json::from_str(json_str)
                            .map_err(|e| cloacina_workflow_plugin::PluginError {
                                code: "DESERIALIZATION_ERROR".to_string(),
                                message: format!("Failed to parse cache entry '{}': {}", source_name, e),
                                details: None,
                            })?;
                        let bytes = cloacina_computation_graph::serialize(&value)
                            .map_err(|e| cloacina_workflow_plugin::PluginError {
                                code: "SERIALIZATION_ERROR".to_string(),
                                message: format!("Failed to serialize cache entry '{}': {}", source_name, e),
                                details: None,
                            })?;
                        cache.update(
                            cloacina_computation_graph::SourceName::new(source_name),
                            bytes,
                        );
                    }

                    // Execute the compiled graph
                    let result = rt.block_on(async {
                        super::#compiled_fn_name(&cache).await
                    });

                    match result {
                        cloacina_computation_graph::GraphResult::Completed { outputs } => {
                            // Serialize terminal outputs to JSON strings
                            let terminal_json: Vec<String> = outputs
                                .iter()
                                .filter_map(|o| {
                                    // Try to downcast to common types and serialize
                                    if let Some(val) = o.downcast_ref::<serde_json::Value>() {
                                        Some(serde_json::to_string(val).unwrap_or_default())
                                    } else {
                                        None
                                    }
                                })
                                .collect();

                            Ok(cloacina_workflow_plugin::GraphExecutionResult {
                                success: true,
                                terminal_outputs_json: if terminal_json.is_empty() { None } else { Some(terminal_json) },
                                error: None,
                            })
                        }
                        cloacina_computation_graph::GraphResult::Error(e) => {
                            Ok(cloacina_workflow_plugin::GraphExecutionResult {
                                success: false,
                                terminal_outputs_json: None,
                                error: Some(format!("{}", e)),
                            })
                        }
                    }
                }
            }

            cloacina_workflow_plugin::fidius_plugin_registry!();
        }
    };

    let (compiled_fn_body, ctor_body) = if is_cloacina_crate_early {
        // Inside the cloacina crate — use `crate::` paths
        let fn_body = quote! {
            #vis async fn #compiled_fn_name(
                cache: &crate::computation_graph::InputCache,
            ) -> crate::computation_graph::GraphResult {
                #[allow(unused_imports)]
                use #mod_name::*;
                #(#routing_use_stmts)*
                #compiled_fn
            }
        };
        let ctor = quote! {
            #[cfg(not(test))]
            #[cfg(not(feature = "packaged"))]
            crate::inventory::submit! {
                crate::ComputationGraphEntry {
                    name: #mod_name_str,
                    constructor: || crate::ComputationGraphRegistration {
                        graph_fn: std::sync::Arc::new(|cache: crate::computation_graph::InputCache| {
                            Box::pin(async move {
                                #compiled_fn_name(&cache).await
                            })
                        }),
                        entry_accumulators: #entry_accs_vec,
                        trigger_reactor: #trigger_reactor_expr,
                        accumulator_names: #legacy_acc_names_expr,
                        reaction_mode: #legacy_reaction_mode_expr,
                    },
                }
            }
        };
        (fn_body, ctor)
    } else {
        // External crate — use `cloacina_computation_graph::` paths
        let fn_body = quote! {
            #vis async fn #compiled_fn_name(
                cache: &cloacina_computation_graph::InputCache,
            ) -> cloacina_computation_graph::GraphResult {
                #[allow(unused_imports)]
                use #mod_name::*;
                #(#routing_use_stmts)*
                #compiled_fn
            }
        };
        let ctor = quote! {
            #[cfg(not(test))]
            #[cfg(not(feature = "packaged"))]
            cloacina::inventory::submit! {
                cloacina::ComputationGraphEntry {
                    name: #mod_name_str,
                    constructor: || cloacina_computation_graph::ComputationGraphRegistration {
                        graph_fn: std::sync::Arc::new(|cache: cloacina_computation_graph::InputCache| {
                            Box::pin(async move {
                                #compiled_fn_name(&cache).await
                            })
                        }),
                        entry_accumulators: #entry_accs_vec,
                        trigger_reactor: #trigger_reactor_expr,
                        accumulator_names: #legacy_acc_names_expr,
                        reaction_mode: #legacy_reaction_mode_expr,
                    },
                }
            }
        };
        (fn_body, ctor)
    };

    // Compile-time graph handle: a unit struct that implements `Graph`.
    // Other macros (notably `#[task(invokes = computation_graph(H))]`)
    // reference graphs by type path through this handle so trigger-less-ness
    // and the registered name can be const-checked at expansion time.
    let handle_ident = format_ident!("__CGHandle_{}", mod_name);
    let is_triggerless = matches!(&ir.trigger, TriggerSpec::None);
    let graph_handle_decl = quote! {
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        pub struct #handle_ident;

        impl #cg_path::Graph for #handle_ident {
            const NAME: &'static str = #mod_name_str;
            const IS_TRIGGERLESS: bool = #is_triggerless;
        }
    };

    Ok(quote! {
        #(#mod_attrs)*
        #vis mod #mod_name {
            #(#content)*
        }

        #compiled_fn_body

        // Split form: trigger-reactor type alias (for FFI scoping) +
        // const-eval check that the graph's entry accumulators are a
        // subset of the referenced reactor's ACCUMULATORS. Empty for
        // trigger-less.
        #type_binding_check

        // Compile-time handle struct + Graph trait impl
        #graph_handle_decl

        // Embedded mode: inventory registration for global registry
        #ctor_body

        // Packaged mode: FFI plugin exports for fidius
        #packaged_ffi
    })
}

/// Extract named async functions from a module.
fn extract_functions(module: &ItemMod) -> syn::Result<HashMap<String, ItemFn>> {
    let mut functions = HashMap::new();

    if let Some((_, items)) = &module.content {
        for item in items {
            if let syn::Item::Fn(func) = item {
                let name = func.sig.ident.to_string();
                functions.insert(name, func.clone());
            }
        }
    } else {
        return Err(syn::Error::new(
            module.ident.span(),
            "computation_graph module must have inline content (use `mod name { ... }`, not `mod name;`)",
        ));
    }

    Ok(functions)
}

/// Check if a function has `#[node(blocking)]` attribute.
fn has_blocking_attr(func: &ItemFn) -> bool {
    func.attrs.iter().any(|attr| {
        if attr.path().is_ident("node") {
            if let Ok(meta) = attr.parse_args::<Ident>() {
                return meta == "blocking";
            }
        }
        false
    })
}

/// Generate the body of the compiled async function.
///
/// The strategy: find entry nodes (no incoming edges), generate code for each,
/// and recursively generate downstream code following the topological order.
fn generate_compiled_function(
    ir: &GraphIR,
    functions: &HashMap<String, ItemFn>,
    blocking_nodes: &HashSet<String>,
    is_cloacina_crate: bool,
) -> syn::Result<TokenStream> {
    let entry_nodes = ir.entry_nodes();

    if entry_nodes.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "computation graph has no entry nodes (all nodes have incoming edges — possible cycle)",
        ));
    }

    // Generate cache reads for all accumulator inputs
    let cache_reads = generate_cache_reads(ir);

    // Generate the execution code starting from entry nodes
    // Terminal nodes push into __terminal_results instead of being collected at the end.
    // This handles the scoping issue where terminals live inside match arms.
    let mut exec_stmts = Vec::new();
    let mut generated_nodes: HashSet<String> = HashSet::new();

    for node_name in &ir.sorted_nodes {
        if generated_nodes.contains(node_name) {
            continue;
        }
        let node = ir.get_node(node_name).unwrap();
        let stmt = generate_node_execution(
            ir,
            node,
            functions,
            blocking_nodes,
            &mut generated_nodes,
            is_cloacina_crate,
        )?;
        exec_stmts.push(stmt);
    }

    let graph_result_completed = if is_cloacina_crate {
        quote! { crate::computation_graph::GraphResult::completed(__terminal_results) }
    } else {
        quote! { cloacina_computation_graph::GraphResult::completed(__terminal_results) }
    };

    Ok(quote! {
        let mut __terminal_results: Vec<Box<dyn std::any::Any + Send>> = Vec::new();
        #cache_reads
        #(#exec_stmts)*
        #graph_result_completed
    })
}

/// Generate `let` bindings for cache reads.
fn generate_cache_reads(ir: &GraphIR) -> TokenStream {
    let mut reads = Vec::new();
    let mut seen_inputs: HashSet<String> = HashSet::new();

    for node in ir.nodes.values() {
        for input in &node.cache_inputs {
            if seen_inputs.insert(input.clone()) {
                let var_name = format_ident!("__cache_{}", input);
                let input_str = input.as_str();
                reads.push(quote! {
                    let #var_name = cache.get(#input_str);
                });
            }
        }
    }

    quote! { #(#reads)* }
}

/// Generate execution code for a single node.
fn generate_node_execution(
    ir: &GraphIR,
    node: &GraphNode,
    functions: &HashMap<String, ItemFn>,
    blocking_nodes: &HashSet<String>,
    generated: &mut HashSet<String>,
    is_cloacina_crate: bool,
) -> syn::Result<TokenStream> {
    if generated.contains(&node.name) {
        return Ok(quote! {});
    }
    generated.insert(node.name.clone());

    let fn_ident = format_ident!("{}", node.name);
    let result_var = format_ident!("__result_{}", node.name);
    let is_blocking = blocking_nodes.contains(&node.name);

    // Build the argument list for the function call
    let args = generate_call_args(ir, node);

    // Generate the function call (with optional spawn_blocking)
    let call = if is_blocking {
        let graph_error_path = if is_cloacina_crate {
            quote! { crate::computation_graph::GraphError::NodeExecution }
        } else {
            quote! { cloacina_computation_graph::GraphError::NodeExecution }
        };
        quote! {
            let #result_var = tokio::task::spawn_blocking(move || {
                tokio::runtime::Handle::current().block_on(async {
                    #fn_ident(#args).await
                })
            }).await.map_err(|e| #graph_error_path(
                format!("blocking node '{}' panicked: {}", stringify!(#fn_ident), e)
            ))?;
        }
    } else {
        quote! {
            let #result_var = #fn_ident(#args).await;
        }
    };

    // Generate downstream handling based on edge type
    if node.edges_out.is_empty() {
        // Terminal node — call and push result into __terminal_results
        Ok(quote! {
            #call
            __terminal_results.push(Box::new(#result_var) as Box<dyn std::any::Any + Send>);
        })
    } else if node.edges_out.len() == 1 {
        match &node.edges_out[0] {
            GraphEdge::Linear { .. } => {
                // Linear: call node, result available for downstream
                Ok(call)
            }
            GraphEdge::Routing { variants } => {
                // Routing: generate match arms
                let match_arms = generate_routing_match(
                    ir,
                    &node.name,
                    variants,
                    functions,
                    blocking_nodes,
                    generated,
                    is_cloacina_crate,
                )?;
                Ok(quote! {
                    #call
                    #match_arms
                })
            }
        }
    } else {
        // Multiple outgoing edges (fan-out, all linear)
        // Call once, result available for all downstream
        Ok(call)
    }
}

/// Generate the argument list for a node function call.
fn generate_call_args(ir: &GraphIR, node: &GraphNode) -> TokenStream {
    let mut args = Vec::new();

    // Cache inputs first (accumulator data)
    for input in &node.cache_inputs {
        let var_name = format_ident!("__cache_{}", input);
        args.push(quote! { #var_name.as_ref().map(|r| r.as_ref().ok()).flatten() });
    }

    // Incoming edge data (upstream node outputs)
    for incoming in &node.edges_in {
        let from_var = format_ident!("__result_{}", incoming.from);
        // If this comes from a routing variant, the variable is already the unwrapped variant value
        if incoming.variant.is_some() {
            let variant_var = format_ident!(
                "__variant_{}_{}_{}",
                incoming.from,
                incoming.variant.as_ref().unwrap(),
                node.name
            );
            args.push(quote! { &#variant_var });
        } else {
            args.push(quote! { &#from_var });
        }
    }

    quote! { #(#args),* }
}

/// Generate match arms for a routing node.
fn generate_routing_match(
    ir: &GraphIR,
    from_name: &str,
    variants: &[super::graph_ir::GraphRoutingVariant],
    functions: &HashMap<String, ItemFn>,
    blocking_nodes: &HashSet<String>,
    generated: &mut HashSet<String>,
    is_cloacina_crate: bool,
) -> syn::Result<TokenStream> {
    let result_var = format_ident!("__result_{}", from_name);

    let mut arms = Vec::new();
    for variant in variants {
        let variant_ident = format_ident!("{}", variant.variant_name);
        let variant_var = format_ident!(
            "__variant_{}_{}_{}",
            from_name,
            variant.variant_name,
            variant.target
        );
        let target_node = ir.get_node(&variant.target).ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("routing target '{}' not found in graph", variant.target),
            )
        })?;

        let downstream = generate_node_execution(
            ir,
            target_node,
            functions,
            blocking_nodes,
            generated,
            is_cloacina_crate,
        )?;

        arms.push(quote! {
            #variant_ident(#variant_var) => {
                #downstream
            }
        });
    }

    Ok(quote! {
        match #result_var {
            #(#arms)*
        }
    })
}

/// Generate `use ModName::ReturnType::*;` for routing nodes so enum variant
/// patterns resolve in match arms.
fn generate_routing_use_stmts(
    ir: &GraphIR,
    functions: &HashMap<String, ItemFn>,
    mod_name: &Ident,
) -> Vec<TokenStream> {
    let mut stmts = Vec::new();

    for node in ir.nodes.values() {
        let has_routing = node
            .edges_out
            .iter()
            .any(|e| matches!(e, GraphEdge::Routing { .. }));
        if !has_routing {
            continue;
        }

        if let Some(func) = functions.get(&node.name) {
            // Extract the return type and generate `use mod_name::ReturnType::*;`
            if let syn::ReturnType::Type(_, ty) = &func.sig.output {
                stmts.push(quote! {
                    #[allow(unused_imports)]
                    use #mod_name::#ty::*;
                });
            }
        }
    }

    stmts
}
