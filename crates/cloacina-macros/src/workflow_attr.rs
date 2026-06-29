/*
 *  Copyright 2026 Colliery Software
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

//! Unified `#[workflow]` attribute macro.
//!
//! Applied to a `pub mod` containing `#[task]` functions. Auto-discovers tasks,
//! validates dependencies, and generates registration code.
//!
//! - Without `packaged` feature: emits `inventory::submit!` entries that
//!   `cloacina::Runtime::seed_from_inventory` walks at runtime (embedded mode)
//! - With `packaged` feature: generates FFI exports (packaged mode)

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use std::collections::{hash_map::DefaultHasher, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use syn::{
    parse::{Parse, ParseStream},
    Expr, Ident, ItemMod, LitStr, Result as SynResult, Token,
};

use crate::packaged_workflow::{
    build_package_graph_data, detect_package_cycles, find_similar_package_task_names,
};
use crate::registry::get_registry;
use crate::tasks::TaskAttributes;

/// Attributes for the unified `#[workflow]` macro.
///
/// # Fields
///
/// * `name` - Unique identifier for the workflow (required)
/// * `tenant` - Tenant identifier (optional, defaults to "public")
/// * `description` - Optional description
/// * `author` - Optional author information
pub struct UnifiedWorkflowAttributes {
    pub name: String,
    pub tenant: String,
    pub description: Option<String>,
    pub author: Option<String>,
    /// I-0102 / T-A: trigger names this workflow subscribes to. The
    /// reconciler binds each named trigger → this workflow at load time
    /// (replaces the manifest-side `[[triggers]]` table that T-E removes).
    pub triggers: Vec<String>,
    /// CLOACI-I-0128 / T-0756: declared workflow params from
    /// `#[workflow(params( name: Type [= default], … ))]`. Surfaced as
    /// `InputSlot`s (JSON-Schema typed) via the input-interface FFI entrypoint.
    pub params: Vec<WorkflowParam>,
}

/// One declared workflow parameter (CLOACI-I-0128). `default = None` means the
/// param is required; `Some(expr)` makes it optional with that default.
pub struct WorkflowParam {
    pub name: String,
    pub ty: syn::Type,
    pub default: Option<syn::Expr>,
}

/// CLOACI-T-0829: one `constructor!(...)` declaration found inside a `#[workflow]`
/// module — a packaged constructor wired into the DAG as a primitive node.
///
/// The consumer form:
///
/// ```rust,ignore
/// constructor!(
///     id = "greet",                    // the DAG node id other tasks depend on
///     from = "acme/text@0.1",          // the provider package (name[@version])
///     constructor = "prefix",          // which constructor inside the provider
///     config = { prefix = "hello, " }, // bound once at load
///     dependencies = ["load_user"],    // wired into the DAG like a #[task]
/// );
/// ```
struct ConstructorNodeDecl {
    /// DAG node id (what dependents reference; distinct from the constructor's
    /// own `constructor.json` name).
    id: String,
    /// Provider package reference: `name[@version]`.
    from: String,
    /// Which constructor inside the provider (its `constructor.json` name).
    constructor: String,
    /// `config = { key = expr, … }` bound once at load (key → value expr).
    config: Vec<(String, Expr)>,
    /// Upstream DAG node ids this constructor depends on.
    dependencies: Vec<String>,
}

impl Parse for ConstructorNodeDecl {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut id: Option<String> = None;
        let mut from: Option<String> = None;
        let mut constructor: Option<String> = None;
        let mut config: Vec<(String, Expr)> = Vec::new();
        let mut dependencies: Vec<String> = Vec::new();

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "id" => id = Some(input.parse::<LitStr>()?.value()),
                "from" => from = Some(input.parse::<LitStr>()?.value()),
                "constructor" => constructor = Some(input.parse::<LitStr>()?.value()),
                "config" => {
                    // `config = { key = expr, … }`
                    let content;
                    syn::braced!(content in input);
                    while !content.is_empty() {
                        let ckey: Ident = content.parse()?;
                        content.parse::<Token![=]>()?;
                        let cval: Expr = content.parse()?;
                        config.push((ckey.to_string(), cval));
                        if !content.is_empty() {
                            content.parse::<Token![,]>()?;
                        }
                    }
                }
                "dependencies" => {
                    // `dependencies = ["a", "b"]`
                    let content;
                    syn::bracketed!(content in input);
                    while !content.is_empty() {
                        dependencies.push(content.parse::<LitStr>()?.value());
                        if !content.is_empty() {
                            content.parse::<Token![,]>()?;
                        }
                    }
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!(
                            "unknown constructor! field '{}'. Valid fields: id, from, \
                             constructor, config, dependencies",
                            other
                        ),
                    ));
                }
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        let id =
            id.ok_or_else(|| syn::Error::new(Span::call_site(), "constructor! requires an `id`"))?;
        let from = from.ok_or_else(|| {
            syn::Error::new(Span::call_site(), "constructor! requires a `from` provider")
        })?;
        let constructor = constructor.ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                "constructor! requires a `constructor` name",
            )
        })?;

        Ok(ConstructorNodeDecl {
            id,
            from,
            constructor,
            config,
            dependencies,
        })
    }
}

/// True if `item` is a `constructor!(...)` macro invocation (the consumer form the
/// `#[workflow]` macro lowers + strips).
fn is_constructor_macro(item: &syn::Item) -> bool {
    matches!(item, syn::Item::Macro(m) if m.mac.path.is_ident("constructor"))
}

impl Parse for UnifiedWorkflowAttributes {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut name = None;
        let mut tenant = None;
        let mut description = None;
        let mut author = None;
        let mut triggers: Vec<String> = Vec::new();
        let mut params: Vec<WorkflowParam> = Vec::new();

        while !input.is_empty() {
            let field_name: Ident = input.parse()?;

            // CLOACI-I-0128: `params( name: Type [= default], … )` uses call
            // syntax (parens), not `field = value` — handle it before the `=`.
            if field_name == "params" {
                let content;
                syn::parenthesized!(content in input);
                while !content.is_empty() {
                    let pname: Ident = content.parse()?;
                    content.parse::<Token![:]>()?;
                    let ty: syn::Type = content.parse()?;
                    let default = if content.peek(Token![=]) {
                        content.parse::<Token![=]>()?;
                        Some(content.parse::<syn::Expr>()?)
                    } else {
                        None
                    };
                    let pname_str = pname.to_string();
                    if params.iter().any(|p| p.name == pname_str) {
                        return Err(syn::Error::new(
                            pname.span(),
                            format!("duplicate workflow param: '{}'", pname_str),
                        ));
                    }
                    params.push(WorkflowParam {
                        name: pname_str,
                        ty,
                        default,
                    });
                    if !content.is_empty() {
                        content.parse::<Token![,]>()?;
                    }
                }
                if !input.is_empty() {
                    input.parse::<Token![,]>()?;
                }
                continue;
            }

            input.parse::<Token![=]>()?;

            match field_name.to_string().as_str() {
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
                "triggers" => {
                    // Array of string literals: triggers = ["t1", "t2"]
                    let content;
                    syn::bracketed!(content in input);
                    while !content.is_empty() {
                        let lit: LitStr = content.parse()?;
                        triggers.push(lit.value());
                        if !content.is_empty() {
                            content.parse::<Token![,]>()?;
                        }
                    }
                }
                _ => {
                    return Err(syn::Error::new(
                        field_name.span(),
                        format!(
                            "Unknown attribute: '{}'. Valid attributes: name, tenant, description, author, triggers, params",
                            field_name
                        ),
                    ));
                }
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        let name = name.ok_or_else(|| {
            syn::Error::new(Span::call_site(), "#[workflow] requires 'name' attribute")
        })?;

        Ok(UnifiedWorkflowAttributes {
            name,
            tenant: tenant.unwrap_or_else(|| "public".to_string()),
            description,
            author,
            triggers,
            params,
        })
    }
}

/// Entry point for the `#[workflow]` attribute macro.
pub fn workflow_attr(args: TokenStream, input: TokenStream) -> TokenStream {
    let attrs = match syn::parse::<UnifiedWorkflowAttributes>(args) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error().into(),
    };

    let input_mod = match syn::parse::<ItemMod>(input) {
        Ok(m) => m,
        Err(e) => {
            return syn::Error::new(
                Span::call_site(),
                format!("#[workflow] must be applied to a module: {}", e),
            )
            .to_compile_error()
            .into();
        }
    };

    generate_workflow_attr(attrs, input_mod).into()
}

/// Generate the unified workflow implementation.
///
/// In embedded mode (no `packaged` feature), generates:
/// - The original module with all task functions
/// - A workflow constructor function
/// - `inventory::submit!` entries for workflow + tasks (consumed by
///   `Runtime::seed_from_inventory` at runtime)
fn generate_workflow_attr(attrs: UnifiedWorkflowAttributes, input: ItemMod) -> TokenStream2 {
    let mod_name = &input.ident;
    let mod_vis = &input.vis;
    let mod_attrs = &input.attrs;
    let mod_content = &input.content;

    let workflow_name = &attrs.name;
    let tenant = &attrs.tenant;
    let description = attrs.description.as_deref().unwrap_or("").to_string();
    let author = attrs.author.as_deref().unwrap_or("").to_string();

    // Scan module for #[task] functions
    let mut detected_tasks: HashMap<String, syn::Ident> = HashMap::new();
    let mut task_dependencies: HashMap<String, Vec<String>> = HashMap::new();
    // CLOACI-T-0829: scan for `constructor!(...)` consumer declarations.
    let mut constructor_nodes: Vec<ConstructorNodeDecl> = Vec::new();

    if let Some((_, items)) = mod_content {
        for item in items {
            if is_constructor_macro(item) {
                if let syn::Item::Macro(item_macro) = item {
                    match syn::parse2::<ConstructorNodeDecl>(item_macro.mac.tokens.clone()) {
                        Ok(decl) => constructor_nodes.push(decl),
                        Err(e) => return e.to_compile_error(),
                    }
                }
                continue;
            }
            if let syn::Item::Fn(item_fn) = item {
                for attr in &item_fn.attrs {
                    if attr.path().is_ident("task") {
                        let fn_name = &item_fn.sig.ident;
                        // A bare `#[task]` (no parens) is a `Meta::Path`, so
                        // `parse_args` fails — fall back to default attrs rather
                        // than dropping the task from the DAG entirely
                        // (CLOACI-T-0732). `#[task(...)]` parses normally.
                        let task_attrs = match &attr.meta {
                            syn::Meta::Path(_) => TaskAttributes::default(),
                            _ => match attr.parse_args::<TaskAttributes>() {
                                Ok(a) => a,
                                Err(_) => {
                                    break;
                                }
                            },
                        };
                        // `id` defaults to the function name when omitted
                        // (CLOACI-T-0732). The task proc-macro applies the same
                        // default at expansion; the workflow macro reads the
                        // attrs directly to build the compile-time DAG, so it
                        // must resolve the default here too — otherwise a bare
                        // `#[task]` registers under an empty id.
                        let task_id = if task_attrs.id.is_empty() {
                            fn_name.to_string()
                        } else {
                            task_attrs.id.clone()
                        };
                        detected_tasks.insert(task_id.clone(), fn_name.clone());
                        task_dependencies.insert(task_id, task_attrs.dependencies.clone());
                        break;
                    }
                }
            }
        }
    }

    if detected_tasks.is_empty() && constructor_nodes.is_empty() {
        return syn::Error::new(
            mod_name.span(),
            "#[workflow] module must contain at least one #[task] function or constructor!(…) node",
        )
        .to_compile_error();
    }

    // CLOACI-T-0829: combine #[task] nodes + constructor!(…) nodes for dependency
    // validation and cycle detection so a task may depend on a constructor node
    // (and vice versa) and so a constructor node's own deps are checked.
    let mut combined_deps: HashMap<String, Vec<String>> = task_dependencies.clone();
    let mut available_ids: HashSet<String> = detected_tasks.keys().cloned().collect();
    for decl in &constructor_nodes {
        if available_ids.contains(&decl.id) {
            return syn::Error::new(
                mod_name.span(),
                format!(
                    "constructor!(id = \"{}\") collides with another node id in workflow '{}'",
                    decl.id, workflow_name
                ),
            )
            .to_compile_error();
        }
        available_ids.insert(decl.id.clone());
        combined_deps.insert(decl.id.clone(), decl.dependencies.clone());
    }

    // Validate dependencies
    let validation_error = validate_dependencies(workflow_name, &available_ids, &combined_deps);
    if let Some(err) = validation_error {
        return err;
    }

    // Check for cycles
    if let Err(cycle_error) = detect_package_cycles(&combined_deps) {
        let error_msg = format!(
            "Circular dependency detected in workflow '{}': {}\n\n\
            Hint: Review your task dependencies to eliminate cycles.",
            workflow_name, cycle_error
        );
        return quote! { compile_error!(#error_msg); };
    }

    // Generate fingerprint
    let mut hasher = DefaultHasher::new();
    workflow_name.hash(&mut hasher);
    if let Some((_, items)) = mod_content {
        for item in items {
            quote!(#item).to_string().hash(&mut hasher);
        }
    }
    let fingerprint = format!("{:016x}", hasher.finish());

    // Build graph data
    let graph_data_json =
        build_package_graph_data(&detected_tasks, &task_dependencies, workflow_name);

    // Generate the module items (original content preserved).
    //
    // CLOACI-T-0734: auto-injecting `use super::*;` here was tried and reverted
    // — it makes every existing workflow module's own `use super::*;` a
    // redundant glob and emits an `unused_imports` warning across the whole
    // codebase. The net ergonomic win (dropping one line) isn't worth the
    // warning noise unless we also sweep the manual imports out everywhere; the
    // signature de-ceremony (bare `Context` / `Result<()>`) is the substantive
    // win and lands without that side effect.
    // CLOACI-T-0829: strip the `constructor!(…)` invocations — they are lowered to
    // DAG-node registration (below), not re-emitted as module items.
    let module_items = if let Some((_, items)) = mod_content {
        let kept: Vec<&syn::Item> = items
            .iter()
            .filter(|it| !is_constructor_macro(it))
            .collect();
        quote! { #(#kept)* }
    } else {
        quote! {}
    };

    // I-0102 / T-C: TaskEntry inventory submissions emitted unconditionally
    // so the unified `cloacina::package!()` shell can walk them in packaged
    // cdylib builds. Embedded mode also needs them (Runtime::new() seeds
    // tasks from inventory).
    let task_inventory_entries =
        build_task_inventory_entries(tenant, workflow_name, mod_name, &detected_tasks);

    // CLOACI-T-0829: constructor!(…) DAG nodes register as TaskEntry's too (so the
    // executor resolves them via Runtime::get_task), but only in EMBEDDED mode —
    // the consumer form resolves + loads a WASM provider through cloacina's loader,
    // which a packaged (cdylib) build does not link. Packaged-mode constructor
    // nodes are a noted follow-on.
    let constructor_inventory_entries =
        build_constructor_inventory_entries(tenant, workflow_name, &constructor_nodes);

    // I-0102 / T-C: WorkflowDescriptorEntry carries metadata the shell
    // can't derive from TaskEntry alone (description, author, fingerprint,
    // graph_data, triggers).
    //
    // Emitted in both modes (library `cargo run` and packaged cdylib) so
    // `Runtime::new()` and `package!()` both pick it up. The cfg-gated paths
    // resolve through whichever crate the user actually depends on:
    // `cloacina` re-exports `cloacina_workflow_plugin` for library mode;
    // packaged cdylibs depend on `cloacina-workflow-plugin` directly.
    let triggers_vec: Vec<String> = attrs.triggers.clone();

    // CLOACI-I-0128 / T-0756: build the `params` descriptor fn for the
    // WorkflowDescriptorEntry. Emitted once per crate-path context — embedded
    // resolves through `cloacina`'s re-export, packaged through
    // `cloacina-workflow` directly. The fn runs at metadata-extraction time and
    // produces a JSON array of `InputSlot` (schemars-typed) for the params.
    let make_params_fn = |prefix: TokenStream2| -> TokenStream2 {
        let slot_exprs: Vec<TokenStream2> = attrs
            .params
            .iter()
            .map(|p| {
                let pname = &p.name;
                let ty = &p.ty;
                match &p.default {
                    None => quote! {
                        #prefix::InputSlot::required(#pname, #prefix::schema_for::<#ty>())
                    },
                    Some(def) => quote! {
                        #prefix::InputSlot::optional(
                            #pname,
                            #prefix::schema_for::<#ty>(),
                            #prefix::default_json(#def),
                        )
                    },
                }
            })
            .collect();
        quote! {
            || {
                let slots: ::std::vec::Vec<#prefix::InputSlot> =
                    ::std::vec![ #(#slot_exprs),* ];
                #prefix::slots_to_json(&slots)
            }
        }
    };
    let params_fn_embedded = make_params_fn(quote! { ::cloacina::input_interface });
    let params_fn_packaged = make_params_fn(quote! { ::cloacina_workflow::input_interface });

    let workflow_descriptor_entry = quote! {
        #[cfg(not(feature = "packaged"))]
        ::cloacina::cloacina_workflow_plugin::inventory::submit! {
            ::cloacina::cloacina_workflow_plugin::WorkflowDescriptorEntry {
                name: #workflow_name,
                description: #description,
                author: #author,
                fingerprint: #fingerprint,
                graph_data_json: #graph_data_json,
                triggers: || vec![#(#triggers_vec.to_string()),*],
                params: #params_fn_embedded,
            }
        }

        #[cfg(feature = "packaged")]
        ::cloacina_workflow_plugin::inventory::submit! {
            ::cloacina_workflow_plugin::WorkflowDescriptorEntry {
                name: #workflow_name,
                description: #description,
                author: #author,
                fingerprint: #fingerprint,
                graph_data_json: #graph_data_json,
                triggers: || vec![#(#triggers_vec.to_string()),*],
                params: #params_fn_packaged,
            }
        }
    };

    // Generate embedded registration (when `packaged` feature is NOT active)
    let embedded_registration = generate_embedded_registration(
        mod_name,
        workflow_name,
        tenant,
        &description,
        &author,
        &fingerprint,
        &detected_tasks,
        &task_dependencies,
        &constructor_nodes,
    );

    // Generate packaged FFI exports (when `packaged` feature IS active)
    let packaged_registration = generate_packaged_registration(
        mod_name,
        workflow_name,
        &description,
        &author,
        &fingerprint,
        &graph_data_json,
        &detected_tasks,
        &task_dependencies,
        &attrs.triggers,
    );

    // I-0102 / T-C: per-macro `_ffi` plugin emission stripped. The unified
    // `cloacina::package!()` shell at the crate root is now the sole path
    // that produces a CloacinaPlugin export. Packaged cdylibs MUST declare
    // `cloacina::package!();` at the crate root for fidius to find them.
    let _ = packaged_registration;

    quote! {
        #(#mod_attrs)*
        #mod_vis mod #mod_name {
            #module_items
        }

        // I-0102 / T-C: TaskEntry + WorkflowDescriptorEntry inventory
        // submissions fire in BOTH packaged and embedded modes. The unified
        // `cloacina::package!()` shell walks these from packaged cdylibs.
        #(#task_inventory_entries)*

        // CLOACI-T-0829: constructor!(…) node registrations (embedded-only).
        #(#constructor_inventory_entries)*

        #workflow_descriptor_entry

        #[cfg(not(feature = "packaged"))]
        const _: () = {
            #embedded_registration
        };
    }
}

/// Validate task dependencies within the module. `available_ids` is the combined
/// set of `#[task]` ids + `constructor!(…)` node ids declared in this module
/// (CLOACI-T-0829), so a dependency may point at either node kind.
fn validate_dependencies(
    workflow_name: &str,
    available_ids: &HashSet<String>,
    task_dependencies: &HashMap<String, Vec<String>>,
) -> Option<TokenStream2> {
    // Check if we're in a test environment
    let is_test_env = std::env::var("CARGO_CRATE_NAME")
        .map(|name| name.contains("test") || name == "cloacina")
        .unwrap_or(false)
        || std::env::var("CARGO_PKG_NAME")
            .map(|name| name.contains("test") || name == "cloacina")
            .unwrap_or(false);

    if is_test_env {
        return None;
    }

    for (task_id, dependencies) in task_dependencies {
        for dep in dependencies {
            if !available_ids.contains(dep) {
                // Check global registry
                let validation = match get_registry().try_lock() {
                    Ok(registry) => {
                        if registry.get_all_task_ids().contains(dep) {
                            Ok(())
                        } else {
                            let available: Vec<String> = available_ids.iter().cloned().collect();
                            let suggestions = find_similar_package_task_names(dep, &available);

                            let mut msg = format!(
                                "Task '{}' in workflow '{}' depends on undefined task '{}'.\n\n",
                                task_id, workflow_name, dep
                            );
                            if !suggestions.is_empty() {
                                msg.push_str(&format!(
                                    "Did you mean: {}?\n\n",
                                    suggestions.join(", ")
                                ));
                            }
                            msg.push_str(&format!("Available tasks: [{}]", available.join(", ")));
                            Err(msg)
                        }
                    }
                    Err(_) => Ok(()), // Skip if can't lock
                };

                if let Err(error_msg) = validation {
                    return Some(quote! { compile_error!(#error_msg); });
                }
            }
        }
    }

    None
}

/// Generate embedded mode registration code.
///
/// Creates task constructors, workflow constructor, namespace registration,
/// and `inventory::submit!` entries for runtime seeding.
#[allow(clippy::too_many_arguments)]
fn generate_embedded_registration(
    mod_name: &syn::Ident,
    workflow_name: &str,
    tenant: &str,
    description: &str,
    author: &str,
    _fingerprint: &str,
    detected_tasks: &HashMap<String, syn::Ident>,
    _task_dependencies: &HashMap<String, Vec<String>>,
    constructor_nodes: &[ConstructorNodeDecl],
) -> TokenStream2 {
    let mod_path_prefix = quote! { #mod_name };

    // CLOACI-T-0829: add each constructor!(…) node to the DAG. The node is the
    // dynamic analog of a #[task]: it carries the author-chosen id + namespaced
    // dependencies, and its `execute` delegates into the loaded WASM constructor.
    let constructor_addition_code: Vec<TokenStream2> = constructor_nodes
        .iter()
        .map(|decl| {
            let load_block = constructor_node_load_block(decl, tenant, workflow_name);
            quote! {
                workflow
                    .add_task(#load_block)
                    .expect("Failed to add constructor node to workflow");
            }
        })
        .collect();

    // Generate workflow constructor
    let task_addition_code: Vec<TokenStream2> = detected_tasks.values().map(|fn_name| {
            let constructor_name = syn::Ident::new(
                &format!("{}_task", fn_name),
                fn_name.span(),
            );
            let task_str = fn_name.to_string();
            let parts: Vec<&str> = task_str.split('_').collect();
            let pascal_case = parts
                .iter()
                .map(|part| {
                    let mut chars = part.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                })
                .collect::<String>();
            let struct_name = syn::Ident::new(&format!("{}Task", pascal_case), fn_name.span());

            let rewrite_trigger_rules = generate_trigger_rules_rewrite(tenant, workflow_name);

            quote! {
                {
                    let task = #mod_path_prefix::#constructor_name();
                    let rewritten_trigger_rules = {
                        let task_ref = &task;
                        #rewrite_trigger_rules
                    };
                    let dep_ids = #mod_path_prefix::#struct_name::dependency_task_ids();
                    let pkg_name = env!("CARGO_PKG_NAME");
                    let dep_namespaces: Vec<cloacina_workflow::TaskNamespace> = dep_ids.iter()
                        .map(|dep_id| cloacina_workflow::TaskNamespace::new(
                            #tenant,
                            pkg_name,
                            #workflow_name,
                            dep_id
                        ))
                        .collect();

                    let task_with_deps = task.with_dependencies(dep_namespaces);

                    struct TaskWithNamespacedTriggers<T> {
                        inner: T,
                        rewritten_trigger_rules: serde_json::Value,
                    }

                    #[async_trait::async_trait]
                    impl<T: cloacina_workflow::Task> cloacina_workflow::Task for TaskWithNamespacedTriggers<T> {
                        async fn execute(&self, context: cloacina_workflow::Context<serde_json::Value>)
                            -> Result<cloacina_workflow::Context<serde_json::Value>, cloacina_workflow::TaskError> {
                            self.inner.execute(context).await
                        }
                        fn id(&self) -> &str { self.inner.id() }
                        fn dependencies(&self) -> &[cloacina_workflow::TaskNamespace] { self.inner.dependencies() }
                        fn retry_policy(&self) -> cloacina_workflow::retry::RetryPolicy { self.inner.retry_policy() }
                        fn trigger_rules(&self) -> serde_json::Value { self.rewritten_trigger_rules.clone() }
                        fn code_fingerprint(&self) -> Option<String> { self.inner.code_fingerprint() }
                        fn requires_handle(&self) -> bool { self.inner.requires_handle() }
                    }

                    workflow.add_task(std::sync::Arc::new(TaskWithNamespacedTriggers {
                        inner: task_with_deps,
                        rewritten_trigger_rules,
                    })).expect("Failed to add task to workflow");
                }
            }
        })
        .collect();

    let safe_name = workflow_name.replace(['-', ' '], "_");
    let workflow_constructor_name = syn::Ident::new(
        &format!("_workflow_{}_constructor", safe_name),
        Span::call_site(),
    );

    let description_field = if !description.is_empty() {
        quote! { workflow.set_description(#description); }
    } else {
        quote! {}
    };

    let author_field = if !author.is_empty() {
        quote! { workflow.add_tag("author", #author); }
    } else {
        quote! {}
    };

    quote! {
        fn #workflow_constructor_name() -> cloacina::Workflow {
            let pkg_name = env!("CARGO_PKG_NAME");

            let mut workflow = cloacina::Workflow::new(#workflow_name);
            workflow.set_tenant(#tenant);
            workflow.set_package(pkg_name);
            #description_field
            #author_field

            // Add tasks
            #(#task_addition_code)*

            // Add constructor!(…) DAG nodes (CLOACI-T-0829)
            #(#constructor_addition_code)*

            workflow.validate().expect("Workflow validation failed");
            workflow.finalize()
        }

        cloacina::inventory::submit! {
            cloacina::WorkflowEntry {
                name: #workflow_name,
                constructor: #workflow_constructor_name,
            }
        }
    }
}

/// CLOACI-T-0829: an expression evaluating to a cached
/// `Arc<dyn cloacina_workflow::Task>` for one `constructor!(…)` node.
///
/// Resolves the provider + loads the WASM constructor through cloacina's loader
/// (binding `config` once), wrapping it as a DAG node carrying the author-chosen
/// id + namespaced dependencies. The load is memoized in a per-call-site
/// `OnceLock` so re-instantiating the workflow / resolving the task does not
/// re-load the component each time.
fn constructor_node_load_block(
    decl: &ConstructorNodeDecl,
    tenant: &str,
    workflow_name: &str,
) -> TokenStream2 {
    let id = &decl.id;
    let from = &decl.from;
    let constructor = &decl.constructor;
    let deps = &decl.dependencies;
    // CLOACI-T-0829: `config = { name = value }` is NAME-keyed (true kwarg
    // semantics). fidius binds `#[config]` via bincode (positional, not
    // self-describing), so the author's WRITTEN order can't be trusted — instead we
    // hand the loader each `(name, value)` pair (value lowered to a `serde_json`
    // literal) and it reorders them into the guest's `#[config]` DECLARATION order
    // (read from the manifest) before serializing the bincode config tuple.
    let cfg_pairs = decl.config.iter().map(|(k, v)| {
        quote! { (#k.to_string(), ::cloacina::serde_json::json!(#v)) }
    });

    quote! {
        {
            static __NODE: ::std::sync::OnceLock<
                ::std::sync::Arc<dyn ::cloacina_workflow::Task>,
            > = ::std::sync::OnceLock::new();
            __NODE
                .get_or_init(|| {
                    let pkg_name = env!("CARGO_PKG_NAME");
                    let __deps: ::std::vec::Vec<::cloacina_workflow::TaskNamespace> = ::std::vec![
                        #(
                            ::cloacina_workflow::TaskNamespace::new(
                                #tenant, pkg_name, #workflow_name, #deps,
                            )
                        ),*
                    ];
                    let __config: ::std::vec::Vec<(
                        ::std::string::String,
                        ::cloacina::serde_json::Value,
                    )> = ::std::vec![ #(#cfg_pairs),* ];
                    ::cloacina::registry::loader::load_constructor_node(
                        #id, #from, #constructor, __config, __deps,
                    )
                    .unwrap_or_else(|e| {
                        ::std::panic!("constructor!(id = \"{}\"): {}", #id, e)
                    })
                })
                .clone()
        }
    }
}

/// CLOACI-T-0829: `TaskEntry` inventory submissions for `constructor!(…)` nodes so
/// `Runtime::get_task` resolves them at execution time. Embedded-only (the loader
/// path is not linked in packaged cdylib builds).
fn build_constructor_inventory_entries(
    tenant: &str,
    workflow_name: &str,
    constructor_nodes: &[ConstructorNodeDecl],
) -> Vec<TokenStream2> {
    constructor_nodes
        .iter()
        .map(|decl| {
            let id = &decl.id;
            let load_block = constructor_node_load_block(decl, tenant, workflow_name);
            quote! {
                #[cfg(not(feature = "packaged"))]
                ::cloacina::cloacina_workflow_plugin::inventory::submit! {
                    ::cloacina::cloacina_workflow_plugin::TaskEntry {
                        namespace: || ::cloacina_workflow::TaskNamespace::new(
                            #tenant,
                            env!("CARGO_PKG_NAME"),
                            #workflow_name,
                            #id,
                        ),
                        constructor: || #load_block,
                    }
                }
            }
        })
        .collect()
}

/// Build `inventory::submit!` blocks for each task in the workflow.
///
/// Emitted unconditionally (both `feature = "packaged"` and not) so the
/// unified `cloacina::package!()` shell can walk
/// `inventory::iter::<TaskEntry>` from packaged cdylib builds. (T-C / I-0102)
fn build_task_inventory_entries(
    tenant: &str,
    workflow_name: &str,
    mod_name: &syn::Ident,
    detected_tasks: &HashMap<String, syn::Ident>,
) -> Vec<TokenStream2> {
    let mod_path_prefix = quote! { #mod_name };
    let rewrite_trigger_rules_for_inventory = generate_trigger_rules_rewrite(tenant, workflow_name);

    detected_tasks
        .iter()
        .map(|(task_id, fn_name)| {
            let constructor_name = syn::Ident::new(&format!("{}_task", fn_name), fn_name.span());
            let task_str = fn_name.to_string();
            let parts: Vec<&str> = task_str.split('_').collect();
            let pascal_case = parts
                .iter()
                .map(|part| {
                    let mut chars = part.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                })
                .collect::<String>();
            let struct_name = syn::Ident::new(&format!("{}Task", pascal_case), fn_name.span());
            let rewrite_body = rewrite_trigger_rules_for_inventory.clone();
            // The TaskEntry struct-literal body. References to
            // `cloacina_workflow::*` resolve via the leaf authoring crate
            // (always a direct dep). The outer `cloacina_workflow_plugin`
            // path is cfg-conditional below so it resolves through
            // `::cloacina::*` in library mode and directly in packaged mode.
            let task_entry_body = quote! {
                {
                    namespace: || cloacina_workflow::TaskNamespace::new(
                        #tenant,
                        env!("CARGO_PKG_NAME"),
                        #workflow_name,
                        #task_id,
                    ),
                    constructor: || {
                        let task = #mod_path_prefix::#constructor_name();
                        let rewritten_trigger_rules = {
                            let task_ref = &task;
                            #rewrite_body
                        };

                        let dep_ids = #mod_path_prefix::#struct_name::dependency_task_ids();
                        let pkg_name = env!("CARGO_PKG_NAME");
                        let dep_namespaces: Vec<cloacina_workflow::TaskNamespace> = dep_ids
                            .iter()
                            .map(|dep_id| cloacina_workflow::TaskNamespace::new(
                                #tenant,
                                pkg_name,
                                #workflow_name,
                                dep_id,
                            ))
                            .collect();

                        let task_with_deps = task.with_dependencies(dep_namespaces);

                        struct TaskWithNamespacedTriggers<T> {
                            inner: T,
                            rewritten_trigger_rules: serde_json::Value,
                        }

                        #[async_trait::async_trait]
                        impl<T: cloacina_workflow::Task> cloacina_workflow::Task for TaskWithNamespacedTriggers<T> {
                            async fn execute(
                                &self,
                                context: cloacina_workflow::Context<serde_json::Value>,
                            ) -> Result<cloacina_workflow::Context<serde_json::Value>, cloacina_workflow::TaskError>
                            {
                                self.inner.execute(context).await
                            }
                            fn id(&self) -> &str { self.inner.id() }
                            fn dependencies(&self) -> &[cloacina_workflow::TaskNamespace] {
                                self.inner.dependencies()
                            }
                            fn retry_policy(&self) -> cloacina_workflow::retry::RetryPolicy {
                                self.inner.retry_policy()
                            }
                            fn trigger_rules(&self) -> serde_json::Value {
                                self.rewritten_trigger_rules.clone()
                            }
                            fn code_fingerprint(&self) -> Option<String> {
                                self.inner.code_fingerprint()
                            }
                            fn requires_handle(&self) -> bool {
                                self.inner.requires_handle()
                            }
                        }

                        std::sync::Arc::new(TaskWithNamespacedTriggers {
                            inner: task_with_deps,
                            rewritten_trigger_rules,
                        })
                            as std::sync::Arc<dyn cloacina_workflow::Task>
                    },
                }
            };
            quote! {
                #[cfg(not(feature = "packaged"))]
                ::cloacina::cloacina_workflow_plugin::inventory::submit! {
                    ::cloacina::cloacina_workflow_plugin::TaskEntry #task_entry_body
                }

                #[cfg(feature = "packaged")]
                ::cloacina_workflow_plugin::inventory::submit! {
                    ::cloacina_workflow_plugin::TaskEntry #task_entry_body
                }
            }
        })
        .collect()
}

/// Generate trigger rules rewrite code (namespace task names in trigger conditions).
fn generate_trigger_rules_rewrite(tenant: &str, workflow_name: &str) -> TokenStream2 {
    quote! {
        {
            let trigger_rules = task_ref.trigger_rules();
            let mut rules_json: serde_json::Value = trigger_rules;
            let pkg_name = env!("CARGO_PKG_NAME");

            fn rewrite_task_names_in_value(
                value: &mut serde_json::Value,
                tenant: &str,
                package: &str,
                workflow_name: &str,
            ) {
                match value {
                    serde_json::Value::Object(map) => {
                        if let (Some(condition_type), Some(task_name)) = (
                            map.get("type").and_then(|v| v.as_str()),
                            map.get("task_name").and_then(|v| v.as_str())
                        ) {
                            if matches!(condition_type, "TaskSuccess" | "TaskFailed" | "TaskSkipped") {
                                if !task_name.contains("::") {
                                    let full_name = format!("{}::{}::{}::{}", tenant, package, workflow_name, task_name);
                                    map.insert("task_name".to_string(), serde_json::Value::String(full_name));
                                }
                            }
                        }
                        for (_, v) in map.iter_mut() {
                            rewrite_task_names_in_value(v, tenant, package, workflow_name);
                        }
                    }
                    serde_json::Value::Array(arr) => {
                        for item in arr.iter_mut() {
                            rewrite_task_names_in_value(item, tenant, package, workflow_name);
                        }
                    }
                    _ => {}
                }
            }

            rewrite_task_names_in_value(&mut rules_json, #tenant, pkg_name, #workflow_name);
            rules_json
        }
    }
}

/// Generate packaged mode FFI exports.
///
/// Creates C-compatible metadata structures and FFI entry points for
/// dynamic loading via `PackageLoader`. Package name comes from `CARGO_PKG_NAME`.
#[allow(clippy::too_many_arguments)]
fn generate_packaged_registration(
    mod_name: &syn::Ident,
    workflow_name: &str,
    description: &str,
    author: &str,
    fingerprint: &str,
    graph_data_json: &str,
    detected_tasks: &HashMap<String, syn::Ident>,
    task_dependencies: &HashMap<String, Vec<String>>,
    triggers: &[String],
) -> TokenStream2 {
    let package_description = if description.is_empty() {
        format!("Workflow: {}", workflow_name)
    } else {
        description.to_string()
    };
    let package_author = if author.is_empty() {
        "Unknown".to_string()
    } else {
        author.to_string()
    };

    // I-0102 / T-A: Workflow's trigger subscriptions (string names) flow
    // through to PackageTasksMetadata.triggers; the reconciler binds each
    // named trigger to this workflow at load time.
    let triggers_lits: Vec<String> = triggers.to_vec();

    // Generate task execution match arms
    let mut task_execution_cases = Vec::new();

    for (task_id, fn_name) in detected_tasks.iter() {
        task_execution_cases.push(quote! {
            #task_id => {
                match #fn_name(&mut context).await {
                    Ok(()) => Ok(()),
                    Err(e) => Err(format!("Task '{}' failed: {:?}", #task_id, e))
                }
            }
        });
    }

    // Build task metadata entries for get_task_metadata() response
    let metadata_entries: Vec<_> = detected_tasks
        .iter()
        .enumerate()
        .map(|(i, (task_id, _))| {
            let deps = task_dependencies.get(task_id).cloned().unwrap_or_default();
            let namespaced_id = format!("{{tenant}}::{{pkg}}::{}::{}", workflow_name, task_id);
            let source_location = format!("src/{}.rs", mod_name);
            let dep_strs: Vec<_> = deps.iter().map(|d| quote! { #d.to_string() }).collect();
            let idx = i as u32;

            quote! {
                cloacina_workflow_plugin::TaskMetadataEntry {
                    index: #idx,
                    id: #task_id.to_string(),
                    namespaced_id_template: #namespaced_id.to_string(),
                    dependencies: vec![#(#dep_strs),*],
                    description: format!("Task: {}", #task_id),
                    source_location: #source_location.to_string(),
                }
            }
        })
        .collect();

    quote! {
        use cloacina_workflow_plugin::__fidius_CloacinaPlugin;
        use cloacina_workflow_plugin::CloacinaPlugin as _;

        /// Workflow plugin implementation for fidius.
        pub struct _WorkflowPlugin;

        #[cloacina_workflow_plugin::plugin_impl(CloacinaPlugin, crate = "cloacina_workflow_plugin")]
        impl cloacina_workflow_plugin::CloacinaPlugin for _WorkflowPlugin {
            fn get_task_metadata(&self) -> Result<cloacina_workflow_plugin::PackageTasksMetadata, cloacina_workflow_plugin::PluginError> {
                Ok(cloacina_workflow_plugin::PackageTasksMetadata {
                    workflow_name: #workflow_name.to_string(),
                    package_name: env!("CARGO_PKG_NAME").to_string(),
                    package_description: Some(#package_description.to_string()),
                    package_author: Some(#package_author.to_string()),
                    workflow_fingerprint: Some(#fingerprint.to_string()),
                    graph_data_json: Some(#graph_data_json.to_string()),
                    tasks: vec![
                        #(#metadata_entries),*
                    ],
                    triggers: vec![#(#triggers_lits.to_string()),*],
                })
            }

            fn execute_task(&self, request: cloacina_workflow_plugin::TaskExecutionRequest) -> Result<cloacina_workflow_plugin::TaskExecutionResult, cloacina_workflow_plugin::PluginError> {
                static CDYLIB_RUNTIME: std::sync::OnceLock<cloacina_workflow::__private::tokio::runtime::Runtime> = std::sync::OnceLock::new();

                let rt = CDYLIB_RUNTIME.get_or_init(|| {
                    cloacina_workflow::__private::tokio::runtime::Builder::new_multi_thread()
                        .enable_all()
                        .worker_threads(2)
                        .thread_name("cdylib-worker")
                        .build()
                        .expect("Failed to create cdylib tokio runtime")
                });

                let mut context = cloacina_workflow::Context::from_json(request.context_json)
                    .map_err(|e| cloacina_workflow_plugin::PluginError {
                        code: "CONTEXT_ERROR".to_string(),
                        message: format!("Failed to create context: {}", e),
                        details: None,
                    })?;

                let task_result = rt.block_on(async {
                    match request.task_name.as_str() {
                        #(#task_execution_cases)*
                        _ => Err(format!("Unknown task: {}", request.task_name))
                    }
                });

                match task_result {
                    Ok(()) => {
                        let ctx_json = context.to_json().map_err(|e| cloacina_workflow_plugin::PluginError {
                            code: "SERIALIZATION_ERROR".to_string(),
                            message: format!("Failed to serialize context: {}", e),
                            details: None,
                        })?;
                        Ok(cloacina_workflow_plugin::TaskExecutionResult {
                            success: true,
                            context_json: Some(ctx_json),
                            error: None,
                        })
                    }
                    Err(e) => Ok(cloacina_workflow_plugin::TaskExecutionResult {
                        success: false,
                        context_json: None,
                        error: Some(e),
                    }),
                }
            }

            fn get_graph_metadata(&self) -> Result<cloacina_workflow_plugin::GraphPackageMetadata, cloacina_workflow_plugin::PluginError> {
                Err(cloacina_workflow_plugin::PluginError {
                    code: "NOT_SUPPORTED".to_string(),
                    message: "This is a workflow package, not a computation graph package".to_string(),
                    details: None,
                })
            }

            fn execute_graph(&self, _request: cloacina_workflow_plugin::GraphExecutionRequest) -> Result<cloacina_workflow_plugin::GraphExecutionResult, cloacina_workflow_plugin::PluginError> {
                Err(cloacina_workflow_plugin::PluginError {
                    code: "NOT_SUPPORTED".to_string(),
                    message: "This is a workflow package, not a computation graph package".to_string(),
                    details: None,
                })
            }

            // T-A / I-0102: per-macro `_ffi` blocks predate the unified
            // `cloacina::package!()` shell. They report "no reactors" /
            // "no triggers" — packages built against the new shell expose
            // reactor + trigger metadata directly via the shell instead.
            // Method order in the impl block must match trait declaration
            // order (fidius's vtable is positional).
            fn get_reactor_metadata(&self) -> Result<Vec<cloacina_workflow_plugin::ReactorPackageMetadata>, cloacina_workflow_plugin::PluginError> {
                Ok(Vec::new())
            }

            fn get_trigger_metadata(&self) -> Result<Vec<cloacina_workflow_plugin::TriggerPackageMetadata>, cloacina_workflow_plugin::PluginError> {
                Ok(Vec::new())
            }
        }

        cloacina_workflow_plugin::fidius_plugin_registry!();
    }
}
