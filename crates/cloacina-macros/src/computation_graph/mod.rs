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

mod codegen;
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
    // Step 1: Parse the attribute arguments (react + graph topology)
    let topology = syn::parse2::<parser::ParsedTopology>(args)?;

    // Step 2: Parse the module
    let module = syn::parse2::<syn::ItemMod>(input)?;

    // Step 3: Build Graph IR from parsed topology
    let ir = graph_ir::GraphIR::from_parsed(topology)
        .map_err(|e| syn::Error::new(proc_macro2::Span::call_site(), e.to_string()))?;

    // Step 4: Validate and generate the compiled function
    codegen::generate(&ir, &module)
}
