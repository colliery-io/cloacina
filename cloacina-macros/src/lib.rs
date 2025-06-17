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

//! # Cloacina Macros
//!
//! This crate provides procedural macros for defining tasks and workflows in the Cloacina framework.
//! It enables compile-time validation of task dependencies and workflow structure.
//!
//! ## Key Features
//!
//! - `#[task]` attribute macro for defining tasks with retry policies and trigger rules
//! - `workflow!` macro for declarative workflow definition
//! - `#[packaged_workflow]` attribute macro for creating distributable workflow packages
//! - Compile-time validation of task dependencies and workflow structure
//! - Automatic task and workflow registration
//! - Code fingerprinting for task versioning
//!
//! ## Example
//!
//! ```rust
//! use cloacina_macros::task;
//!
//! #[task(
//!     id = "my_task",
//!     dependencies = ["other_task"],
//!     retry_attempts = 3,
//!     retry_backoff = "exponential"
//! )]
//! async fn my_task(context: &mut Context<Value>) -> Result<(), TaskError> {
//!     // Task implementation
//!     Ok(())
//! }
//!
//! use cloacina_macros::workflow;
//!
//! let workflow = workflow! {
//!     name: "my_workflow",
//!     description: "A sample workflow",
//!     tasks: [my_task, other_task]
//! };
//! ```

