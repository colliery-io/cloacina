/*
 *  Copyright 2026 Colliery Software
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

//! Poll triggers whose code runs on an agent (CLOACINA-T-1077).
//!
//! A server that loads packages from their metadata (CLOACINA-T-1075) has no
//! code of a poll trigger. It registers a [`DispatchedTrigger`] in the
//! trigger's place: the scheduler polls it as any trigger, and its `poll()`
//! asks a [`PollDispatcher`] to run the poll on an agent of the tenant and
//! give back the answer. The server sets the dispatcher on each runner after
//! the runner is made, through a [`PollDispatcherSlot`].

use std::sync::{Arc, RwLock};
use std::time::Duration;

use async_trait::async_trait;
use cloacina_workflow::{Trigger, TriggerError, TriggerResult};

/// What a dispatcher needs to run one poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PollRequest {
    /// The tenant of the runner that loaded the package.
    pub tenant_id: String,
    /// The package that declares the trigger, and its version.
    pub package_name: String,
    pub package_version: String,
    /// The trigger's name in the package.
    pub trigger_name: String,
    /// The workflow the trigger fires.
    pub workflow_name: String,
}

/// What a dispatcher needs to run one poll of a polling accumulator
/// (CLOACINA-T-1156).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccumulatorPollRequest {
    /// The tenant of the runner that loaded the package.
    pub tenant_id: String,
    /// The package that declares the accumulator, and its version.
    pub package_name: String,
    pub package_version: String,
    /// The accumulator's name in the package.
    pub accumulator_name: String,
}

/// Runs a poll somewhere other than this process and gives back the answer.
#[async_trait]
pub trait PollDispatcher: Send + Sync {
    /// Run one poll. `Ok`: the trigger's answer. `Err`: no answer (no
    /// agent, no answer in time, the agent refused, the trigger failed),
    /// with the reason.
    async fn poll(&self, request: &PollRequest) -> Result<TriggerResult, TriggerError>;

    /// Run one poll of a polling accumulator (CLOACINA-T-1156).
    /// `Ok(Some(value))`: the value the poll function returned.
    /// `Ok(None)`: no change. `Err`: no answer, with the reason.
    async fn poll_accumulator(
        &self,
        request: &AccumulatorPollRequest,
    ) -> Result<Option<serde_json::Value>, String> {
        Err(format!(
            "polling accumulator '{}' of package '{}' not polled: this dispatcher does not run \
             polls of polling accumulators",
            request.accumulator_name, request.package_name
        ))
    }

    /// How long one [`poll`](Self::poll) can take at most. The scheduler
    /// waits this long for it.
    fn wait(&self) -> Duration;
}

/// The dispatcher of a runner, set after the runner is made. Shared by the
/// runner and each [`DispatchedTrigger`] its reconciler registers.
pub type PollDispatcherSlot = Arc<RwLock<Option<Arc<dyn PollDispatcher>>>>;

/// An empty slot.
pub fn empty_poll_dispatcher_slot() -> PollDispatcherSlot {
    Arc::new(RwLock::new(None))
}

/// The reason a poll gets when no dispatcher is set.
pub fn no_dispatcher_reason(request: &PollRequest) -> String {
    format!(
        "poll trigger '{}' of package '{}' runs only on the tenant's agents: this server loads \
         packages from their metadata and runs no tenant code, and it has no agent fleet to \
         send the poll to (--default-executor fleet)",
        request.trigger_name, request.package_name
    )
}

/// A poll trigger of a package loaded from its metadata document. It holds
/// the trigger's declaration and no code.
pub struct DispatchedTrigger {
    request: PollRequest,
    poll_interval: Duration,
    allow_concurrent: bool,
    dispatcher: PollDispatcherSlot,
}

impl DispatchedTrigger {
    pub fn new(
        request: PollRequest,
        poll_interval: Duration,
        allow_concurrent: bool,
        dispatcher: PollDispatcherSlot,
    ) -> Self {
        Self {
            request,
            poll_interval,
            allow_concurrent,
            dispatcher,
        }
    }

    /// What this trigger sends to its dispatcher.
    pub fn request(&self) -> &PollRequest {
        &self.request
    }

    fn dispatcher(&self) -> Option<Arc<dyn PollDispatcher>> {
        self.dispatcher
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl std::fmt::Debug for DispatchedTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DispatchedTrigger")
            .field("request", &self.request)
            .field("poll_interval", &self.poll_interval)
            .field("allow_concurrent", &self.allow_concurrent)
            .finish()
    }
}

#[async_trait]
impl Trigger for DispatchedTrigger {
    fn name(&self) -> &str {
        &self.request.trigger_name
    }

    fn poll_interval(&self) -> Duration {
        self.poll_interval
    }

    fn allow_concurrent(&self) -> bool {
        self.allow_concurrent
    }

    fn workflow_name(&self) -> &str {
        &self.request.workflow_name
    }

    async fn poll(&self) -> Result<TriggerResult, TriggerError> {
        match self.dispatcher() {
            Some(dispatcher) => dispatcher.poll(&self.request).await,
            None => Err(TriggerError::PollError {
                message: no_dispatcher_reason(&self.request),
            }),
        }
    }

    fn poll_wait(&self) -> Option<Duration> {
        self.dispatcher().map(|d| d.wait())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Recording {
        seen: Mutex<Vec<PollRequest>>,
    }

    #[async_trait]
    impl PollDispatcher for Recording {
        async fn poll(&self, request: &PollRequest) -> Result<TriggerResult, TriggerError> {
            self.seen.lock().unwrap().push(request.clone());
            Ok(TriggerResult::Skip)
        }
        fn wait(&self) -> Duration {
            Duration::from_secs(45)
        }
    }

    fn request() -> PollRequest {
        PollRequest {
            tenant_id: "acme".into(),
            package_name: "pkg".into(),
            package_version: "1.0.0".into(),
            trigger_name: "watch_inbox".into(),
            workflow_name: "wf".into(),
        }
    }

    #[tokio::test]
    async fn with_no_dispatcher_a_poll_fails_with_the_reason() {
        let trigger = DispatchedTrigger::new(
            request(),
            Duration::from_secs(5),
            false,
            empty_poll_dispatcher_slot(),
        );
        let err = trigger.poll().await.unwrap_err();
        assert!(
            err.to_string().contains("runs only on the tenant's agents"),
            "{err}"
        );
        assert_eq!(trigger.poll_wait(), None);
    }

    #[tokio::test]
    async fn a_dispatcher_set_later_gets_the_poll() {
        let slot = empty_poll_dispatcher_slot();
        let trigger = DispatchedTrigger::new(request(), Duration::from_secs(5), true, slot.clone());
        let recording = Arc::new(Recording {
            seen: Mutex::new(Vec::new()),
        });
        *slot.write().unwrap() = Some(recording.clone());

        let result = trigger.poll().await.unwrap();
        assert!(!result.should_fire());
        assert_eq!(recording.seen.lock().unwrap().as_slice(), &[request()]);
        assert_eq!(trigger.poll_wait(), Some(Duration::from_secs(45)));
        assert_eq!(trigger.workflow_name(), "wf");
        assert!(trigger.allow_concurrent());
    }
}
