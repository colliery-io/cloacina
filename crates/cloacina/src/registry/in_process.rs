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

//! What of a package would run tenant code in the process that loads it
//! (CLOACINA-T-1078).
//!
//! A server loads a package from its metadata document and runs no tenant
//! code (CLOACINA-A-0019). Most of the reactive layer of a package is data:
//! a reactor is a name, accumulators and a mode, and an accumulator is host
//! code picked by its type. What is tenant code, and so refused on a server:
//!
//! - a poll trigger: its code runs at each poll;
//! - a stream accumulator whose provider is native: the provider is a library
//!   opened in the process that runs the accumulator;
//! - a Python polling accumulator: its poll function runs at each interval;
//! - a Python stream accumulator: its source runs in the process that runs
//!   the accumulator;
//! - a polling accumulator of a Rust package: it has no poll code in a
//!   package, so it would never emit.
//!
//! A native provider that only a constructor-node task uses is not refused:
//! the task runs on the tenant's agents.
//!
//! The compiler records the list in the document (format 2,
//! [`PackageDescription::in_process`]); the server refuses a package whose
//! list is not empty, and computes the list again from the document, so a
//! document with a missing list is not trusted for it.

use serde::{Deserialize, Serialize};

use cloacina_workflow_plugin::types::{AccumulatorConfig, AccumulatorDeclarationEntry};

use crate::registry::package_description::PackageDescription;

/// A poll trigger.
pub const POLL_TRIGGER: &str = "poll_trigger";
/// A native provider named by a stream accumulator.
pub const NATIVE_STREAM_PROVIDER: &str = "native_stream_provider";
/// A polling accumulator of a Python package.
pub const PYTHON_POLLING_ACCUMULATOR: &str = "python_polling_accumulator";
/// A stream accumulator of a Python package.
pub const PYTHON_STREAM_ACCUMULATOR: &str = "python_stream_accumulator";
/// A polling accumulator of a Rust package.
pub const RUST_POLLING_ACCUMULATOR: &str = "rust_polling_accumulator";

/// One kind of code that would run in the process that loads the package,
/// with the names of what the package uses of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InProcessReason {
    /// One of the kinds of this module (`poll_trigger`, ...).
    pub kind: String,
    /// The triggers, providers or accumulators, sorted.
    pub names: Vec<String>,
}

impl InProcessReason {
    /// The reason as a publisher reads it.
    pub fn text(&self) -> String {
        let names = self
            .names
            .iter()
            .map(|n| format!("'{n}'"))
            .collect::<Vec<_>>()
            .join(", ");
        match self.kind.as_str() {
            POLL_TRIGGER => {
                format!("it declares the poll trigger(s) {names}, whose code runs at each poll")
            }
            NATIVE_STREAM_PROVIDER => format!(
                "its stream accumulator(s) use the native provider(s) {names}, a library opened \
                 in the process that runs the accumulator"
            ),
            PYTHON_POLLING_ACCUMULATOR => format!(
                "it holds the Python polling accumulator(s) {names}, whose poll function runs in \
                 the process that runs the accumulator"
            ),
            PYTHON_STREAM_ACCUMULATOR => format!(
                "it holds the Python stream accumulator(s) {names}, whose source runs in the \
                 process that runs the accumulator"
            ),
            RUST_POLLING_ACCUMULATOR => format!(
                "it declares the polling accumulator(s) {names}, which have no poll code in a \
                 Rust package and would never emit"
            ),
            other => format!("it uses {other} {names}"),
        }
    }
}

/// A bundled provider of a package, with its runtime (format 2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderDescription {
    /// The provider's name, as `provider.json` gives it.
    pub name: String,
    /// `"native"` or `"wasm"`.
    pub runtime: String,
}

/// The accumulators of the package's reactors and graph, each once, with the
/// manifest's `[[metadata.accumulators]]` entry applied by name.
fn effective_accumulators(
    document: &PackageDescription,
    overrides: &[AccumulatorConfig],
) -> Vec<AccumulatorDeclarationEntry> {
    let mut out: Vec<AccumulatorDeclarationEntry> = Vec::new();
    let declared = document
        .reactors
        .iter()
        .flat_map(|r| r.accumulators.iter())
        .chain(document.graph.iter().flat_map(|g| g.accumulators.iter()));
    for acc in declared {
        if out.iter().any(|a| a.name == acc.name) {
            continue;
        }
        let mut acc = acc.clone();
        if let Some(cfg) = overrides.iter().find(|cfg| cfg.name == acc.name) {
            acc.accumulator_type = cfg.accumulator_type.clone();
            acc.config = cfg.config.clone();
        }
        out.push(acc);
    }
    out
}

