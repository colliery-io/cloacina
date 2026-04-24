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

//! `#[reactor]` attribute macro.
//!
//! Applied to a unit struct. Emits:
//! - the struct (preserved),
//! - `impl cloacina_computation_graph::Reactor for <Struct> { ... }` exposing
//!   the declared name, accumulator list, and reaction mode as `const`s,
//! - an `inventory::submit!` of `ReactorEntry` so the runtime registry picks
//!   it up at startup.
//!
//! ```rust,ignore
//! #[reactor(
//!     name = "risk_signals",
//!     accumulators = [alpha, beta],
//!     criteria = when_any(alpha, beta),
//! )]
//! pub struct RiskSignals;
//! ```

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, parse2, Ident, ItemStruct, LitStr, Token};

/// Parsed form of the `#[reactor(...)]` arguments.
struct ReactorArgs {
    name: LitStr,
    accumulators: Vec<Ident>,
    criteria_mode: CriteriaMode,
    criteria_accumulators: Vec<Ident>,
    criteria_span: Span,
}

impl std::fmt::Debug for ReactorArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReactorArgs")
            .field("name", &self.name.value())
            .field(
                "accumulators",
                &self
                    .accumulators
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>(),
            )
            .field("criteria_mode", &self.criteria_mode)
            .field(
                "criteria_accumulators",
                &self
                    .criteria_accumulators
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CriteriaMode {
    WhenAny,
    WhenAll,
}

impl CriteriaMode {
    fn as_rust_variant(&self) -> proc_macro2::TokenStream {
        match self {
            CriteriaMode::WhenAny => quote! { WhenAny },
            CriteriaMode::WhenAll => quote! { WhenAll },
        }
    }
}

impl Parse for ReactorArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name: Option<LitStr> = None;
        let mut accumulators: Option<Vec<Ident>> = None;
        let mut criteria: Option<(CriteriaMode, Vec<Ident>, Span)> = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match key.to_string().as_str() {
                "name" => {
                    if name.is_some() {
                        return Err(syn::Error::new(key.span(), "duplicate 'name' field"));
                    }
                    let lit: LitStr = input.parse()?;
                    if lit.value().is_empty() {
                        return Err(syn::Error::new(
                            lit.span(),
                            "reactor 'name' cannot be empty",
                        ));
                    }
                    if lit.value().starts_with("__Reactor_") {
                        return Err(syn::Error::new(
                            lit.span(),
                            "reactor 'name' cannot start with '__Reactor_' — that prefix is \
                             reserved for the synthesized reactors emitted by the bundled \
                             #[computation_graph] form",
                        ));
                    }
                    name = Some(lit);
                }
                "accumulators" => {
                    if accumulators.is_some() {
                        return Err(syn::Error::new(
                            key.span(),
                            "duplicate 'accumulators' field",
                        ));
                    }
                    let content;
                    syn::bracketed!(content in input);
                    let idents: Punctuated<Ident, Token![,]> =
                        content.parse_terminated(Ident::parse, Token![,])?;
                    let list: Vec<Ident> = idents.into_iter().collect();
                    if list.is_empty() {
                        return Err(syn::Error::new(
                            key.span(),
                            "reactor 'accumulators' list cannot be empty",
                        ));
                    }
                    // Reject duplicates
                    let mut seen = std::collections::HashSet::new();
                    for id in &list {
                        if !seen.insert(id.to_string()) {
                            return Err(syn::Error::new(
                                id.span(),
                                format!(
                                    "accumulator '{}' listed more than once in reactor declaration",
                                    id
                                ),
                            ));
                        }
                    }
                    accumulators = Some(list);
                }
                "criteria" => {
                    if criteria.is_some() {
                        return Err(syn::Error::new(key.span(), "duplicate 'criteria' field"));
                    }
                    let mode_ident: Ident = input.parse()?;
                    let mode_span = mode_ident.span();
                    let mode = match mode_ident.to_string().as_str() {
                        "when_any" => CriteriaMode::WhenAny,
                        "when_all" => CriteriaMode::WhenAll,
                        other => {
                            return Err(syn::Error::new(
                                mode_ident.span(),
                                format!(
                                    "unknown reaction mode '{}', expected 'when_any' or \
                                     'when_all'",
                                    other
                                ),
                            ));
                        }
                    };
                    let paren;
                    syn::parenthesized!(paren in input);
                    let idents: Punctuated<Ident, Token![,]> =
                        paren.parse_terminated(Ident::parse, Token![,])?;
                    let list: Vec<Ident> = idents.into_iter().collect();
                    if list.is_empty() {
                        return Err(syn::Error::new(
                            mode_ident.span(),
                            "criteria accumulator list cannot be empty",
                        ));
                    }
                    criteria = Some((mode, list, mode_span));
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!(
                            "unknown field '{}' in #[reactor(...)] — expected 'name', \
                             'accumulators', or 'criteria'",
                            other
                        ),
                    ));
                }
            }

            let _ = input.parse::<Token![,]>();
        }

        let name = name.ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                "#[reactor] requires a 'name' field (e.g. name = \"risk_signals\")",
            )
        })?;
        let accumulators = accumulators.ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                "#[reactor] requires an 'accumulators' field (e.g. accumulators = [alpha, beta])",
            )
        })?;
        let (criteria_mode, criteria_accumulators, criteria_span) = criteria.ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                "#[reactor] requires a 'criteria' field (e.g. criteria = when_any(alpha, beta))",
            )
        })?;

        // Validate: every criteria accumulator must be in the accumulators list.
        let acc_names: std::collections::HashSet<String> =
            accumulators.iter().map(|i| i.to_string()).collect();
        for ca in &criteria_accumulators {
            if !acc_names.contains(&ca.to_string()) {
                return Err(syn::Error::new(
                    ca.span(),
                    format!(
                        "criteria references accumulator '{}' which is not in the 'accumulators' \
                         list",
                        ca
                    ),
                ));
            }
        }

        Ok(ReactorArgs {
            name,
            accumulators,
            criteria_mode,
            criteria_accumulators,
            criteria_span,
        })
    }
}

