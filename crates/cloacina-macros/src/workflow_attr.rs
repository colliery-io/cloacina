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
//! - Without `packaged` feature: generates `#[ctor]` auto-registration (embedded mode)
//! - With `packaged` feature: generates FFI exports (packaged mode) — added in T-0303

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use syn::{
    parse::{Parse, ParseStream},
    Ident, ItemMod, LitStr, Result as SynResult, Token,
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
}

impl Parse for UnifiedWorkflowAttributes {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut name = None;
        let mut tenant = None;
        let mut description = None;
        let mut author = None;

        while !input.is_empty() {
            let field_name: Ident = input.parse()?;
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
                _ => {
                    return Err(syn::Error::new(
                        field_name.span(),
                        format!(
                            "Unknown attribute: '{}'. Valid attributes: name, tenant, description, author",
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
/// - `#[ctor]` auto-registration for workflow + tasks
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

    if let Some((_, items)) = mod_content {
        for item in items {
            if let syn::Item::Fn(item_fn) = item {
                for attr in &item_fn.attrs {
                    if attr.path().is_ident("task") {
                        if let Ok(task_attrs) = attr.parse_args::<TaskAttributes>() {
                            let fn_name = &item_fn.sig.ident;
                            detected_tasks.insert(task_attrs.id.clone(), fn_name.clone());
                            task_dependencies
                                .insert(task_attrs.id.clone(), task_attrs.dependencies.clone());
                        }
                        break;
                    }
                }
            }
        }
    }

    if detected_tasks.is_empty() {
        return syn::Error::new(
            mod_name.span(),
            "#[workflow] module must contain at least one #[task] function",
        )
        .to_compile_error();
    }

    // Validate dependencies
    let validation_error =
        validate_dependencies(workflow_name, &detected_tasks, &task_dependencies);
    if let Some(err) = validation_error {
        return err;
    }

    // Check for cycles
    if let Err(cycle_error) = detect_package_cycles(&task_dependencies) {
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

    // Generate the module items (original content preserved)
    let module_items = if let Some((_, items)) = mod_content {
        quote! { #(#items)* }
    } else {
        quote! {}
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
    );

    let _packaged_mod_name = syn::Ident::new(
        &format!("_packaged_ffi_{}", workflow_name.replace(['-', ' '], "_")),
        Span::call_site(),
    );

    quote! {
        #(#mod_attrs)*
        #mod_vis mod #mod_name {
            #module_items

            // Packaged FFI code lives inside the module (same scope as task functions)
            // Wrapped in cfg-gated sub-module to exclude all items when not packaged
            #[cfg(feature = "packaged")]
            pub mod _ffi {
                use super::*;
                #packaged_registration
            }
        }

        #[cfg(not(feature = "packaged"))]
        const _: () = {
            #embedded_registration
        };
    }
}

/// Validate task dependencies within the module.
fn validate_dependencies(
    workflow_name: &str,
    detected_tasks: &HashMap<String, syn::Ident>,
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
            if !detected_tasks.contains_key(dep) {
                // Check global registry
                let validation = match get_registry().try_lock() {
                    Ok(registry) => {
                        if registry.get_all_task_ids().contains(dep) {
                            Ok(())
                        } else {
                            let available: Vec<String> = detected_tasks.keys().cloned().collect();
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
/// and `#[ctor]` auto-registration.
#[allow(clippy::too_many_arguments)]
fn generate_embedded_registration(
    mod_name: &syn::Ident,
    workflow_name: &str,
    tenant: &str,
    description: &str,
    author: &str,
    _fingerprint: &str,
    detected_tasks: &HashMap<String, syn::Ident>,
    task_dependencies: &HashMap<String, Vec<String>>,
) -> TokenStream2 {
    let mod_path_prefix = quote! { #mod_name };

    // Generate task registration code for each task
    let task_registrations: Vec<TokenStream2> = detected_tasks
        .iter()
        .map(|(task_id, fn_name)| {
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

            let deps = task_dependencies
                .get(task_id)
                .cloned()
                .unwrap_or_default();

            let _dep_namespace_exprs: Vec<TokenStream2> = deps
                .iter()
                .map(|dep_id| {
                    quote! {
                        cloacina::TaskNamespace::new(
                            #tenant,
                            pkg_name,
                            #workflow_name,
                            #dep_id
                        )
                    }
                })
                .collect();

            let rewrite_trigger_rules = generate_trigger_rules_rewrite(tenant, workflow_name);

            quote! {
                {
                    let namespace = cloacina::TaskNamespace::new(
                        #tenant,
                        pkg_name,
                        #workflow_name,
                        #task_id
                    );

                    cloacina::register_task_constructor(
                        namespace,
                        || {
                            let task = #mod_path_prefix::#constructor_name();
                            let rewritten_trigger_rules = {
                                let task_ref = &task;
                                #rewrite_trigger_rules
                            };

                            let dep_ids = #mod_path_prefix::#struct_name::dependency_task_ids();
                            let pkg_name = env!("CARGO_PKG_NAME");
                            let dep_namespaces: Vec<cloacina::TaskNamespace> = dep_ids.iter()
                                .map(|dep_id| cloacina::TaskNamespace::new(
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
                            impl<T: cloacina::Task> cloacina::Task for TaskWithNamespacedTriggers<T> {
                                async fn execute(&self, context: cloacina::Context<serde_json::Value>)
                                    -> Result<cloacina::Context<serde_json::Value>, cloacina::TaskError> {
                                    self.inner.execute(context).await
                                }
                                fn id(&self) -> &str { self.inner.id() }
                                fn dependencies(&self) -> &[cloacina::TaskNamespace] { self.inner.dependencies() }
                                fn retry_policy(&self) -> cloacina::retry::RetryPolicy { self.inner.retry_policy() }
                                fn trigger_rules(&self) -> serde_json::Value { self.rewritten_trigger_rules.clone() }
                                fn code_fingerprint(&self) -> Option<String> { self.inner.code_fingerprint() }
                                fn requires_handle(&self) -> bool { self.inner.requires_handle() }
                            }

                            std::sync::Arc::new(TaskWithNamespacedTriggers {
                                inner: task_with_deps,
                                rewritten_trigger_rules,
                            })
                        }
                    );
                }
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
                    let dep_namespaces: Vec<cloacina::TaskNamespace> = dep_ids.iter()
                        .map(|dep_id| cloacina::TaskNamespace::new(
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
                    impl<T: cloacina::Task> cloacina::Task for TaskWithNamespacedTriggers<T> {
                        async fn execute(&self, context: cloacina::Context<serde_json::Value>)
                            -> Result<cloacina::Context<serde_json::Value>, cloacina::TaskError> {
                            self.inner.execute(context).await
                        }
                        fn id(&self) -> &str { self.inner.id() }
                        fn dependencies(&self) -> &[cloacina::TaskNamespace] { self.inner.dependencies() }
                        fn retry_policy(&self) -> cloacina::retry::RetryPolicy { self.inner.retry_policy() }
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
    let auto_register_name = syn::Ident::new(
        &format!("_auto_register_workflow_{}", safe_name),
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

    // Inventory entries for the workflow itself and each contained task.
    // The TaskEntry constructor mirrors the full construction performed by the
    // legacy `register_task_constructor` closure — trigger-rule rewriting,
    // dep-namespace resolution, and the TaskWithNamespacedTriggers wrapper —
    // so `Runtime::new()` can seed tasks directly from inventory without
    // relying on the old global-registry side-effect path (T-0506).
    let rewrite_trigger_rules_for_inventory = generate_trigger_rules_rewrite(tenant, workflow_name);
    let task_inventory_entries: Vec<TokenStream2> = detected_tasks
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
            quote! {
                cloacina::inventory::submit! {
                    cloacina::TaskEntry {
                        namespace: || cloacina::TaskNamespace::new(
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
                            let dep_namespaces: Vec<cloacina::TaskNamespace> = dep_ids
                                .iter()
                                .map(|dep_id| cloacina::TaskNamespace::new(
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
                            impl<T: cloacina::Task> cloacina::Task for TaskWithNamespacedTriggers<T> {
                                async fn execute(
                                    &self,
                                    context: cloacina::Context<serde_json::Value>,
                                ) -> Result<cloacina::Context<serde_json::Value>, cloacina::TaskError>
                                {
                                    self.inner.execute(context).await
                                }
                                fn id(&self) -> &str { self.inner.id() }
                                fn dependencies(&self) -> &[cloacina::TaskNamespace] {
                                    self.inner.dependencies()
                                }
                                fn retry_policy(&self) -> cloacina::retry::RetryPolicy {
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
                                as std::sync::Arc<dyn cloacina::Task>
                        },
                    }
                }
            }
        })
        .collect();

    quote! {
        fn #workflow_constructor_name() -> cloacina::Workflow {
            let pkg_name = env!("CARGO_PKG_NAME");

            // Register all tasks with proper namespaces
            #(#task_registrations)*

            let mut workflow = cloacina::Workflow::new(#workflow_name);
            workflow.set_tenant(#tenant);
            workflow.set_package(pkg_name);
            #description_field
            #author_field

            // Add tasks
            #(#task_addition_code)*

            workflow.validate().expect("Workflow validation failed");
            workflow.finalize()
        }

        #[ctor::ctor]
        fn #auto_register_name() {
            cloacina::register_workflow_constructor(
                #workflow_name.to_string(),
                #workflow_constructor_name
            );
        }

        cloacina::inventory::submit! {
            cloacina::WorkflowEntry {
                name: #workflow_name,
                constructor: #workflow_constructor_name,
            }
        }

        #(#task_inventory_entries)*
    }
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
        }

        cloacina_workflow_plugin::fidius_plugin_registry!();
    }
}
