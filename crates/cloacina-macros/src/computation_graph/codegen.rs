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

    // Trigger-less graphs operate on a `Context<Value>` instead of an
    // `InputCache`. They must not declare cache inputs anywhere in the
    // topology — there is no cache for entry nodes to read from. Validate
    // before generating any code so the diagnostic points at the macro
    // invocation rather than a generated identifier.
    let is_triggerless = matches!(&ir.trigger, TriggerSpec::None);
    if is_triggerless {
        for node in ir.nodes.values() {
            if !node.cache_inputs.is_empty() {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!(
                        "trigger-less computation graphs cannot declare cache inputs \
                         (node '{}' lists `({})`). Trigger-less graphs receive a \
                         `&Context<Value>` instead of an `InputCache` — entry nodes \
                         should be declared as bare names like `entry -> next` and \
                         their fn signatures should take `(ctx: &Context<Value>)`.",
                        node.name,
                        node.cache_inputs.join(", "),
                    ),
                ));
            }
        }
    }

    // Generate the compiled function
    let compiled_fn = generate_compiled_function(
        ir,
        &functions,
        &blocking_nodes,
        is_cloacina_crate_early,
        is_triggerless,
    )?;

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

    // Path prefix: same absolute path in internal and external builds since
    // cloacina re-exports the computation-graph crate.
    let cg_path = quote! { ::cloacina_computation_graph };
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
        type_binding_check,
    ) = match &ir.trigger {
        TriggerSpec::ByReactor(reactor_name) => {
            // I-0102 / T-A: reactor reference is a string name. Compile-time
            // accumulator-subset / reaction-mode lookups via `<TypePath as
            // Reactor>::ACCUMULATORS` are gone — the binding is resolved at
            // load time by the reconciler's runtime contract validator.
            let reactor_name_lit = reactor_name.clone();

            let legacy_accs = quote! { Vec::<String>::new() };
            let legacy_mode = quote! { "when_any".to_string() };
            let trigger_reactor = quote! { Some(#reactor_name_lit.to_string()) };

            (
                legacy_accs,
                legacy_mode,
                trigger_reactor,
                proc_macro2::TokenStream::new(),
            )
        }
        TriggerSpec::None => {
            let legacy_accs = quote! { Vec::<String>::new() };
            let legacy_mode = quote! { "none".to_string() };
            let trigger_reactor = quote! { None::<String> };
            (
                legacy_accs,
                legacy_mode,
                trigger_reactor,
                proc_macro2::TokenStream::new(),
            )
        }
    };

    // Path roots for emitted code differ between in-crate (cloacina) and
    // external consumers; trigger-less code paths additionally need access
    // to `cloacina::Context` and the `TriggerlessGraph*` types defined in
    // cloacina (not in the leaf cg crate).
    let cloacina_root = if is_cloacina_crate_early {
        quote! { crate }
    } else {
        quote! { cloacina }
    };
    let cg_runtime_root = if is_cloacina_crate_early {
        quote! { crate::computation_graph }
    } else {
        quote! { cloacina_computation_graph }
    };
    let terminal_node_names: Vec<String> = ir
        .nodes
        .values()
        .filter(|n| n.is_terminal)
        .map(|n| n.name.clone())
        .collect();

    // Serialized node/edge topology for the FFI metadata, so the API/UI can
    // render this graph's DAG. Emitted as a string literal in every
    // ComputationGraphEntry submission below. (CLOACI-T-0673)
    let graph_data_json_lit = proc_macro2::Literal::string(&graph_topology_json(ir));

    // CLOACI-I-0128 (T-0758): derive the input interface — one InputSlot per
    // cache source (accumulator name → boundary type from the consuming node's
    // fn signature). Built once, emitted into every ComputationGraphEntry below
    // with the crate-path appropriate to the build mode. Boundary typing is
    // opt-in: types deriving `JsonSchema` get a rich schema, others a permissive
    // one (via the `SchemaProbe` autoref specialization).
    let source_boundary_types = extract_source_boundary_types(ir, &functions);
    let input_interface_via_cloacina = build_input_interface_fn(
        &source_boundary_types,
        &quote! { ::cloacina::input_interface },
    );
    let input_interface_via_workflow = build_input_interface_fn(
        &source_boundary_types,
        &quote! { ::cloacina_workflow::input_interface },
    );

    let (compiled_fn_body, ctor_body) = if is_triggerless {
        // Trigger-less form: the compiled fn takes a workflow `Context<Value>`
        // and the runtime registration goes into `TriggerlessGraphEntry`.
        let fn_body = quote! {
            #vis async fn #compiled_fn_name(
                context: &#cloacina_root::Context<::serde_json::Value>,
            ) -> #cg_runtime_root::GraphResult {
                #[allow(unused_imports)]
                use #mod_name::*;
                #(#routing_use_stmts)*
                #compiled_fn
            }
        };
        let ctor = quote! {
            // T-0552: TriggerlessGraphEntry submission un-gated for both
            // packaged and embedded modes (was previously embedded-only).
            // Cfg-gated paths so the submission resolves through `cloacina`
            // in library mode and `cloacina-workflow-plugin` direct in
            // packaged mode (CLOACI-I-0103 follow-up).
            #[cfg(not(feature = "packaged"))]
            ::cloacina::cloacina_workflow_plugin::inventory::submit! {
                ::cloacina::cloacina_workflow_plugin::TriggerlessGraphEntry {
                    name: #mod_name_str,
                    constructor: || ::cloacina::cloacina_workflow_plugin::TriggerlessGraphRegistration {
                        name: #mod_name_str.to_string(),
                        graph_fn: ::std::sync::Arc::new(|context: ::cloacina::cloacina_workflow::Context<::serde_json::Value>| {
                            Box::pin(async move {
                                #compiled_fn_name(&context).await
                            })
                        }),
                        terminal_node_names: vec![#(#terminal_node_names.to_string()),*],
                    },
                }
            }
            #[cfg(feature = "packaged")]
            ::cloacina_workflow_plugin::inventory::submit! {
                ::cloacina_workflow_plugin::TriggerlessGraphEntry {
                    name: #mod_name_str,
                    constructor: || ::cloacina_workflow_plugin::TriggerlessGraphRegistration {
                        name: #mod_name_str.to_string(),
                        graph_fn: ::std::sync::Arc::new(|context: ::cloacina_workflow::Context<::serde_json::Value>| {
                            Box::pin(async move {
                                #compiled_fn_name(&context).await
                            })
                        }),
                        terminal_node_names: vec![#(#terminal_node_names.to_string()),*],
                    },
                }
            }
        };
        (fn_body, ctor)
    } else if is_cloacina_crate_early {
        // Triggered (split) form, inside cloacina crate.
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
            // I-0102 / T-C: ComputationGraphEntry submission emits in both
            // packaged and embedded modes so the unified
            // `cloacina::package!()` shell can walk it. (Inside cloacina.)
            cloacina_workflow_plugin::inventory::submit! {
                cloacina_workflow_plugin::ComputationGraphEntry {
                    name: #mod_name_str,
                    constructor: || cloacina_computation_graph::ComputationGraphRegistration {
                        graph_fn: std::sync::Arc::new(|cache: cloacina_computation_graph::InputCache| {
                            Box::pin(async move {
                                #compiled_fn_name(&cache).await
                            })
                        }),
                        trigger_reactor: #trigger_reactor_expr,
                        accumulator_names: #legacy_acc_names_expr,
                        reaction_mode: #legacy_reaction_mode_expr,
                    },
                    graph_data_json: #graph_data_json_lit,
                    input_interface: #input_interface_via_workflow,
                }
            }
        };
        (fn_body, ctor)
    } else {
        // Triggered (split) form, external crate.
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
            // I-0102 / T-C: ComputationGraphEntry submission emits in both
            // packaged and embedded modes (external crate path). Cfg-gated
            // so library users (only have `cloacina`) and packaged cdylibs
            // (have `cloacina-workflow-plugin` direct) both resolve.
            #[cfg(not(feature = "packaged"))]
            ::cloacina::cloacina_workflow_plugin::inventory::submit! {
                ::cloacina::cloacina_workflow_plugin::ComputationGraphEntry {
                    name: #mod_name_str,
                    constructor: || cloacina_computation_graph::ComputationGraphRegistration {
                        graph_fn: std::sync::Arc::new(|cache: cloacina_computation_graph::InputCache| {
                            Box::pin(async move {
                                #compiled_fn_name(&cache).await
                            })
                        }),
                        trigger_reactor: #trigger_reactor_expr,
                        accumulator_names: #legacy_acc_names_expr,
                        reaction_mode: #legacy_reaction_mode_expr,
                    },
                    graph_data_json: #graph_data_json_lit,
                    input_interface: #input_interface_via_cloacina,
                }
            }
            #[cfg(feature = "packaged")]
            ::cloacina_workflow_plugin::inventory::submit! {
                ::cloacina_workflow_plugin::ComputationGraphEntry {
                    name: #mod_name_str,
                    constructor: || cloacina_computation_graph::ComputationGraphRegistration {
                        graph_fn: std::sync::Arc::new(|cache: cloacina_computation_graph::InputCache| {
                            Box::pin(async move {
                                #compiled_fn_name(&cache).await
                            })
                        }),
                        trigger_reactor: #trigger_reactor_expr,
                        accumulator_names: #legacy_acc_names_expr,
                        reaction_mode: #legacy_reaction_mode_expr,
                    },
                    graph_data_json: #graph_data_json_lit,
                    input_interface: #input_interface_via_workflow,
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
    // For trigger-less graphs, additionally implement `TriggerlessGraph` so
    // `#[task(invokes = computation_graph(H))]` can reach the compiled fn +
    // terminal-name routing through a single trait. Reactor-triggered graphs
    // intentionally do not implement this trait — that's the compile-time
    // gate keeping tasks from invoking them.
    let triggerless_graph_impl = if is_triggerless {
        // T-0552: TriggerlessGraph trait + types relocated to
        // cloacina-workflow-plugin. Cfg-gated path resolution per the same
        // pattern as the inventory submissions above.
        quote! {
            #[cfg(not(feature = "packaged"))]
            impl ::cloacina::cloacina_workflow_plugin::TriggerlessGraph for #handle_ident {
                fn compiled_fn() -> ::cloacina::cloacina_workflow_plugin::TriggerlessGraphFn {
                    ::std::sync::Arc::new(|context: ::cloacina::cloacina_workflow::Context<::serde_json::Value>| {
                        Box::pin(async move { #compiled_fn_name(&context).await })
                    })
                }
                fn terminal_node_names() -> &'static [&'static str] {
                    &[#(#terminal_node_names),*]
                }
            }
            #[cfg(feature = "packaged")]
            impl ::cloacina_workflow_plugin::TriggerlessGraph for #handle_ident {
                fn compiled_fn() -> ::cloacina_workflow_plugin::TriggerlessGraphFn {
                    ::std::sync::Arc::new(|context: ::cloacina_workflow::Context<::serde_json::Value>| {
                        Box::pin(async move { #compiled_fn_name(&context).await })
                    })
                }
                fn terminal_node_names() -> &'static [&'static str] {
                    &[#(#terminal_node_names),*]
                }
            }
        }
    } else {
        proc_macro2::TokenStream::new()
    };
    let graph_handle_decl = quote! {
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        pub struct #handle_ident;

        impl #cg_path::Graph for #handle_ident {
            const NAME: &'static str = #mod_name_str;
            const IS_TRIGGERLESS: bool = #is_triggerless;
        }

        #triggerless_graph_impl
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

        // Inventory registration. I-0102 / T-C: gated only on
        // `cfg(not(feature = "packaged"))` for now — the
        // ComputationGraphEntry inventory submission needs to happen in
        // packaged mode too once paths are migrated; see follow-up.
        #ctor_body
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

/// CLOACI-I-0128 (T-0758): map each cache source (accumulator name) to its
/// boundary type, read from the consuming node fn's parameter of the same name.
/// First declaration wins; a source with no matching typed param is skipped —
/// it degrades to a permissive schema at emission (the probe fallback). Iterated
/// in a stable order (`sorted_nodes`) so the emitted slot order is deterministic.
fn extract_source_boundary_types(
    ir: &GraphIR,
    functions: &HashMap<String, ItemFn>,
) -> Vec<(String, syn::Type)> {
    let mut out: Vec<(String, syn::Type)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for node_name in &ir.sorted_nodes {
        let node = match ir.get_node(node_name) {
            Some(n) => n,
            None => continue,
        };
        let func = match functions.get(node_name) {
            Some(f) => f,
            None => continue,
        };
        for input in &node.cache_inputs {
            if !seen.insert(input.clone()) {
                continue;
            }
            if let Some(ty) = param_type_by_name(func, input) {
                out.push((input.clone(), boundary_inner_type(&ty)));
            }
        }
    }
    out
}

/// Find a fn parameter by its binding name and return its declared type.
fn param_type_by_name(func: &ItemFn, name: &str) -> Option<syn::Type> {
    for arg in &func.sig.inputs {
        if let syn::FnArg::Typed(pt) = arg {
            if let syn::Pat::Ident(pi) = &*pt.pat {
                if pi.ident == name {
                    return Some((*pt.ty).clone());
                }
            }
        }
    }
    None
}

/// Strip `Option<…>` and references to recover the boundary value type
/// (`Option<&AlphaIn>` → `AlphaIn`).
fn boundary_inner_type(ty: &syn::Type) -> syn::Type {
    match ty {
        syn::Type::Reference(r) => boundary_inner_type(&r.elem),
        syn::Type::Path(p) => {
            if let Some(seg) = p.path.segments.last() {
                if seg.ident == "Option" {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                            return boundary_inner_type(inner);
                        }
                    }
                }
            }
            ty.clone()
        }
        _ => ty.clone(),
    }
}

/// Build the `input_interface: fn() -> String` token stream for a
/// `ComputationGraphEntry`. `prefix` is the path to the `input_interface`
/// helpers (mode-dependent). Each source becomes an optional `InputSlot` whose
/// schema is resolved by the opt-in `SchemaProbe` (typed if the boundary derives
/// `JsonSchema`, permissive `{}` otherwise). No sources → `|| "[]"`.
fn build_input_interface_fn(sources: &[(String, syn::Type)], prefix: &TokenStream) -> TokenStream {
    if sources.is_empty() {
        return quote! { || ::std::string::String::from("[]") };
    }
    let slot_exprs = sources.iter().map(|(name, ty)| {
        quote! {
            #prefix::InputSlot::optional(
                #name,
                {
                    #[allow(unused_imports)]
                    use #prefix::{ProbeFallback as _, ProbeTyped as _};
                    (&#prefix::SchemaProbe::<#ty>::new()).probe_input_schema()
                },
                ::std::option::Option::None,
            )
        }
    });
    quote! {
        || {
            let slots: ::std::vec::Vec<#prefix::InputSlot> = ::std::vec![ #(#slot_exprs),* ];
            #prefix::slots_to_json(&slots)
        }
    }
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
    is_triggerless: bool,
) -> syn::Result<TokenStream> {
    let entry_nodes = ir.entry_nodes();

    if entry_nodes.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "computation graph has no entry nodes (all nodes have incoming edges — possible cycle)",
        ));
    }

    // Generate cache reads for all accumulator inputs. Trigger-less graphs
    // have no cache, so the block is empty.
    let cache_reads = if is_triggerless {
        TokenStream::new()
    } else {
        generate_cache_reads(ir)
    };

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
            is_triggerless,
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
    is_triggerless: bool,
) -> syn::Result<TokenStream> {
    if generated.contains(&node.name) {
        return Ok(quote! {});
    }
    generated.insert(node.name.clone());

    let fn_ident = format_ident!("{}", node.name);
    let result_var = format_ident!("__result_{}", node.name);
    let is_blocking = blocking_nodes.contains(&node.name);

    // Build the argument list for the function call
    let args = generate_call_args(ir, node, is_triggerless);

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
        // Terminal node — call and push result into __terminal_results.
        // For trigger-less graphs we serialize to `serde_json::Value` at the
        // terminal site so workflow tasks can route outputs back into the
        // context via a cheap downcast-to-Value (rather than relying on the
        // executor knowing every concrete terminal type).
        let terminal_push = if is_triggerless {
            let node_name_str = node.name.to_string();
            quote! {
                let __serialized: ::serde_json::Value =
                    ::serde_json::to_value(&#result_var).unwrap_or_else(|e| {
                        panic!(
                            "trigger-less graph terminal '{}' must produce a value \
                             that implements Serialize: {}",
                            #node_name_str, e
                        )
                    });
                __terminal_results
                    .push(Box::new(__serialized) as Box<dyn std::any::Any + Send>);
            }
        } else {
            // CLOACI-T-0775: reactor-driven graphs also serialize their terminal
            // to `serde_json::Value` so the reactor can record per-fire outputs
            // (the FFI bridge captures them via downcast-to-Value). Unlike the
            // trigger-less path this is observability, not a routed result — fall
            // back to Null rather than panicking if the terminal isn't Serialize.
            quote! {
                let __serialized: ::serde_json::Value =
                    ::serde_json::to_value(&#result_var).unwrap_or(::serde_json::Value::Null);
                __terminal_results
                    .push(Box::new(__serialized) as Box<dyn std::any::Any + Send>);
            }
        };
        Ok(quote! {
            #call
            #terminal_push
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
                    is_triggerless,
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
fn generate_call_args(_ir: &GraphIR, node: &GraphNode, is_triggerless: bool) -> TokenStream {
    let mut args = Vec::new();

    // Trigger-less entry nodes receive the workflow context directly.
    // Identified by having no incoming edges (and, by trigger-less invariant,
    // no cache inputs either).
    if is_triggerless && node.edges_in.is_empty() {
        args.push(quote! { context });
    }

    // Cache inputs first (accumulator data)
    for input in &node.cache_inputs {
        let var_name = format_ident!("__cache_{}", input);
        args.push(quote! { #var_name.as_ref().map(|r| r.as_ref().ok()).flatten() });
    }

    // Incoming edge data (upstream node outputs)
    for incoming in &node.edges_in {
        let from_var = format_ident!("__result_{}", incoming.from);
        // If this comes from a routing variant, the variable is already the unwrapped variant value
        if let Some(variant) = &incoming.variant {
            let variant_var =
                format_ident!("__variant_{}_{}_{}", incoming.from, variant, node.name);
            args.push(quote! { &#variant_var });
        } else {
            args.push(quote! { &#from_var });
        }
    }

    quote! { #(#args),* }
}

/// Generate match arms for a routing node.
#[allow(clippy::too_many_arguments)]
fn generate_routing_match(
    ir: &GraphIR,
    from_name: &str,
    variants: &[super::graph_ir::GraphRoutingVariant],
    functions: &HashMap<String, ItemFn>,
    blocking_nodes: &HashSet<String>,
    generated: &mut HashSet<String>,
    is_cloacina_crate: bool,
    is_triggerless: bool,
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
            is_triggerless,
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

/// Serialize the graph IR's node/edge topology to a compact JSON string for
/// `ComputationGraphEntry.graph_data_json`, so the API/UI can render the CG DAG.
/// Shape: `{"nodes":[{"id","inputs":[..]}],"edges":[{"from","to","label":null|"Variant"}]}`.
/// Linear edges have `label: null`; routing edges carry the variant name. The
/// node order follows the IR's topological sort. (CLOACI-T-0673)
pub(super) fn graph_topology_json(ir: &GraphIR) -> String {
    use serde_json::{json, Value};

    let nodes: Vec<Value> = ir
        .sorted_nodes
        .iter()
        .filter_map(|n| ir.nodes.get(n))
        .map(|node| json!({ "id": node.name, "inputs": node.cache_inputs }))
        .collect();

    let mut edges: Vec<Value> = Vec::new();
    for name in &ir.sorted_nodes {
        if let Some(node) = ir.nodes.get(name) {
            for edge in &node.edges_out {
                match edge {
                    GraphEdge::Linear { target } => edges.push(json!({
                        "from": node.name,
                        "to": target,
                        "label": Value::Null,
                    })),
                    GraphEdge::Routing { variants } => {
                        for v in variants {
                            edges.push(json!({
                                "from": node.name,
                                "to": v.target,
                                "label": v.variant_name,
                            }));
                        }
                    }
                }
            }
        }
    }

    serde_json::to_string(&json!({ "nodes": nodes, "edges": edges })).unwrap_or_default()
}

#[cfg(test)]
mod topology_tests {
    //! Guards the CG topology emission (CLOACI-T-0673): the macro must serialize
    //! the GraphIR's nodes + edges into the JSON the API/UI render as a DAG.
    use super::super::graph_ir::GraphRoutingVariant;
    use super::{graph_topology_json, GraphEdge, GraphIR, GraphNode, TriggerSpec};
    use std::collections::HashMap;

    fn node(name: &str, inputs: &[&str], edges: Vec<GraphEdge>, terminal: bool) -> GraphNode {
        GraphNode {
            name: name.to_string(),
            cache_inputs: inputs.iter().map(|s| s.to_string()).collect(),
            edges_out: edges,
            edges_in: vec![],
            is_terminal: terminal,
        }
    }

    #[test]
    fn emits_linear_nodes_and_edges() {
        // compute(alpha) -> output
        let mut nodes = HashMap::new();
        nodes.insert(
            "compute".to_string(),
            node(
                "compute",
                &["alpha"],
                vec![GraphEdge::Linear {
                    target: "output".to_string(),
                }],
                false,
            ),
        );
        nodes.insert("output".to_string(), node("output", &[], vec![], true));
        let ir = GraphIR {
            trigger: TriggerSpec::ByReactor("rx".to_string()),
            sorted_nodes: vec!["compute".to_string(), "output".to_string()],
            nodes,
        };

        let v: serde_json::Value = serde_json::from_str(&graph_topology_json(&ir)).unwrap();
        let ns = v["nodes"].as_array().unwrap();
        assert_eq!(ns.len(), 2);
        assert_eq!(ns[0]["id"], "compute");
        assert_eq!(ns[0]["inputs"][0], "alpha");
        let es = v["edges"].as_array().unwrap();
        assert_eq!(es.len(), 1);
        assert_eq!(es[0]["from"], "compute");
        assert_eq!(es[0]["to"], "output");
        assert!(es[0]["label"].is_null(), "linear edges have a null label");
    }

    #[test]
    fn emits_routing_variant_labels() {
        // decision(alpha) => { Trade -> handler, NoAction -> audit }
        let mut nodes = HashMap::new();
        nodes.insert(
            "decision".to_string(),
            node(
                "decision",
                &["alpha"],
                vec![GraphEdge::Routing {
                    variants: vec![
                        GraphRoutingVariant {
                            variant_name: "Trade".to_string(),
                            target: "handler".to_string(),
                        },
                        GraphRoutingVariant {
                            variant_name: "NoAction".to_string(),
                            target: "audit".to_string(),
                        },
                    ],
                }],
                false,
            ),
        );
        nodes.insert("handler".to_string(), node("handler", &[], vec![], true));
        nodes.insert("audit".to_string(), node("audit", &[], vec![], true));
        let ir = GraphIR {
            trigger: TriggerSpec::ByReactor("rx".to_string()),
            sorted_nodes: vec![
                "decision".to_string(),
                "handler".to_string(),
                "audit".to_string(),
            ],
            nodes,
        };

        let v: serde_json::Value = serde_json::from_str(&graph_topology_json(&ir)).unwrap();
        let es = v["edges"].as_array().unwrap();
        assert_eq!(es.len(), 2, "one edge per routing variant");
        let labels: Vec<&str> = es.iter().map(|e| e["label"].as_str().unwrap()).collect();
        assert!(
            labels.contains(&"Trade") && labels.contains(&"NoAction"),
            "{es:?}"
        );
    }
}
