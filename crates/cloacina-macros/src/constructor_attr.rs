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

//! `#[constructor]` authoring macro (CLOACI-T-0826).
//!
//! Lets an author write the *clean* constructor form and generates the *raw*
//! fidius constructor contract the loader (`crates/cloacina/src/registry/loader/
//! constructor_loader.rs`) already consumes:
//!
//! 1. the SYNC `#[plugin_interface] <Kind>Constructor` trait + a
//!    `#[plugin_impl(config = <generated Config>)]` impl whose single sync
//!    method deserializes the per-primitive `*Invocation`, pulls the declared
//!    `#[param]` fields out of the task context (required/optional honored),
//!    runs the author body with `#[config]` fields bound, and serializes the
//!    `*Outcome`;
//! 2. the `configure` hook binding the `#[config]` fields once at load;
//! 3. a manifest-producing item `pub fn __constructor_manifest() -> ConstructorManifest`
//!    carrying `primitive_kind`, `name`, `version`, `interface`, and
//!    `params: Vec<InputSlot>` derived from the `#[param]` fields (reusing the
//!    CLOACI-I-0128 `InputSlot` shape). The macro cannot write a file; packaging
//!    (CLOACI-T-0827) writes `constructor.json` from this fn.
//!
//! ## Author surface (TASK kind — proven end-to-end)
//!
//! ```rust,ignore
//! #[constructor(kind = task, name = "prefix", version = "0.1.0")]
//! struct Prefix {
//!     #[config] prefix: String,          // bound once per instance
//!     #[param(required)] name: String,   // declared input, pulled from context
//! }
//! impl Prefix {
//!     fn execute(&self) -> Result<(), ConstructorError> {   // the ONLY thing written
//!         self.set("result", format!("{}{}", self.prefix, self.name));
//!         Ok(())
//!     }
//! }
//! ```
//!
//! The fidius guest glue is emitted under `#[cfg(target_arch = "wasm32")]` so the
//! same crate compiles on the host (where only the struct + `set`/`get` + the
//! `__constructor_manifest()` fn exist) — that is how packaging / a host harness can
//! read the manifest without the wasm-only exports.
//!
//! ## Deferred (noted continuation)
//!
//! Only `kind = task` is fully code-generated + fixture-proven here. The sibling
//! kinds map cleanly onto the same shape — each to its own sync trait + body:
//!
//! | kind          | trait               | body fn    | wire in / out                                  |
//! |---------------|---------------------|------------|------------------------------------------------|
//! | `task`        | `TaskConstructor`      | `execute`  | `TaskInvocation` / `TaskOutcome`               |
//! | `trigger`     | `TriggerConstructor`   | `poll`     | `TriggerInvocation` / `PollOutcome`            |
//! | `accumulator` | `AccumulatorConstructor` | `ingest` | `AccumulatorInvocation` / `AccumulatorOutcome` |
//! | `reactor`     | `ReactorConstructor`   | `evaluate` | `ReactorInvocation` / `ReactorOutcome`         |
//!
//! Their full codegen + fixtures are deferred (their host bridges are themselves
//! a noted T-0824 continuation). The consumer-side `constructor!(...)` workflow
//! instantiation is a separate follow-up.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Data, DeriveInput, Fields, Ident, LitStr, Path, Result as SynResult, Token,
    Type,
};

/// Which cloacina primitive the constructor plugs into.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Task,
    Trigger,
    Accumulator,
    Reactor,
}

impl Kind {
    fn from_ident(id: &Ident) -> SynResult<Self> {
        match id.to_string().as_str() {
            "task" => Ok(Kind::Task),
            "trigger" => Ok(Kind::Trigger),
            "accumulator" => Ok(Kind::Accumulator),
            "reactor" => Ok(Kind::Reactor),
            other => Err(syn::Error::new(
                id.span(),
                format!("unknown constructor kind `{other}`; expected one of: task, trigger, accumulator, reactor"),
            )),
        }
    }

    /// Default fidius interface name (the kebab the package.toml `interface` and
    /// the manifest carry).
    fn interface(self) -> &'static str {
        match self {
            Kind::Task => "task-constructor",
            Kind::Trigger => "trigger-constructor",
            Kind::Accumulator => "accumulator-constructor",
            Kind::Reactor => "reactor-constructor",
        }
    }

    fn primitive_kind_variant(self) -> Ident {
        let name = match self {
            Kind::Task => "Task",
            Kind::Trigger => "Trigger",
            Kind::Accumulator => "Accumulator",
            Kind::Reactor => "Reactor",
        };
        format_ident!("{name}")
    }
}

