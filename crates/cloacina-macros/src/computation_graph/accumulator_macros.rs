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

//! Proc macros for `#[passthrough_accumulator]` and `#[stream_accumulator]`.
//!
//! These generate structs implementing the `Accumulator` trait.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemFn, LitStr, Token, Type};

/// Parsed args for `#[stream_accumulator(type = "...", topic = "...", ...)]`
struct StreamAccumulatorArgs {
    backend_type: String,
    topic: String,
    group: Option<String>,
    state_type: Option<Type>,
}

impl Parse for StreamAccumulatorArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut backend_type = None;
        let mut topic = None;
        let mut group = None;
        let mut state_type = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match key.to_string().as_str() {
                "type" => {
                    let val: LitStr = input.parse()?;
                    backend_type = Some(val.value());
                }
                "topic" => {
                    let val: LitStr = input.parse()?;
                    topic = Some(val.value());
                }
                "group" => {
                    let val: LitStr = input.parse()?;
                    group = Some(val.value());
                }
                "state" => {
                    let ty: Type = input.parse()?;
                    state_type = Some(ty);
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown stream_accumulator argument '{}'", other),
                    ));
                }
            }

            let _ = input.parse::<Token![,]>();
        }

        Ok(StreamAccumulatorArgs {
            backend_type: backend_type.ok_or_else(|| {
                syn::Error::new(proc_macro2::Span::call_site(), "missing 'type' argument")
            })?,
            topic: topic.ok_or_else(|| {
                syn::Error::new(proc_macro2::Span::call_site(), "missing 'topic' argument")
            })?,
            group,
            state_type,
        })
    }
}

/// Generate code for `#[passthrough_accumulator]`.
///
/// Takes a function `fn name(event: EventType) -> OutputType` and generates
/// a struct implementing `Accumulator` with no event loop (socket-only).
pub fn passthrough_accumulator_impl(
    _args: TokenStream,
    input: TokenStream,
) -> syn::Result<TokenStream> {
    let func: ItemFn = syn::parse2(input)?;
    let fn_name = &func.sig.ident;
    let struct_name = format_ident!("{}Accumulator", pascal_case(&fn_name.to_string()));

    // Extract the function's input and output types from the signature
    let inputs = &func.sig.inputs;
    let output = &func.sig.output;

    // Get the event type from the first parameter
    let event_type = extract_first_param_type(inputs)?;

    // Get the output type from the return type
    let output_type = extract_return_type(output)?;

    Ok(quote! {
        // Keep the original function
        #func

        // Generate the accumulator struct
        pub struct #struct_name;

        #[async_trait::async_trait]
        impl cloacina::computation_graph::Accumulator for #struct_name {
            type Event = #event_type;
            type Output = #output_type;

            fn process(&mut self, event: Self::Event) -> Option<Self::Output> {
                Some(#fn_name(event))
            }

            // No run() override — socket-only (passthrough)
        }
    })
}

/// Generate code for `#[stream_accumulator(type = "...", topic = "...")]`.
///
/// Takes a function and generates a struct implementing `Accumulator` with
/// a stream backend event loop.
pub fn stream_accumulator_impl(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let parsed_args: StreamAccumulatorArgs = syn::parse2(args)?;
    let func: ItemFn = syn::parse2(input)?;
    let fn_name = &func.sig.ident;
    let struct_name = format_ident!("{}Accumulator", pascal_case(&fn_name.to_string()));

    let inputs = &func.sig.inputs;
    let output = &func.sig.output;

    let event_type = extract_first_param_type(inputs)?;
    let output_type = extract_return_type(output)?;

    let backend_type_str = &parsed_args.backend_type;
    let topic_str = &parsed_args.topic;
    let group_str = parsed_args
        .group
        .unwrap_or_else(|| format!("{}_group", fn_name));

    // Check if stateful (has a state parameter)
    let has_state = parsed_args.state_type.is_some();

    if has_state {
        let state_type = parsed_args.state_type.unwrap();
        Ok(quote! {
            #func

            pub struct #struct_name {
                pub state: #state_type,
                pub backend_type: String,
                pub topic: String,
                pub group: String,
            }

            impl #struct_name {
                pub fn new(initial_state: #state_type) -> Self {
                    Self {
                        state: initial_state,
                        backend_type: #backend_type_str.to_string(),
                        topic: #topic_str.to_string(),
                        group: #group_str.to_string(),
                    }
                }
            }

            #[async_trait::async_trait]
            impl cloacina::computation_graph::Accumulator for #struct_name {
                type Event = #event_type;
                type Output = #output_type;

                fn process(&mut self, event: Self::Event) -> Option<Self::Output> {
                    Some(#fn_name(event, &mut self.state))
                }
            }
        })
    } else {
        Ok(quote! {
            #func

            pub struct #struct_name {
                pub backend_type: String,
                pub topic: String,
                pub group: String,
            }

            impl #struct_name {
                pub fn new() -> Self {
                    Self {
                        backend_type: #backend_type_str.to_string(),
                        topic: #topic_str.to_string(),
                        group: #group_str.to_string(),
                    }
                }
            }

            impl Default for #struct_name {
                fn default() -> Self {
                    Self::new()
                }
            }

            #[async_trait::async_trait]
            impl cloacina::computation_graph::Accumulator for #struct_name {
                type Event = #event_type;
                type Output = #output_type;

                fn process(&mut self, event: Self::Event) -> Option<Self::Output> {
                    Some(#fn_name(event))
                }
            }
        })
    }
}

