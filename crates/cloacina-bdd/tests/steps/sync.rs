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
//! Sync steps: tables, a sync between them, a pass, the cursor, and the
//! faults a sync must survive.

use std::time::Duration;

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

#[given("the destination's database refuses connections")]
#[when("the destination's database refuses connections")]
async fn destination_down(w: &mut SyncWorld) {
    let table = destination(w);
    w.destination = Some(Side::refusing(&table));
}

#[given("the destination's database is back")]
#[when("the destination's database is back")]
async fn destination_back(w: &mut SyncWorld) {
    let table = destination(w);
    w.destination = Some(Side::postgres(&table));
}

fn destination(w: &SyncWorld) -> String {
    w.destination
        .as_ref()
        .and_then(|d| d.table.clone())
        .expect("a Postgres destination")
}

#[given(
    "the write task dies once, after the destination has the segment and before the cursor commits"
)]
async fn write_dies_once(w: &mut SyncWorld) {
    w.fault = Some("fail_after_write_once".into());
}

#[when("the runner dies while it writes the segment, before the cursor commits")]
async fn runner_dies(w: &mut SyncWorld) {
    w.kill_runner_mid_write(Duration::from_secs(3)).await;
}

#[given("a new runner runs a pass")]
#[when("a new runner runs a pass")]
async fn new_runner_pass(w: &mut SyncWorld) {
    w.run_pass().await;
}

#[then("the pass fails")]
async fn pass_fails(w: &mut SyncWorld) {
    let outcome = w.outcome.as_ref().expect("a pass ran");
    assert!(outcome.is_err(), "the pass ended with {outcome:?}");
}

#[then("the segment's write ran twice")]
async fn write_ran_twice(w: &mut SyncWorld) {
    let attempts = w.write_attempts().await;
    assert!(
        attempts.iter().filter(|a| **a == 2).count() == 1 && attempts.iter().all(|a| *a <= 2),
        "write attempts per segment: {attempts:?}"
    );
}

#[then("no cursor is committed")]
async fn no_cursor(w: &mut SyncWorld) {
    let cursor = w.committed_cursor().await;
    assert_eq!(cursor, None, "the committed cursor");
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
