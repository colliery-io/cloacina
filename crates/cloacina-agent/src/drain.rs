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

//! The agent's drain on SIGTERM (CLOACINA-T-1029).
//!
//! Kubernetes sends SIGTERM when it deletes an agent pod: a scale-down, a
//! roll (a new key, a new image), a node drain. Then the agent:
//!
//! 1. takes no new work: a work packet, a graph firing or a poll that
//!    arrives is refused as "stopping" (a `Shutdown` refusal, which the
//!    server reads as "the agent did not run it": the task waits, with no use
//!    of a retry attempt), and its heartbeat offers no free slot;
//! 2. lets what it runs finish and report, for up to the drain time;
//! 3. at the end of the drain, stops the workers that still run and hands
//!    their work back to the server (a `Shutdown` refusal: the task is Ready
//!    again with the same attempt), waits a moment for those reports, and
//!    exits.
//!
//! The drain time is `--drain-seconds` / `CLOACINA_AGENT_DRAIN_SECONDS`
//! (default 25 s, inside the Kubernetes default grace period of 30 s). A
//! task run inline (in the agent process) and a poll cannot be stopped:
//! they end with the process.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use cloacina::fleet::{AgentOutcome, RefusalReason};
use tracing::{info, warn};

use crate::worker::PacketEnd;

/// Default drain time, in seconds.
pub(crate) const DEFAULT_DRAIN_SECONDS: u64 = 25;

/// How long the agent waits, after it handed work back, for those reports.
pub(crate) const HAND_BACK_REPORT_WAIT: Duration = Duration::from_secs(10);

/// How often the drain looks at the work in flight.
const POLL: Duration = Duration::from_millis(100);

/// Set once SIGTERM came: the agent takes no new work.
static DRAINING: AtomicBool = AtomicBool::new(false);

/// Whether the agent is stopping: it takes no new work.
pub(crate) fn draining() -> bool {
    DRAINING.load(Ordering::SeqCst)
}

/// The message of a refusal while the agent stops.
pub(crate) const STOPPING: &str = "agent is stopping (SIGTERM); the work goes to another agent";

/// The message of a hand-back at the end of the drain.
pub(crate) const HANDED_BACK: &str =
    "agent stopped (SIGTERM) before the work ended; handed back to the server";

/// What to report for a packet that ended: its outcome; for a worker the
/// agent stopped at the end of its drain, a `Shutdown` refusal (the work is
/// handed back); for a dispatch the server cancelled, nothing.
pub(crate) fn report_of(end: PacketEnd, stop_id: Option<&str>) -> Option<AgentOutcome> {
    match end {
        PacketEnd::Report(outcome) => Some(outcome),
        PacketEnd::Cancelled => {
            let handed_back =
                stop_id.is_some_and(|id| crate::worker::dispatch_registry().take_handed_back(id));
            handed_back.then(|| AgentOutcome::Refused {
                reason: RefusalReason::Shutdown,
                message: HANDED_BACK.to_string(),
            })
        }
    }
}

/// How a drain ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DrainEnd {
    /// Work in flight when the drain began.
    pub in_flight_at_start: u32,
    /// Workers stopped and handed back at the end of the drain.
    pub handed_back: usize,
    /// Work still in flight when the drain ends (inline tasks, polls, or
    /// reports not done in time).
    pub left: u32,
}

/// Drain: set `draining` (no new work), wait up to `wait` for the work in
/// flight to end, then `hand_back` what still runs and wait up to
/// `report_wait` for those reports. `in_flight` is the agent's counter of
/// work in flight.
pub(crate) async fn drain<F>(
    draining: &AtomicBool,
    in_flight: &AtomicU32,
    wait: Duration,
    report_wait: Duration,
    hand_back: F,
) -> DrainEnd
where
    F: FnOnce() -> usize,
{
    draining.store(true, Ordering::SeqCst);
    let in_flight_at_start = in_flight.load(Ordering::SeqCst);
    info!(
        in_flight = in_flight_at_start,
        drain_s = wait.as_secs(),
        "SIGTERM: the agent takes no new work and lets what it runs finish"
    );
    wait_until_idle(in_flight, wait).await;
    let mut handed_back = 0;
    if in_flight.load(Ordering::SeqCst) > 0 {
        handed_back = hand_back();
        warn!(
            in_flight = in_flight.load(Ordering::SeqCst),
            handed_back, "drain time over: the agent hands back the work that still runs"
        );
        wait_until_idle(in_flight, report_wait).await;
    }
    let left = in_flight.load(Ordering::SeqCst);
    info!(in_flight_at_start, handed_back, left, "the agent stops");
    DrainEnd {
        in_flight_at_start,
        handed_back,
        left,
    }
}

async fn wait_until_idle(in_flight: &AtomicU32, wait: Duration) {
    let deadline = tokio::time::Instant::now() + wait;
    while in_flight.load(Ordering::SeqCst) > 0 && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(POLL).await;
    }
}

/// Wait for SIGTERM (or Ctrl-C), drain, and exit the process.
pub(crate) fn spawn_on_sigterm(in_flight: Arc<AtomicU32>, wait: Duration) {
    tokio::spawn(async move {
        wait_for_stop_signal().await;
        let end = drain(&DRAINING, &in_flight, wait, HAND_BACK_REPORT_WAIT, || {
            crate::worker::dispatch_registry().hand_back_all()
        })
        .await;
        std::process::exit(if end.left == 0 { 0 } else { 1 });
    });
}

