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

//! Round-trip exercise for the `CloacinaHost` callback channel (CLOACI-T-0897).
//!
//! Proves cloacina's own host interface works through fidius's generated
//! machinery: a host implementation is bound, the plugin-side client resolves
//! it, and calls cross the full encode → dispatch → decode → host → encode →
//! decode path, including a host-raised typed error.
//!
//! Bound state is a process-global cell and binds are once-only, so this lives
//! in its own test binary rather than the crate's unit tests.
//!
//! Scope note: this is the IN-PROCESS binding. It exercises every layer except
//! the dylib boundary itself — fidius's own `host_functions_e2e` covers that
//! over a real loaded cdylib. What is proven here is that *cloacina's* three-
//! method interface is well-formed and dispatches correctly end to end.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use cloacina_workflow_plugin::fidius::PluginError;
use cloacina_workflow_plugin::{CloacinaHost, CloacinaHostBinding, CloacinaHostClient};

/// Records every callback so the test can assert the host actually ran, rather
/// than inferring it from a non-error return.
#[derive(Default)]
struct RecordingHost {
    calls: Mutex<Vec<String>>,
    reclaims: AtomicU32,
}

impl RecordingHost {
    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
    fn record(&self, entry: String) {
        // Taking this lock inside a callback is fine: the contract only forbids
        // the host holding locks ACROSS a plugin call, which we never do.
        self.calls.lock().unwrap().push(entry);
    }
}

impl CloacinaHost for RecordingHost {
    fn release_slot(&self, task_execution_id: String) -> Result<(), PluginError> {
        // A task id the host refuses, so the test can observe a host-raised
        // typed error crossing back rather than a panic or a silent success.
        if task_execution_id == "refuse-me" {
            return Err(PluginError::new(
                "SLOT_RELEASE_REFUSED",
                "host refused to release this slot",
            ));
        }
        self.record(format!("release:{task_execution_id}"));
        Ok(())
    }

    fn reclaim_slot(&self, task_execution_id: String) -> Result<(), PluginError> {
        self.reclaims.fetch_add(1, Ordering::SeqCst);
        self.record(format!("reclaim:{task_execution_id}"));
        Ok(())
    }

    fn set_sub_status(
        &self,
        task_execution_id: String,
        sub_status: String,
    ) -> Result<(), PluginError> {
        self.record(format!("sub_status:{task_execution_id}={sub_status}"));
        Ok(())
    }
}

#[test]
fn cloacina_host_interface_round_trips() {
    let host = Arc::new(RecordingHost::default());
    CloacinaHostBinding::bind_in_process(host.clone() as Arc<dyn CloacinaHost>)
        .expect("bind CloacinaHost in process");

    let client = CloacinaHostClient::bound().expect("client resolves the bound host");

    // The exact sequence a packaged `defer_until` performs.
    client
        .set_sub_status(&"task-1".to_string(), &"Deferred".to_string())
        .expect("set_sub_status Deferred");
    client
        .release_slot(&"task-1".to_string())
        .expect("release_slot");
    client
        .reclaim_slot(&"task-1".to_string())
        .expect("reclaim_slot");
    client
        .set_sub_status(&"task-1".to_string(), &"Active".to_string())
        .expect("set_sub_status Active");

    assert_eq!(
        host.calls(),
        vec![
            "sub_status:task-1=Deferred".to_string(),
            "release:task-1".to_string(),
            "reclaim:task-1".to_string(),
            "sub_status:task-1=Active".to_string(),
        ],
        "callbacks must arrive in order, with arguments intact across bincode"
    );
    assert_eq!(host.reclaims.load(Ordering::SeqCst), 1);

    // A host-raised error must surface to the caller as an error, not as a
    // panic and not as a silent success — this is what lets a deferred task
    // fail cleanly when the executor is shutting down.
    let err = client
        .release_slot(&"refuse-me".to_string())
        .expect_err("host refusal must surface as an error");
    assert!(
        format!("{err}").contains("SLOT_RELEASE_REFUSED") || format!("{err}").contains("refused"),
        "typed host error should carry its code/message, got: {err}"
    );

    // The refusal must not have been recorded — proof the error came from the
    // host body rather than the transport inventing one.
    assert!(
        !host.calls().iter().any(|c| c.contains("refuse-me")),
        "refused call must not record: {:?}",
        host.calls()
    );
}

/// A second bind is refused. Cloacina binds once per loaded library, so this
/// pins the behavior we rely on rather than leaving it to chance.
#[test]
fn double_bind_is_refused() {
    // This test binary shares the global cell with the test above; whichever
    // runs first performs the real bind. Either way a SECOND bind must fail,
    // which is the property being asserted.
    let host = Arc::new(RecordingHost::default());
    let first = CloacinaHostBinding::bind_in_process(host.clone() as Arc<dyn CloacinaHost>);
    let second = CloacinaHostBinding::bind_in_process(host as Arc<dyn CloacinaHost>);
    assert!(
        first.is_err() || second.is_err(),
        "binding twice in one process must be refused"
    );
}
