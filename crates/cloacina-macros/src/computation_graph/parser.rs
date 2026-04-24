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

//! Topology parser for `#[computation_graph]`.
//!
//! Parses the macro attribute syntax:
//! ```text
//! #[computation_graph(
//!     react = when_any(alpha, beta, gamma),
//!     graph = {
//!         decision_engine(alpha, beta, gamma) => {
//!             Signal -> risk_check,
//!             NoAction -> audit_logger,
//!         },
//!         risk_check(gamma) => {
//!             Approved -> output_handler,
//!             Blocked -> alert_handler,
//!         },
//!     }
//! )]
//! ```

use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, Ident, Token, TypePath};

/// The full parsed topology from the macro attribute.
///
/// Supports three shapes:
/// - `react = when_any(...)` + `graph = { ... }` — bundled form; desugars to a
///   synthesized reactor named `__Reactor_<graphname>`.
/// - `trigger = reactor(TypePath)` + `graph = { ... }` — split form; the graph
///   binds at runtime to the already-declared reactor whose struct is
///   `TypePath`, and the graph macro emits a compile-time subset check
///   between entry accumulators and `<TypePath as Reactor>::ACCUMULATORS`.
/// - `graph = { ... }` only — trigger-less form; the graph is registered by
///   name only and is invoked directly (T-02 workflow tasks, T-03 Python).
pub struct ParsedTopology {
    pub trigger: TriggerSpec,
    pub edges: Vec<ParsedEdge>,
}

impl std::fmt::Debug for ParsedTopology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParsedTopology")
            .field("trigger", &self.trigger)
            .field("edges", &self.edges)
            .finish()
    }
}

/// Which form of trigger the user declared.
pub enum TriggerSpec {
    /// `react = when_any(a, b)` — bundled form.
    Bundled(ReactionCriteria),
    /// `trigger = reactor(TypePath)` — split form referencing a standalone
    /// reactor declaration by type path.
    ByReactor(TypePath),
    /// No `react` and no `trigger` — the graph is trigger-less.
    None,
}

impl std::fmt::Debug for TriggerSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TriggerSpec::Bundled(r) => f.debug_tuple("Bundled").field(r).finish(),
            TriggerSpec::ByReactor(tp) => {
                let segs: Vec<String> = tp
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                f.debug_tuple("ByReactor").field(&segs.join("::")).finish()
            }
            TriggerSpec::None => f.write_str("None"),
        }
    }
}

/// Reaction criteria: when_any or when_all with accumulator names.
#[derive(Debug, Clone)]
pub struct ReactionCriteria {
    pub mode: ReactionMode,
    pub accumulators: Vec<Ident>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReactionMode {
    WhenAny,
    WhenAll,
}

/// A parsed edge in the topology.
#[derive(Debug)]
pub enum ParsedEdge {
    /// `node_a(inputs) -> node_b`
    Linear {
        from: Ident,
        from_inputs: Vec<Ident>,
        to: Ident,
    },
    /// `node_a(inputs) => { Variant -> node_b, Variant2 -> node_c }`
    Routing {
        from: Ident,
        from_inputs: Vec<Ident>,
        variants: Vec<RoutingVariant>,
    },
}

/// A single variant -> downstream mapping in a routing edge.
#[derive(Debug)]
pub struct RoutingVariant {
    pub variant_name: Ident,
    pub target: Ident,
}

impl ParsedEdge {
    pub fn from_name(&self) -> &Ident {
        match self {
            ParsedEdge::Linear { from, .. } => from,
            ParsedEdge::Routing { from, .. } => from,
        }
    }