#[cfg(unix)]
async fn wait_for_stop_signal() {
    use tokio::signal::unix::{SignalKind, signal};
    match signal(SignalKind::terminate()) {
        Ok(mut term) => {
            tokio::select! {
                _ = term.recv() => {}
                _ = tokio::signal::ctrl_c() => {}
            }
        }
        Err(e) => {
            warn!(error = %e, "no SIGTERM handler; the agent drains on Ctrl-C only");
            let _ = tokio::signal::ctrl_c().await;
        }
    }
}

#[cfg(not(unix))]
async fn wait_for_stop_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::memory::{AgentMemory, Watchdog};
    use crate::worker::tests::{TestAction, job, test_settings};
    use crate::worker::{dispatch_registry, run_here_or_in_worker};
    use std::sync::Mutex;

    /// Run a test job as the agent runs a packet: count it in flight, run it
    /// in a worker, and record what the agent would report.
    fn start(
        name: &str,
        action: TestAction,
        in_flight: Arc<AtomicU32>,
        reports: Arc<Mutex<Vec<AgentOutcome>>>,
    ) -> tokio::task::JoinHandle<()> {
        let settings = test_settings(
            name,
            Watchdog::with_interval(AgentMemory::Unknown, Duration::from_millis(50)),
        );
        std::fs::create_dir_all(&settings.work_root).unwrap();
        let job = job(name, action);
        let stop_id = job.packet.dispatch_id.clone();
        in_flight.fetch_add(1, Ordering::SeqCst);
        tokio::spawn(async move {
            let end = run_here_or_in_worker(job, &settings).await;
            if let Some(outcome) = report_of(end, stop_id.as_deref()) {
                reports.lock().unwrap().push(outcome);
            }
            in_flight.fetch_sub(1, Ordering::SeqCst);
        })
    }

    /// Wait until the worker of `dispatch` runs (it is in the registry).
    async fn running(dispatch: &str) {
        for _ in 0..400 {
            if dispatch_registry().is_running(dispatch) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("{dispatch} did not start");
    }

    /// A worker job in flight when SIGTERM comes finishes within the drain
    /// and is reported once, with its own outcome. New work is refused from
    /// the start of the drain.
    #[tokio::test]
    async fn a_job_in_flight_at_sigterm_finishes_and_is_reported_once() {
        let flag = AtomicBool::new(false);
        let in_flight = Arc::new(AtomicU32::new(0));
        let reports = Arc::new(Mutex::new(Vec::new()));
        let run = start(
            "drain-finish",
            TestAction::Block { ms: 1_500 },
            in_flight.clone(),
            reports.clone(),
        );
        running("dispatch-drain-finish").await;

        let end = drain(
            &flag,
            &in_flight,
            Duration::from_secs(60),
            Duration::from_secs(5),
            || panic!("nothing to hand back: the job ends within the drain"),
        )
        .await;
        assert!(flag.load(Ordering::SeqCst), "the agent takes no new work");
        run.await.unwrap();
        assert_eq!(
            end,
            DrainEnd {
                in_flight_at_start: 1,
                handed_back: 0,
                left: 0
            }
        );
        let reports = reports.lock().unwrap();
        assert_eq!(reports.len(), 1, "{reports:?}");
        assert!(
            matches!(reports[0], AgentOutcome::Success { .. }),
            "{reports:?}"
        );
    }

    /// A job still running at the end of the drain is stopped and handed
    /// back: one `Shutdown` refusal, which the server reads as "not run" (the
    /// task is Ready again with its attempt). Not lost, not reported twice.
    #[tokio::test]
    async fn a_job_running_at_the_end_of_the_drain_is_handed_back() {
        let flag = AtomicBool::new(false);
        let in_flight = Arc::new(AtomicU32::new(0));
        let reports = Arc::new(Mutex::new(Vec::new()));
        let run = start(
            "drain-hand-back",
            TestAction::Block { ms: 60_000 },
            in_flight.clone(),
            reports.clone(),
        );
        running("dispatch-drain-hand-back").await;

        let started = std::time::Instant::now();
        let end = drain(
            &flag,
            &in_flight,
            Duration::from_millis(500),
            Duration::from_secs(20),
            || dispatch_registry().hand_back_ids(&["dispatch-drain-hand-back".to_string()]),
        )
        .await;
        run.await.unwrap();
        assert!(started.elapsed() < Duration::from_secs(20));
        assert_eq!(
            end,
            DrainEnd {
                in_flight_at_start: 1,
                handed_back: 1,
                left: 0
            }
        );
        let reports = reports.lock().unwrap();
        assert_eq!(reports.len(), 1, "{reports:?}");
        match &reports[0] {
            AgentOutcome::Refused { reason, message } => {
                assert_eq!(*reason, RefusalReason::Shutdown);
                assert_eq!(message, HANDED_BACK);
            }
            other => panic!("expected a hand-back, got {other:?}"),
        }
    }

    /// A dispatch the server cancelled is not reported; one the agent handed
    /// back is reported one time.
    #[test]
    fn a_cancel_reports_nothing_and_a_hand_back_reports_once() {
        assert!(report_of(PacketEnd::Cancelled, Some("never-handed-back")).is_none());
        let mut registry = dispatch_registry();
        let rx = registry.begin("handed-back-once").expect("not cancelled");
        assert_eq!(registry.hand_back_ids(&["handed-back-once".to_string()]), 1);
        drop(registry);
        drop(rx);
        assert!(report_of(PacketEnd::Cancelled, Some("handed-back-once")).is_some());
        assert!(report_of(PacketEnd::Cancelled, Some("handed-back-once")).is_none());
        assert!(report_of(PacketEnd::Cancelled, None).is_none());
    }
}
