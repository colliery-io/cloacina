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
//! Sync steps: tables, a sync between them, a pass, the cursor.

use cloacina_sync::dispatcher::PassOutcome;
use cucumber::gherkin::Step;
use cucumber::{given, then, when};

use crate::support::pg;
use crate::support::world::{Side, SyncWorld};

fn table_of(step: &Step) -> Vec<Vec<String>> {
    step.table
        .as_ref()
        .unwrap_or_else(|| panic!("step `{}` needs a table", step.value))
        .rows
        .clone()
}

#[given(expr = "a Postgres source table {string} with the records")]
async fn source_table(w: &mut SyncWorld, step: &Step, name: String) {
    let table = w.table(&name);
    pg::create_table(&table, &table_of(step)).await;
    w.source = Some(Side::postgres(&table));
}

#[given(expr = "a Postgres destination table {string}")]
async fn destination_table(w: &mut SyncWorld, name: String) {
    let table = w.table(&name);
    w.destination = Some(Side::postgres(&table));
}

#[given(expr = "the sync copies {string} by the cursor {string}, keyed on {string}")]
async fn copies(w: &mut SyncWorld, name: String, cursor: String, key: String) {
    w.copy_stream(&name, &cursor, &key);
}

#[given(expr = "the source table {string} gains the records")]
#[when(expr = "the source table {string} gains the records")]
async fn gains(w: &mut SyncWorld, step: &Step, name: String) {
    let table = w.table(&name);
    pg::insert(&table, &table_of(step)).await;
}

#[given("the sync runs a pass")]
#[when("the sync runs a pass")]
async fn runs_a_pass(w: &mut SyncWorld) {
    w.run_pass().await;
}

#[then("the pass ends with the source exhausted")]
async fn exhausted(w: &mut SyncWorld) {
    match w.outcome.as_ref().expect("a pass ran") {
        Ok(PassOutcome::Exhausted { .. }) => {}
        other => panic!("the pass ended with {other:?}"),
    }
}

#[then(expr = "the destination table {string} holds the records")]
async fn holds(w: &mut SyncWorld, step: &Step, name: String) {
    let table = w.table(&name);
    let (header, expected) = table_of(step)
        .split_first()
        .map(|(h, r)| (h.clone(), r.to_vec()))
        .expect("a header row");
    let actual = pg::rows(&table, &header).await;
    assert_eq!(actual, expected, "{table} holds {actual:?}");
}

#[then(expr = "the committed cursor is {string}")]
async fn cursor_is(w: &mut SyncWorld, expected: String) {
    let cursor = w.committed_cursor().await;
    assert_eq!(
        cursor.as_deref(),
        Some(expected.as_str()),
        "the committed cursor"
    );
}