    pub fn from_inputs(&self) -> &[Ident] {
        match self {
            ParsedEdge::Linear { from_inputs, .. } => from_inputs,
            ParsedEdge::Routing { from_inputs, .. } => from_inputs,
        }
    }
}

// --- Parsing implementation ---

impl Parse for ParsedTopology {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut react: Option<ReactionCriteria> = None;
        let mut trigger_reactor: Option<TypePath> = None;
        let mut edges: Option<Vec<ParsedEdge>> = None;
        let mut react_key_span: Option<proc_macro2::Span> = None;
        let mut trigger_key_span: Option<proc_macro2::Span> = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match key.to_string().as_str() {
                "react" => {
                    if react.is_some() {
                        return Err(syn::Error::new(key.span(), "duplicate 'react' field"));
                    }
                    react_key_span = Some(key.span());
                    react = Some(input.parse()?);
                }
                "trigger" => {
                    if trigger_reactor.is_some() {
                        return Err(syn::Error::new(key.span(), "duplicate 'trigger' field"));
                    }
                    trigger_key_span = Some(key.span());
                    // Expect: reactor(TypePath)
                    let kind: Ident = input.parse()?;
                    if kind != "reactor" {
                        return Err(syn::Error::new(
                            kind.span(),
                            format!(
                                "unknown trigger kind '{}', expected 'reactor' \
                                 (e.g. trigger = reactor(RiskSignals))",
                                kind
                            ),
                        ));
                    }
                    let paren;
                    syn::parenthesized!(paren in input);
                    let type_path: TypePath = paren.parse()?;
                    if !paren.is_empty() {
                        return Err(syn::Error::new(
                            paren.span(),
                            "reactor(...) takes exactly one type path argument",
                        ));
                    }
                    trigger_reactor = Some(type_path);
                }
                "graph" => {
                    if edges.is_some() {
                        return Err(syn::Error::new(key.span(), "duplicate 'graph' field"));
                    }
                    edges = Some(parse_graph_block(input)?);
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!(
                            "unknown field '{}', expected 'react', 'trigger', or 'graph'",
                            other
                        ),
                    ));
                }
            }

            // Optional trailing comma between top-level fields
            let _ = input.parse::<Token![,]>();
        }

        let edges = edges.ok_or_else(|| {
            syn::Error::new(proc_macro2::Span::call_site(), "missing 'graph' field")
        })?;

        let trigger = match (react, trigger_reactor) {
            (Some(_), Some(_)) => {
                let span = trigger_key_span
                    .or(react_key_span)
                    .unwrap_or_else(proc_macro2::Span::call_site);
                return Err(syn::Error::new(
                    span,
                    "'react' and 'trigger' cannot both be set — pick one. \
                     'react = when_any(...)' is the bundled form; \
                     'trigger = reactor(T)' is the split form referencing a \
                     standalone #[reactor] declaration",
                ));
            }
            (Some(r), None) => TriggerSpec::Bundled(r),
            (None, Some(t)) => TriggerSpec::ByReactor(t),
            (None, None) => TriggerSpec::None,
        };

        Ok(ParsedTopology { trigger, edges })
    }
}

impl Parse for ReactionCriteria {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mode_ident: Ident = input.parse()?;
        let mode = match mode_ident.to_string().as_str() {
            "when_any" => ReactionMode::WhenAny,
            "when_all" => ReactionMode::WhenAll,
            other => {
                return Err(syn::Error::new(
                    mode_ident.span(),
                    format!(
                        "unknown reaction mode '{}', expected 'when_any' or 'when_all'",
                        other
                    ),
                ));
            }
        };

        let content;
        syn::parenthesized!(content in input);
        let accumulators: Punctuated<Ident, Token![,]> =
            content.parse_terminated(Ident::parse, Token![,])?;

        Ok(ReactionCriteria {
            mode,
            accumulators: accumulators.into_iter().collect(),
        })
    }
}

/// Parse the `graph = { ... }` block containing edge declarations.
fn parse_graph_block(input: ParseStream) -> syn::Result<Vec<ParsedEdge>> {
    let content;
    braced!(content in input);

    let mut edges = Vec::new();

    while !content.is_empty() {
        edges.push(parse_edge(&content)?);
        // Optional trailing comma between edges
        let _ = content.parse::<Token![,]>();
    }

    Ok(edges)
}