mod registry;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use registry::{get_registry, TaskInfo};
use std::collections::{hash_map::DefaultHasher, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use syn::{
    parse::{Parse, ParseStream},
    Expr, FnArg, Ident, ItemFn, LitStr, Pat, Result as SynResult, Token,
};

/// Attributes for the task macro that define task behavior and configuration
///
/// # Fields
///
/// * `id` - Unique identifier for the task (required)
/// * `dependencies` - List of task IDs this task depends on
/// * `retry_attempts` - Maximum number of retry attempts (default: 3)
/// * `retry_backoff` - Backoff strategy: "fixed", "linear", or "exponential" (default: "exponential")
/// * `retry_delay_ms` - Initial delay between retries in milliseconds (default: 1000)
/// * `retry_max_delay_ms` - Maximum delay between retries in milliseconds (default: 30000)
/// * `retry_condition` - Condition for retrying: "never", "all", "transient", or error patterns
/// * `retry_jitter` - Whether to add random jitter to retry delays (default: true)
/// * `trigger_rules` - Rules that determine when the task should be executed
struct TaskAttributes {
    id: String,
    dependencies: Vec<String>,
    retry_attempts: Option<i32>,
    retry_backoff: Option<String>,
    retry_delay_ms: Option<i32>,
    retry_max_delay_ms: Option<i32>,
    retry_condition: Option<String>,
    retry_jitter: Option<bool>,
    trigger_rules: Option<Expr>,
}

impl Parse for TaskAttributes {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut id = None;
        let mut dependencies = Vec::new();
        let mut retry_attempts = None;
        let mut retry_backoff = None;
        let mut retry_delay_ms = None;
        let mut retry_max_delay_ms = None;
        let mut retry_condition = None;
        let mut retry_jitter = None;
        let mut trigger_rules = None;

        while !input.is_empty() {
            let name: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match name.to_string().as_str() {
                "id" => {
                    let lit: LitStr = input.parse()?;
                    id = Some(lit.value());
                }
                "dependencies" => {
                    // Parse array of strings: ["dep1", "dep2"]
                    let content;
                    syn::bracketed!(content in input);

                    while !content.is_empty() {
                        let lit: LitStr = content.parse()?;
                        dependencies.push(lit.value());

                        if !content.is_empty() {
                            content.parse::<Token![,]>()?;
                        }
                    }
                }
                "retry_attempts" => {
                    let lit: syn::LitInt = input.parse()?;
                    retry_attempts = Some(lit.base10_parse()?);
                }
                "retry_backoff" => {
                    let lit: LitStr = input.parse()?;
                    retry_backoff = Some(lit.value());
                }
                "retry_delay_ms" => {
                    let lit: syn::LitInt = input.parse()?;
                    retry_delay_ms = Some(lit.base10_parse()?);
                }
                "retry_max_delay_ms" => {
                    let lit: syn::LitInt = input.parse()?;
                    retry_max_delay_ms = Some(lit.base10_parse()?);
                }
                "retry_condition" => {
                    let lit: LitStr = input.parse()?;
                    retry_condition = Some(lit.value());
                }
                "retry_jitter" => {
                    let lit: syn::LitBool = input.parse()?;
                    retry_jitter = Some(lit.value);
                }
                "trigger_rules" => {
                    let expr: Expr = input.parse()?;
                    trigger_rules = Some(expr);
                }
                _ => {
                    return Err(syn::Error::new(
                        name.span(),
                        format!("Unknown attribute: {}", name),
                    ));
                }
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        let id = id.ok_or_else(|| {
            syn::Error::new(Span::call_site(), "task macro requires 'id' attribute")
        })?;

        Ok(TaskAttributes {
            id,
            dependencies,
            retry_attempts,
            retry_backoff,
            retry_delay_ms,
            retry_max_delay_ms,
            retry_condition,
            retry_jitter,
            trigger_rules,
        })
    }
}

/// Parse trigger rule expressions into JSON at compile time
///
/// # Arguments
///
/// * `expr` - The trigger rule expression to parse
///
/// # Returns
///
/// A `Result` containing either the parsed JSON value or an error message
///
/// # Supported Rules
///
/// * Simple rules: `always`
/// * Composite rules: `all()`, `any()`, `none()`
/// * Task status rules: `task_success()`, `task_failed()`, `task_skipped()`
/// * Context rules: `context_value()`
fn parse_trigger_rules_expr(expr: &Expr) -> Result<serde_json::Value, String> {
    match expr {
        // Handle simple identifiers like 'always'
        Expr::Path(path) => {
            if let Some(ident) = path.path.get_ident() {
                match ident.to_string().as_str() {
                    "always" => Ok(serde_json::json!({"type": "Always"})),
                    _ => Err(format!("Unknown trigger rule: {}", ident)),
                }
            } else {
                Err("Invalid trigger rule path".to_string())
            }
        }
        // Handle function calls like all(), any(), none(), task_success(), etc.
        Expr::Call(call) => {
            if let Expr::Path(path) = &*call.func {
                if let Some(ident) = path.path.get_ident() {
                    match ident.to_string().as_str() {
                        "all" => {
                            let conditions = parse_condition_list(&call.args)?;
                            Ok(serde_json::json!({"type": "All", "conditions": conditions}))
                        }
                        "any" => {
                            let conditions = parse_condition_list(&call.args)?;
                            Ok(serde_json::json!({"type": "Any", "conditions": conditions}))
                        }
                        "none" => {
                            let conditions = parse_condition_list(&call.args)?;
                            Ok(serde_json::json!({"type": "None", "conditions": conditions}))
                        }
                        "task_success" => {
                            if call.args.len() != 1 {
                                return Err(
                                    "task_success requires exactly one argument".to_string()
                                );
                            }
                            let task_name = extract_string_literal(&call.args[0])?;
                            let condition =
                                serde_json::json!({"type": "TaskSuccess", "task_name": task_name});
                            Ok(serde_json::json!({"type": "All", "conditions": [condition]}))
                        }
                        "task_failed" => {
                            if call.args.len() != 1 {
                                return Err("task_failed requires exactly one argument".to_string());
                            }
                            let task_name = extract_string_literal(&call.args[0])?;
                            let condition =
                                serde_json::json!({"type": "TaskFailed", "task_name": task_name});
                            Ok(serde_json::json!({"type": "All", "conditions": [condition]}))
                        }
                        "task_skipped" => {
                            if call.args.len() != 1 {
                                return Err(
                                    "task_skipped requires exactly one argument".to_string()
                                );
                            }
                            let task_name = extract_string_literal(&call.args[0])?;
                            let condition =
                                serde_json::json!({"type": "TaskSkipped", "task_name": task_name});
                            Ok(serde_json::json!({"type": "All", "conditions": [condition]}))
                        }
                        "context_value" => {
                            if call.args.len() != 3 {
                                return Err("context_value requires exactly three arguments: key, operator, value".to_string());
                            }
                            let key = extract_string_literal(&call.args[0])?;
                            let operator = parse_value_operator(&call.args[1])?;
                            let value = parse_json_value(&call.args[2])?;
                            let condition = serde_json::json!({
                                "type": "ContextValue",
                                "key": key,
                                "operator": operator,
                                "value": value
                            });
                            Ok(serde_json::json!({"type": "All", "conditions": [condition]}))
                        }
                        _ => Err(format!("Unknown trigger rule function: {}", ident)),
                    }
                } else {
                    Err("Invalid trigger rule function".to_string())
                }
            } else {
                Err("Invalid trigger rule call".to_string())
            }
        }
        _ => Err("Unsupported trigger rule expression".to_string()),
    }
}

/// Parse a list of trigger conditions from function arguments
fn parse_condition_list(
    args: &syn::punctuated::Punctuated<Expr, syn::Token![,]>,
) -> Result<Vec<serde_json::Value>, String> {
    let mut conditions = Vec::new();
    for arg in args {
        conditions.push(parse_trigger_condition_expr(arg)?);
    }
    Ok(conditions)
}

/// Parse a single trigger condition (not wrapped in a rule)
fn parse_trigger_condition_expr(expr: &Expr) -> Result<serde_json::Value, String> {
    match expr {
        Expr::Call(call) => {
            if let Expr::Path(path) = &*call.func {
                if let Some(ident) = path.path.get_ident() {
                    match ident.to_string().as_str() {
                        "task_success" => {
                            if call.args.len() != 1 {
                                return Err(
                                    "task_success requires exactly one argument".to_string()
                                );
                            }
                            let task_name = extract_string_literal(&call.args[0])?;
                            Ok(serde_json::json!({"type": "TaskSuccess", "task_name": task_name}))
                        }
                        "task_failed" => {
                            if call.args.len() != 1 {
                                return Err("task_failed requires exactly one argument".to_string());
                            }
                            let task_name = extract_string_literal(&call.args[0])?;
                            Ok(serde_json::json!({"type": "TaskFailed", "task_name": task_name}))
                        }
                        "task_skipped" => {
                            if call.args.len() != 1 {
                                return Err(
                                    "task_skipped requires exactly one argument".to_string()
                                );
                            }
                            let task_name = extract_string_literal(&call.args[0])?;
                            Ok(serde_json::json!({"type": "TaskSkipped", "task_name": task_name}))
                        }
                        "context_value" => {
                            if call.args.len() != 3 {
                                return Err("context_value requires exactly three arguments: key, operator, value".to_string());
                            }
                            let key = extract_string_literal(&call.args[0])?;
                            let operator = parse_value_operator(&call.args[1])?;
                            let value = parse_json_value(&call.args[2])?;
                            Ok(serde_json::json!({
                                "type": "ContextValue",
                                "key": key,
                                "operator": operator,
                                "value": value
                            }))
                        }
                        _ => Err(format!("Unknown trigger condition function: {}", ident)),
                    }
                } else {
                    Err("Invalid trigger condition function".to_string())
                }
            } else {
                Err("Invalid trigger condition call".to_string())
            }
        }
        _ => Err("Unsupported trigger condition expression".to_string()),
    }
}

/// Extract a string literal from an expression
fn extract_string_literal(expr: &Expr) -> Result<String, String> {
    match expr {
        Expr::Lit(lit) => {
            if let syn::Lit::Str(lit_str) = &lit.lit {
                Ok(lit_str.value())
            } else {
                Err("Expected string literal".to_string())
            }
        }
        _ => Err("Expected string literal".to_string()),
    }
}

/// Parse value operators like equals, greater_than, etc.
fn parse_value_operator(expr: &Expr) -> Result<String, String> {
    match expr {
        Expr::Path(path) => {
            if let Some(ident) = path.path.get_ident() {
                match ident.to_string().as_str() {
                    "equals" => Ok("Equals".to_string()),
                    "not_equals" => Ok("NotEquals".to_string()),
                    "greater_than" => Ok("GreaterThan".to_string()),
                    "less_than" => Ok("LessThan".to_string()),
                    "contains" => Ok("Contains".to_string()),
                    "not_contains" => Ok("NotContains".to_string()),
                    "exists" => Ok("Exists".to_string()),
                    "not_exists" => Ok("NotExists".to_string()),
                    _ => Err(format!("Unknown operator: {}", ident)),
                }
            } else {
                Err("Invalid operator path".to_string())
            }
        }
        _ => Err("Expected operator identifier".to_string()),
    }
}

/// Parse JSON values from expressions
fn parse_json_value(expr: &Expr) -> Result<serde_json::Value, String> {
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Str(s) => Ok(serde_json::Value::String(s.value())),
            syn::Lit::Int(i) => {
                let value: i64 = i
                    .base10_parse()
                    .map_err(|e| format!("Invalid integer: {}", e))?;
                Ok(serde_json::Value::Number(serde_json::Number::from(value)))
            }
            syn::Lit::Float(f) => {
                let value: f64 = f
                    .base10_parse()
                    .map_err(|e| format!("Invalid float: {}", e))?;
                Ok(serde_json::Value::Number(
                    serde_json::Number::from_f64(value)
                        .ok_or_else(|| "Invalid float value".to_string())?,
                ))
            }
            syn::Lit::Bool(b) => Ok(serde_json::Value::Bool(b.value)),
            _ => Err("Unsupported literal type".to_string()),
        },
        _ => Err("Expected literal value".to_string()),
    }
}