/// Parsed `#[constructor(...)]` attribute arguments.
struct ConstructorArgs {
    kind: Kind,
    name: String,
    version: String,
    /// Path to the constructor-contract crate (default `::cloacina_constructor_contract`).
    contract: Path,
    /// fidius guest crate string passed to the fidius macros (default `fidius_guest`).
    fidius_crate: String,
    interface: Option<String>,
    interface_version: u32,
    description: Option<String>,
    author: Option<String>,
}

impl Parse for ConstructorArgs {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut kind: Option<Kind> = None;
        let mut name: Option<String> = None;
        let mut version: Option<String> = None;
        let mut contract: Option<Path> = None;
        let mut fidius_crate: Option<String> = None;
        let mut interface: Option<String> = None;
        let mut interface_version: u32 = 1;
        let mut description: Option<String> = None;
        let mut author: Option<String> = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "kind" => {
                    let id: Ident = input.parse()?;
                    kind = Some(Kind::from_ident(&id)?);
                }
                "name" => name = Some(input.parse::<LitStr>()?.value()),
                "version" => version = Some(input.parse::<LitStr>()?.value()),
                "contract" => contract = Some(input.parse::<Path>()?),
                "fidius_crate" => fidius_crate = Some(input.parse::<LitStr>()?.value()),
                "interface" => interface = Some(input.parse::<LitStr>()?.value()),
                "interface_version" => {
                    interface_version = input.parse::<syn::LitInt>()?.base10_parse()?
                }
                "description" => description = Some(input.parse::<LitStr>()?.value()),
                "author" => author = Some(input.parse::<LitStr>()?.value()),
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown #[constructor] argument `{other}`; valid: kind, name, version, contract, fidius_crate, interface, interface_version, description, author"),
                    ));
                }
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        let kind =
            kind.ok_or_else(|| syn::Error::new(input.span(), "#[constructor] requires `kind`"))?;
        let name =
            name.ok_or_else(|| syn::Error::new(input.span(), "#[constructor] requires `name`"))?;
        let version = version
            .ok_or_else(|| syn::Error::new(input.span(), "#[constructor] requires `version`"))?;
        let contract =
            contract.unwrap_or_else(|| syn::parse_quote!(::cloacina_constructor_contract));
        let fidius_crate = fidius_crate.unwrap_or_else(|| "fidius_guest".to_string());

        Ok(ConstructorArgs {
            kind,
            name,
            version,
            contract,
            fidius_crate,
            interface,
            interface_version,
            description,
            author,
        })
    }
}

/// One declared `#[param]` field.
struct ParamField {
    ident: Ident,
    ty: Type,
    required: bool,
    /// Inner `T` of `Option<T>` for optional params (used for the schema).
    inner_ty: Type,
}

/// One declared `#[config]` field.
struct ConfigField {
    ident: Ident,
    ty: Type,
}

