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

//! Declarative syncs (COLLIERY-I-0634 decision 3, CLOACINA-T-0993): a sync
//! side that names a connector definition instead of a provider.
//!
//! A side sets `source_definition` (or `destination_definition`) to a
//! definition's name. Its `*_config` is then the user's config for the
//! definition, its `*_secret` the secret (or local binding) its auth reads,
//! and the stream's name picks the definition's stream (or a destination's
//! object). [`lower_source`] compiles all that into an ordinary
//! [`Endpoint`] on the REST runtime: the `rest` or `rest-dest` provider, the
//! compiled config, the credential the host adds, and grants to the
//! definition's host when the side declares none. It runs wherever the params
//! are read: in each Sync task on the agent, and on the server when a sync
//! definition is created, so a bad definition or a missing config key is
//! refused before it is scheduled.
//!
//! The definitions are the embedded corpus, plus the `*.yaml` of the
//! directory named by `CLOACINA_SYNC_DEFINITIONS_DIR`, if set. A server and
//! its agents must see the same directory.

use std::sync::OnceLock;

use cloacina_constructor_contract::sync::{MappingSpec, StreamSchema};
use serde_json::{Map, Value, json};

use crate::corpus::Definitions;
use crate::credential::CredentialSpec;
use crate::definition::{compile_destination, compile_source};
use crate::params::Endpoint;

/// The directory of extra connector definitions.
pub const DEFINITIONS_DIR_ENV: &str = "CLOACINA_SYNC_DEFINITIONS_DIR";
/// The REST runtime's provider packages.
pub const REST_SOURCE_PROVIDER: &str = "cloacina-connector-rest";
pub const REST_DESTINATION_PROVIDER: &str = "cloacina-connector-rest-dest";
/// The secret binding a side's auth reads when the side names none.
pub const SOURCE_SECRET_BINDING: &str = "source_secret";
pub const DESTINATION_SECRET_BINDING: &str = "destination_secret";

static DEFINITIONS: OnceLock<Result<Definitions, String>> = OnceLock::new();

/// The process's definitions: the corpus, and the directory if one is set.
/// Loaded once; an error is kept and returned to every caller.
pub fn definitions() -> Result<&'static Definitions, String> {
    DEFINITIONS
        .get_or_init(|| {
            let corpus = Definitions::corpus().map_err(|e| e.to_string())?;
            match std::env::var(DEFINITIONS_DIR_ENV) {
                Ok(dir) if !dir.is_empty() => corpus
                    .with_dir(std::path::Path::new(&dir))
                    .map_err(|e| e.to_string()),
                _ => Ok(corpus),
            }
        })
        .as_ref()
        .map_err(Clone::clone)
}

/// A source side, lowered: the endpoint, and the definition's mapping and
/// schema for the stream.
#[derive(Debug, Clone)]
pub struct LoweredSource {
    pub endpoint: Endpoint,
    pub mapping: MappingSpec,
    pub schema: StreamSchema,
}

/// Lower a source side that names definition `name`, for `stream`.
/// `grants` and `credential` are the side's own, when given; they win.
pub fn lower_source(
    name: &str,
    stream: &str,
    user: &Map<String, Value>,
    secret: &str,
    grants: Option<Value>,
    credential: Option<CredentialSpec>,
) -> Result<LoweredSource, String> {
    let defs = definitions()?;
    let def = defs.source(name).map_err(|e| e.to_string())?;
    let c = compile_source(def, stream, user, Some(secret)).map_err(|e| e.to_string())?;
    Ok(LoweredSource {
        endpoint: Endpoint {
            provider: REST_SOURCE_PROVIDER.to_string(),
            member: "source".to_string(),
            grants: Some(grants.unwrap_or_else(|| host_grant(&c.config.base_url))),
            credential: credential.or(c.credential),
            config: serde_json::to_string(&c.config).map_err(|e| e.to_string())?,
        },
        mapping: c.mapping,
        schema: c.schema,
    })
}

/// Lower a destination side that names definition `name`; the stream's
/// name picks the object.
pub fn lower_destination(
    name: &str,
    object: &str,
    user: &Map<String, Value>,
    secret: &str,
    grants: Option<Value>,
    credential: Option<CredentialSpec>,
) -> Result<Endpoint, String> {
    let defs = definitions()?;
    let def = defs.destination(name).map_err(|e| e.to_string())?;
    let c = compile_destination(def, object, user, Some(secret)).map_err(|e| e.to_string())?;
    Ok(Endpoint {
        provider: REST_DESTINATION_PROVIDER.to_string(),
        member: "destination".to_string(),
        grants: Some(grants.unwrap_or_else(|| host_grant(&c.config.base_url))),
        credential: credential.or(c.credential),
        config: serde_json::to_string(&c.config).map_err(|e| e.to_string())?,
    })
}

/// An http grant to the host (and port) of `base_url`.
fn host_grant(base_url: &str) -> Value {
    let rest = base_url.split_once("://").map_or(base_url, |(_, r)| r);
    let authority = rest.split(['/', '?']).next().unwrap_or(rest);
    json!({ "http": [authority] })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grant_is_the_base_urls_host() {
        assert_eq!(
            host_grant("https://api.github.com"),
            json!({"http": ["api.github.com"]})
        );
        assert_eq!(
            host_grant("http://127.0.0.1:8080/api/v2?x=1"),
            json!({"http": ["127.0.0.1:8080"]})
        );
    }

    #[test]
    fn a_corpus_source_lowers_onto_the_rest_runtime() {
        let user = json!({"base_url": "http://127.0.0.1:9"})
            .as_object()
            .unwrap()
            .clone();
        let l = lower_source("github", "repositories", &user, "source_secret", None, None).unwrap();
        assert_eq!(l.endpoint.provider, REST_SOURCE_PROVIDER);
        assert_eq!(l.endpoint.grants, Some(json!({"http": ["127.0.0.1:9"]})));
        assert!(matches!(
            l.endpoint.credential,
            Some(CredentialSpec::Header { ref secret, ref field, .. }) if secret == "source_secret" && field == "api_key"
        ));
        assert!(l.endpoint.config.contains("\"/user/repos\""));
        assert!(!l.schema.fields.is_empty());
        let err = lower_source("nope", "x", &Map::new(), "s", None, None).unwrap_err();
        assert!(err.contains("nope"), "{err}");
        let err = lower_source("github", "nope", &Map::new(), "s", None, None).unwrap_err();
        assert!(err.contains("repositories"), "{err}");
    }

    #[test]
    fn a_corpus_destination_lowers_onto_the_rest_runtime() {
        let user = json!({"instance": "acme"}).as_object().unwrap().clone();
        let e = lower_destination("salesforce-dest", "contacts", &user, "sf", None, None).unwrap();
        assert_eq!(e.provider, REST_DESTINATION_PROVIDER);
        assert_eq!(e.grants, Some(json!({"http": ["acme.my.salesforce.com"]})));
        assert!(matches!(e.credential, Some(CredentialSpec::OAuth2 { .. })));
        let err = lower_destination("salesforce-dest", "contacts", &Map::new(), "sf", None, None)
            .unwrap_err();
        assert!(err.contains("instance"), "{err}");
    }
}
