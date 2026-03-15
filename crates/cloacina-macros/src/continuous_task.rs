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

//! `#[continuous_task]` proc macro for continuous scheduling.
//!
//! Generates a task struct with metadata about triggering and referenced
//! data sources for graph assembly. The task function has the same signature
//! as `#[task]` — `DataSourceMap` is injected via context by the scheduler.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    Ident, ItemFn, LitStr, Result as SynResult, Token,
};

use crate::tasks::calculate_function_fingerprint;

/// Attributes for the continuous_task macro.
pub struct ContinuousTaskAttributes {
    pub id: String,
    pub sources: Vec<String>,
    pub referenced: Vec<String>,
}

impl Parse for ContinuousTaskAttributes {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut id = None;
        let mut sources = Vec::new();
        let mut referenced = Vec::new();

        while !input.is_empty() {
            let name: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match name.to_string().as_str() {
                "id" => {
                    let lit: LitStr = input.parse()?;
                    id = Some(lit.value());
                }
                "sources" => {
                    let content;
                    syn::bracketed!(content in input);
                    while !content.is_empty() {
                        let lit: LitStr = content.parse()?;
                        sources.push(lit.value());
                        if !content.is_empty() {
                            content.parse::<Token![,]>()?;
                        }
                    }
                }
                "referenced" => {
                    let content;
                    syn::bracketed!(content in input);
                    while !content.is_empty() {
                        let lit: LitStr = content.parse()?;
                        referenced.push(lit.value());
                        if !content.is_empty() {
                            content.parse::<Token![,]>()?;
                        }
                    }
                }
                other => {
                    return Err(syn::Error::new(
                        name.span(),
                        format!(
                            "unknown attribute: '{}'. Expected: id, sources, referenced",
                            other
                        ),
                    ));
                }
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        let id = id.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "missing required attribute: id",
            )
        })?;

        if sources.is_empty() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "continuous_task requires at least one source in 'sources'",
            ));
        }

        // Check for overlap between sources and referenced
        for s in &sources {
            if referenced.contains(s) {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!("'{}' appears in both sources and referenced", s),
                ));
            }
        }

        Ok(Self {
            id,
            sources,
            referenced,
        })
    }
}

/// The continuous_task proc macro implementation.
pub fn continuous_task(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = TokenStream2::from(args);
    let attrs: ContinuousTaskAttributes = match syn::parse2(args) {
        Ok(attrs) => attrs,
        Err(e) => return e.to_compile_error().into(),
    };

    let fn_item: ItemFn = match syn::parse(input) {
        Ok(item) => item,
        Err(e) => return e.to_compile_error().into(),
    };

    let expanded = generate_continuous_task(&attrs, &fn_item);
    expanded.into()
}

fn generate_continuous_task(attrs: &ContinuousTaskAttributes, fn_item: &ItemFn) -> TokenStream2 {
    let fn_name = &fn_item.sig.ident;
    let fn_vis = &fn_item.vis;
    let fn_block = &fn_item.block;
    let fn_asyncness = &fn_item.sig.asyncness;
    let fn_inputs = &fn_item.sig.inputs;
    let fn_output = &fn_item.sig.output;

    let task_id = &attrs.id;
    let sources = &attrs.sources;
    let referenced = &attrs.referenced;

    // Code fingerprint
    let code_fingerprint = calculate_function_fingerprint(fn_item);

    // Generate struct name: fn_name in PascalCase + "Task"
    let struct_name_str = format!(
        "{}Task",
        fn_name
            .to_string()
            .split('_')
            .map(|s| {
                let mut c = s.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<String>()
    );
    let task_struct_name = Ident::new(&struct_name_str, fn_name.span());

    // Constructor function name
    let constructor_name = Ident::new(&format!("{}_task", fn_name), fn_name.span());

    quote! {
        // Keep the original function for direct testing
        #fn_vis #fn_asyncness fn #fn_name(#fn_inputs) #fn_output #fn_block

        /// Generated continuous task struct.
        ///
        /// Sources: #(#sources),*
        #[derive(Debug)]
        #fn_vis struct #task_struct_name {
            dependencies: Vec<::cloacina_workflow::TaskNamespace>,
        }

        impl #task_struct_name {
            pub fn new() -> Self {
                Self {
                    dependencies: Vec::new(),
                }
            }

            /// Data sources that trigger this task's execution.
            pub fn sources() -> &'static [&'static str] {
                &[#(#sources),*]
            }

            /// Data sources available but not triggering execution.
            pub fn referenced_sources() -> &'static [&'static str] {
                &[#(#referenced),*]
            }

            /// Whether this is a continuous task (always true).
            pub fn is_continuous() -> bool {
                true
            }

            /// Get the code fingerprint for this task.
            pub fn code_fingerprint_value() -> &'static str {
                #code_fingerprint
            }
        }

        #[async_trait::async_trait]
        impl ::cloacina_workflow::Task for #task_struct_name {
            async fn execute(
                &self,
                mut context: ::cloacina_workflow::Context<serde_json::Value>,
            ) -> Result<
                ::cloacina_workflow::Context<serde_json::Value>,
                ::cloacina_workflow::TaskError,
            > {
                match #fn_name(&mut context).await {
                    Ok(()) => Ok(context),
                    Err(e) => Err(::cloacina_workflow::TaskError::ExecutionFailed {
                        message: format!("{:?}", e),
                        task_id: #task_id.to_string(),
                        timestamp: chrono::Utc::now(),
                    }),
                }
            }

            fn id(&self) -> &str {
                #task_id
            }

            fn dependencies(&self) -> &[::cloacina_workflow::TaskNamespace] {
                &self.dependencies
            }

            fn code_fingerprint(&self) -> Option<String> {
                Some(Self::code_fingerprint_value().to_string())
            }
        }

        /// Convenience constructor.
        #fn_vis fn #constructor_name() -> #task_struct_name {
            #task_struct_name::new()
        }
    }
}