/// Parsed args for `#[polling_accumulator(interval = "...")]`
struct PollingAccumulatorArgs {
    interval_str: String,
}

impl Parse for PollingAccumulatorArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut interval_str = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match key.to_string().as_str() {
                "interval" => {
                    let val: LitStr = input.parse()?;
                    interval_str = Some(val.value());
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown polling_accumulator argument '{}'", other),
                    ));
                }
            }

            let _ = input.parse::<Token![,]>();
        }

        Ok(PollingAccumulatorArgs {
            interval_str: interval_str.ok_or_else(|| {
                syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "missing 'interval' argument (e.g., interval = \"5s\")",
                )
            })?,
        })
    }
}

/// Parse a duration string like "5s", "100ms", "1m" into milliseconds.
fn parse_duration_ms(s: &str) -> syn::Result<u64> {
    let s = s.trim();
    if let Some(val) = s.strip_suffix("ms") {
        val.parse::<u64>()
            .map_err(|_| syn::Error::new(proc_macro2::Span::call_site(), "invalid ms value"))
    } else if let Some(val) = s.strip_suffix('s') {
        val.parse::<u64>()
            .map(|v| v * 1000)
            .map_err(|_| syn::Error::new(proc_macro2::Span::call_site(), "invalid s value"))
    } else if let Some(val) = s.strip_suffix('m') {
        val.parse::<u64>()
            .map(|v| v * 60_000)
            .map_err(|_| syn::Error::new(proc_macro2::Span::call_site(), "invalid m value"))
    } else {
        Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("invalid interval '{}'. Use suffix: ms, s, or m", s),
        ))
    }
}

/// Generate code for `#[polling_accumulator(interval = "5s")]`.
///
/// Takes an async function `async fn name() -> Option<OutputType>` and generates
/// a struct implementing `PollingAccumulator`.
pub fn polling_accumulator_impl(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let parsed_args: PollingAccumulatorArgs = syn::parse2(args)?;
    let func: ItemFn = syn::parse2(input)?;
    let fn_name = &func.sig.ident;
    let struct_name = format_ident!("{}Accumulator", pascal_case(&fn_name.to_string()));

    let output = &func.sig.output;
    let output_type = extract_return_type(output)?;

    // The return type should be Option<T> — extract the inner T
    let inner_type = extract_option_inner(&output_type)?;

    let interval_ms = parse_duration_ms(&parsed_args.interval_str)?;

    Ok(quote! {
        #func

        pub struct #struct_name;

        #[async_trait::async_trait]
        impl cloacina::computation_graph::PollingAccumulator for #struct_name {
            type Output = #inner_type;

            async fn poll(&mut self) -> Option<Self::Output> {
                #fn_name().await
            }

            fn interval(&self) -> std::time::Duration {
                std::time::Duration::from_millis(#interval_ms)
            }
        }
    })
}

/// Extract the inner type T from Option<T>.
fn extract_option_inner(ty: &Type) -> syn::Result<Type> {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Ok(inner.clone());
                    }
                }
            }
        }
    }
    Err(syn::Error::new(
        proc_macro2::Span::call_site(),
        "polling_accumulator function must return Option<T>",
    ))
}

/// Convert snake_case to PascalCase.
fn pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

/// Extract the type of the first function parameter.
fn extract_first_param_type(
    inputs: &syn::punctuated::Punctuated<syn::FnArg, Token![,]>,
) -> syn::Result<Type> {
    let first = inputs.first().ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "accumulator function must have at least one parameter (event type)",
        )
    })?;

    match first {
        syn::FnArg::Typed(pat_type) => Ok((*pat_type.ty).clone()),
        syn::FnArg::Receiver(_) => Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "accumulator function cannot have &self parameter",
        )),
    }
}

/// Extract the return type from a function signature.
fn extract_return_type(output: &syn::ReturnType) -> syn::Result<Type> {
    match output {
        syn::ReturnType::Type(_, ty) => Ok((**ty).clone()),
        syn::ReturnType::Default => Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "accumulator function must have a return type",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pascal_case() {
        assert_eq!(pascal_case("alpha"), "Alpha");
        assert_eq!(pascal_case("my_accumulator"), "MyAccumulator");
        assert_eq!(pascal_case("a_b_c"), "ABC");
    }
}
