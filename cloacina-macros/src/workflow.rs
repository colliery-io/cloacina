use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    Ident, LitStr, Result as SynResult, Token,
};

use crate::registry::get_registry;

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

    // Generate task constructor calls
    let task_constructors: Vec<_> = tasks
        .iter()
        .map(|task| {
            let constructor_name = syn::Ident::new(&format!("{}_task", task), task.span());
            quote! { #constructor_name() }
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

    // Generate task registrations with proper namespace
    let task_registrations: Vec<_> = tasks
        .iter()
        .map(|task| {
            let task_id = task.to_string();
            let constructor_name = syn::Ident::new(&format!("{}_task", task), task.span());
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
                        || std::sync::Arc::new(#constructor_name())
                    );
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

                #(
                    workflow.add_task(std::sync::Arc::new(#task_constructors)).expect("Failed to add task to workflow");
                )*

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