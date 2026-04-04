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

//! `#[computation_graph]` attribute macro implementation.
//!
//! Parses the topology declaration from the macro attribute, builds a graph IR,
//! validates it, and generates a compiled async function.

pub(crate) mod graph_ir;
mod parser;

use proc_macro::TokenStream;

/// The `#[computation_graph]` attribute macro entry point.
///
/// Applied to a module containing async node functions. The attribute declares
/// the graph topology and reaction criteria. The macro compiles the module into
/// a single async function.
pub fn computation_graph_attr(args: TokenStream, input: TokenStream) -> TokenStream {
    let args2 = proc_macro2::TokenStream::from(args);
    let input2 = proc_macro2::TokenStream::from(input);

    match computation_graph_impl(args2, input2) {
        Ok(output) => output.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn computation_graph_impl(
    args: proc_macro2::TokenStream,
    input: proc_macro2::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    // Parse the attribute arguments (react + graph topology)
    let topology = syn::parse2::<parser::ParsedTopology>(args)?;

    // Parse the module
    let module = syn::parse2::<syn::ItemMod>(input)?;

    // For now, just pass through the module unchanged (skeleton)
    // T-0360 and T-0361 will add graph IR building and code generation
    let _ = topology; // suppress unused warning

    Ok(quote::quote! { #module })
}
