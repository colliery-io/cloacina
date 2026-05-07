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

//! `#[trigger]` attribute macro for defining event-driven triggers.
//!
//! Applied to a standalone async function, generates a struct implementing
//! the `Trigger` trait. The `on` parameter binds it to a workflow by name.
//!
//! Two modes:
//! - **Custom poll**: function body with `poll_interval` — user writes poll logic
//! - **Cron**: `cron` parameter, no function body — framework provides poll logic (T-0305)

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    Ident, ItemFn, LitStr, Result as SynResult, Token,
};

use crate::tasks::to_pascal_case;

/// Attributes for the `#[trigger]` macro.
pub struct TriggerAttributes {
    pub on: String,
    pub poll_interval: Option<String>,
    pub cron: Option<String>,
    pub timezone: Option<String>,
    pub allow_concurrent: bool,
    pub name: Option<String>,
}

impl Parse for TriggerAttributes {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut on = None;
        let mut poll_interval = None;
        let mut cron = None;
        let mut timezone = None;
        let mut allow_concurrent = false;
        let mut name = None;

        while !input.is_empty() {
            let field_name: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match field_name.to_string().as_str() {
                "on" => {
                    let lit: LitStr = input.parse()?;
                    on = Some(lit.value());
                }
                "poll_interval" => {
                    let lit: LitStr = input.parse()?;
                    poll_interval = Some(lit.value());
                }
                "cron" => {
                    let lit: LitStr = input.parse()?;
                    cron = Some(lit.value());
                }
                "timezone" => {
                    let lit: LitStr = input.parse()?;
                    timezone = Some(lit.value());
                }
                "allow_concurrent" => {
                    let lit: syn::LitBool = input.parse()?;
                    allow_concurrent = lit.value();
                }
                "name" => {
                    let lit: LitStr = input.parse()?;
                    name = Some(lit.value());
                }
                _ => {
                    return Err(syn::Error::new(
                        field_name.span(),
                        format!(
                            "Unknown attribute: '{}'. Valid: on, poll_interval, cron, timezone, allow_concurrent, name",
                            field_name
                        ),
                    ));
                }
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        let on = on.ok_or_else(|| {
            syn::Error::new(Span::call_site(), "#[trigger] requires 'on' attribute")
        })?;

        if poll_interval.is_none() && cron.is_none() {
            return Err(syn::Error::new(
                Span::call_site(),
                "#[trigger] requires either 'poll_interval' or 'cron' attribute",
            ));
        }

        if poll_interval.is_some() && cron.is_some() {
            return Err(syn::Error::new(
                Span::call_site(),
                "#[trigger] cannot have both 'poll_interval' and 'cron' — pick one",
            ));
        }

        Ok(TriggerAttributes {
            on,
            poll_interval,
            cron,
            timezone,
            allow_concurrent,
            name,
        })
    }
}

/// Entry point for the `#[trigger]` attribute macro.
pub fn trigger_attr(args: TokenStream, input: TokenStream) -> TokenStream {
    let attrs = match syn::parse::<TriggerAttributes>(args) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error().into(),
    };

    if attrs.cron.is_some() {
        // Cron mode — applied to a function (body ignored, name used as trigger name)
        let input_fn = match syn::parse::<ItemFn>(input) {
            Ok(f) => f,
            Err(e) => {
                return syn::Error::new(
                    Span::call_site(),
                    format!("#[trigger] with cron must be applied to a function: {}", e),
                )
                .to_compile_error()
                .into();
            }
        };
        return generate_cron_trigger(attrs, input_fn).into();
    }

    let input_fn = match syn::parse::<ItemFn>(input) {
        Ok(f) => f,
        Err(e) => {
            return syn::Error::new(
                Span::call_site(),
                format!(
                    "#[trigger] with poll_interval must be applied to an async function: {}",
                    e
                ),
            )
            .to_compile_error()
            .into();
        }
    };

    generate_custom_trigger(attrs, input_fn).into()
}