/// What of `document` would run tenant code in the process that loads it,
/// computed from the document. `overrides` are the manifest's
/// `[[metadata.accumulators]]` entries. Empty: nothing.
pub fn in_process_reasons(
    document: &PackageDescription,
    overrides: &[AccumulatorConfig],
) -> Vec<InProcessReason> {
    let mut reasons = Vec::new();
    let mut push = |kind: &str, mut names: Vec<String>| {
        names.sort();
        names.dedup();
        if !names.is_empty() {
            reasons.push(InProcessReason {
                kind: kind.to_string(),
                names,
            });
        }
    };

    push(
        POLL_TRIGGER,
        document
            .triggers
            .iter()
            .filter(|t| t.cron_expression.is_none())
            .map(|t| t.name.clone())
            .collect(),
    );

    let accumulators = effective_accumulators(document, overrides);
    let of_type = |kind: &str| -> Vec<String> {
        accumulators
            .iter()
            .filter(|a| a.accumulator_type == kind)
            .map(|a| a.name.clone())
            .collect()
    };

    let native: Vec<String> = accumulators
        .iter()
        .filter(|a| a.accumulator_type == "stream")
        .filter_map(|a| a.config.get("provider"))
        .map(|from| provider_name(from).to_string())
        .filter(|name| {
            document
                .providers
                .iter()
                .any(|p| &p.name == name && p.runtime == "native")
        })
        .collect();
    push(NATIVE_STREAM_PROVIDER, native);

    if document.language == "python" {
        push(PYTHON_POLLING_ACCUMULATOR, of_type("polling"));
        push(PYTHON_STREAM_ACCUMULATOR, of_type("stream"));
    } else {
        push(RUST_POLLING_ACCUMULATOR, of_type("polling"));
    }
    reasons
}

/// The reasons a server refuses `document` for: those the compiler recorded
/// and those computed from the document again, merged by kind. A format 1
/// document has no record; its caller keeps the rule of format 1.
pub fn refused_on_server(
    document: &PackageDescription,
    overrides: &[AccumulatorConfig],
) -> Vec<InProcessReason> {
    let mut merged = in_process_reasons(document, overrides);
    for recorded in &document.in_process {
        match merged.iter_mut().find(|r| r.kind == recorded.kind) {
            Some(reason) => {
                reason.names.extend(recorded.names.iter().cloned());
                reason.names.sort();
                reason.names.dedup();
            }
            None => merged.push(recorded.clone()),
        }
    }
    merged
}

