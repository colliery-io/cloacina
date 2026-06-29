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
use syn::{parse2, Expr, Ident, ItemStruct, LitStr, Token};

/// Parsed form of the `#[reactor(...)]` arguments.
struct ReactorArgs {
    name: LitStr,
    accumulators: Vec<Ident>,
    criteria_mode: CriteriaMode,
    criteria_accumulators: Vec<Ident>,
    criteria_span: Span,
    /// CLOACI-T-0830: optional packaged reactor-constructor reference. When
    /// `from`/`constructor` are present, the reactor's firing decision is
    /// delegated to the named WASM constructor's `evaluate` (resolved + installed
    /// by the CG scheduler via `Reactor::with_evaluator`) instead of the built-in
    /// `criteria`. `config = { name = value }` is bound BY NAME at load, exactly
    /// as the T-0829 `constructor!(...)` consumer form does for task/trigger.
    from: Option<LitStr>,
    constructor: Option<LitStr>,
    config: Vec<(String, Expr)>,
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
        let mut from: Option<LitStr> = None;
        let mut constructor: Option<LitStr> = None;
        let mut config: Vec<(String, Expr)> = Vec::new();

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
                "from" => {
                    if from.is_some() {
                        return Err(syn::Error::new(key.span(), "duplicate 'from' field"));
                    }
                    let lit: LitStr = input.parse()?;
                    if lit.value().is_empty() {
                        return Err(syn::Error::new(
                            lit.span(),
                            "reactor 'from' provider cannot be empty",
                        ));
                    }
                    from = Some(lit);
                }
                "constructor" => {
                    if constructor.is_some() {
                        return Err(syn::Error::new(key.span(), "duplicate 'constructor' field"));
                    }
                    let lit: LitStr = input.parse()?;
                    if lit.value().is_empty() {
                        return Err(syn::Error::new(
                            lit.span(),
                            "reactor 'constructor' name cannot be empty",
                        ));
                    }
                    constructor = Some(lit);
                }
                "config" => {
                    // `config = { key = expr, … }` — bound BY NAME at load.
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
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!(
                            "unknown field '{}' in #[reactor(...)] — expected 'name', \
                             'accumulators', 'criteria', 'from', 'constructor', or 'config'",
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

        // CLOACI-T-0830: `from`/`constructor` are both-or-neither, and `config`
        // only makes sense when a constructor is named.
        if from.is_some() != constructor.is_some() {
            return Err(syn::Error::new(
                Span::call_site(),
                "#[reactor] reactor-constructor reference requires BOTH 'from' and \
                 'constructor' (e.g. from = \"acme/gate@0.1\", constructor = \"gate\")",
            ));
        }
        if constructor.is_none() && !config.is_empty() {
            return Err(syn::Error::new(
                Span::call_site(),
                "#[reactor] 'config' is only valid alongside a 'constructor' reference",
            ));
        }

        Ok(ReactorArgs {
            name,
            accumulators,
            criteria_mode,
            criteria_accumulators,
            criteria_span,
            from,
            constructor,
            config,
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

    // Path prefix: both internal and external builds resolve through the
    // same absolute crate path now that cloacina re-exports it.
    let cg_path = quote! { ::cloacina_computation_graph };

    // I-0102 / T-A: ReactorEntry lives in cloacina-workflow-plugin so
    // packaged cdylibs can collect entries at link time. We emit two
    // cfg-gated submissions below so library-mode users (only depend on
    // `cloacina`) can resolve via `::cloacina::cloacina_workflow_plugin::*`
    // and packaged cdylibs (slim crates with `cloacina-workflow-plugin`
    // direct) can resolve via `::cloacina_workflow_plugin::*`.
    let lib_inventory_path = quote! { ::cloacina::cloacina_workflow_plugin::inventory };
    let lib_reactor_entry_path = quote! { ::cloacina::cloacina_workflow_plugin::ReactorEntry };
    let pkg_inventory_path = quote! { ::cloacina_workflow_plugin::inventory };
    let pkg_reactor_entry_path = quote! { ::cloacina_workflow_plugin::ReactorEntry };

    // Sanity: suppress an unused warning on `criteria_span` — it's kept for
    // future diagnostics use.
    let _ = parsed.criteria_span;
    let _ = parsed.criteria_accumulators;

    // CLOACI-T-0830: build the optional reactor-constructor reference token for
    // the LIB (embedded) inventory submission. The scheduler resolves it against
    // the T-0829 provider search path and installs the WASM `evaluate` as the
    // reactor's firing decider. Only emitted in lib mode — provider resolution is
    // an embedded-host concern (behind `constructors-wasm`), not a packaged-cdylib
    // one, so the packaged submission always carries `None`. `config` values are
    // lowered to `serde_json` literals and bound BY NAME at load (T-0829).
    let lib_constructor_ref = match (&parsed.from, &parsed.constructor) {
        (Some(from_lit), Some(ctor_lit)) => {
            let cfg_pairs = parsed.config.iter().map(|(k, v)| {
                quote! { (#k.to_string(), ::cloacina::serde_json::json!(#v)) }
            });
            quote! {
                ::std::option::Option::Some(#cg_path::ReactorConstructorRef {
                    from: #from_lit.to_string(),
                    constructor: #ctor_lit.to_string(),
                    config: ::std::vec![ #(#cfg_pairs),* ],
                })
            }
        }
        _ => quote! { ::std::option::Option::None },
    };

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

        // I-0102 / T-A: emitted in both modes so the unified
        // `cloacina::package!()` shell (packaged cdylibs) and
        // `Runtime::new()` (embedded) can walk inventory uniformly.
        // Cfg-gated paths so the right crate path resolves per build mode.
        #[cfg(not(feature = "packaged"))]
        #lib_inventory_path::submit! {
            #lib_reactor_entry_path {
                name: #reactor_name_lit,
                constructor: || #cg_path::ReactorRegistration {
                    name: #reactor_name_lit.to_string(),
                    accumulator_names: vec![#(#accumulator_strs.to_string()),*],
                    reaction_mode: #cg_path::ReactionMode::#mode_variant,
                    constructor: #lib_constructor_ref,
                },
            }
        }

        #[cfg(feature = "packaged")]
        #pkg_inventory_path::submit! {
            #pkg_reactor_entry_path {
                name: #reactor_name_lit,
                constructor: || #cg_path::ReactorRegistration {
                    name: #reactor_name_lit.to_string(),
                    accumulator_names: vec![#(#accumulator_strs.to_string()),*],
                    reaction_mode: #cg_path::ReactionMode::#mode_variant,
                    constructor: ::std::option::Option::None,
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

    // -----------------------------------------------------------------------
    // CLOACI-T-0830: reactor-constructor reference (from/constructor/config)
    // -----------------------------------------------------------------------

    #[test]
    fn parse_reactor_constructor_ref() {
        let args = quote! {
            name = "gate",
            accumulators = [x],
            criteria = when_any(x),
            from = "acme/gate@0.1",
            constructor = "gate",
            config = { gate = 5.0 },
        };
        let parsed: ReactorArgs = syn::parse2(args).unwrap();
        assert_eq!(parsed.from.as_ref().unwrap().value(), "acme/gate@0.1");
        assert_eq!(parsed.constructor.as_ref().unwrap().value(), "gate");
        assert_eq!(parsed.config.len(), 1);
        assert_eq!(parsed.config[0].0, "gate");
    }

    #[test]
    fn error_from_without_constructor() {
        let args = quote! {
            name = "gate",
            accumulators = [x],
            criteria = when_any(x),
            from = "acme/gate@0.1",
        };
        let err = syn::parse2::<ReactorArgs>(args).unwrap_err();
        assert!(err.to_string().contains("BOTH 'from' and 'constructor'"));
    }

    #[test]
    fn error_config_without_constructor() {
        let args = quote! {
            name = "gate",
            accumulators = [x],
            criteria = when_any(x),
            config = { gate = 5.0 },
        };
        let err = syn::parse2::<ReactorArgs>(args).unwrap_err();
        assert!(err
            .to_string()
            .contains("'config' is only valid alongside a 'constructor'"));
    }

    #[test]
    fn impl_emits_constructor_ref() {
        let args = quote! {
            name = "gate",
            accumulators = [x],
            criteria = when_any(x),
            from = "acme/gate@0.1",
            constructor = "gate",
            config = { gate = 5.0 },
        };
        let input = quote! { pub struct Gate; };
        let out = reactor_impl(args, input).unwrap().to_string();
        assert!(out.contains("ReactorConstructorRef"));
        assert!(out.contains("\"acme/gate@0.1\""));
        // The packaged submission always carries None for the ref.
        assert!(out.contains("None"));
    }

    #[test]
    fn impl_no_constructor_ref_when_absent() {
        let args = quote! {
            name = "rx",
            accumulators = [alpha],
            criteria = when_any(alpha),
        };
        let input = quote! { pub struct Rx; };
        let out = reactor_impl(args, input).unwrap().to_string();
        assert!(!out.contains("ReactorConstructorRef"));
        // Both submissions emit `constructor: None`.
        assert!(out.contains("constructor"));
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
