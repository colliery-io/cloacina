use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use syn::{
    parse::{Parse, ParseStream},
    Expr, FnArg, Ident, ItemFn, LitStr, Pat, Result as SynResult, Token,
};

use crate::registry::{get_registry, TaskInfo};

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
pub struct TaskAttributes {
    pub id: String,
    pub dependencies: Vec<String>,
    pub retry_attempts: Option<i32>,
    pub retry_backoff: Option<String>,
    pub retry_delay_ms: Option<i32>,
    pub retry_max_delay_ms: Option<i32>,
    pub retry_condition: Option<String>,
    pub retry_jitter: Option<bool>,
    pub trigger_rules: Option<Expr>,
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
pub fn calculate_function_fingerprint(func: &ItemFn) -> String {
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

/// Generate retry policy creation code based on task attributes
///
/// # Arguments
///
/// * `attrs` - The task attributes containing retry policy configuration
///
/// # Returns
///
/// A `TokenStream2` containing the generated code for retry policy creation
pub fn generate_retry_policy_code(attrs: &TaskAttributes) -> TokenStream2 {
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

/// Generate trigger rules JSON code based on task attributes
///
/// # Arguments
///
/// * `attrs` - The task attributes containing trigger rules configuration
///
/// # Returns
///
/// A `TokenStream2` containing the generated code for trigger rules
pub fn generate_trigger_rules_code(attrs: &TaskAttributes) -> TokenStream2 {
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
pub fn parse_trigger_rules_expr(expr: &Expr) -> Result<serde_json::Value, String> {
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

/// Convert snake_case to PascalCase
///
/// # Arguments
///
/// * `s` - The string to convert
///
/// # Returns
///
/// The converted string in PascalCase
pub fn to_pascal_case(s: &str) -> String {
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

/// Generate the task implementation
///
/// Creates the task struct, implementation, and registration code based on
/// the provided attributes and function.
///
/// # Arguments
///
/// * `attrs` - The task attributes
/// * `input` - The input function to be wrapped as a task
///
/// # Returns
///
/// A `TokenStream2` containing the generated task implementation
pub fn generate_task_impl(attrs: TaskAttributes, input: ItemFn) -> TokenStream2 {
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
    }
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
    generate_task_impl(attrs, input_fn).into()
} 