/// Strip an optional `@version` from a `name[@version]` provider reference.
fn provider_name(from: &str) -> &str {
    match from.split_once('@') {
        Some((name, _)) => name,
        None => from,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloacina_workflow_plugin::{
        GraphPackageMetadata, ReactorPackageMetadata, TriggerPackageMetadata,
    };
    use std::collections::HashMap;

    fn document(language: &str) -> PackageDescription {
        serde_json::from_value(serde_json::json!({
            "format_version": crate::registry::PACKAGE_DESCRIPTION_VERSION,
            "language": language,
            "described_by": "test",
            "package_name": "pkg",
        }))
        .unwrap()
    }

    fn acc(name: &str, kind: &str, config: &[(&str, &str)]) -> AccumulatorDeclarationEntry {
        AccumulatorDeclarationEntry {
            name: name.to_string(),
            accumulator_type: kind.to_string(),
            config: config
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    fn reactor(accumulators: Vec<AccumulatorDeclarationEntry>) -> ReactorPackageMetadata {
        ReactorPackageMetadata {
            name: "rx".to_string(),
            package_name: "pkg".to_string(),
            reaction_mode: "when_any".to_string(),
            accumulators,
        }
    }

    fn provider(name: &str, runtime: &str) -> ProviderDescription {
        ProviderDescription {
            name: name.to_string(),
            runtime: runtime.to_string(),
        }
    }

    /// A native provider refuses the package only when a stream accumulator
    /// names it; one used by a constructor node alone does not.
    #[test]
    fn a_native_provider_refuses_only_through_a_stream_accumulator() {
        let mut doc = document("rust");
        doc.providers = vec![
            provider("cloacina-provider-kafka", "native"),
            provider("cloacina-provider-fs", "native"),
            provider("cloacina-provider-http", "wasm"),
        ];
        assert!(in_process_reasons(&doc, &[]).is_empty());

        doc.reactors = vec![reactor(vec![
            acc(
                "orders",
                "stream",
                &[("provider", "cloacina-provider-kafka@0.2")],
            ),
            acc("feed", "stream", &[("provider", "cloacina-provider-http")]),
            acc("window", "state", &[]),
        ])];
        let reasons = in_process_reasons(&doc, &[]);
        assert_eq!(
            reasons,
            vec![InProcessReason {
                kind: NATIVE_STREAM_PROVIDER.to_string(),
                names: vec!["cloacina-provider-kafka".to_string()],
            }]
        );
        assert!(reasons[0].text().contains("'cloacina-provider-kafka'"));
    }

    /// A manifest entry changes an accumulator's type before the check.
    #[test]
    fn a_manifest_entry_is_applied_by_name() {
        let mut doc = document("rust");
        doc.providers = vec![provider("cloacina-provider-kafka", "native")];
        doc.graph = Some(GraphPackageMetadata {
            graph_name: "g".to_string(),
            package_name: "pkg".to_string(),
            reaction_mode: "when_any".to_string(),
            input_strategy: "latest".to_string(),
            accumulators: vec![acc("orders", "passthrough", &[])],
            trigger_reactor: None,
            graph_data_json: None,
        });
        let overrides = vec![AccumulatorConfig {
            name: "orders".to_string(),
            accumulator_type: "stream".to_string(),
            config: HashMap::from([(
                "provider".to_string(),
                "cloacina-provider-kafka".to_string(),
            )]),
        }];
        assert!(in_process_reasons(&doc, &[]).is_empty());
        assert_eq!(
            in_process_reasons(&doc, &overrides)[0].kind,
            NATIVE_STREAM_PROVIDER
        );
    }

    /// A Rust polling accumulator is refused: it has no code in a package.
    #[test]
    fn a_rust_polling_accumulator_is_refused() {
        let mut doc = document("rust");
        doc.reactors = vec![reactor(vec![
            acc("prices", "polling", &[("interval", "5s")]),
            acc("alpha", "passthrough", &[]),
        ])];
        let reasons = in_process_reasons(&doc, &[]);
        assert_eq!(reasons.len(), 1);
        assert_eq!(reasons[0].kind, RUST_POLLING_ACCUMULATOR);
        assert_eq!(reasons[0].names, vec!["prices"]);
        assert!(reasons[0].text().contains("no poll code"));
    }

    /// A Python package with passthrough, batch and state accumulators has
    /// nothing in process; a polling or stream one is named.
    #[test]
    fn python_polling_and_stream_accumulators_are_refused_and_the_rest_load() {
        let mut doc = document("python");
        doc.reactors = vec![reactor(vec![
            acc("a", "passthrough", &[]),
            acc("b", "batch", &[]),
            acc("c", "state", &[("capacity", "5")]),
        ])];
        assert!(in_process_reasons(&doc, &[]).is_empty());

        doc.reactors[0]
            .accumulators
            .push(acc("ticks", "polling", &[("interval", "2s")]));
        doc.reactors[0]
            .accumulators
            .push(acc("feed", "stream", &[]));
        let kinds: Vec<String> = in_process_reasons(&doc, &[])
            .into_iter()
            .map(|r| r.kind)
            .collect();
        assert_eq!(
            kinds,
            vec![PYTHON_POLLING_ACCUMULATOR, PYTHON_STREAM_ACCUMULATOR]
        );
    }

    /// A poll trigger is named, as before.
    #[test]
    fn a_poll_trigger_is_named() {
        let mut doc = document("python");
        doc.triggers = vec![TriggerPackageMetadata {
            name: "watch".to_string(),
            workflow_name: "wf".to_string(),
            package_name: "pkg".to_string(),
            poll_interval: "60s".to_string(),
            cron_expression: None,
            allow_concurrent: false,
            timezone: None,
            catchup_policy: None,
            max_active_runs: None,
        }];
        let reasons = in_process_reasons(&doc, &[]);
        assert_eq!(reasons[0].kind, POLL_TRIGGER);
        assert!(reasons[0].text().contains("poll trigger(s) 'watch'"));
    }

    /// The server merges what the compiler recorded with what it computes.
    #[test]
    fn the_server_merges_the_record_and_its_own_reading() {
        let mut doc = document("rust");
        doc.reactors = vec![reactor(vec![acc("prices", "polling", &[])])];
        doc.in_process = vec![
            InProcessReason {
                kind: RUST_POLLING_ACCUMULATOR.to_string(),
                names: vec!["other".to_string()],
            },
            InProcessReason {
                kind: NATIVE_STREAM_PROVIDER.to_string(),
                names: vec!["p".to_string()],
            },
        ];
        let merged = refused_on_server(&doc, &[]);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].names, vec!["other", "prices"]);
        assert_eq!(merged[1].kind, NATIVE_STREAM_PROVIDER);
    }
}
