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

//! The catalog (COLLIERY-I-0634 / CLOACINA-T-0993): the connectors a process
//! has, compiled and declarative alike.
//!
//! A **compiled** connector is a provider package in the provider search path
//! whose members implement the `connector` interface. A **declarative**
//! connector is a connector definition (the corpus, and the definitions
//! directory), run by the REST runtime's provider. A sync side names a
//! compiled one by `*_provider` and a declarative one by `*_definition`.

use std::path::Path;

use cloacina::registry::loader::provider_search_path;
use cloacina_constructor_contract::{CONNECTOR_INTERFACE, ProviderManifest};
use serde::Serialize;

use crate::declarative::{REST_DESTINATION_PROVIDER, REST_SOURCE_PROVIDER, definitions};

/// One connector of the catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CatalogEntry {
    /// The name a sync names it by: the provider package, or the definition.
    pub name: String,
    /// `compiled` or `declarative`.
    pub kind: &'static str,
    /// The provider package that runs it.
    pub provider: String,
    pub version: Option<String>,
    /// `Source`, `Destination`.
    pub roles: Vec<String>,
    pub sync_modes: Vec<String>,
    /// A definition's streams (a destination's objects); empty for a compiled
    /// connector, whose streams are what `discover` finds.
    pub streams: Vec<String>,
}

/// The catalog: the compiled connectors in the provider search path, then the
/// declarative definitions, each sorted by name.
pub fn catalog() -> Result<Vec<CatalogEntry>, String> {
    let mut out = compiled(&provider_search_path());
    let defs = definitions()?;
    for (name, def) in defs.sources() {
        let incremental = def.streams.iter().any(|s| s.incremental.is_some());
        out.push(CatalogEntry {
            name: name.to_string(),
            kind: "declarative",
            provider: REST_SOURCE_PROVIDER.to_string(),
            version: Some(def.spec.version.clone()),
            roles: vec!["Source".to_string()],
            sync_modes: if incremental {
                vec!["FullRefresh".to_string(), "Incremental".to_string()]
            } else {
                vec!["FullRefresh".to_string()]
            },
            streams: def.stream_names(),
        });
    }
    for (name, def) in defs.destinations() {
        out.push(CatalogEntry {
            name: name.to_string(),
            kind: "declarative",
            provider: REST_DESTINATION_PROVIDER.to_string(),
            version: Some(def.spec.version.clone()),
            roles: vec!["Destination".to_string()],
            sync_modes: vec!["FullRefresh".to_string(), "Incremental".to_string()],
            streams: def.objects.iter().map(|o| o.name.clone()).collect(),
        });
    }
    Ok(out)
}

/// The connector providers staged under `root`: each subdirectory with a
/// `provider.json` that has a connector member. Unreadable entries are
/// skipped; a catalog lists what can be loaded.
pub fn compiled(root: &Path) -> Vec<CatalogEntry> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut out: Vec<CatalogEntry> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| std::fs::read_to_string(e.path().join("provider.json")).ok())
        .filter_map(|text| ProviderManifest::from_json(&text).ok())
        .filter_map(|m| {
            let members: Vec<_> = m
                .constructors
                .iter()
                .filter(|c| c.interface == CONNECTOR_INTERFACE)
                .filter_map(|c| c.connector.as_ref())
                .collect();
            if members.is_empty() {
                return None;
            }
            let mut roles: Vec<String> = members
                .iter()
                .flat_map(|c| c.roles.iter().map(|r| format!("{r:?}")))
                .collect();
            roles.sort();
            roles.dedup();
            let mut modes: Vec<String> = members
                .iter()
                .flat_map(|c| c.supported_sync_modes.iter().map(|s| format!("{s:?}")))
                .collect();
            modes.sort();
            modes.dedup();
            Some(CatalogEntry {
                name: m.name.clone(),
                kind: "compiled",
                provider: m.name,
                version: Some(m.version),
                roles,
                sync_modes: modes,
                streams: Vec::new(),
            })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_declarative_part_lists_the_corpus() {
        let c = catalog().unwrap();
        let github = c.iter().find(|e| e.name == "github").expect("github");
        assert_eq!(github.kind, "declarative");
        assert_eq!(github.provider, REST_SOURCE_PROVIDER);
        assert!(github.streams.contains(&"repositories".to_string()));
        assert!(github.sync_modes.contains(&"Incremental".to_string()));
        let hubspot = c
            .iter()
            .find(|e| e.name == "hubspot-dest")
            .expect("hubspot-dest");
        assert_eq!(hubspot.roles, vec!["Destination"]);
        assert_eq!(hubspot.streams, vec!["contacts"]);
    }

    #[test]
    fn a_staged_connector_provider_is_compiled() {
        use cloacina_constructor_contract::sync::{ConnectorRole, SyncMode};
        use cloacina_constructor_contract::{
            CONNECTOR_INTERFACE_VERSION, ConfigField, ConnectorMeta, ConstructorManifest,
            PrimitiveKind, ProviderRuntime,
        };
        let dir = tempfile::tempdir().unwrap();
        let member = |name: &str, kind, role| ConstructorManifest {
            name: name.into(),
            version: "0.1.0".into(),
            primitive_kind: kind,
            interface: CONNECTOR_INTERFACE.into(),
            interface_version: CONNECTOR_INTERFACE_VERSION,
            params: vec![],
            config_fields: vec![ConfigField {
                name: "json".into(),
                ty: "String".into(),
            }],
            dependencies: vec![],
            description: None,
            author: None,
            connector: Some(ConnectorMeta {
                roles: vec![role],
                supported_sync_modes: vec![SyncMode::FullRefresh, SyncMode::Incremental],
                config_schema: "{}".into(),
            }),
        };
        let pkg = dir.path().join("cloacina-connector-pg");
        std::fs::create_dir_all(&pkg).unwrap();
        let m = ProviderManifest {
            name: "cloacina-connector-pg".into(),
            version: "0.1.0".into(),
            component: "x.wasm".into(),
            runtime: ProviderRuntime::Wasm,
            constructors: vec![
                member("source", PrimitiveKind::Source, ConnectorRole::Source),
                member(
                    "destination",
                    PrimitiveKind::Destination,
                    ConnectorRole::Destination,
                ),
            ],
        };
        std::fs::write(pkg.join("provider.json"), m.to_json().unwrap()).unwrap();
        std::fs::create_dir_all(dir.path().join("not-a-provider")).unwrap();
        let c = compiled(dir.path());
        assert_eq!(c.len(), 1, "{c:?}");
        assert_eq!(c[0].name, "cloacina-connector-pg");
        assert_eq!(c[0].kind, "compiled");
        assert_eq!(c[0].roles, vec!["Destination", "Source"]);
        assert_eq!(c[0].sync_modes, vec!["FullRefresh", "Incremental"]);
    }
}
