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
use syn::{
    parse::{Parse, ParseStream},
    Ident, LitStr, Result as SynResult, Token,
};

use crate::registry::get_registry;

/// Rewrite task names in trigger rules JSON to use full namespaces
///
/// This function takes trigger rules JSON and converts any simple task names
/// to full namespaced task names within the workflow context.
fn rewrite_trigger_rules_with_namespace(
    tenant: &str,
    package: &str,
    workflow_name: &str,
) -> TokenStream2 {
    quote! {
        {
            let trigger_rules = task.trigger_rules();
            let mut rules_json: serde_json::Value = trigger_rules;

            // Helper function to rewrite task names in trigger conditions
            fn rewrite_task_names_in_value(
                value: &mut serde_json::Value,
                tenant: &str,
                package: &str,
                workflow_name: &str,
            ) {
                match value {
                    serde_json::Value::Object(map) => {
                        // Check if this is a task status condition
                        if let (Some(condition_type), Some(task_name)) = (
                            map.get("type").and_then(|v| v.as_str()),
                            map.get("task_name").and_then(|v| v.as_str())
                        ) {
                            if matches!(condition_type, "TaskSuccess" | "TaskFailed" | "TaskSkipped") {
                                // Check if task_name is a simple name (no :: separator)
                                if !task_name.contains("::") {
                                    let full_name = format!("{}::{}::{}::{}", tenant, package, workflow_name, task_name);
                                    map.insert("task_name".to_string(), serde_json::Value::String(full_name));
                                }
                            }
                        }

                        // Recursively process all values in the object
                        for (_, v) in map.iter_mut() {
                            rewrite_task_names_in_value(v, tenant, package, workflow_name);
                        }
                    }
                    serde_json::Value::Array(arr) => {
                        // Recursively process all values in the array
                        for item in arr.iter_mut() {
                            rewrite_task_names_in_value(item, tenant, package, workflow_name);
                        }
                    }
                    _ => {
                        // Primitive values don't need processing
                    }
                }
            }

            rewrite_task_names_in_value(&mut rules_json, #tenant, #package, #workflow_name);
            rules_json
        }
    }
}

/// Workflow macro attributes
///
/// # Fields
///
/// * `name` - Unique identifier for the workflow (required)
/// * `tenant` - Tenant identifier for the workflow (optional, defaults to "public")
/// * `package` - Package name for namespace isolation (optional, defaults to "embedded")
/// * `description` - Optional description of the workflow
/// * `author` - Optional author information
/// * `tasks` - List of task identifiers to include in the workflow (at least one required)
pub struct WorkflowAttributes {
    pub name: String,
    pub tenant: String,
    pub package: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub tasks: Vec<Ident>,
}

impl Parse for WorkflowAttributes {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut name = None;
        let mut tenant = None;
        let mut package = None;
        let mut description = None;
        let mut author = None;
        let mut tasks = Vec::new();

        while !input.is_empty() {
            let field_name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;

            match field_name.to_string().as_str() {
                "name" => {
                    let lit: LitStr = input.parse()?;
                    name = Some(lit.value());
                }
                "tenant" => {
                    let lit: LitStr = input.parse()?;
                    tenant = Some(lit.value());
                }
                "package" => {
                    let lit: LitStr = input.parse()?;
                    package = Some(lit.value());
                }
                "description" => {
                    let lit: LitStr = input.parse()?;
                    description = Some(lit.value());
                }
                "author" => {
                    let lit: LitStr = input.parse()?;
                    author = Some(lit.value());
                }
                "tasks" => {
                    let content;
                    syn::bracketed!(content in input);

                    while !content.is_empty() {
                        let task_name: Ident = content.parse()?;
                        tasks.push(task_name);

                        if !content.is_empty() {
                            content.parse::<Token![,]>()?;
                        }
                    }
                }
                _ => {
                    return Err(syn::Error::new(
                        field_name.span(),
                        format!("Unknown field: {}", field_name),
                    ));
                }
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        let name = name.ok_or_else(|| {
            syn::Error::new(Span::call_site(), "workflow macro requires 'name' field")
        })?;

        if tasks.is_empty() {
            return Err(syn::Error::new(
                Span::call_site(),
                "workflow macro requires at least one task in 'tasks' field",
            ));
        }

        Ok(WorkflowAttributes {
            name,
            tenant: tenant.unwrap_or_else(|| "public".to_string()),
            package: package.unwrap_or_else(|| "embedded".to_string()),
            description,
            author,
            tasks,
        })
    }
}

/// Generate Workflow with auto-versioning and compile-time validation
///
/// Creates a workflow implementation with automatic version calculation
/// and compile-time validation of task dependencies.
///
/// # Arguments
///
/// * `attrs` - The workflow attributes
///
/// # Returns
///
/// A `TokenStream2` containing the generated workflow implementation
pub fn generate_workflow_impl(attrs: WorkflowAttributes) -> TokenStream2 {
    let workflow_name = &attrs.name;
    let workflow_tenant = &attrs.tenant;
    let workflow_package = &attrs.package;
    let description = attrs.description;
    let author = attrs.author;
    let tasks = &attrs.tasks;

    // Convert task identifiers to strings for validation
    let task_strings: Vec<String> = tasks.iter().map(|t| t.to_string()).collect();

    // PHASE 1: Validate all tasks exist and detect cycles
    let validation_result = {
        // Try to acquire the lock with a timeout approach to avoid hanging
        match get_registry().try_lock() {
            Ok(registry) => {
                // Check that all referenced tasks exist - collect all errors first
                let mut validation_errors = Vec::new();
                for task_name in &task_strings {
                    if let Err(e) = registry.validate_dependencies(task_name) {
                        validation_errors.push(e);
                    }
                }

                // Return first validation error if any
                if let Some(first_error) = validation_errors.into_iter().next() {
                    Err(first_error)
                } else {
                    // Run cycle detection on the entire graph
                    registry.detect_cycles()
                }
            }
            Err(_) => {
                // If we can't acquire the lock, skip validation to avoid hanging
                // This can happen during parallel compilation
                Ok(())
            }
        }
    };

    if let Err(e) = validation_result {
        #[allow(clippy::useless_conversion)]
        return e.to_compile_error().into();
    }

    // Generate task constructor calls and struct names
    let task_info: Vec<_> = tasks
        .iter()
        .map(|task| {
            let constructor_name = syn::Ident::new(&format!("{}_task", task), task.span());

            // Convert snake_case to PascalCase for struct name
            let task_str = task.to_string();
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
            let struct_name = syn::Ident::new(&format!("{}Task", pascal_case), task.span());

            (constructor_name, struct_name)
        })
        .collect();

    let description_field = if let Some(desc) = description {
        quote! { workflow.set_description(#desc); }
    } else {
        quote! {}
    };

    let author_field = if let Some(auth) = author {
        quote! { workflow.add_tag("author", #auth); }
    } else {
        quote! {}
    };

    // Generate a unique variable name for the workflow constructor
    let workflow_constructor_name = syn::Ident::new(
        &format!(
            "_workflow_{}_constructor",
            workflow_name.replace("-", "_").replace(" ", "_")
        ),
        Span::call_site(),
    );

    // Generate task registrations with proper namespace and dependencies
    let task_registrations: Vec<_> = task_info.iter().map(|(constructor, struct_name)| {
        let task_id = constructor.to_string().replace("_task", "");
        let rewrite_trigger_rules = rewrite_trigger_rules_with_namespace(workflow_tenant, workflow_package, workflow_name);

        quote! {
            {
                let namespace = cloacina::TaskNamespace::new(
                    #workflow_tenant,
                    #workflow_package,
                    #workflow_name,
                    #task_id
                );

                cloacina::register_task_constructor(
                    namespace,
                    || {
                        let task = #constructor();

                        // Get trigger rules before moving the task
                        let rewritten_trigger_rules = #rewrite_trigger_rules;

                        // Get the task's static dependencies
                        let dep_ids = #struct_name::dependency_task_ids();

                        // Convert dependency IDs to full namespaces within this workflow
                        let dep_namespaces: Vec<cloacina::TaskNamespace> = dep_ids.iter()
                            .map(|dep_id| cloacina::TaskNamespace::new(
                                #workflow_tenant,
                                #workflow_package,
                                #workflow_name,
                                dep_id
                            ))
                            .collect();

                        // Create task with resolved dependencies
                        let task_with_deps = task.with_dependencies(dep_namespaces);

                        // Create a wrapper that rewrites trigger rules with full namespaces
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

                            fn id(&self) -> &str {
                                self.inner.id()
                            }

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
                        }

                        let wrapper = TaskWithNamespacedTriggers {
                            inner: task_with_deps,
                            rewritten_trigger_rules,
                        };

                        std::sync::Arc::new(wrapper)
                    }
                );
            }
        }
    }).collect();

    // Generate task addition code
    let task_additions: Vec<_> = task_info
        .iter()
        .map(|(constructor, struct_name)| {
            let rewrite_trigger_rules = rewrite_trigger_rules_with_namespace(
                workflow_tenant,
                workflow_package,
                workflow_name,
            );

            quote! {
                {
                    let task = #constructor();

                    // Get trigger rules before moving the task
                    let rewritten_trigger_rules = #rewrite_trigger_rules;

                    // Get the task's static dependencies
                    let dep_ids = #struct_name::dependency_task_ids();

                    // Convert dependency IDs to full namespaces within this workflow
                    let dep_namespaces: Vec<cloacina::TaskNamespace> = dep_ids.iter()
                        .map(|dep_id| cloacina::TaskNamespace::new(
                            #workflow_tenant,
                            #workflow_package,
                            #workflow_name,
                            dep_id
                        ))
                        .collect();

                    // Create task with resolved dependencies
                    let task_with_deps = task.with_dependencies(dep_namespaces);

                    // Create a wrapper that rewrites trigger rules with full namespaces
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

                        fn id(&self) -> &str {
                            self.inner.id()
                        }

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
                    }

                    let wrapper = TaskWithNamespacedTriggers {
                        inner: task_with_deps,
                        rewritten_trigger_rules,
                    };

                    workflow.add_task(std::sync::Arc::new(wrapper))
                        .expect("Failed to add task to workflow");
                }
            }
        })
        .collect();

    quote! {
        {
            // Register all tasks with proper namespaces
            #(#task_registrations)*

            // Define workflow constructor function
            fn #workflow_constructor_name() -> cloacina::Workflow {
                let mut workflow = cloacina::Workflow::new(#workflow_name);
                workflow.set_tenant(#workflow_tenant);
                workflow.set_package(#workflow_package);
                #description_field
                #author_field

                // Add tasks with resolved dependencies
                #(#task_additions)*

                workflow.validate().expect("Workflow validation failed");
                // Auto-calculate version when finalizing
                workflow.finalize()
            }

            // Auto-register the workflow in the global registry
            const _: () = {
                #[ctor::ctor]
                fn auto_register_workflow() {
                    cloacina::register_workflow_constructor(
                        #workflow_name.to_string(),
                        #workflow_constructor_name
                    );
                }
            };

            // Return the workflow instance
            #workflow_constructor_name()
        }
    }
}

/// The workflow! macro for declarative workflow definition
///
/// # Usage
///
/// ```rust
/// let workflow = workflow! {
///     name: "my_workflow",
///     description: "A sample workflow",
///     tasks: [task1, task2]
/// };
/// ```
///
/// # Attributes
///
/// See `WorkflowAttributes` for available configuration options.
pub fn workflow(input: TokenStream) -> TokenStream {
    let input = TokenStream2::from(input);

    let attrs = match syn::parse2::<WorkflowAttributes>(input) {
        Ok(attrs) => attrs,
        Err(e) => {
            return syn::Error::new(
                Span::call_site(),
                format!("Invalid workflow attributes: {}", e),
            )
            .to_compile_error()
            .into();
        }
    };

    generate_workflow_impl(attrs).into()
}
