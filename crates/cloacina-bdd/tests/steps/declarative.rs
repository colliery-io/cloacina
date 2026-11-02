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
//! Declarative steps: a sync side that names a connector definition, the
//! check when the sync is created, and a GitHub API for the corpus's `github`.

use cloacina_constructor_contract::sync::FieldType;
use cloacina_testing::connectors::rest_mock::{Endpoint, Filter, Paging};
use cucumber::gherkin::Step;
use cucumber::{given, then, when};
use serde_json::{Map, Value, json};

use crate::steps::credentials::records;
use crate::support::world::{Side, SyncWorld};

/// The path the corpus `github` definition reads its repositories from.
const REPOS: &str = "/user/repos";

/// A repository with every field the `github` definition declares for its
/// `repositories` stream, the table's values over sample ones.
fn repository(given: &Value) -> Value {
    let defs = cloacina_sync::declarative::definitions().expect("the definitions load");
    let mut user = Map::new();
    user.insert("base_url".into(), json!("http://x"));
    let schema = cloacina_sync::definition::compile_source(
        defs.source("github").expect("the github definition"),
        "repositories",
        &user,
        Some("s"),
    )
    .expect("the repositories stream compiles")
    .schema;
    let mut rec = Map::new();
    for f in &schema.fields {
        let v = match f.field_type {
            FieldType::Integer => json!(1),
            FieldType::Float => json!(1.5),
            FieldType::Boolean => json!(false),
            FieldType::Timestamp => json!("2026-01-01T00:00:00Z"),
            _ => json!(format!("{}-sample", f.name)),
        };
        rec.insert(f.name.clone(), v);
    }
    for (k, v) in given.as_object().expect("a record") {
        rec.insert(k.clone(), v.clone());
    }
    Value::Object(rec)
}

/// Serve the repositories as GitHub does: by page, `since` an `updated_at`,
/// to the bearer token only.
fn serve_repositories(w: &mut SyncWorld, token: &str) {
    let records = w.repositories.clone();
    w.api().serve(
        REPOS,
        Endpoint {
            records,
            paging: Paging::Page {
                page_param: "page".into(),
                size_param: "per_page".into(),
            },
            filter: Some(Filter {
                param: "since".into(),
                field: "updated_at".into(),
            }),
            require_header: Some(("Authorization".into(), format!("Bearer {token}"))),
            ..Default::default()
        },
    );
}

#[given(
    expr = "the GitHub API serves these repositories to the bearer token {string}, each with every field the definition declares"
)]
async fn github_serves(w: &mut SyncWorld, step: &Step, token: String) {
    w.repositories = records(step).iter().map(repository).collect();
    serve_repositories(w, &token);
}

#[given(expr = "the GitHub API also serves these repositories to the bearer token {string}")]
#[when(expr = "the GitHub API also serves these repositories to the bearer token {string}")]
async fn github_also_serves(w: &mut SyncWorld, step: &Step, token: String) {
    let more: Vec<Value> = records(step).iter().map(repository).collect();
    w.repositories.extend(more);
    serve_repositories(w, &token);
}

#[given(
    expr = "the source is the connector definition {string}, at the API's address, with the secret {string}"
)]
async fn definition_source(w: &mut SyncWorld, name: String, secret: String) {
    let base_url = w.api().base_url();
    w.source = Some(Side::definition(
        &name,
        json!({ "base_url": base_url }),
        &secret,
    ));
}

#[given(expr = "the sync reads the stream {string} by the cursor {string}, keyed on {string}")]
async fn reads_stream(w: &mut SyncWorld, stream: String, cursor: String, key: String) {
    w.incremental_stream(&stream, &cursor, &key);
}

#[given(expr = "the sync reads the whole stream {string}, keyed on {string}")]
async fn reads_whole_stream(w: &mut SyncWorld, stream: String, key: String) {
    w.refresh_stream(&stream, &key);
}

#[when("the sync is created")]
async fn created(w: &mut SyncWorld) {
    w.create();
}

#[then("the sync is accepted")]
async fn accepted(w: &mut SyncWorld) {
    let created = w.created.as_ref().expect("the sync was created");
    assert!(created.is_ok(), "the sync was refused: {created:?}");
}

#[then(expr = "the sync is refused, naming {string}")]
async fn refused(w: &mut SyncWorld, name: String) {
    match w.created.as_ref().expect("the sync was created") {
        Err(e) => assert!(e.contains(&name), "the refusal names `{name}`: {e}"),
        Ok(()) => panic!("the sync was accepted"),
    }
}

#[then(expr = "the committed cursor is the time {string}")]
async fn cursor_time(w: &mut SyncWorld, time: String) {
    let cursor = w.committed_cursor().await;
    assert_eq!(
        cursor.as_deref(),
        Some(json!(time).to_string().as_str()),
        "the committed cursor"
    );
}

#[then(expr = "the GitHub API was last asked for repositories since {string}")]
async fn last_since(w: &mut SyncWorld, since: String) {
    let last = w.api().requests_to(REPOS).pop().expect("a request");
    assert_eq!(
        last.params.get("since"),
        Some(&since),
        "the last request: {:?}",
        last.params
    );
}
