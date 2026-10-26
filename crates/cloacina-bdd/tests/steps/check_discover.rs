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
//! Check and discover steps: one side of a sync, probed where connectors run.

use cloacina_api_types::ConnectorProbeRequest;
use cloacina_testing::connectors::rest_mock::Endpoint;
use cucumber::gherkin::Step;
use cucumber::{given, then, when};
use serde_json::{Value, json};

use crate::steps::credentials::records;
use crate::support::providers;
use crate::support::world::SyncWorld;

/// The path of the API the check reads.
const ITEMS: &str = "/items";

#[given(expr = "an API that serves the records at {string} to the bearer token {string}")]
async fn api_wants(w: &mut SyncWorld, step: &Step, path: String, token: String) {
    let records = records(step);
    w.api().serve(
        &path,
        Endpoint {
            records,
            require_header: Some(("Authorization".into(), format!("Bearer {token}"))),
            ..Default::default()
        },
    );
}

#[when(expr = "the API source is checked with the secret {string} as a bearer token")]
async fn check_api(w: &mut SyncWorld, secret: String) {
    let api = w.api();
    let request = ConnectorProbeRequest {
        side: "source".into(),
        provider: Some(providers::REST.into()),
        member: Some("source".into()),
        config: Some(json!({ "base_url": api.base_url(), "path": ITEMS, "max_retries": 0 })),
        grants: Some(json!({ "http": [api.grant()] })),
        credential: Some(json!({
            "kind": "header", "secret": secret, "name": "Authorization",
            "field": "token", "prefix": "Bearer "
        })),
        ..Default::default()
    };
    w.probed = Some(w.probe("check", &request).await);
}

#[when(expr = "the source definition {string} is discovered")]
async fn discover_definition(w: &mut SyncWorld, name: String) {
    let request = ConnectorProbeRequest {
        side: "source".into(),
        definition: Some(name),
        ..Default::default()
    };
    w.probed = Some(w.probe("discover", &request).await);
}

#[when("the Postgres source is discovered")]
async fn discover_postgres(w: &mut SyncWorld) {
    let source = w.source.clone().expect("a Postgres source");
    let request = ConnectorProbeRequest {
        side: "source".into(),
        provider: Some(source.provider),
        member: Some("source".into()),
        config: Some(source.config),
        grants: source.grants,
        ..Default::default()
    };
    w.probed = Some(w.probe("discover", &request).await);
}

fn answer(w: &SyncWorld) -> &Value {
    match w.probed.as_ref().expect("a probe ran") {
        Ok(v) => v,
        Err(e) => panic!("the probe could not run: {e}"),
    }
}

#[then("the check passes")]
async fn check_passes(w: &mut SyncWorld) {
    let a = answer(w);
    assert_eq!(a["ok"], true, "{a}");
}

#[then(expr = "the check fails, saying {string}")]
async fn check_fails(w: &mut SyncWorld, words: String) {
    let a = answer(w);
    assert_eq!(a["ok"], false, "{a}");
    let message = a["message"].as_str().unwrap_or_default();
    assert!(message.contains(&words), "the check says `{words}`: {a}");
}

#[then(expr = "the probe cannot run, naming {string}")]
async fn probe_fails(w: &mut SyncWorld, name: String) {
    match w.probed.as_ref().expect("a probe ran") {
        Err(e) => assert!(e.contains(&name), "the failure names `{name}`: {e}"),
        Ok(v) => panic!("the probe answered {v}"),
    }
}

fn stream<'a>(a: &'a Value, name: &str) -> &'a Value {
    a["streams"]
        .as_array()
        .and_then(|s| s.iter().find(|s| s["name"] == name))
        .unwrap_or_else(|| panic!("no stream `{name}` in {a}"))
}

#[then(
    expr = "the stream {string} is offered incrementally by {string}, with the field {string} as an integer"
)]
async fn offered_incrementally(w: &mut SyncWorld, name: String, cursor: String, field: String) {
    let s = stream(answer(w), &name);
    assert_eq!(s["default_cursor_field"], cursor, "{s}");
    assert!(
        s["sync_modes"]
            .as_array()
            .unwrap()
            .contains(&json!("Incremental")),
        "{s}"
    );
    assert!(
        s["fields"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["name"] == field && f["type"] == "Integer"),
        "{s}"
    );
}

#[then(expr = "the source table {string} is offered as a stream")]
async fn table_offered(w: &mut SyncWorld, name: String) {
    let table = w.table(&name);
    stream(answer(w), &table);
}

#[when("the Postgres connector is asked to describe itself")]
async fn spec_postgres(w: &mut SyncWorld) {
    let request = ConnectorProbeRequest {
        side: "source".into(),
        provider: Some(providers::POSTGRES.into()),
        member: Some("source".into()),
        ..Default::default()
    };
    w.probed = Some(w.probe("spec", &request).await);
}

#[then(expr = "its config has the field {string}, and {string} is secret")]
async fn spec_fields(w: &mut SyncWorld, field: String, secret: String) {
    let a = answer(w);
    let props = &a["config_schema"]["properties"];
    assert!(props.get(&field).is_some(), "{a}");
    assert_eq!(props[&secret]["airbyte_secret"], true, "{a}");
}

#[when(expr = "the connector {string} is asked to describe itself, with no grants")]
async fn spec_any(w: &mut SyncWorld, provider: String) {
    let request = ConnectorProbeRequest {
        side: "source".into(),
        provider: Some(provider),
        member: Some("source".into()),
        ..Default::default()
    };
    w.probed = Some(w.probe("spec", &request).await);
}

#[then(expr = "it describes itself as {string}")]
async fn described_as(w: &mut SyncWorld, name: String) {
    let a = answer(w);
    assert_eq!(a["name"], name, "{a}");
}