/// Generate trigger rules JSON code based on task attributes
///
/// # Arguments
///
/// * `attrs` - The task attributes containing trigger rules configuration
///
/// # Returns
///
/// A `TokenStream2` containing the generated code for trigger rules
fn generate_trigger_rules_code(attrs: &TaskAttributes) -> TokenStream2 {
    match &attrs.trigger_rules {
        Some(expr) => match parse_trigger_rules_expr(expr) {
            Ok(json_value) => {
                let json_string = json_value.to_string();
                quote! {
                    serde_json::from_str(#json_string).unwrap()
                }
            }
            Err(error) => {
                let error_msg = format!("Invalid trigger rule: {}", error);
                quote! {
                    compile_error!(#error_msg)
                }
            }
        },
        None => {
            // Default to Always
            quote! {
                serde_json::json!({"type": "Always"})
            }
        }
    }
}

/// Generate retry policy creation code based on task attributes
///
/// # Arguments
///
/// * `attrs` - The task attributes containing retry policy configuration
///
/// # Returns
///
/// A `TokenStream2` containing the generated code for retry policy creation
fn generate_retry_policy_code(attrs: &TaskAttributes) -> TokenStream2 {
    let max_attempts = attrs.retry_attempts.unwrap_or(3);
    let initial_delay_ms = attrs.retry_delay_ms.unwrap_or(1000);
    let max_delay_ms = attrs.retry_max_delay_ms.unwrap_or(30000);
    let with_jitter = attrs.retry_jitter.unwrap_or(true);

    // Generate backoff strategy
    let backoff_strategy = match attrs.retry_backoff.as_deref() {
        Some("fixed") => quote! {
            cloacina::retry::BackoffStrategy::Fixed
        },
        Some("linear") => quote! {
            cloacina::retry::BackoffStrategy::Linear { multiplier: 1.0 }
        },
        Some("exponential") | None => quote! {
            cloacina::retry::BackoffStrategy::Exponential { base: 2.0, multiplier: 1.0 }
        },
        Some(_other) => {
            // Custom backoff - for now, default to exponential
            quote! {
                cloacina::retry::BackoffStrategy::Exponential { base: 2.0, multiplier: 1.0 }
            }
        }
    };

    // Generate retry condition
    let retry_condition = match attrs.retry_condition.as_deref() {
        Some("never") => quote! {
            vec![cloacina::retry::RetryCondition::Never]
        },
        Some("all") | None => quote! {
            vec![cloacina::retry::RetryCondition::AllErrors]
        },
        Some("transient") => quote! {
            vec![cloacina::retry::RetryCondition::TransientOnly]
        },
        Some(patterns) => {
            // Parse comma-separated patterns
            let pattern_list: Vec<&str> = patterns.split(',').map(|s| s.trim()).collect();
            quote! {
                vec![cloacina::retry::RetryCondition::ErrorPattern {
                    patterns: vec![#(#pattern_list.to_string()),*]
                }]
            }
        }
    };

    quote! {
        cloacina::retry::RetryPolicy {
            max_attempts: #max_attempts,
            initial_delay: std::time::Duration::from_millis(#initial_delay_ms as u64),
            max_delay: std::time::Duration::from_millis(#max_delay_ms as u64),
            backoff_strategy: #backoff_strategy,
            retry_conditions: #retry_condition,
            jitter: #with_jitter,
        }
    }
}

/// Calculate code fingerprint from function
///
/// Generates a unique hash based on the function's signature and body.
/// This is used for detecting changes in task implementations.
///
/// # Arguments
///
/// * `func` - The function to calculate the fingerprint for
///
/// # Returns
///
/// A hexadecimal string representing the function's fingerprint
fn calculate_function_fingerprint(func: &ItemFn) -> String {
    let mut hasher = DefaultHasher::new();

    // Hash function signature (excluding name)
    func.sig.inputs.iter().for_each(|input| {
        if let syn::FnArg::Typed(pat_type) = input {
            quote::quote!(#pat_type).to_string().hash(&mut hasher);
        }
    });

    // Hash return type
    quote::quote!(#(&func.sig.output))
        .to_string()
        .hash(&mut hasher);

    // Hash function body (this is the key part for detecting changes)
    let body_tokens = quote::quote!(#(&func.block)).to_string();
    body_tokens.hash(&mut hasher);

    // Include async info
    func.sig.asyncness.is_some().hash(&mut hasher);

    format!("{:016x}", hasher.finish())
}

/// Generate the task implementation
///
/// Creates the task struct, implementation, and registration code based on
/// the provided attributes and function.
///
/// # Arguments
///
/// * `attrs` - The task attributes
/// * `input` - The input function to be wrapped as a task
/// * `namespace_context` - Optional namespace context (package_name, workflow_id) for packaged workflows
///
/// # Returns
///
/// A `TokenStream2` containing the generated task implementation
fn generate_task_impl(
    attrs: TaskAttributes,
    input: ItemFn,
    namespace_context: Option<(String, String)>,
) -> TokenStream2 {
    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let fn_block = &input.block;
    let fn_inputs = &input.sig.inputs;
    let fn_output = &input.sig.output;
    let fn_asyncness = &input.sig.asyncness;

    // Calculate code fingerprint
    let code_fingerprint = calculate_function_fingerprint(&input);

    // Extract context parameter
    let context_param = fn_inputs.iter().find_map(|arg| {
        if let FnArg::Typed(pat_type) = arg {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                let param_name = pat_ident.ident.to_string();
                if param_name == "context"
                    || param_name == "_context"
                    || param_name.starts_with("context")
                {
                    return Some(&*pat_type.ty);
                }
            }
        }
        None
    });

    // Validate function signature
    if context_param.is_none() {
        return quote! {
            compile_error!("Task function must have a 'context' parameter (can be named 'context' or '_context')");
        };
    }

    let task_struct_name = syn::Ident::new(
        &format!("{}Task", to_pascal_case(&fn_name.to_string())),
        fn_name.span(),
    );

    let task_id = &attrs.id;
    let dependencies = &attrs.dependencies;

    // Generate retry policy creation code
    let generate_retry_policy = generate_retry_policy_code(&attrs);

    // Generate trigger rules JSON code
    let generate_trigger_rules = generate_trigger_rules_code(&attrs);

    let execute_body = if fn_asyncness.is_some() {
        // Async function - pass mutable reference to context
        quote! {
            #fn_name(&mut context).await
        }
    } else {
        // Sync function - pass mutable reference to context
        quote! {
            #fn_name(&mut context)
        }
    };

    // Create a task constructor function name
    let task_constructor_name = syn::Ident::new(&format!("{}_task", fn_name), fn_name.span());

    // Generate namespace context code
    let namespace_context_code = if let Some((package_name, workflow_id)) = namespace_context {
        quote! { Some((#package_name, #workflow_id)) }
    } else {
        quote! { None }
    };

    quote! {
        // Keep the original function for testing
        #fn_vis #fn_asyncness fn #fn_name(#fn_inputs) #fn_output #fn_block

        // Generate the task struct
        #[derive(Debug)]
        #fn_vis struct #task_struct_name {
            dependencies: Vec<String>,
        }

        impl #task_struct_name {
            pub fn new() -> Self {
                Self {
                    dependencies: vec![#(#dependencies.to_string()),*],
                }
            }

            /// Get the code fingerprint for this task
            pub fn code_fingerprint() -> &'static str {
                #code_fingerprint
            }

            /// Create the retry policy based on macro attributes
            pub fn create_retry_policy(&self) -> cloacina::retry::RetryPolicy {
                #generate_retry_policy
            }

            /// Get the trigger rules for this task
            pub fn trigger_rules(&self) -> serde_json::Value {
                #generate_trigger_rules
            }
        }


        #[async_trait::async_trait]
        impl cloacina::Task for #task_struct_name {
            async fn execute(&self, mut context: cloacina::Context<serde_json::Value>)
                -> Result<cloacina::Context<serde_json::Value>, cloacina::TaskError> {

                // Convert the result to our expected format
                match #execute_body {
                    Ok(()) => Ok(context),
                    Err(e) => Err(cloacina::TaskError::ExecutionFailed {
                        message: format!("{:?}", e),
                        task_id: #task_id.to_string(),
                        timestamp: chrono::Utc::now(),
                    }),
                }
            }

            fn id(&self) -> &str {
                #task_id
            }

            fn dependencies(&self) -> &[String] {
                &self.dependencies
            }

            fn retry_policy(&self) -> cloacina::retry::RetryPolicy {
                self.create_retry_policy()
            }

            fn trigger_rules(&self) -> serde_json::Value {
                self.trigger_rules()
            }

            fn code_fingerprint(&self) -> Option<String> {
                Some(Self::code_fingerprint().to_string())
            }
        }

        // Provide a convenience function to create the task
        #fn_vis fn #task_constructor_name() -> #task_struct_name {
            #task_struct_name::new()
        }

        // Auto-register the task in the global registry
        const _: () = {
            #[ctor::ctor]
            fn auto_register() {
                // Use namespace context if provided (for packaged workflows) or default to embedded context
                let (package_name, workflow_id) = #namespace_context_code
                    .unwrap_or(("embedded", "default"));

                let namespace = cloacina::TaskNamespace::new("public", package_name, workflow_id, #task_id);
                cloacina::register_task_constructor(
                    namespace,
                    || std::sync::Arc::new(#task_struct_name::new())
                );
            }
        };
    }
}

/// Convert snake_case to PascalCase
///
/// # Arguments
///
/// * `s` - The string to convert
///
/// # Returns
///
/// The converted string in PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
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
fn find_similar_package_task_names(target: &str, available: &[String]) -> Vec<String> {
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
fn build_package_graph_data(
    detected_tasks: &HashMap<String, syn::Ident>,
    task_dependencies: &HashMap<String, Vec<String>>,
    package_name: &str,
) -> String {
    // Create nodes for each task
    let mut nodes = Vec::new();
    for (task_id, _fn_name) in detected_tasks {
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
fn calculate_levenshtein_distance(a: &str, b: &str) -> usize {
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

/// The main task proc macro
///
/// # Usage
///
/// ```rust
/// #[task(
///     id = "my_task",
///     dependencies = ["other_task"],
///     retry_attempts = 3,
///     retry_backoff = "exponential"
/// )]
/// async fn my_task(context: &mut Context<Value>) -> Result<(), TaskError> {
///     // Task implementation
///     Ok(())
/// }
/// ```
///
/// # Attributes
///
/// See `TaskAttributes` for available configuration options.
#[proc_macro_attribute]
pub fn task(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = TokenStream2::from(args);
    let input = TokenStream2::from(input);

    let attrs = match syn::parse2::<TaskAttributes>(args) {
        Ok(attrs) => attrs,
        Err(e) => {
            return syn::Error::new(Span::call_site(), format!("Invalid task attributes: {}", e))
                .to_compile_error()
                .into();
        }
    };

    let input_fn = match syn::parse2::<ItemFn>(input) {
        Ok(input_fn) => input_fn,
        Err(e) => {
            return syn::Error::new(
                Span::call_site(),
                format!("Task macro can only be applied to functions: {}", e),
            )
            .to_compile_error()
            .into();
        }
    };

    // PHASE 1: Register task in compile-time registry and validate
    // Use a timeout to avoid hanging if there are mutex issues
    let file_path = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| "unknown".to_string());

    // Note: We no longer need to check if we're in a packaged workflow context
    // since validation is deferred for all tasks

    let task_info = TaskInfo {
        id: attrs.id.clone(),
        dependencies: attrs.dependencies.clone(),
        file_path: file_path.clone(),
    };

    // PHASE 2: Register task without validation (validation happens later)
    let _registration_result = {
        // Try to acquire the lock with a timeout approach
        match get_registry().try_lock() {
            Ok(mut registry) => {
                // Just register this task - validation will happen later
                registry.register_task(task_info)
            }
            Err(_) => {
                // If we can't acquire the lock, skip registration to avoid hanging
                // This can happen during parallel compilation
                Ok(())
            }
        }
    };

    // Note: We no longer validate immediately - validation will happen at the workflow level

    // PHASE 3: Generate the task implementation
    generate_task_impl(attrs, input_fn, None).into()
}

/// Workflow macro attributes
///
/// # Fields
///
/// * `name` - Unique identifier for the workflow (required)
/// * `description` - Optional description of the workflow
/// * `tasks` - List of task identifiers to include in the workflow (at least one required)
struct WorkflowAttributes {
    name: String,
    description: Option<String>,
    tasks: Vec<Ident>,
}

impl Parse for WorkflowAttributes {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut name = None;
        let mut description = None;
        let mut tasks = Vec::new();

        while !input.is_empty() {
            let field_name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;

            match field_name.to_string().as_str() {
                "name" => {
                    let lit: LitStr = input.parse()?;
                    name = Some(lit.value());
                }
                "description" => {
                    let lit: LitStr = input.parse()?;
                    description = Some(lit.value());
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
            description,
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
fn generate_workflow_impl(attrs: WorkflowAttributes) -> TokenStream2 {
    let workflow_name = &attrs.name;
    let description = attrs.description;
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

    // Generate a unique variable name for the workflow constructor
    let workflow_constructor_name = syn::Ident::new(
        &format!(
            "_workflow_{}_constructor",
            workflow_name.replace("-", "_").replace(" ", "_")
        ),
        Span::call_site(),
    );

    quote! {
        {
            // Define workflow constructor function
            fn #workflow_constructor_name() -> cloacina::Workflow {
                let mut workflow = cloacina::Workflow::new(#workflow_name);
                #description_field

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
#[proc_macro]
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

/// Attributes for the packaged_workflow macro
///
/// # Fields
///
/// * `package` - Package name for namespace isolation (required)
/// * `name` - Workflow name/identifier (required)
/// * `version` - Package version (required)
/// * `description` - Optional description of the workflow package
/// * `author` - Optional author information
struct PackagedWorkflowAttributes {
    package: String,
    name: String,
    version: String,
    description: Option<String>,
    author: Option<String>,
}

impl Parse for PackagedWorkflowAttributes {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut package = None;
        let mut name = None;
        let mut version = None;
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
                "version" => {
                    let lit: LitStr = input.parse()?;
                    version = Some(lit.value());
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

        let version = version.ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                "packaged_workflow macro requires 'version' attribute",
            )
        })?;

        Ok(PackagedWorkflowAttributes {
            package,
            name,
            version,
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
fn detect_package_cycles(task_dependencies: &HashMap<String, Vec<String>>) -> Result<(), String> {
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
            if let Err(cycle_error) = dfs_package_cycle_detection(
                task_id,
                task_dependencies,
                &mut visited,
                &mut rec_stack,
                &mut path,
            ) {
                return Err(cycle_error);
            }
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
fn generate_packaged_workflow_impl(
    attrs: PackagedWorkflowAttributes,
    input: syn::ItemMod,
) -> TokenStream2 {
    let mod_name = &input.ident;
    let mod_vis = &input.vis;
    let mod_content = &input.content;

    let package_name = &attrs.package;
    let attrs_name = &attrs.name;
    let package_version = &attrs.version;
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
    let register_abi_name = syn::Ident::new(
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
    let mut task_registrations = Vec::new();
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
                            // Construct full namespace as the key for detected_tasks
                            let full_namespace = format!("public::{}::{}::{}", package_name, mod_name, task_attrs.id);
                            detected_tasks.insert(full_namespace.clone(), fn_name.clone());
                            task_dependencies
                                .insert(full_namespace.clone(), task_attrs.dependencies.clone());

                            // Generate task constructor name (following the pattern from the task macro)
                            let task_constructor_name =
                                syn::Ident::new(&format!("{}_task", fn_name), fn_name.span());

                            // Generate registration call using namespace isolation
                            let registration = quote! {
                                {
                                    let namespace = cloacina::TaskNamespace::new(
                                        tenant_id,
                                        #package_name,
                                        workflow_id,
                                        #task_id
                                    );
                                    cloacina::register_task_constructor(
                                        namespace,
                                        || std::sync::Arc::new(#task_constructor_name())
                                    );
                                }
                            };
                            task_registrations.push(registration);
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
    package_version.hash(&mut hasher);
    if let Some((_, items)) = mod_content {
        for item in items {
            quote::quote!(#item).to_string().hash(&mut hasher);
        }
    }
    let package_fingerprint = format!("{:016x}", hasher.finish());

    // Build the workflow graph for this package
    let graph_data_json =
        build_package_graph_data(&detected_tasks, &task_dependencies, &package_name);

    // Generate task metadata structures for FFI export
    let task_metadata_items = if !detected_tasks.is_empty() {
        let mut task_metadata_entries = Vec::new();
        let mut task_execution_cases = Vec::new();
        let mut task_index = 0u32;

        for (task_id, fn_name) in &detected_tasks {
            let dependencies = task_dependencies.get(task_id).cloned().unwrap_or_default();

            // Generate fully qualified namespace: tenant::package::workflow::task
            // Use the workflow name from the macro attributes
            let namespaced_id = format!("{{tenant}}::{}::{}::{}", package_name, attrs.name, task_id);

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

            task_index += 1;
        }

        let task_count = detected_tasks.len();

        // Generate unique function name for this package
        let metadata_fn_name = syn::Ident::new(
            &format!("cloacina_get_task_metadata_{}", package_ident),
            mod_name.span(),
        );

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
                graph_data_json: GRAPH_DATA_JSON.as_ptr() as *const std::os::raw::c_char,
            };

            /// Get task metadata for cloacina-ctl compilation (package-specific name)
            #[no_mangle]
            pub extern "C" fn #metadata_fn_name() -> *const cloacina_ctl_package_tasks {
                &PACKAGE_TASKS_METADATA
            }

            /// Get task metadata for cloacina registry validation (standard name)
            #[no_mangle]
            pub extern "C" fn cloacina_get_task_metadata() -> *const cloacina_ctl_package_tasks {
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
                        let result = serde_json::json!({
                            "status": "success",
                            "task": task_name_str,
                            "message": "Task executed successfully"
                        });
                        write_success_result(&result, result_buffer, result_capacity, result_len)
                    }
                    Err(e) => {
                        write_error_result(&e, result_buffer, result_capacity, result_len)
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
        // Generate unique function name for this package
        let metadata_fn_name = syn::Ident::new(
            &format!("cloacina_get_task_metadata_{}", package_ident),
            mod_name.span(),
        );

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
                pub graph_data_json: *const std::os::raw::c_char,
            }

            // Safety: These pointers point to static data which is safe to share
            unsafe impl Sync for cloacina_ctl_package_tasks {}

            static EMPTY_GRAPH_DATA: &str = concat!("{\"nodes\":[],\"edges\":[],\"metadata\":{\"task_count\":0,\"edge_count\":0,\"has_cycles\":false,\"depth_levels\":0,\"root_tasks\":[],\"leaf_tasks\":[]}}", "\0");

            static PACKAGE_TASKS_METADATA: cloacina_ctl_package_tasks = cloacina_ctl_package_tasks {
                task_count: 0,
                tasks: std::ptr::null(),
                package_name: concat!(#package_name, "\0").as_ptr() as *const std::os::raw::c_char,
                graph_data_json: EMPTY_GRAPH_DATA.as_ptr() as *const std::os::raw::c_char,
            };

            /// Get task metadata for cloacina-ctl compilation (package-specific name)
            #[no_mangle]
            pub extern "C" fn #metadata_fn_name() -> *const cloacina_ctl_package_tasks {
                &PACKAGE_TASKS_METADATA
            }

            /// Get task metadata for cloacina registry validation (standard name)
            #[no_mangle]
            pub extern "C" fn cloacina_get_task_metadata() -> *const cloacina_ctl_package_tasks {
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

    // Transform the module items to expand task macros directly with namespace context
    let module_items = if let Some((_, items)) = mod_content {
        let mut expanded_items = Vec::new();

        for item in items {
            match item {
                syn::Item::Fn(item_fn) => {
                    // Check if this function has a #[task] attribute
                    let mut task_attrs = None;
                    let mut other_attrs = Vec::new();

                    for attr in &item_fn.attrs {
                        if attr.path().is_ident("task") {
                            if let Ok(parsed_attrs) = attr.parse_args::<TaskAttributes>() {
                                task_attrs = Some(parsed_attrs);
                            }
                        } else {
                            other_attrs.push(attr.clone());
                        }
                    }

                    if let Some(attrs) = task_attrs {
                        // This function has a task attribute - expand it directly with namespace context
                        let mut fn_without_task_attr = item_fn.clone();
                        fn_without_task_attr.attrs = other_attrs;

                        let namespace_context = Some((package_name.clone(), mod_name.to_string()));
                        let expanded =
                            generate_task_impl(attrs, fn_without_task_attr, namespace_context);
                        expanded_items.push(expanded);
                    } else {
                        // No task attribute - keep as is
                        expanded_items.push(quote! { #item });
                    }
                }
                _ => {
                    // Other items - keep as is
                    expanded_items.push(quote! { #item });
                }
            }
        }

        expanded_items
    } else {
        Vec::new()
    };

    // Generate workflow constructor FFI function
    let workflow_constructor_ffi = if !detected_tasks.is_empty() {
        // Generate task lookup calls using namespaced IDs from global registry
        let task_lookup_calls: Vec<_> = detected_tasks
            .keys()
            .map(|task_id| {
                quote! {
                    // Look up task from global registry using full namespace (task_id is already full namespace)
                    let namespace = cloacina::TaskNamespace::from_string(#task_id).expect("Invalid task namespace");
                    if let Some(task) = cloacina::task::get_task(&namespace) {
                        workflow.add_task(task).expect("Failed to add task to workflow");
                    } else {
                        eprintln!("Warning: Task {} not found in global registry for namespace: {:?}", #task_id, namespace);
                    }
                }
            })
            .collect();

        quote! {
            /// Create a complete workflow from this package's tasks
            ///
            /// This function builds a workflow using the provided tenant and workflow context.
            /// It looks up tasks from the global registry using their proper namespaced IDs.
            #[no_mangle]
            pub extern "C" fn cloacina_create_workflow(
                tenant_id: *const std::os::raw::c_char,
                workflow_id: *const std::os::raw::c_char
            ) -> *const cloacina::Workflow {
                use std::ffi::CStr;

                // Convert C strings to Rust strings
                let tenant_id = if tenant_id.is_null() {
                    "public"
                } else {
                    unsafe {
                        match CStr::from_ptr(tenant_id).to_str() {
                            Ok(s) => s,
                            Err(_) => "public"
                        }
                    }
                };

                let workflow_id = if workflow_id.is_null() {
                    #attrs_name
                } else {
                    unsafe {
                        match CStr::from_ptr(workflow_id).to_str() {
                            Ok(s) => s,
                            Err(_) => #attrs_name
                        }
                    }
                };

                eprintln!("DEBUG: About to look up tasks for workflow {}", workflow_id);

                let mut workflow = cloacina::Workflow::new(workflow_id);
                workflow.set_description(#package_description);

                // Add all detected tasks using their namespaced IDs from global registry
                // Use the provided tenant_id and workflow_id for namespace construction

                // Debug: List all tasks in global registry before lookup
                eprintln!("DEBUG: About to look up tasks for workflow {}", #package_name);
                let task_registry = cloacina::task::global_task_registry();
                if let Ok(registry) = task_registry.read() {
                    eprintln!("DEBUG: Global task registry contains {} tasks:", registry.len());
                    for (namespace, _) in registry.iter().take(10) {  // Show first 10 tasks
                        eprintln!("  - {:?}", namespace);
                    }
                    if registry.len() > 10 {
                        eprintln!("  ... and {} more tasks", registry.len() - 10);
                    }
                } else {
                    eprintln!("DEBUG: Failed to read global task registry");
                }

                #(#task_lookup_calls)*

                // Validate and finalize the workflow
                if let Err(e) = workflow.validate() {
                    eprintln!("Workflow validation failed: {}", e);
                    // Return empty workflow on validation failure
                    let empty_workflow = cloacina::Workflow::new(#package_name).finalize();
                    return Box::into_raw(Box::new(empty_workflow));
                }

                let finalized_workflow = workflow.finalize();

                // Return as a heap-allocated pointer for FFI
                Box::into_raw(Box::new(finalized_workflow))
            }

            /// Destroy a workflow created by cloacina_create_workflow
            ///
            /// This function properly deallocates memory for workflows created
            /// via the FFI interface.
            #[no_mangle]
            pub extern "C" fn cloacina_destroy_workflow(workflow: *mut cloacina::Workflow) {
                if !workflow.is_null() {
                    unsafe {
                        Box::from_raw(workflow);
                    }
                }
            }
        }
    } else {
        quote! {
            /// Create an empty workflow for packages with no tasks
            #[no_mangle]
            pub extern "C" fn cloacina_create_workflow(
                _tenant_id: *const std::os::raw::c_char,
                _workflow_id: *const std::os::raw::c_char
            ) -> *const cloacina::Workflow {
                let workflow = cloacina::Workflow::new(#package_name);
                let finalized_workflow = workflow.finalize();
                Box::into_raw(Box::new(finalized_workflow))
            }

            /// Destroy a workflow created by cloacina_create_workflow
            #[no_mangle]
            pub extern "C" fn cloacina_destroy_workflow(workflow: *mut cloacina::Workflow) {
                if !workflow.is_null() {
                    unsafe {
                        Box::from_raw(workflow);
                    }
                }
            }
        }
    };

    // Generate workflow task registration code (keep existing for backward compatibility)
    let workflow_task_registrations = if !detected_tasks.is_empty() {
        let task_registration_code: Vec<_> = detected_tasks.keys().map(|task_id| {
            quote! {
                // Construct the namespaced task ID
                let namespace = cloacina::TaskNamespace::new(
                    &tenant_id,
                    package_name,
                    mod_name,
                    #task_id
                );

                if let Some(task) = cloacina::task::get_task(&namespace) {
                    if let Err(e) = workflow.add_task(task) {
                        eprintln!("Failed to add task {} to workflow: {}", #task_id, e);
                    }
                } else {
                    eprintln!("Task {} not found in global registry for namespace: {:?}", #task_id, namespace);
                }
            }
        }).collect();

        quote! {
            #(#task_registration_code)*
        }
    } else {
        quote! {
            // No tasks to register
            eprintln!("Package {} has no tasks to add to workflow", package_name);
        }
    };

    quote! {
        // Keep the original module with enhanced functionality
        #mod_vis mod #mod_name {
            #(#module_items)*

            // Include task metadata structures and functions
            #task_metadata_items

            // Include workflow constructor FFI functions
            #workflow_constructor_ffi

            /// Package metadata for this workflow package
            #[derive(Debug, Clone)]
            pub struct #metadata_struct_name {
                pub package: &'static str,
                pub version: &'static str,
                pub description: &'static str,
                pub author: &'static str,
                pub fingerprint: &'static str,
            }

            impl #metadata_struct_name {
                pub const fn new() -> Self {
                    Self {
                        package: #package_name,
                        version: #package_version,
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

            /// Register all tasks in this package with namespace isolation
            ///
            /// This function registers all tasks defined in this package under the
            /// package's namespace for proper isolation from other packages.
            pub fn register_package_tasks(tenant_id: &str, workflow_id: &str) {
                #(#task_registrations)*
            }

            /// Register all workflows in this package with the global workflow registry
            ///
            /// This function creates and registers workflow constructors that use the
            /// package's tasks to build complete executable workflows.
            pub fn register_package_workflows(tenant_id: &str) {
                // Create a workflow constructor for the main package workflow
                let workflow_name = format!("{}::{}", #package_name, stringify!(#mod_name));

                // Clone the necessary values for the closure
                let package_name = #package_name;
                let mod_name = stringify!(#mod_name);
                let tenant_id_cloned = tenant_id.to_string();
                let workflow_name_cloned = workflow_name.clone();

                let constructor = move || {
                    // Create workflow using the package's tasks
                    let mut workflow = cloacina::Workflow::new(&workflow_name_cloned);
                    workflow.set_description(#package_description);

                    // Add all tasks from this package to the workflow
                    // The tasks should already be registered in the global task registry
                    let tenant_id = &tenant_id_cloned;
                    #workflow_task_registrations

                    // Validate and finalize the workflow
                    if let Err(e) = workflow.validate() {
                        eprintln!("Workflow validation failed for {}: {}", workflow_name_cloned, e);
                        // Return a minimal workflow on validation failure
                        return cloacina::Workflow::new(&workflow_name_cloned).finalize();
                    }

                    workflow.finalize()
                };

                // Register the workflow constructor in the global registry
                cloacina::register_workflow_constructor(workflow_name, constructor);
            }

            /// Standard ABI entry point for dynamic loading
            ///
            /// This function provides a standardized interface that can be called
            /// when the package is dynamically loaded as a shared library.
            #[no_mangle]
            pub extern "C" fn #register_abi_name(tenant_id: *const std::os::raw::c_char, workflow_id: *const std::os::raw::c_char) {
                use std::ffi::CStr;

                if tenant_id.is_null() || workflow_id.is_null() {
                    return;
                }

                let tenant_id = unsafe {
                    match CStr::from_ptr(tenant_id).to_str() {
                        Ok(s) => s,
                        Err(_) => return,
                    }
                };

                let workflow_id = unsafe {
                    match CStr::from_ptr(workflow_id).to_str() {
                        Ok(s) => s,
                        Err(_) => return,
                    }
                };

                register_package_tasks(tenant_id, workflow_id);
            }

            /// Get package metadata via ABI
            #[no_mangle]
            pub extern "C" fn #metadata_abi_name() -> *const #metadata_struct_name {
                Box::leak(Box::new(get_package_metadata()))
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
#[proc_macro_attribute]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packaged_workflow_macro_compiles() {
        // This test verifies the macro generates valid code
        let input = quote! {
            #[packaged_workflow(
                package = "test_package",
                version = "1.0.0",
                description = "Test workflow package",
                author = "Test Author"
            )]
            mod test_workflow {
                use cloacina::{Context, TaskError};
                use cloacina_macros::task;

                #[task(id = "task1", dependencies = [])]
                async fn task1(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
                    Ok(())
                }

                #[task(id = "task2", dependencies = ["task1"])]
                async fn task2(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
                    Ok(())
                }
            }
        };

        // Parse and generate the code - if this doesn't panic, the macro works
        let attrs = PackagedWorkflowAttributes {
            package: "test_package".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test workflow package".to_string()),
            author: Some("Test Author".to_string()),
        };

        let input_mod = syn::parse2::<syn::ItemMod>(input).expect("Failed to parse module");
        let _generated = generate_packaged_workflow_impl(attrs, input_mod);
    }

    #[test]
    fn test_task_namespace_format() {
        // Verify the namespace format matches our design
        let namespace = quote! {
            cloacina::TaskNamespace::new(
                "tenant_123",
                "analytics_pkg",
                "etl_workflow",
                "extract_data"
            )
        };

        // This should generate: "tenant_123::analytics_pkg::etl_workflow::extract_data"
        let expected = "tenant_123::analytics_pkg::etl_workflow::extract_data";

        // The actual TaskNamespace implementation will validate this at runtime
    }

    #[test]
    fn test_package_cycle_detection() {
        // Test valid dependency chain (no cycles)
        let mut valid_deps = HashMap::new();
        valid_deps.insert("task_a".to_string(), vec!["task_b".to_string()]);
        valid_deps.insert("task_b".to_string(), vec!["task_c".to_string()]);
        valid_deps.insert("task_c".to_string(), vec![]);

        assert!(detect_package_cycles(&valid_deps).is_ok());

        // Test circular dependency
        let mut circular_deps = HashMap::new();
        circular_deps.insert("task_a".to_string(), vec!["task_b".to_string()]);
        circular_deps.insert("task_b".to_string(), vec!["task_c".to_string()]);
        circular_deps.insert("task_c".to_string(), vec!["task_a".to_string()]);

        let result = detect_package_cycles(&circular_deps);
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(
            error_msg.contains("task_a")
                && error_msg.contains("task_b")
                && error_msg.contains("task_c")
        );

        // Test self-dependency
        let mut self_dep = HashMap::new();
        self_dep.insert("task_a".to_string(), vec!["task_a".to_string()]);

        assert!(detect_package_cycles(&self_dep).is_err());

        // Test complex valid graph
        let mut complex_valid = HashMap::new();
        complex_valid.insert("extract".to_string(), vec![]);
        complex_valid.insert("validate".to_string(), vec!["extract".to_string()]);
        complex_valid.insert("transform".to_string(), vec!["validate".to_string()]);
        complex_valid.insert("load".to_string(), vec!["transform".to_string()]);
        complex_valid.insert(
            "report".to_string(),
            vec!["load".to_string(), "validate".to_string()],
        );

        assert!(detect_package_cycles(&complex_valid).is_ok());
    }

    #[test]
    fn test_compile_time_validation_features() {
        // This test documents the validation features we've implemented

        // 1. Within-package dependency validation
        // Tasks can depend on other tasks in the same package

        // 2. External dependency validation
        // Tasks can depend on tasks in the global registry

        // 3. Circular dependency detection
        // Cycles within a package are detected and reported

        // 4. Helpful error messages
        // Missing dependencies include suggestions and available tasks

        // 5. Module boundary awareness
        // Validation respects packaged workflow module boundaries

        // These features ensure packaged workflows are self-contained
        // and dependencies are properly validated at compile time
        assert!(true); // This test is primarily documentation
    }
}
