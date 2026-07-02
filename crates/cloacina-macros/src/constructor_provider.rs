/*
 *  Copyright 2026 Colliery Software
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

//! `constructor_provider!` — the crate-level **suite shell** (CLOACI-A-0011 / T-0837).
//!
//! A provider is a SUITE: one provider crate compiles to ONE WASM component that
//! exposes N member `#[constructor]`s behind ONE per-kind fidius interface, with
//! the member chosen by NAME carried in the `configure` payload. This macro is the
//! aggregator the design hinges on. Each `#[constructor]` now emits only its
//! object-safe member (`impl <Kind>Object for __<Struct>Configured`) plus associated
//! metadata fns (`__constructor_name` / `__constructor_manifest` /
//! `__constructor_make`); this shell wires those members into the fidius glue:
//!
//! For each KIND that has at least one listed member, the shell emits (wasm-guest
//! only):
//!   * the ONE `#[plugin_interface] <Kind>Constructor` trait — byte-identical to the
//!     host loader's re-declaration so the fidius interface hash matches;
//!   * a `__Provider<Kind>Configure { name: String, config: Vec<u8> }` wire type —
//!     the `(member-name, that-member's-config-bytes)` the host serializes at load;
//!   * a `__Provider<Kind> { inner: Option<Box<dyn <Kind>Object>> }` holder + its
//!     `#[plugin_impl(<Kind>Constructor, config = __Provider<Kind>Configure)]`, whose
//!     single method dispatches to the configured member;
//!   * a name-dispatched `configure(cfg)` that selects the member by
//!     `<Member>::__constructor_name()` and builds it via `<Member>::__constructor_make`.
//!
//! And ONE crate-level, on every target:
//!   * `pub fn __provider_manifest() -> ProviderManifest` listing every member's
//!     `__constructor_manifest()` — the `provider.json` packaging emits and the
//!     loader reads.
//!
//! A single-constructor provider is just a suite of one. Multiple KINDS in one
//! provider means multiple fidius plugins in one component (fidius multi-plugin); a
//! homogeneous (single-kind) suite — the common case — is a single plugin.
//!
//! ```rust,ignore
//! constructor_provider!(
//!     name = "cloacina-provider-fs",
//!     version = "0.1.0",
//!     task = [ReadFile, WriteFile],
//! );
//! ```

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Ident, LitStr, Path, Result as SynResult, Token,
};

/// The four primitive kinds a provider may expose.
#[derive(Clone, Copy)]
enum Kind {
    Task,
    Trigger,
    Accumulator,
    Reactor,
}

impl Kind {
    /// The fidius interface trait the shell declares + impls for this kind. Must
    /// match the host loader's re-declaration (`constructor_loader.rs`) verbatim.
    fn trait_ident(self) -> Ident {
        format_ident!(
            "{}",
            match self {
                Kind::Task => "TaskConstructor",
                Kind::Trigger => "TriggerConstructor",
                Kind::Accumulator => "AccumulatorConstructor",
                Kind::Reactor => "ReactorConstructor",
            }
        )
    }

    /// The single sync method name on the interface (and the object-safe trait).
    fn method_ident(self) -> Ident {
        format_ident!(
            "{}",
            match self {
                Kind::Task => "execute",
                Kind::Trigger => "poll",
                Kind::Accumulator => "ingest",
                Kind::Reactor => "evaluate",
            }
        )
    }

    /// The object-safe member trait (in the contract crate) the shell boxes.
    fn object_ident(self) -> Ident {
        format_ident!(
            "{}",
            match self {
                Kind::Task => "TaskObject",
                Kind::Trigger => "TriggerObject",
                Kind::Accumulator => "AccumulatorObject",
                Kind::Reactor => "ReactorObject",
            }
        )
    }

    /// A short, distinct infix for the generated shell type idents.
    fn shell_infix(self) -> &'static str {
        match self {
            Kind::Task => "Task",
            Kind::Trigger => "Trigger",
            Kind::Accumulator => "Accumulator",
            Kind::Reactor => "Reactor",
        }
    }

    /// The fallback outcome JSON when no member is configured (defensive — the
    /// loader validates the name before calling, so this should be unreachable).
    fn no_member_outcome(self) -> &'static str {
        match self {
            Kind::Task => r#"{"success":false,"error":"provider: no configured member"}"#,
            Kind::Trigger | Kind::Reactor => {
                r#"{"fire":false,"error":"provider: no configured member"}"#
            }
            Kind::Accumulator => r#"{"error":"provider: no configured member"}"#,
        }
    }
}

/// Parsed `constructor_provider!(...)` arguments.
struct ProviderArgs {
    /// The provider name. `None` → defaults to `env!("CARGO_PKG_NAME")` (the
    /// provider crate's own Cargo package name), so the fidius provider name a
    /// consumer resolves via `find_wasm_package` always equals the Cargo package
    /// name a build resolves via `cargo metadata` (CLOACI-A-0010).
    name: Option<String>,
    version: String,
    /// The `.wasm` component filename. `None` → derived from `name` (`-`→`_` + `.wasm`).
    component: Option<String>,
    /// Path to the constructor-contract crate (default `::cloacina_constructor_contract`).
    contract: Path,
    /// fidius guest crate string passed to the fidius macros (default `fidius_guest`).
    fidius_crate: String,
    task: Vec<Ident>,
    trigger: Vec<Ident>,
    accumulator: Vec<Ident>,
    reactor: Vec<Ident>,
}

fn parse_ident_list(input: ParseStream) -> SynResult<Vec<Ident>> {
    let content;
    bracketed!(content in input);
    let punct = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?;
    Ok(punct.into_iter().collect())
}

impl Parse for ProviderArgs {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut name: Option<String> = None;
        let mut version: Option<String> = None;
        let mut component: Option<String> = None;
        let mut contract: Option<Path> = None;
        let mut fidius_crate: Option<String> = None;
        let mut task: Vec<Ident> = Vec::new();
        let mut trigger: Vec<Ident> = Vec::new();
        let mut accumulator: Vec<Ident> = Vec::new();
        let mut reactor: Vec<Ident> = Vec::new();

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "name" => name = Some(input.parse::<LitStr>()?.value()),
                "version" => version = Some(input.parse::<LitStr>()?.value()),
                "component" => component = Some(input.parse::<LitStr>()?.value()),
                "contract" => contract = Some(input.parse::<Path>()?),
                "fidius_crate" => fidius_crate = Some(input.parse::<LitStr>()?.value()),
                "task" => task = parse_ident_list(input)?,
                "trigger" => trigger = parse_ident_list(input)?,
                "accumulator" => accumulator = parse_ident_list(input)?,
                "reactor" => reactor = parse_ident_list(input)?,
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown constructor_provider! argument `{other}`; valid: name, version, component, contract, fidius_crate, task, trigger, accumulator, reactor"),
                    ));
                }
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        let version = version.ok_or_else(|| {
            syn::Error::new(input.span(), "constructor_provider! requires `version`")
        })?;
        let contract =
            contract.unwrap_or_else(|| syn::parse_quote!(::cloacina_constructor_contract));
        let fidius_crate = fidius_crate.unwrap_or_else(|| "fidius_guest".to_string());

        if task.is_empty() && trigger.is_empty() && accumulator.is_empty() && reactor.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "constructor_provider! requires at least one member (task/trigger/accumulator/reactor = [..])",
            ));
        }

        Ok(ProviderArgs {
            name,
            version,
            component,
            contract,
            fidius_crate,
            task,
            trigger,
            accumulator,
            reactor,
        })
    }
}

/// Entry point for the `constructor_provider!` function-like macro.
pub fn constructor_provider(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as ProviderArgs);
    expand(args).into()
}

fn expand(args: ProviderArgs) -> TokenStream2 {
    let contract = &args.contract;
    let fidius_crate = LitStr::new(&args.fidius_crate, Span::call_site());

    // The provider name defaults to the provider crate's own Cargo package name so
    // build-time (`cargo metadata`) and load-time (`find_wasm_package`) resolve the
    // consumer's `from` against the SAME string (A-0010). `env!` in the emitted code
    // resolves in the provider crate, i.e. its own `CARGO_PKG_NAME`.
    let name_expr = match &args.name {
        Some(n) => quote! { #n.to_string() },
        None => quote! { ::std::string::String::from(env!("CARGO_PKG_NAME")) },
    };
    // Component filename default (`-`→`_` + `.wasm`); packaging overrides it with the
    // actual built artifact, so this is only a hint.
    let component_expr = match (&args.component, &args.name) {
        (Some(c), _) => quote! { #c.to_string() },
        (None, Some(n)) => {
            let c = format!("{}.wasm", n.replace('-', "_"));
            quote! { #c.to_string() }
        }
        (None, None) => {
            quote! { format!("{}.wasm", env!("CARGO_PKG_NAME").replace('-', "_")) }
        }
    };

    // One fidius shell per KIND present.
    let mut shells: Vec<TokenStream2> = Vec::new();
    for (kind, members) in [
        (Kind::Task, &args.task),
        (Kind::Trigger, &args.trigger),
        (Kind::Accumulator, &args.accumulator),
        (Kind::Reactor, &args.reactor),
    ] {
        if members.is_empty() {
            continue;
        }
        shells.push(kind_shell(kind, members, contract, &fidius_crate));
    }

    // The crate-level provider manifest — every member across every kind. Pure
    // serde, emitted on all targets (the packaging `emit_manifest` bin reads it).
    let all_members: Vec<&Ident> = args
        .task
        .iter()
        .chain(args.trigger.iter())
        .chain(args.accumulator.iter())
        .chain(args.reactor.iter())
        .collect();
    let manifest_exprs = all_members
        .iter()
        .map(|m| quote! { #m::__constructor_manifest() });

    let provider_version = &args.version;

    quote! {
        #(#shells)*

        /// CLOACI-A-0011/T-0837: the provider manifest (`provider.json`) — the
        /// `List[Constructor]` index over this suite's members. Packaging's
        /// `emit_manifest` bin serializes this; the loader reads it to select a
        /// member by `constructor = "<name>"`. Emitted on every target.
        #[allow(dead_code)]
        pub fn __provider_manifest() -> #contract::ProviderManifest {
            #contract::ProviderManifest {
                name: #name_expr,
                version: #provider_version.to_string(),
                component: #component_expr,
                constructors: ::std::vec![ #(#manifest_exprs),* ],
            }
        }
    }
}

/// The wasm-guest fidius shell for one kind: the interface, the configure wire
/// type, the holder + `#[plugin_impl]`, and the name-dispatched `configure`.
fn kind_shell(
    kind: Kind,
    members: &[Ident],
    contract: &Path,
    fidius_crate: &LitStr,
) -> TokenStream2 {
    let trait_ident = kind.trait_ident();
    let method_ident = kind.method_ident();
    let object_ident = kind.object_ident();
    let infix = kind.shell_infix();
    let configure_ty = format_ident!("__Provider{}Configure", infix);
    let holder_ty = format_ident!("__Provider{}", infix);
    let no_member = LitStr::new(kind.no_member_outcome(), Span::call_site());

    // The interface trait — byte-identical to the host loader's re-declaration so
    // the fidius interface hash matches at load.
    let interface_decl = quote! {
        #[cfg(target_arch = "wasm32")]
        #[::fidius_macro::plugin_interface(version = 1, buffer = PluginAllocated, crate = #fidius_crate)]
        pub trait #trait_ident: Send + Sync {
            fn #method_ident(&self, invocation_json: String) -> String;
        }
    };

    // The name-dispatch: pick the member whose `__constructor_name()` matches and
    // build it from the per-member config bytes.
    let dispatch_arms = members.iter().map(|m| {
        quote! {
            if __c.name == #m::__constructor_name() {
                return Self { inner: ::std::option::Option::Some(#m::__constructor_make(&__c.config)) };
            }
        }
    });

    quote! {
        #interface_decl

        /// The suite `configure` payload for this kind: `(member-name, that
        /// member's config bytes)`. The host serializes it at load; fidius's
        /// `fidius-configure` decodes it here before name-dispatch.
        #[cfg(target_arch = "wasm32")]
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        pub struct #configure_ty {
            pub name: ::std::string::String,
            pub config: ::std::vec::Vec<u8>,
        }

        #[cfg(target_arch = "wasm32")]
        pub struct #holder_ty {
            inner: ::std::option::Option<::std::boxed::Box<dyn #contract::#object_ident>>,
        }

        #[cfg(target_arch = "wasm32")]
        #[::fidius_macro::plugin_impl(#trait_ident, crate = #fidius_crate, config = #configure_ty)]
        impl #trait_ident for #holder_ty {
            fn #method_ident(&self, invocation_json: String) -> String {
                match &self.inner {
                    ::std::option::Option::Some(__m) => #contract::#object_ident::#method_ident(__m.as_ref(), invocation_json),
                    ::std::option::Option::None => #no_member.to_string(),
                }
            }
        }

        #[cfg(target_arch = "wasm32")]
        impl #holder_ty {
            // The fidius `configure` hook: select the member named in the payload
            // and build it once at load.
            fn configure(__c: #configure_ty) -> Self {
                #(#dispatch_arms)*
                Self { inner: ::std::option::Option::None }
            }
        }
    }
}