pub fn reactor_attr(args: TokenStream, input: TokenStream) -> TokenStream {
    let args2 = proc_macro2::TokenStream::from(args);
    let input2 = proc_macro2::TokenStream::from(input);
    match reactor_impl(args2, input2) {
        Ok(output) => output.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn reactor_impl(
    args: proc_macro2::TokenStream,
    input: proc_macro2::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    let parsed: ReactorArgs = parse2(args)?;

    // Parse the target as a unit struct. Only unit structs are supported —
    // they give us a zero-size type path that the #[computation_graph] macro
    // can reference without forcing the user to construct an instance.
    let struct_item: ItemStruct = parse2(input)?;
    if !matches!(struct_item.fields, syn::Fields::Unit) {
        return Err(syn::Error::new_spanned(
            &struct_item,
            "#[reactor] must be applied to a unit struct (e.g. `pub struct Name;`)",
        ));
    }
    if !struct_item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &struct_item.generics,
            "#[reactor] structs cannot have generic parameters",
        ));
    }

    let struct_ident = &struct_item.ident;
    let struct_vis = &struct_item.vis;
    let struct_attrs = &struct_item.attrs;

    let reactor_name_lit = &parsed.name;
    let reactor_name_str = parsed.name.value();
    let accumulator_strs: Vec<String> = parsed.accumulators.iter().map(|i| i.to_string()).collect();
    let mode_variant = parsed.criteria_mode.as_rust_variant();
    let mode_str = match parsed.criteria_mode {
        CriteriaMode::WhenAny => "when_any",
        CriteriaMode::WhenAll => "when_all",
    };

    // Determine whether we're inside the cloacina crate (path prefix choice).
    let is_cloacina_crate = std::env::var("CARGO_CRATE_NAME")
        .map(|n| n == "cloacina")
        .unwrap_or(false);

    let cg_path = if is_cloacina_crate {
        quote! { ::cloacina_computation_graph }
    } else {
        quote! { ::cloacina_computation_graph }
    };

    let inventory_path = if is_cloacina_crate {
        quote! { crate::inventory }
    } else {
        quote! { ::cloacina::inventory }
    };

    let reactor_entry_path = if is_cloacina_crate {
        quote! { crate::inventory_entries::ReactorEntry }
    } else {
        quote! { ::cloacina::ReactorEntry }
    };

    // Sanity: suppress an unused warning on `criteria_span` — it's kept for
    // future diagnostics use.
    let _ = parsed.criteria_span;
    let _ = parsed.criteria_accumulators;

    let auto_register_ident = format_ident!(
        "_auto_register_reactor_{}",
        reactor_name_str.replace('-', "_")
    );
    // The above ident isn't actually emitted as a symbol; it's a readability
    // hint. Inventory submissions are anonymous.
    let _ = auto_register_ident;

    Ok(quote! {
        #(#struct_attrs)*
        #struct_vis struct #struct_ident;

        impl #cg_path::Reactor for #struct_ident {
            const NAME: &'static str = #reactor_name_lit;
            const ACCUMULATORS: &'static [&'static str] = &[#(#accumulator_strs),*];
            const REACTION_MODE: #cg_path::ReactionMode = #cg_path::ReactionMode::#mode_variant;
        }

        #[cfg(not(test))]
        #[cfg(not(feature = "packaged"))]
        #inventory_path::submit! {
            #reactor_entry_path {
                name: #reactor_name_lit,
                constructor: || #cg_path::ReactorRegistration {
                    name: #reactor_name_lit.to_string(),
                    accumulator_names: vec![#(#accumulator_strs.to_string()),*],
                    reaction_mode: #cg_path::ReactionMode::#mode_variant,
                },
            }
        }

        // Silence `unused` warnings when the struct is referenced only by
        // a graph's `trigger = reactor(T)` clause.
        #[allow(dead_code)]
        const _: &'static str = #mode_str;
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn parse_minimal_reactor_args() {
        let args = quote! {
            name = "rx",
            accumulators = [alpha, beta],
            criteria = when_any(alpha, beta),
        };
        let parsed: ReactorArgs = syn::parse2(args).unwrap();
        assert_eq!(parsed.name.value(), "rx");
        assert_eq!(parsed.accumulators.len(), 2);
        assert_eq!(parsed.criteria_mode, CriteriaMode::WhenAny);
    }

    #[test]
    fn parse_when_all() {
        let args = quote! {
            name = "rx",
            accumulators = [alpha, beta],
            criteria = when_all(alpha, beta),
        };
        let parsed: ReactorArgs = syn::parse2(args).unwrap();
        assert_eq!(parsed.criteria_mode, CriteriaMode::WhenAll);
    }

    #[test]
    fn error_empty_name() {
        let args = quote! {
            name = "",
            accumulators = [alpha],
            criteria = when_any(alpha),
        };
        let err = syn::parse2::<ReactorArgs>(args).unwrap_err();
        assert!(err.to_string().contains("cannot be empty"));
    }

    #[test]
    fn error_reserved_prefix() {
        let args = quote! {
            name = "__Reactor_cheat",
            accumulators = [alpha],
            criteria = when_any(alpha),
        };
        let err = syn::parse2::<ReactorArgs>(args).unwrap_err();
        assert!(err.to_string().contains("__Reactor_"));
    }

    #[test]
    fn error_duplicate_accumulator() {
        let args = quote! {
            name = "rx",
            accumulators = [alpha, alpha],
            criteria = when_any(alpha),
        };
        let err = syn::parse2::<ReactorArgs>(args).unwrap_err();
        assert!(err.to_string().contains("listed more than once"));
    }

    #[test]
    fn error_criteria_accumulator_not_declared() {
        let args = quote! {
            name = "rx",
            accumulators = [alpha],
            criteria = when_any(beta),
        };
        let err = syn::parse2::<ReactorArgs>(args).unwrap_err();
        assert!(err.to_string().contains("not in the 'accumulators' list"));
    }

    #[test]
    fn error_missing_name() {
        let args = quote! {
            accumulators = [alpha],
            criteria = when_any(alpha),
        };
        let err = syn::parse2::<ReactorArgs>(args).unwrap_err();
        assert!(err.to_string().contains("requires a 'name' field"));
    }

    #[test]
    fn error_missing_accumulators() {
        let args = quote! {
            name = "rx",
            criteria = when_any(alpha),
        };
        let err = syn::parse2::<ReactorArgs>(args).unwrap_err();
        assert!(err.to_string().contains("requires an 'accumulators' field"));
    }

    #[test]
    fn error_missing_criteria() {
        let args = quote! {
            name = "rx",
            accumulators = [alpha],
        };
        let err = syn::parse2::<ReactorArgs>(args).unwrap_err();
        assert!(err.to_string().contains("requires a 'criteria' field"));
    }

    #[test]
    fn error_unknown_field() {
        let args = quote! {
            name = "rx",
            accumulators = [alpha],
            criteria = when_any(alpha),
            bogus = 1,
        };
        let err = syn::parse2::<ReactorArgs>(args).unwrap_err();
        assert!(err.to_string().contains("unknown field 'bogus'"));
    }

    #[test]
    fn error_unknown_mode() {
        let args = quote! {
            name = "rx",
            accumulators = [alpha],
            criteria = when_sometimes(alpha),
        };
        let err = syn::parse2::<ReactorArgs>(args).unwrap_err();
        assert!(err.to_string().contains("unknown reaction mode"));
    }

    #[test]
    fn impl_emits_on_unit_struct() {
        let args = quote! {
            name = "rx",
            accumulators = [alpha, beta],
            criteria = when_any(alpha, beta),
        };
        let input = quote! { pub struct Rx; };
        let out = reactor_impl(args, input).unwrap().to_string();
        assert!(out.contains("impl"));
        assert!(out.contains("Reactor"));
        assert!(out.contains("\"rx\""));
        assert!(out.contains("\"alpha\""));
        assert!(out.contains("\"beta\""));
        assert!(out.contains("WhenAny"));
    }

    #[test]
    fn rejects_non_unit_struct() {
        let args = quote! {
            name = "rx",
            accumulators = [alpha],
            criteria = when_any(alpha),
        };
        let input = quote! { pub struct Rx { x: u32 } };
        let err = reactor_impl(args, input).unwrap_err();
        assert!(err.to_string().contains("unit struct"));
    }

    #[test]
    fn rejects_generic_struct() {
        let args = quote! {
            name = "rx",
            accumulators = [alpha],
            criteria = when_any(alpha),
        };
        let input = quote! { pub struct Rx<T>; };
        let err = reactor_impl(args, input).unwrap_err();
        assert!(err.to_string().contains("generic"));
    }
}