/// Parse a duration string like "100ms", "5s", "2m", "1h" into milliseconds.
fn parse_duration_ms(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if let Some(ms) = s.strip_suffix("ms") {
        ms.parse::<u64>()
            .map_err(|_| format!("Invalid milliseconds: {}", ms))
    } else if let Some(secs) = s.strip_suffix('s') {
        secs.parse::<u64>()
            .map(|v| v * 1000)
            .map_err(|_| format!("Invalid seconds: {}", secs))
    } else if let Some(mins) = s.strip_suffix('m') {
        mins.parse::<u64>()
            .map(|v| v * 60 * 1000)
            .map_err(|_| format!("Invalid minutes: {}", mins))
    } else if let Some(hrs) = s.strip_suffix('h') {
        hrs.parse::<u64>()
            .map(|v| v * 3600 * 1000)
            .map_err(|_| format!("Invalid hours: {}", hrs))
    } else {
        Err(format!(
            "Invalid duration format: '{}'. Use: 100ms, 5s, 2m, 1h",
            s
        ))
    }
}

/// Generate a custom poll trigger (function body provides poll logic).
fn generate_custom_trigger(attrs: TriggerAttributes, input_fn: ItemFn) -> TokenStream2 {
    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
    let fn_block = &input_fn.block;

    let trigger_name = attrs.name.unwrap_or_else(|| fn_name.to_string());
    let _workflow_name = &attrs.on;
    let allow_concurrent = attrs.allow_concurrent;

    let poll_interval_str = attrs.poll_interval.as_deref().unwrap_or("30s");
    let poll_interval_ms = match parse_duration_ms(poll_interval_str) {
        Ok(ms) => ms,
        Err(e) => {
            return syn::Error::new(Span::call_site(), e).to_compile_error();
        }
    };

    // Generate struct name from function name
    let struct_name = syn::Ident::new(
        &format!("{}Trigger", to_pascal_case(&fn_name.to_string())),
        fn_name.span(),
    );

    // Trigger emission targets cdylib-reachable paths.
    // `cloacina_workflow::Trigger` is the (relocated) trait; its `poll()`
    // returns leaf-crate `TriggerResult` / `TriggerError`.
    let embedded_code = quote! {
        #[derive(Debug, Clone)]
        struct #struct_name;

        #[async_trait::async_trait]
        impl cloacina_workflow::Trigger for #struct_name {
            fn name(&self) -> &str {
                #trigger_name
            }

            fn poll_interval(&self) -> std::time::Duration {
                std::time::Duration::from_millis(#poll_interval_ms)
            }

            fn allow_concurrent(&self) -> bool {
                #allow_concurrent
            }

            async fn poll(&self) -> Result<cloacina_workflow::TriggerResult, cloacina_workflow::TriggerError> {
                _trigger_poll_impl().await
            }
        }

        async fn _trigger_poll_impl() -> Result<cloacina_workflow::TriggerResult, cloacina_workflow::TriggerError> {
            #fn_block
        }

        // T-0552: TriggerEntry inventory submission emits in BOTH
        // packaged and embedded modes so the unified `cloacina::package!()`
        // shell can walk it from packaged cdylibs. Cfg-gated paths so the
        // submission resolves through the right crate per build mode (library
        // mode uses `cloacina::cloacina_workflow_plugin::*`; packaged mode
        // uses `cloacina_workflow_plugin::*` direct).
        #[cfg(not(feature = "packaged"))]
        ::cloacina::cloacina_workflow_plugin::inventory::submit! {
            ::cloacina::cloacina_workflow_plugin::TriggerEntry {
                name: #trigger_name,
                constructor: || std::sync::Arc::new(#struct_name) as std::sync::Arc<dyn cloacina_workflow::Trigger>,
            }
        }
        #[cfg(feature = "packaged")]
        ::cloacina_workflow_plugin::inventory::submit! {
            ::cloacina_workflow_plugin::TriggerEntry {
                name: #trigger_name,
                constructor: || std::sync::Arc::new(#struct_name) as std::sync::Arc<dyn cloacina_workflow::Trigger>,
            }
        }
    };

    quote! {
        // Preserve original function for direct calls/testing
        #fn_vis async fn #fn_name() -> Result<cloacina_workflow::TriggerResult, cloacina_workflow::TriggerError>
            #fn_block

        // T-0552: emission un-gated — fires in both packaged and embedded
        // modes. The unified shell macro consumes inventory in packaged
        // builds; the engine's Runtime::new() seeds from inventory in
        // embedded builds.
        const _: () = {
            #embedded_code
        };
    }
}

/// Generate a cron trigger (schedule expression provides the poll logic).
fn generate_cron_trigger(attrs: TriggerAttributes, input_fn: ItemFn) -> TokenStream2 {
    let fn_name = &input_fn.sig.ident;

    let trigger_name = attrs.name.unwrap_or_else(|| fn_name.to_string());
    let _workflow_name = &attrs.on;
    let cron_expression = attrs.cron.as_deref().unwrap();
    let timezone = attrs.timezone.as_deref().unwrap_or("UTC");
    let allow_concurrent = attrs.allow_concurrent;

    // Compile-time validation of the cron expression
    // We parse it here to give a compile error if invalid
    if let Err(e) = validate_cron_expression(cron_expression) {
        return syn::Error::new(Span::call_site(), e).to_compile_error();
    }

    let struct_name = syn::Ident::new(
        &format!("{}CronTrigger", to_pascal_case(&fn_name.to_string())),
        fn_name.span(),
    );

    // Poll interval for cron: check every 30 seconds
    let cron_poll_ms: u64 = 30_000;

    let embedded_code = quote! {
        #[derive(Debug, Clone)]
        struct #struct_name {
            evaluator: cloacina_workflow::cron_evaluator::CronEvaluator,
            last_fire: std::sync::Arc<std::sync::Mutex<Option<chrono::DateTime<chrono::Utc>>>>,
        }

        impl #struct_name {
            fn new() -> Self {
                let evaluator = cloacina_workflow::cron_evaluator::CronEvaluator::new(
                    #cron_expression,
                    #timezone,
                ).expect("Invalid cron expression — this should have been caught at compile time");

                Self {
                    evaluator,
                    last_fire: std::sync::Arc::new(std::sync::Mutex::new(None)),
                }
            }
        }

        #[async_trait::async_trait]
        impl cloacina_workflow::Trigger for #struct_name {
            fn name(&self) -> &str {
                #trigger_name
            }

            fn poll_interval(&self) -> std::time::Duration {
                std::time::Duration::from_millis(#cron_poll_ms)
            }

            fn allow_concurrent(&self) -> bool {
                #allow_concurrent
            }

            fn cron_expression(&self) -> Option<String> {
                Some(#cron_expression.to_string())
            }

            async fn poll(&self) -> Result<cloacina_workflow::TriggerResult, cloacina_workflow::TriggerError> {
                let now = chrono::Utc::now();
                let mut last_fire = self.last_fire.lock().unwrap();

                let check_from = last_fire.unwrap_or(now - chrono::Duration::seconds(1));
                match self.evaluator.next_execution(check_from) {
                    Ok(next_run) => {
                        if next_run <= now {
                            *last_fire = Some(now);
                            Ok(cloacina_workflow::TriggerResult::Fire(None))
                        } else {
                            Ok(cloacina_workflow::TriggerResult::Skip)
                        }
                    }
                    Err(e) => {
                        Err(cloacina_workflow::TriggerError::PollError {
                            message: format!("Cron evaluation error: {}", e),
                        })
                    }
                }
            }
        }

        // Cfg-gated path so submission resolves correctly in both library
        // mode (via cloacina re-export) and packaged mode (direct dep).
        #[cfg(not(feature = "packaged"))]
        ::cloacina::cloacina_workflow_plugin::inventory::submit! {
            ::cloacina::cloacina_workflow_plugin::TriggerEntry {
                name: #trigger_name,
                constructor: || std::sync::Arc::new(#struct_name::new()) as std::sync::Arc<dyn cloacina_workflow::Trigger>,
            }
        }
        #[cfg(feature = "packaged")]
        ::cloacina_workflow_plugin::inventory::submit! {
            ::cloacina_workflow_plugin::TriggerEntry {
                name: #trigger_name,
                constructor: || std::sync::Arc::new(#struct_name::new()) as std::sync::Arc<dyn cloacina_workflow::Trigger>,
            }
        }
    };

    quote! {
        // T-0552: emission un-gated for both packaged and embedded modes.
        const _: () = {
            #embedded_code
        };
    }
}

/// Validate a cron expression at compile time.
fn validate_cron_expression(expr: &str) -> Result<(), String> {
    // Basic validation — check field count and characters
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() < 5 || fields.len() > 7 {
        return Err(format!(
            "Invalid cron expression '{}': expected 5-7 fields (minute hour day month weekday [year] [seconds]), got {}",
            expr, fields.len()
        ));
    }

    // Validate each field contains only valid cron characters
    let valid_chars = |c: char| c.is_ascii_digit() || ",-*/".contains(c);
    for (i, field) in fields.iter().enumerate() {
        if !field.chars().all(valid_chars) {
            return Err(format!(
                "Invalid cron expression '{}': field {} ('{}') contains invalid characters",
                expr,
                i + 1,
                field
            ));
        }
    }

    Ok(())
}