/// Parse a single edge declaration.
///
/// Either:
/// - `node_name(inputs) -> target` (linear)
/// - `node_name(inputs) => { Variant -> target, ... }` (routing)
/// - `node_name -> target` (linear, no inputs)
/// - `node_name` (terminal node, no edges — just declares it exists)
fn parse_edge(input: ParseStream) -> syn::Result<ParsedEdge> {
    let from: Ident = input.parse()?;

    // Parse optional parenthesized cache inputs
    let from_inputs = if input.peek(syn::token::Paren) {
        let content;
        syn::parenthesized!(content in input);
        let inputs: Punctuated<Ident, Token![,]> =
            content.parse_terminated(Ident::parse, Token![,])?;
        inputs.into_iter().collect()
    } else {
        Vec::new()
    };

    // Determine edge type by lookahead
    if input.peek(Token![=>]) {
        // Routing edge: node => { Variant -> target, ... }
        input.parse::<Token![=>]>()?;
        let variants_content;
        braced!(variants_content in input);

        let mut variants = Vec::new();
        while !variants_content.is_empty() {
            let variant_name: Ident = variants_content.parse()?;
            variants_content.parse::<Token![->]>()?;
            let target: Ident = variants_content.parse()?;
            variants.push(RoutingVariant {
                variant_name,
                target,
            });
            // Optional trailing comma
            let _ = variants_content.parse::<Token![,]>();
        }

        if variants.is_empty() {
            return Err(syn::Error::new(
                from.span(),
                "routing edge must have at least one variant",
            ));
        }

        Ok(ParsedEdge::Routing {
            from,
            from_inputs,
            variants,
        })
    } else if input.peek(Token![->]) {
        // Linear edge: node -> target
        input.parse::<Token![->]>()?;
        let to: Ident = input.parse()?;
        Ok(ParsedEdge::Linear {
            from,
            from_inputs,
            to,
        })
    } else {
        // Terminal node with no downstream — this is valid but we represent it
        // as a linear edge to nowhere. The graph IR will handle terminal detection.
        // For now, error — we require explicit edges.
        Err(syn::Error::new(
            from.span(),
            format!(
                "expected '->' or '=>' after node '{}'. Terminal nodes are detected automatically \
                 from the graph — they don't need explicit declaration.",
                from
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    fn parse_topology(tokens: proc_macro2::TokenStream) -> syn::Result<ParsedTopology> {
        syn::parse2::<ParsedTopology>(tokens)
    }

    fn bundled(trigger: &TriggerSpec) -> &ReactionCriteria {
        match trigger {
            TriggerSpec::Bundled(r) => r,
            other => panic!("expected TriggerSpec::Bundled, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_when_any() {
        let tokens = quote! {
            react = when_any(alpha, beta, gamma),
            graph = {
                entry(alpha, beta) -> output,
            }
        };
        let topology = parse_topology(tokens).unwrap();
        let react = bundled(&topology.trigger);
        assert_eq!(react.mode, ReactionMode::WhenAny);
        assert_eq!(react.accumulators.len(), 3);
        assert_eq!(react.accumulators[0].to_string(), "alpha");
        assert_eq!(react.accumulators[1].to_string(), "beta");
        assert_eq!(react.accumulators[2].to_string(), "gamma");
    }

    #[test]
    fn test_parse_when_all() {
        let tokens = quote! {
            react = when_all(a, b),
            graph = {
                entry(a, b) -> output,
            }
        };
        let topology = parse_topology(tokens).unwrap();
        let react = bundled(&topology.trigger);
        assert_eq!(react.mode, ReactionMode::WhenAll);
        assert_eq!(react.accumulators.len(), 2);
    }

    #[test]
    fn test_parse_split_form_trigger_reactor() {
        let tokens = quote! {
            trigger = reactor(my_crate::RiskSignals),
            graph = {
                entry(alpha) -> output,
            }
        };
        let topology = parse_topology(tokens).unwrap();
        match topology.trigger {
            TriggerSpec::ByReactor(tp) => {
                // Reconstruct the path string for assertion
                let path = tp.path.segments.last().unwrap().ident.to_string();
                assert_eq!(path, "RiskSignals");
            }
            other => panic!("expected ByReactor, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_triggerless_form() {
        let tokens = quote! {
            graph = {
                entry(alpha) -> output,
            }
        };
        let topology = parse_topology(tokens).unwrap();
        assert!(matches!(topology.trigger, TriggerSpec::None));
    }

    #[test]
    fn test_error_react_and_trigger_both_set() {
        let tokens = quote! {
            react = when_any(alpha),
            trigger = reactor(SomeReactor),
            graph = { a(alpha) -> b },
        };
        let result = parse_topology(tokens);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cannot both be set"), "got: {}", err);
    }

    #[test]
    fn test_error_trigger_unknown_kind() {
        let tokens = quote! {
            trigger = schedule("0 * * * *"),
            graph = { a -> b },
        };
        let result = parse_topology(tokens);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("unknown trigger kind"), "got: {}", err);
    }

    #[test]
    fn test_parse_linear_edge() {
        let tokens = quote! {
            react = when_any(alpha),
            graph = {
                entry(alpha) -> middle,
                middle -> output,
            }
        };
        let topology = parse_topology(tokens).unwrap();
        assert_eq!(topology.edges.len(), 2);

        match &topology.edges[0] {
            ParsedEdge::Linear {
                from,
                from_inputs,
                to,
            } => {
                assert_eq!(from.to_string(), "entry");
                assert_eq!(from_inputs.len(), 1);
                assert_eq!(from_inputs[0].to_string(), "alpha");
                assert_eq!(to.to_string(), "middle");
            }
            _ => panic!("expected linear edge"),
        }

        match &topology.edges[1] {
            ParsedEdge::Linear {
                from,
                from_inputs,
                to,
            } => {
                assert_eq!(from.to_string(), "middle");
                assert!(from_inputs.is_empty());
                assert_eq!(to.to_string(), "output");
            }
            _ => panic!("expected linear edge"),
        }
    }

    #[test]
    fn test_parse_routing_edge() {
        let tokens = quote! {
            react = when_any(alpha),
            graph = {
                decision(alpha) => {
                    Signal -> handler_a,
                    NoAction -> handler_b,
                },
            }
        };
        let topology = parse_topology(tokens).unwrap();
        assert_eq!(topology.edges.len(), 1);

        match &topology.edges[0] {
            ParsedEdge::Routing {
                from,
                from_inputs,
                variants,
            } => {
                assert_eq!(from.to_string(), "decision");
                assert_eq!(from_inputs.len(), 1);
                assert_eq!(variants.len(), 2);
                assert_eq!(variants[0].variant_name.to_string(), "Signal");
                assert_eq!(variants[0].target.to_string(), "handler_a");
                assert_eq!(variants[1].variant_name.to_string(), "NoAction");
                assert_eq!(variants[1].target.to_string(), "handler_b");
            }
            _ => panic!("expected routing edge"),
        }
    }

    #[test]
    fn test_parse_mixed_edges() {
        let tokens = quote! {
            react = when_any(alpha, beta, gamma),
            graph = {
                decision_engine(alpha, beta, gamma) => {
                    Signal -> risk_check,
                    NoAction -> audit_logger,
                },
                risk_check(gamma) => {
                    Approved -> output_handler,
                    Blocked -> alert_handler,
                },
            }
        };
        let topology = parse_topology(tokens).unwrap();
        assert_eq!(topology.edges.len(), 2);

        // First edge: routing with 3 cache inputs
        match &topology.edges[0] {
            ParsedEdge::Routing {
                from, from_inputs, ..
            } => {
                assert_eq!(from.to_string(), "decision_engine");
                assert_eq!(from_inputs.len(), 3);
            }
            _ => panic!("expected routing edge"),
        }

        // Second edge: routing with 1 cache input
        match &topology.edges[1] {
            ParsedEdge::Routing {
                from,
                from_inputs,
                variants,
            } => {
                assert_eq!(from.to_string(), "risk_check");
                assert_eq!(from_inputs.len(), 1);
                assert_eq!(from_inputs[0].to_string(), "gamma");
                assert_eq!(variants.len(), 2);
            }
            _ => panic!("expected routing edge"),
        }
    }

    #[test]
    fn test_parse_fan_in() {
        let tokens = quote! {
            react = when_any(a, b),
            graph = {
                validate_a(a) -> merge,
                validate_b(b) -> merge,
            }
        };
        let topology = parse_topology(tokens).unwrap();
        assert_eq!(topology.edges.len(), 2);

        // Both edges point to "merge"
        match &topology.edges[0] {
            ParsedEdge::Linear { to, .. } => assert_eq!(to.to_string(), "merge"),
            _ => panic!("expected linear edge"),
        }
        match &topology.edges[1] {
            ParsedEdge::Linear { to, .. } => assert_eq!(to.to_string(), "merge"),
            _ => panic!("expected linear edge"),
        }
    }

    #[test]
    fn test_parse_fan_out() {
        let tokens = quote! {
            react = when_any(a),
            graph = {
                compute(a) -> output_handler,
                compute(a) -> audit_logger,
            }
        };
        let topology = parse_topology(tokens).unwrap();
        assert_eq!(topology.edges.len(), 2);

        match &topology.edges[0] {
            ParsedEdge::Linear { from, to, .. } => {
                assert_eq!(from.to_string(), "compute");
                assert_eq!(to.to_string(), "output_handler");
            }
            _ => panic!("expected linear edge"),
        }
        match &topology.edges[1] {
            ParsedEdge::Linear { from, to, .. } => {
                assert_eq!(from.to_string(), "compute");
                assert_eq!(to.to_string(), "audit_logger");
            }
            _ => panic!("expected linear edge"),
        }
    }

    #[test]
    fn test_error_missing_graph() {
        let tokens = quote! {
            react = when_any(a)
        };
        let result = parse_topology(tokens);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("missing 'graph' field"), "got: {}", err);
    }

    #[test]
    fn test_error_unknown_field() {
        let tokens = quote! {
            react = when_any(a),
            graph = { a -> b },
            bogus = something,
        };
        let result = parse_topology(tokens);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("unknown field 'bogus'"), "got: {}", err);
    }

    #[test]
    fn test_error_unknown_reaction_mode() {
        let tokens = quote! {
            react = when_sometimes(a),
            graph = { a -> b },
        };
        let result = parse_topology(tokens);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("unknown reaction mode"), "got: {}", err);
    }

    #[test]
    fn test_error_empty_routing() {
        let tokens = quote! {
            react = when_any(a),
            graph = {
                a(a) => {},
            }
        };
        let result = parse_topology(tokens);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("at least one variant"), "got: {}", err);
    }

    #[test]
    fn test_error_duplicate_react() {
        let tokens = quote! {
            react = when_any(a),
            react = when_all(b),
            graph = { a -> b },
        };
        let result = parse_topology(tokens);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("duplicate 'react'"), "got: {}", err);
    }
}
