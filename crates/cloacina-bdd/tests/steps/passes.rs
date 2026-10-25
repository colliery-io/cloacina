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
//! Pass steps: a stored sync definition run by name, and its executions
//! grouped into passes.

use std::collections::BTreeMap;

use cucumber::{given, then, when};

use crate::support::world::SyncWorld;

#[given(expr = "a sync definition {string} that reads {int} segments from an echo source")]
async fn definition(w: &mut SyncWorld, name: String, segments: u64) {
    w.define_echo_sync(&name, segments).await;
}

#[given(expr = "the sync definition {string} runs a pass")]
#[when(expr = "the sync definition {string} runs a pass")]
async fn runs(w: &mut SyncWorld, name: String) {
    w.run_definition_pass(&name).await;
    let outcome = w.outcome.as_ref().expect("a pass ran");
    assert!(outcome.is_ok(), "the pass failed: {outcome:?}");
}

#[then(expr = "the sync definition {string} has {int} executions, each naming it")]
async fn executions(w: &mut SyncWorld, name: String, n: usize) {
    let runs = w.instance_executions(&name).await;
    assert_eq!(runs.len(), n, "executions of {name}: {runs:?}");
    for r in &runs {
        assert_eq!(r.instance_name.as_deref(), Some(name.as_str()));
        assert_eq!(r.workflow_name, cloacina_sync::WORKFLOW_NAME);
    }
}

/// `sizes` lists each pass's executions, in the order the passes ran: "3, 1".
#[then(expr = "the passes of {string} have {string} executions")]
async fn passes(w: &mut SyncWorld, name: String, sizes: String) {
    let runs = w.instance_executions(&name).await;
    let mut order: Vec<String> = Vec::new();
    let mut by_pass: BTreeMap<String, usize> = BTreeMap::new();
    for r in &runs {
        let pass = r
            .pass_id
            .unwrap_or_else(|| panic!("execution {} has no pass", r.id.0))
            .0
            .to_string();
        if !by_pass.contains_key(&pass) {
            order.push(pass.clone());
        }
        *by_pass.entry(pass).or_default() += 1;
    }
    let actual: Vec<usize> = order.iter().map(|p| by_pass[p]).collect();
    let expected: Vec<usize> = sizes
        .split(',')
        .map(|n| n.trim().parse().expect("a number of executions"))
        .collect();
    assert_eq!(actual, expected, "executions per pass, in order");
}

#[when(expr = "the sync definition {string} is changed to read {int} segments")]
async fn changed(w: &mut SyncWorld, name: String, segments: u64) {
    w.change_echo_sync(&name, &name, segments).await;
}

#[when(expr = "the sync definition {string} is changed to the sync name {string}")]
async fn renamed(w: &mut SyncWorld, name: String, sync: String) {
    w.change_echo_sync(&name, &sync, 3).await;
}

#[then("the change is accepted")]
async fn change_accepted(w: &mut SyncWorld) {
    let change = w.created.as_ref().expect("a change was made");
    assert!(change.is_ok(), "the change was refused: {change:?}");
}

#[then(expr = "the change is refused, saying {string}")]
async fn change_refused(w: &mut SyncWorld, words: String) {
    match w.created.as_ref().expect("a change was made") {
        Err(e) => assert!(e.contains(&words), "the refusal says `{words}`: {e}"),
        Ok(()) => panic!("the change was accepted"),
    }
}

#[then(expr = "the sync {string} has read up to segment {int}")]
async fn read_up_to(w: &mut SyncWorld, sync: String, segment: u64) {
    let cursor = w.sync_cursor(&sync).await;
    let expected = serde_json::json!(segment.to_string()).to_string();
    assert_eq!(
        cursor.as_deref(),
        Some(expected.as_str()),
        "the committed cursor"
    );
}
