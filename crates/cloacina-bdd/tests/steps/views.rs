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
//! Read steps: what the `/syncs` routes show, through the same read model
//! (`cloacina_sync::views`).

use cloacina_constructor_contract::sync::{Field, FieldType, StreamSchema};
use cucumber::gherkin::Step;
use cucumber::{given, then, when};
use serde_json::json;

use crate::support::world::SyncWorld;

#[given(
    expr = "a sync definition {string} whose echo source sends these records, with the field {string} as an integer"
)]
async fn definition_rows(w: &mut SyncWorld, step: &Step, name: String, field: String) {
    let rows = step.table.as_ref().expect("a records table").rows.clone();
    let (header, rows) = rows.split_first().expect("a header row");
    let records: Vec<String> = rows
        .iter()
        .map(|r| {
            let obj: serde_json::Map<String, serde_json::Value> = header
                .iter()
                .zip(r)
                .map(|(k, v)| {
                    let v = v
                        .parse::<i64>()
                        .map(|n| json!(n))
                        .unwrap_or_else(|_| json!(v));
                    (k.clone(), v)
                })
                .collect();
            serde_json::Value::Object(obj).to_string()
        })
        .collect();
    let schema = StreamSchema {
        fields: vec![Field {
            name: field,
            field_type: FieldType::Integer,
            nullable: false,
        }],
    };
    w.define_echo_sync_rows(&name, records, schema).await;
}

#[then(expr = "the syncs listed are {string}")]
async fn listed(w: &mut SyncWorld, names: String) {
    let dal = w.dal().await;
    let listed: Vec<String> = cloacina_sync::views::list(&dal)
        .await
        .expect("list the syncs")
        .into_iter()
        .map(|s| s.name)
        .collect();
    let expected: Vec<String> = names.split(',').map(|n| n.trim().to_string()).collect();
    assert_eq!(listed, expected);
}

#[then(expr = "the passes of {string} are listed newest first with {string} segments")]
async fn passes_listed(w: &mut SyncWorld, name: String, sizes: String) {
    let dal = w.dal().await;
    let passes = cloacina_sync::views::passes(&dal, &name, 50, 0)
        .await
        .expect("list the passes");
    let actual: Vec<usize> = passes.iter().map(|p| p.segments).collect();
    let expected: Vec<usize> = sizes
        .split(',')
        .map(|n| n.trim().parse().expect("a number"))
        .collect();
    assert_eq!(actual, expected, "{passes:?}");
    for p in &passes {
        assert_eq!(p.status, "Completed", "{p:?}");
        assert_eq!(p.executions.len(), p.segments, "{p:?}");
    }
}

#[then(expr = "the newest pass of {string} ended on segment {int}")]
async fn newest_cursor(w: &mut SyncWorld, name: String, segment: u64) {
    let dal = w.dal().await;
    let passes = cloacina_sync::views::passes(&dal, &name, 1, 0)
        .await
        .expect("list the passes");
    let expected = json!(segment.to_string()).to_string();
    assert_eq!(
        passes[0].cursor.as_deref(),
        Some(expected.as_str()),
        "{passes:?}"
    );
}

#[then(expr = "the dead-letters of {string} list {int} record(s), for its field {string}")]
async fn dead_letters_listed(w: &mut SyncWorld, name: String, n: usize, field: String) {
    let dal = w.dal().await;
    let (items, total) = cloacina_sync::views::dead_letters(&dal, &name, 50, 0)
        .await
        .expect("list the dead-letters");
    assert_eq!(items.len(), n, "{items:?}");
    assert_eq!(total as usize, n);
    for d in &items {
        assert!(d.reason.contains(&format!("`{field}`")), "{d:?}");
        assert!(d.execution_id.is_some(), "{d:?}");
    }
    let summary = cloacina_sync::views::get(&dal, &name)
        .await
        .expect("the sync");
    assert_eq!(summary.summary.dead_letters as usize, n);
}

#[then(
    expr = "the stream of {string} shows the cursor at segment {int}, with the field {string} as {word}"
)]
async fn stream_shown(w: &mut SyncWorld, name: String, segment: u64, field: String, ty: String) {
    let dal = w.dal().await;
    let streams = cloacina_sync::views::streams(&dal, &name)
        .await
        .expect("list the streams");
    assert_eq!(streams.len(), 1, "{streams:?}");
    let s = &streams[0];
    let expected = json!(segment.to_string()).to_string();
    assert_eq!(s.cursor.as_deref(), Some(expected.as_str()), "{s:?}");
    assert!(
        s.fields
            .iter()
            .any(|f| f.name == field && f.field_type == ty),
        "{s:?}"
    );
    assert_eq!(s.broken, None);
}

#[then(expr = "reading the sync {string} says there is no such sync")]
async fn no_such(w: &mut SyncWorld, name: String) {
    let dal = w.dal().await;
    let err = cloacina_sync::views::get(&dal, &name)
        .await
        .expect_err("no such sync");
    assert!(err.to_string().contains(&name), "{err}");
}

#[when(expr = "the sync {string} is run by hand")]
async fn run_by_hand(w: &mut SyncWorld, name: String) {
    w.run_by_hand(&name).await;
}

#[given(
    expr = "a sync definition {string} that copies the Postgres table {string} to an echo destination"
)]
async fn pg_definition(w: &mut SyncWorld, name: String, table: String) {
    w.define_pg_sync(&name, &table).await;
}

#[then(expr = "reading the sync {string} shows its {string} masked, and never the password")]
async fn masked(w: &mut SyncWorld, name: String, field: String) {
    let params = w.read_params(&name).await;
    let config: serde_json::Value =
        serde_json::from_str(params["source_config"].as_str().expect("source_config"))
            .expect("config json");
    assert_eq!(
        config[&field],
        cloacina_sync::config_secrets::SECRET_PLACEHOLDER,
        "{config}"
    );
    let text = serde_json::to_string(&params).unwrap();
    assert!(
        !text.contains(":cloacina@"),
        "a read holds the password: {text}"
    );
}

#[when(expr = "the sync definition {string} is saved again as it was read")]
async fn saved_as_read(w: &mut SyncWorld, name: String) {
    w.change_from_read(&name).await;
}

#[when(expr = "a sync {string} is created from what a read of {string} showed")]
async fn created_from_read(w: &mut SyncWorld, name: String, from: String) {
    w.create_from_read(&from, &name).await;
}