/// Entry point for the `#[constructor]` attribute macro.
pub fn constructor_attr(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as ConstructorArgs);
    let item = parse_macro_input!(input as DeriveInput);

    match expand(args, item) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(args: ConstructorArgs, item: DeriveInput) -> SynResult<TokenStream2> {
    if args.kind != Kind::Task {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[constructor]: only `kind = task` is code-generated today; \
             trigger/accumulator/reactor codegen is a noted CLOACI-T-0826 continuation \
             (the trait/body mapping is documented in constructor_attr.rs)",
        ));
    }

    let struct_ident = item.ident.clone();
    let struct_vis = item.vis.clone();

    if !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.generics,
            "#[constructor] does not support generic constructor structs",
        ));
    }

    let data = match &item.data {
        Data::Struct(d) => d,
        _ => {
            return Err(syn::Error::new_spanned(
                &struct_ident,
                "#[constructor] must be applied to a struct",
            ))
        }
    };
    let named = match &data.fields {
        Fields::Named(n) => n,
        _ => {
            return Err(syn::Error::new_spanned(
                &struct_ident,
                "#[constructor] requires a struct with named fields",
            ))
        }
    };

    // Partition fields into #[config] / #[param] and re-emit them clean.
    let mut config_fields: Vec<ConfigField> = Vec::new();
    let mut param_fields: Vec<ParamField> = Vec::new();
    let mut clean_field_defs: Vec<TokenStream2> = Vec::new();

    for field in &named.named {
        let ident = field
            .ident
            .clone()
            .ok_or_else(|| syn::Error::new_spanned(field, "field must be named"))?;
        let ty = field.ty.clone();

        let is_config = field.attrs.iter().any(|a| a.path().is_ident("config"));
        let param_attr = field.attrs.iter().find(|a| a.path().is_ident("param"));

        if is_config && param_attr.is_some() {
            return Err(syn::Error::new_spanned(
                field,
                "a field cannot be both #[config] and #[param]",
            ));
        }

        // Strip the helper attributes when re-emitting the field.
        let kept_attrs: Vec<&syn::Attribute> = field
            .attrs
            .iter()
            .filter(|a| !a.path().is_ident("config") && !a.path().is_ident("param"))
            .collect();
        let fvis = &field.vis;
        clean_field_defs.push(quote! { #(#kept_attrs)* #fvis #ident: #ty });

        if is_config {
            config_fields.push(ConfigField { ident, ty });
        } else if let Some(attr) = param_attr {
            let required = parse_param_required(attr)?;
            let inner_ty = if required {
                ty.clone()
            } else {
                option_inner(&ty).ok_or_else(|| {
                    syn::Error::new_spanned(
                        &field.ty,
                        "an optional #[param] field must have type `Option<T>`",
                    )
                })?
            };
            param_fields.push(ParamField {
                ident,
                ty,
                required,
                inner_ty,
            });
        } else {
            return Err(syn::Error::new_spanned(
                field,
                "every #[constructor] field must be marked #[config] or #[param]",
            ));
        }
    }

    let contract = &args.contract;
    let fidius_crate = LitStr::new(&args.fidius_crate, proc_macro2::Span::call_site());
    let op_name = &args.name;
    let op_version = &args.version;
    let interface = args
        .interface
        .clone()
        .unwrap_or_else(|| args.kind.interface().to_string());
    let interface_version = args.interface_version;
    let primitive_variant = args.kind.primitive_kind_variant();

    let description_tokens = opt_string(&args.description);
    let author_tokens = opt_string(&args.author);

    // ----- The author struct, re-emitted clean + an output buffer. -----
    let outputs_field = quote! {
        __constructor_outputs:
            ::std::cell::RefCell<::serde_json::Map<::std::string::String, ::serde_json::Value>>
    };
    let clean_struct = quote! {
        #struct_vis struct #struct_ident {
            #(#clean_field_defs,)*
            #outputs_field,
        }
    };

    // Inherent `set` / `get` over the output buffer (what the author body calls).
    let set_get_impl = quote! {
        impl #struct_ident {
            /// Write an output key into the context the constructor returns.
            #[allow(dead_code)]
            fn set(
                &self,
                key: impl ::std::convert::Into<::std::string::String>,
                value: impl ::std::convert::Into<::serde_json::Value>,
            ) {
                self.__constructor_outputs
                    .borrow_mut()
                    .insert(key.into(), value.into());
            }

            /// Read back a value previously written via `set`.
            #[allow(dead_code)]
            fn get(&self, key: &str) -> ::std::option::Option<::serde_json::Value> {
                self.__constructor_outputs.borrow().get(key).cloned()
            }
        }
    };

    // ----- The generated Config (from #[config] fields). -----
    let config_ident = format_ident!("__{}Config", struct_ident);
    let config_field_defs = config_fields.iter().map(|c| {
        let id = &c.ident;
        let ty = &c.ty;
        quote! { pub #id: #ty }
    });
    let config_struct = quote! {
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #struct_vis struct #config_ident {
            #(#config_field_defs,)*
        }
    };

    // ----- Binding code: build the author struct per call. -----
    // Config fields are cloned out of the bound Config (binds once at load).
    let config_binds = config_fields.iter().map(|c| {
        let id = &c.ident;
        quote! { #id: ::std::clone::Clone::clone(&self.cfg.#id) }
    });
    // Param fields are pulled from the task context (required/optional honored).
    let param_binds = param_fields.iter().map(|p| {
        let id = &p.ident;
        let key = id.to_string();
        if p.required {
            let ty = &p.ty;
            quote! {
                #id: match __ctx.get(#key) {
                    ::std::option::Option::Some(__v) => match ::serde_json::from_value::<#ty>(__v.clone()) {
                        ::std::result::Result::Ok(__x) => __x,
                        ::std::result::Result::Err(__e) =>
                            return #contract::TaskOutcome::err(format!("param `{}`: {}", #key, __e)),
                    },
                    ::std::option::Option::None =>
                        return #contract::TaskOutcome::err(format!("context missing required param `{}`", #key)),
                }
            }
        } else {
            let inner = &p.inner_ty;
            quote! {
                #id: match __ctx.get(#key) {
                    ::std::option::Option::Some(__v) => match ::serde_json::from_value::<#inner>(__v.clone()) {
                        ::std::result::Result::Ok(__x) => ::std::option::Option::Some(__x),
                        ::std::result::Result::Err(__e) =>
                            return #contract::TaskOutcome::err(format!("param `{}`: {}", #key, __e)),
                    },
                    ::std::option::Option::None => ::std::option::Option::None,
                }
            }
        }
    });

    // ----- The generated manifest fn (host + wasm). -----
    let slot_exprs = param_fields.iter().map(|p| {
        let key = p.ident.to_string();
        let schema = json_schema_for(&p.inner_ty);
        if p.required {
            quote! { #contract::InputSlot::required(#key, #schema) }
        } else {
            quote! { #contract::InputSlot::optional(#key, #schema, ::std::option::Option::None) }
        }
    });
    let manifest_fn = quote! {
        /// CLOACI-T-0826: the constructor manifest the macro emits. Packaging
        /// (CLOACI-T-0827) serializes this to the sidecar `constructor.json`. Always
        /// `pub` so a host packaging step / harness can read it across crates.
        #[allow(dead_code)]
        pub fn __constructor_manifest() -> #contract::ConstructorManifest {
            #contract::ConstructorManifest {
                name: #op_name.to_string(),
                version: #op_version.to_string(),
                primitive_kind: #contract::PrimitiveKind::#primitive_variant,
                interface: #interface.to_string(),
                interface_version: #interface_version,
                params: ::std::vec![ #(#slot_exprs),* ],
                dependencies: ::std::vec::Vec::new(),
                description: #description_tokens,
                author: #author_tokens,
            }
        }
    };

    // ----- The fidius guest glue (wasm-only). -----
    //
    // Emitted as top-level items rather than nested in a `const _` block: the
    // fidius `#[plugin_interface]` / `#[plugin_impl]` macros emit companion
    // items at the CRATE ROOT (e.g. `__fidius_TaskConstructor`,
    // `__FIDIUS_INSTANCE_*`) and reference them by `crate::` / `super::` path, so
    // the annotated items must live at module scope. Each item is gated with
    // `#[cfg(target_arch = "wasm32")]` placed BEFORE the fidius attribute, so on
    // the host the item (and the fidius expansion) is stripped entirely, leaving
    // only the struct + `__constructor_manifest()`.
    let configured_ident = format_ident!("__{}Configured", struct_ident);
    let guest_glue = quote! {
        // The TASK-constructor sync contract. Identical shape to the host's
        // re-declaration (`crate = "fidius_core"`) so the interface hash matches.
        // The trait tokens MUST match the host's re-declaration verbatim (bare
        // `String` / `Send` / `Sync`, the `invocation_json` arg name): the fidius
        // interface hash is derived from the signature tokens, so any divergence
        // (e.g. `::std::string::String`) fails the load-time hash gate.
        #[cfg(target_arch = "wasm32")]
        #[::fidius_macro::plugin_interface(version = 1, buffer = PluginAllocated, crate = #fidius_crate)]
        pub trait TaskConstructor: Send + Sync {
            /// `JSON(TaskInvocation)` in -> `JSON(TaskOutcome)` out. SYNC.
            fn execute(&self, invocation_json: String) -> String;
        }

        #[cfg(target_arch = "wasm32")]
        pub struct #configured_ident {
            cfg: #config_ident,
        }

        #[cfg(target_arch = "wasm32")]
        #[::fidius_macro::plugin_impl(TaskConstructor, crate = #fidius_crate, config = #config_ident)]
        impl TaskConstructor for #configured_ident {
            fn execute(&self, invocation_json: String) -> String {
                let __outcome = self.__constructor_run(&invocation_json);
                ::serde_json::to_string(&__outcome)
                    .unwrap_or_else(|e| format!(r#"{{"success":false,"error":"encode: {}"}}"#, e))
            }
        }

        #[cfg(target_arch = "wasm32")]
        impl #configured_ident {
            // The macro-emitted `configure` hook: binds #[config] once at load.
            fn configure(cfg: #config_ident) -> Self {
                Self { cfg }
            }

            fn __constructor_run(&self, invocation_json: &str) -> #contract::TaskOutcome {
                let __inv: #contract::TaskInvocation = match ::serde_json::from_str(invocation_json) {
                    ::std::result::Result::Ok(v) => v,
                    ::std::result::Result::Err(e) =>
                        return #contract::TaskOutcome::err(format!("decode invocation: {}", e)),
                };
                let mut __ctx: ::serde_json::Map<::std::string::String, ::serde_json::Value> =
                    match ::serde_json::from_str(&__inv.context_json) {
                        ::std::result::Result::Ok(::serde_json::Value::Object(m)) => m,
                        ::std::result::Result::Ok(_) =>
                            return #contract::TaskOutcome::err("context_json is not a JSON object"),
                        ::std::result::Result::Err(e) =>
                            return #contract::TaskOutcome::err(format!("decode context: {}", e)),
                    };

                // Build the author constructor: #[config] bound, #[param] pulled.
                let __op = #struct_ident {
                    #(#config_binds,)*
                    #(#param_binds,)*
                    __constructor_outputs: ::std::cell::RefCell::new(::serde_json::Map::new()),
                };

                // Run the ONLY thing the author wrote.
                match __op.execute() {
                    ::std::result::Result::Ok(()) => {
                        for (__k, __v) in __op.__constructor_outputs.into_inner() {
                            __ctx.insert(__k, __v);
                        }
                        match ::serde_json::to_string(&::serde_json::Value::Object(__ctx)) {
                            ::std::result::Result::Ok(s) => #contract::TaskOutcome::ok(s),
                            ::std::result::Result::Err(e) =>
                                #contract::TaskOutcome::err(format!("encode context: {}", e)),
                        }
                    }
                    ::std::result::Result::Err(e) =>
                        #contract::TaskOutcome::err(format!("{}", e)),
                }
            }
        }
    };

    Ok(quote! {
        #clean_struct
        #set_get_impl
        #config_struct
        #manifest_fn
        #guest_glue
    })
}

