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
//! Credential steps: a JSON API, stored secrets, and what the API saw.

use std::collections::BTreeMap;

use cloacina_testing::connectors::rest_mock::Endpoint;
use cucumber::gherkin::Step;
use cucumber::{given, then};
use serde_json::{Value, json};

use crate::support::pg;
use crate::support::world::{Side, SyncWorld};

/// The path the source reads in every credential scenario.
const ITEMS: &str = "/items";

/// The records of a Gherkin table, a column of integers as numbers.
fn records(step: &Step) -> Vec<Value> {
    let rows = &step.table.as_ref().expect("a records table").rows;
    let (header, rows) = rows.split_first().expect("a header row");
    rows.iter()
        .map(|r| {
            let obj = header
                .iter()
                .zip(r)
                .map(|(k, v)| {
                    let v = v
                        .parse::<i64>()
                        .map(Value::from)
                        .unwrap_or_else(|_| json!(v));
                    (k.clone(), v)
                })
                .collect();
            Value::Object(obj)
        })
        .collect()
}

#[given(expr = "an API that serves the records at {string}")]
async fn api_serves(w: &mut SyncWorld, step: &Step, path: String) {
    let records = records(step);
    w.api().serve(
        &path,
        Endpoint {
            records,
            ..Default::default()
        },
    );
}

#[given(expr = "the API grants the token {string} at {string}")]
async fn api_grants(w: &mut SyncWorld, token: String, path: String) {
    w.api().serve(
        &path,
        Endpoint {
            body: Some(
                json!({ "access_token": token, "token_type": "bearer", "expires_in": 3600 }),
            ),
            ..Default::default()
        },
    );
}

#[given(expr = "a stored secret {string} with the field {string} set to {string}")]
async fn secret_field(w: &mut SyncWorld, name: String, field: String, value: String) {
    w.secrets.entry(name).or_default().insert(field, value);
}

/// A table of `field | value` rows.
#[given(expr = "a stored secret {string} with the fields")]
async fn secret_fields(w: &mut SyncWorld, step: &Step, name: String) {
    let rows = &step.table.as_ref().expect("a fields table").rows;
    let fields: BTreeMap<String, String> =
        rows.iter().map(|r| (r[0].clone(), r[1].clone())).collect();
    w.secrets.entry(name).or_default().extend(fields);
}

#[given(expr = "the sync copies the API's records, keyed on {string}")]
async fn copies_api(w: &mut SyncWorld, key: String) {
    w.refresh_stream("items", &key);
}

fn read_with(w: &mut SyncWorld, credential: Value) {
    let side = Side::rest(w.api(), ITEMS, Some(credential));
    w.source = Some(side);
}

#[given(expr = "the source reads the API with the secret {string} as a bearer token")]
async fn bearer(w: &mut SyncWorld, secret: String) {
    read_with(
        w,
        json!({ "kind": "header", "secret": secret, "name": "Authorization", "field": "token", "prefix": "Bearer " }),
    );
}

#[given(expr = "the source reads the API with the secret {string} as the query parameter {string}")]
async fn query(w: &mut SyncWorld, secret: String, param: String) {
    read_with(
        w,
        json!({ "kind": "query", "secret": secret, "param": param, "field": "token" }),
    );
}

#[given(
    expr = "the source reads the API with an OAuth2 client-credentials token from the secret {string}, granted at {string}"
)]
async fn oauth2(w: &mut SyncWorld, secret: String, path: String) {
    let token_url = format!("{}{path}", w.api().base_url());
    read_with(
        w,
        json!({ "kind": "oauth2", "secret": secret, "token_url": token_url, "grant": "client_credentials" }),
    );
}

#[given("the source's grants allow another port only")]
async fn other_port(w: &mut SyncWorld) {
    let source = w.source.as_mut().expect("a source");
    source.grants = Some(json!({ "http": ["127.0.0.1:9"] }));
}

#[then(expr = "the API saw the header {string} set to {string} on every request")]
async fn saw_header(w: &mut SyncWorld, name: String, value: String) {
    let seen = w.api().requests_to(ITEMS);
    assert!(!seen.is_empty(), "the API received no request");
    for s in &seen {
        assert_eq!(
            s.headers.get(&name.to_ascii_lowercase()),
            Some(&value),
            "headers: {:?}",
            s.headers
        );
    }
}

#[then(expr = "the API saw the query parameter {string} set to {string}")]
async fn saw_query(w: &mut SyncWorld, name: String, value: String) {
    let seen = w.api().requests_to(ITEMS);
    assert!(!seen.is_empty(), "the API received no request");
    for s in &seen {
        assert_eq!(s.params.get(&name), Some(&value), "params: {:?}", s.params);
    }
}

#[then("the API received no request")]
async fn no_request(w: &mut SyncWorld) {
    let seen = w.api().requests_under("/");
    assert!(seen.is_empty(), "the API received {seen:?}");
}

#[then(expr = "the API granted {int} token(s) at {string}")]
async fn granted(w: &mut SyncWorld, n: usize, path: String) {
    let seen = w.api().requests_to(&path);
    assert_eq!(seen.len(), n, "token requests: {seen:?}");
}

#[then(expr = "{string} appears in no saved context and no record")]
async fn nowhere(w: &mut SyncWorld, secret: String) {
    for ctx in w.saved_contexts().await {
        assert!(
            !ctx.contains(&secret),
            "a saved context holds the secret: {ctx}"
        );
    }
    let destination = w
        .destination
        .as_ref()
        .and_then(|d| d.table.clone())
        .expect("a Postgres destination");
    for row in pg::dump(&destination).await {
        assert!(!row.contains(&secret), "a record holds the secret: {row}");
    }
}

#[then(expr = "the pass fails, naming {string}")]
async fn fails_naming(w: &mut SyncWorld, name: String) {
    match w.outcome.as_ref().expect("a pass ran") {
        Err(e) => assert!(e.contains(&name), "the failure names `{name}`: {e}"),
        Ok(o) => panic!("the pass ended with {o:?}"),
    }
}