/// Parse `#[param]` / `#[param(required)]` / `#[param(optional)]`.
fn parse_param_required(attr: &syn::Attribute) -> SynResult<bool> {
    // Bare `#[param]` defaults to required.
    if matches!(attr.meta, syn::Meta::Path(_)) {
        return Ok(true);
    }
    let mut required = true;
    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("required") {
            required = true;
            Ok(())
        } else if meta.path.is_ident("optional") {
            required = false;
            Ok(())
        } else {
            Err(meta.error("expected `required` or `optional`"))
        }
    })?;
    Ok(required)
}

/// If `ty` is `Option<T>`, return `T`.
fn option_inner(ty: &Type) -> Option<Type> {
    if let Type::Path(tp) = ty {
        let seg = tp.path.segments.last()?;
        if seg.ident == "Option" {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                    return Some(inner.clone());
                }
            }
        }
    }
    None
}

/// Best-effort JSON Schema fragment for a scalar Rust type. Mirrors the spirit
/// of the CLOACI-I-0128 `InputSlot` codegen (a JSON-Schema-typed slot) while
/// staying schemars-free so the manifest fn compiles on a wasm32 guest. Unknown
/// types fall back to an unconstrained `{}` schema.
fn json_schema_for(ty: &Type) -> TokenStream2 {
    let last = if let Type::Path(tp) = ty {
        tp.path.segments.last().map(|s| s.ident.to_string())
    } else {
        None
    };
    let t = match last.as_deref() {
        Some("String" | "str") => Some("string"),
        Some("bool") => Some("boolean"),
        Some(
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128"
            | "usize",
        ) => Some("integer"),
        Some("f32" | "f64") => Some("number"),
        _ => None,
    };
    match t {
        Some(name) => quote! { ::serde_json::json!({ "type": #name }) },
        None => quote! { ::serde_json::json!({}) },
    }
}

/// `Some("x".to_string())` or `None` token for an optional manifest string.
fn opt_string(v: &Option<String>) -> TokenStream2 {
    match v {
        Some(s) => quote! { ::std::option::Option::Some(#s.to_string()) },
        None => quote! { ::std::option::Option::None },
    }
}
