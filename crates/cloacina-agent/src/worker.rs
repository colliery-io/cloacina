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

//! Worker processes (CLOACINA-T-1028).
//!
//! The agent runs each task in a child process of its own: the agent binary
//! started as `cloacina-agent __worker`. A task that uses too much memory, or
//! crashes, ends its own worker and no other task. The agent can stop a
//! worker: at the task's time limit, at its memory limit, and when the server
//! gave up on the dispatch.
//!
//! What crosses the boundary, all as JSON:
//!
//! - **stdin** (a pipe): one line, the [`WorkerJob`]: the work packet, the
//!   task's secrets as plain values, and the paths of the package files the
//!   agent fetched. The agent keeps the pipe open; when it closes the agent
//!   is gone, and the worker kills its own process group.
//! - **the result pipe** (its number is in [`RESULT_FD_ENV`]): the worker
//!   writes [`WorkerEvent::Started`] when the package is loaded, then
//!   [`WorkerEvent::Done`] with the outcome.
//! - **stdout and stderr** are the agent's: a task's log lines land in the
//!   agent's log.
//!
//! A secret is never in the worker's arguments or environment. The worker
//! has no API key: the agent does every request to the server.
//!
//! A task marked `inline` does not come here: it runs in the agent process.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use cloacina::fleet::{AgentOutcome, WorkPacket};
use cloacina::task_logs::LogFormat;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use crate::memory::{AgentMemory, Watchdog};

/// The first argument of the agent binary in worker mode.
pub(crate) const WORKER_ARG: &str = "__worker";

/// The variable that tells the worker the number of its result pipe.
pub(crate) const RESULT_FD_ENV: &str = "CLOACINA_AGENT_WORKER_RESULT_FD";

/// How long a worker may take to load its package before the task's clock
/// starts. The worker ends no later than the task's time limit plus this
/// after its start. The server waits 30 s past the time limit for the report.
pub(crate) const LOAD_ALLOWANCE: Duration = Duration::from_secs(20);

/// The `setrlimit` ceilings of a worker that do not depend on the task.
pub(crate) const WORKER_MAX_OPEN_FILES: u64 = 65_536;
pub(crate) const WORKER_MAX_PROCESSES: u64 = 32_768;
/// Added to the address-space ceiling: a runtime reserves address space it
/// never touches (thread stacks, allocator arenas). A WASM constructor
/// reserves several GiB for each memory, so the room is wide.
pub(crate) const ADDRESS_SPACE_SLACK: u64 = 64 << 30;
/// The default of `--task-address-space-factor`.
pub(crate) const DEFAULT_ADDRESS_SPACE_FACTOR: u64 = 8;

/// The package files of one digest, fetched by the agent into its cache.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct PreparedPackage {
    pub digest: String,
    /// Rust: the library file.
    pub library: Option<PathBuf>,
    /// Python: the source archive.
    pub source_archive: Option<PathBuf>,
    /// Python: the dependencies the compiler vendored for this platform.
    pub vendored_set: Option<PathBuf>,
    pub providers: Providers,
}

/// The constructor providers the server bundles with a package.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "state", content = "value", rename_all = "snake_case")]
pub(crate) enum Providers {
    /// The server has none for the package.
    None,
    /// Unpacked under this directory.
    Staged(PathBuf),
    /// They could not be fetched or unpacked.
    Failed(String),
}

/// What a worker gets on stdin. It holds the task's secrets as plain values:
/// it has no `Debug`, and must not be logged.
#[derive(Serialize, Deserialize)]
pub(crate) struct WorkerJob {
    /// The work packet, with its wrapped secrets taken out.
    pub packet: WorkPacket,
    /// The task's secrets, unwrapped by the agent: name → fields.
    pub secrets: HashMap<String, BTreeMap<String, String>>,
    pub package: PreparedPackage,
    /// Where a Python package is unpacked. For a worker it is a directory of
    /// its own, which the agent removes when the worker has exited.
    pub work_dir: PathBuf,
    /// `text` or `json`: the format of the agent's log lines.
    pub log_format: String,
    pub max_unpacked_bytes: u64,
    /// Tests only: what the job does in place of a packaged task.
    #[cfg(test)]
    #[serde(default)]
    pub test_action: Option<tests::TestAction>,
}

/// What a worker writes on its result pipe, one JSON object per line.
#[derive(Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub(crate) enum WorkerEvent {
    /// The package is loaded and the task starts now.
    Started,
    /// The task ended.
    Done { outcome: AgentOutcome },
}

/// How a packet ended on the agent.
#[derive(Debug)]
pub(crate) enum PacketEnd {
    /// Report this outcome to the server.
    Report(AgentOutcome),
    /// The server gave up on the dispatch: nothing to report.
    Cancelled,
}

/// How the agent starts and holds its workers. One per agent process; tests
/// make their own.
pub(crate) struct WorkerSettings {
    /// The agent binary.
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub watchdog: Arc<Watchdog>,
    /// The memory limit of a task that requests none: its share of the agent.
    pub default_memory_limit: Option<u64>,
    /// The address-space ceiling is the memory limit times this, plus
    /// [`ADDRESS_SPACE_SLACK`]. 0: no ceiling.
    pub address_space_factor: u64,
    pub log_format: LogFormat,
    pub max_unpacked_bytes: u64,
    /// Each worker gets a directory of its own under this one.
    pub work_root: PathBuf,
}

impl WorkerSettings {
    /// Workers of this binary with no memory figure: no limit by default.
    pub(crate) fn unlimited(work_root: PathBuf) -> Self {
        Self {
            program: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("cloacina-agent")),
            args: vec![WORKER_ARG.to_string()],
            env: Vec::new(),
            watchdog: Watchdog::new(AgentMemory::Unknown),
            default_memory_limit: None,
            address_space_factor: DEFAULT_ADDRESS_SPACE_FACTOR,
            log_format: LogFormat::Text,
            max_unpacked_bytes:
                cloacina::cloacina_workflow_plugin::unpack_limit::DEFAULT_MAX_UNPACKED_BYTES,
            work_root,
        }
    }

    /// The memory limit of a task: its request, or its share of the agent.
    pub(crate) fn memory_limit_of(&self, packet: &WorkPacket) -> Option<u64> {
        packet.memory_bytes.or(self.default_memory_limit)
    }
}

static SETTINGS: OnceLock<Arc<WorkerSettings>> = OnceLock::new();

/// Set the settings of this agent process. The first call wins.
pub(crate) fn install_settings(settings: WorkerSettings) {
    let _ = SETTINGS.set(Arc::new(settings));
}

/// The settings of this agent process.
pub(crate) fn settings() -> Arc<WorkerSettings> {
    SETTINGS
        .get_or_init(|| {
            Arc::new(WorkerSettings::unlimited(
                std::env::temp_dir()
                    .join("cloacina-agent-cache")
                    .join("work"),
            ))
        })
        .clone()
}

/// The share of the agent a task with no memory request gets as its limit:
/// what the agent offers to tasks, divided by its slots.
pub(crate) fn default_memory_share(offered: Option<u64>, max_concurrency: u32) -> Option<u64> {
    offered.map(|bytes| bytes / u64::from(max_concurrency.max(1)))
}

// ────────────────────────────────────────────────────────────────────
// Dispatches in work, and the cancels of the server
// ────────────────────────────────────────────────────────────────────

/// How many cancelled dispatch ids the agent remembers.
const CANCELLED_MEMORY: usize = 4096;

/// The dispatches that run in a worker, and the dispatches the server
/// cancelled lately.
#[derive(Default)]
pub(crate) struct DispatchRegistry {
    running: HashMap<String, oneshot::Sender<()>>,
    cancelled: HashSet<String>,
    cancelled_order: VecDeque<String>,
}

impl DispatchRegistry {
    /// A worker starts for the dispatch. `None`: the server cancelled it
    /// already, do not start.
    pub(crate) fn begin(&mut self, dispatch_id: &str) -> Option<oneshot::Receiver<()>> {
        if self.cancelled.contains(dispatch_id) {
            return None;
        }
        let (tx, rx) = oneshot::channel();
        self.running.insert(dispatch_id.to_string(), tx);
        Some(rx)
    }

    /// The worker of the dispatch has exited.
    pub(crate) fn end(&mut self, dispatch_id: &str) {
        self.running.remove(dispatch_id);
    }

    /// The server cancelled the dispatch. Returns whether a worker was told
    /// to stop. The id is remembered either way.
    pub(crate) fn cancel(&mut self, dispatch_id: &str) -> bool {
        if self.cancelled.insert(dispatch_id.to_string()) {
            self.cancelled_order.push_back(dispatch_id.to_string());
            while self.cancelled_order.len() > CANCELLED_MEMORY {
                if let Some(old) = self.cancelled_order.pop_front() {
                    self.cancelled.remove(&old);
                }
            }
        }
        match self.running.remove(dispatch_id) {
            Some(tx) => tx.send(()).is_ok(),
            None => false,
        }
    }

    pub(crate) fn is_cancelled(&self, dispatch_id: &str) -> bool {
        self.cancelled.contains(dispatch_id)
    }
}

/// Process-wide, as the push registry is: a cancel can arrive on a later
/// WebSocket session than the packet.
pub(crate) fn dispatch_registry() -> std::sync::MutexGuard<'static, DispatchRegistry> {
    static REGISTRY: OnceLock<std::sync::Mutex<DispatchRegistry>> = OnceLock::new();
    REGISTRY
        .get_or_init(|| std::sync::Mutex::new(DispatchRegistry::default()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

// ────────────────────────────────────────────────────────────────────
// The agent side: start a worker and hold it
// ────────────────────────────────────────────────────────────────────

/// Run a job: in this process when the task is marked `inline`, else in a
/// worker process.
pub(crate) async fn run_here_or_in_worker(job: WorkerJob, settings: &WorkerSettings) -> PacketEnd {
    if job.packet.inline || !cfg!(unix) {
        let limit = Duration::from_secs(u64::from(job.packet.timeout_seconds.max(1)));
        return PacketEnd::Report(crate::run_job(job, Some(limit), || {}).await);
    }
    #[cfg(unix)]
    {
        run_in_worker(job, settings).await
    }
    #[cfg(not(unix))]
    {
        let _ = settings;
        unreachable!("worker processes need a Unix system")
    }
}

#[cfg(unix)]
mod spawn {
    use std::os::fd::AsRawFd;
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    use std::process::Stdio;
    use std::time::Instant;

    use cloacina::fleet::FailureClassification;
    use cloacina::packaging::build_child::{
        ChildPolicy, ChildRlimits, ChildStep, build_child_command, kill_child_group,
    };
    use tokio::io::AsyncWriteExt;
    use tokio::sync::mpsc;
    use tracing::{debug, info, warn};

    use super::*;
    use crate::memory::{StopReason, format_bytes, oom_kill_count};

    /// How long the agent waits for a worker to exit after its result, before
    /// it kills what is left.
    const EXIT_ALLOWANCE: Duration = Duration::from_secs(5);

    /// The `setrlimit` ceilings of one worker.
    pub(crate) fn worker_rlimits(
        memory_limit: Option<u64>,
        timeout_seconds: u32,
        address_space_factor: u64,
    ) -> ChildRlimits {
        let cpus = std::thread::available_parallelism().map_or(1, |n| n.get() as u64);
        let wall = u64::from(timeout_seconds.max(1)) + LOAD_ALLOWANCE.as_secs();
        ChildRlimits {
            cpu_s: wall.saturating_mul(cpus).saturating_add(60),
            mem_bytes: match memory_limit {
                Some(limit) if address_space_factor > 0 => limit
                    .saturating_mul(address_space_factor)
                    .saturating_add(ADDRESS_SPACE_SLACK),
                _ => u64::MAX,
            },
            files: WORKER_MAX_OPEN_FILES,
            procs: WORKER_MAX_PROCESSES,
        }
    }

    fn failure(classification: FailureClassification, message: String) -> PacketEnd {
        PacketEnd::Report(AgentOutcome::Failure {
            message,
            classification,
        })
    }

    /// Start a worker for the job, hold it to its limits, and wait for its
    /// exit. Returns only when the worker process has exited and its process
    /// group was killed: the caller frees the task's slot after that.
    pub(crate) async fn run_in_worker(mut job: WorkerJob, settings: &WorkerSettings) -> PacketEnd {
        let packet = job.packet.clone();
        let dispatch_id = packet.dispatch_id.clone();
        let mut cancel = match dispatch_id.as_deref() {
            Some(id) => match dispatch_registry().begin(id) {
                Some(rx) => Some(rx),
                None => return PacketEnd::Cancelled,
            },
            None => None,
        };
        let end = supervise(&mut job, &packet, settings, &mut cancel).await;
        if let Some(id) = dispatch_id.as_deref() {
            dispatch_registry().end(id);
        }
        let _ = tokio::fs::remove_dir_all(&job.work_dir).await;
        end
    }

    async fn supervise(
        job: &mut WorkerJob,
        packet: &WorkPacket,
        settings: &WorkerSettings,
        cancel: &mut Option<oneshot::Receiver<()>>,
    ) -> PacketEnd {
        let memory_limit = settings.memory_limit_of(packet);
        let time_limit = Duration::from_secs(u64::from(packet.timeout_seconds.max(1)));

        // A directory of its own: the Python package is unpacked there, and
        // the worker's TMPDIR is in it. Removed when the worker has exited.
        job.work_dir = settings.work_root.join(uuid_like());
        let tmp_dir = job.work_dir.join("tmp");
        if let Err(e) = tokio::fs::create_dir_all(&tmp_dir).await {
            return failure(
                FailureClassification::Transient,
                format!("create the worker directory {:?}: {e}", job.work_dir),
            );
        }
        job.log_format = settings.log_format.to_string();
        job.max_unpacked_bytes = settings.max_unpacked_bytes;
        let mut line = match serde_json::to_vec(&*job) {
            Ok(line) => line,
            Err(e) => {
                return failure(
                    FailureClassification::Validation,
                    format!("serialize the worker job: {e}"),
                );
            }
        };
        line.push(b'\n');

        let (result_read, result_write) = match std::io::pipe() {
            Ok(pipe) => pipe,
            Err(e) => {
                return failure(
                    FailureClassification::Transient,
                    format!("open the result pipe of the worker: {e}"),
                );
            }
        };
        let result_fd = result_write.as_raw_fd();
        let rlimits = worker_rlimits(
            memory_limit,
            packet.timeout_seconds,
            settings.address_space_factor,
        )
        .capped_at_own_limits();
        let mut policy = ChildPolicy::worker(rlimits)
            .env(RESULT_FD_ENV, result_fd.to_string())
            .env("TMPDIR", &tmp_dir);
        for (key, value) in &settings.env {
            policy = policy.env(key, value);
        }
        let Some(program) = settings.program.to_str() else {
            return failure(
                FailureClassification::Transient,
                "the path of the agent binary is not UTF-8".to_string(),
            );
        };
        let mut cmd = build_child_command(program, &policy, ChildStep::Build);
        cmd.args(&settings.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        // SAFETY: the closure runs after fork and before exec. It calls only
        // `fcntl`, which is async-signal-safe, and does not allocate. It lets
        // the write end of the result pipe pass the exec, in this child only.
        unsafe {
            cmd.pre_exec(move || {
                let flags = libc::fcntl(result_fd, libc::F_GETFD);
                if flags < 0 || libc::fcntl(result_fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) < 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut cmd = tokio::process::Command::from(cmd);
        cmd.kill_on_drop(true);

        let oom_kills_before = oom_kill_count();
        let spawned_at = Instant::now();
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                return failure(
                    FailureClassification::Transient,
                    format!("start the task's worker process: {e}"),
                );
            }
        };
        drop(result_write);
        let pid = child.id().unwrap_or(0);

        let (stop_tx, mut stop_rx) = mpsc::unbounded_channel::<StopReason>();
        let _watch = settings.watchdog.watch(pid, memory_limit, stop_tx);

        // The job goes in on stdin. The pipe stays open for the life of the
        // worker: the worker reads its end to learn that the agent is gone.
        let mut stdin = child.stdin.take();
        if let Some(pipe) = stdin.as_mut() {
            if let Err(e) = pipe.write_all(&line).await {
                debug!(pid, error = %e, "the worker did not take its job");
            }
            let _ = pipe.flush().await;
        }
        // The line holds the secrets: do not keep it.
        line.iter_mut().for_each(|b| *b = 0);
        drop(line);

        // The worker's events, read on a blocking thread.
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<WorkerEvent>();
        tokio::task::spawn_blocking(move || {
            use std::io::BufRead;
            for line in std::io::BufReader::new(result_read).lines() {
                let Ok(line) = line else { break };
                if let Ok(event) = serde_json::from_str::<WorkerEvent>(&line)
                    && event_tx.send(event).is_err()
                {
                    break;
                }
            }
        });

        let hard_deadline = spawned_at + time_limit + LOAD_ALLOWANCE;
        let mut deadline = hard_deadline;
        let mut started = false;
        let mut outcome: Option<AgentOutcome> = None;
        let mut stop: Option<StopReason> = None;
        let mut events_open = true;
        let status = loop {
            tokio::select! {
                status = child.wait() => break status,
                event = event_rx.recv(), if events_open => match event {
                    Some(WorkerEvent::Started) => {
                        started = true;
                        deadline = (Instant::now() + time_limit).min(hard_deadline);
                    }
                    Some(WorkerEvent::Done { outcome: done }) => {
                        outcome = Some(done);
                        // The worker exits now. If it does not, what is left
                        // of it is killed; the result stands.
                        deadline = Instant::now() + EXIT_ALLOWANCE;
                    }
                    None => events_open = false,
                },
                Some(reason) = stop_rx.recv(), if stop.is_none() => {
                    if outcome.is_none() {
                        info!(
                            pid,
                            task = %packet.task_name,
                            reason = ?reason,
                            "stopping the task's worker"
                        );
                        stop = Some(reason);
                    }
                    kill_child_group(pid);
                }
                _ = tokio::time::sleep_until(deadline.into()), if stop.is_none() => {
                    if outcome.is_none() {
                        info!(
                            pid,
                            task = %packet.task_name,
                            limit_s = packet.timeout_seconds,
                            "the task is past its time limit; stopping its worker"
                        );
                    }
                    stop = Some(StopReason::TimeLimit);
                    kill_child_group(pid);
                }
                cancelled = async { cancel.as_mut().expect("guarded").await },
                    if cancel.is_some() && stop.is_none() =>
                {
                    *cancel = None;
                    if cancelled.is_ok() {
                        info!(
                            pid,
                            task = %packet.task_name,
                            dispatch_id = ?packet.dispatch_id,
                            "the server cancelled the dispatch; stopping its worker"
                        );
                        stop = Some(StopReason::Cancelled);
                        kill_child_group(pid);
                    }
                }
            }
        };
        // The worker has exited. Kill what it left in its process group, and
        // let the agent's end of stdin go.
        kill_child_group(pid);
        drop(stdin);
        reap_group(pid).await;

        // A result written just before the exit may still be on its way.
        if outcome.is_none() && stop.is_none() && events_open {
            let drain = async {
                while let Some(event) = event_rx.recv().await {
                    if let WorkerEvent::Done { outcome: done } = event {
                        return Some(done);
                    }
                }
                None
            };
            outcome = tokio::time::timeout(Duration::from_secs(2), drain)
                .await
                .ok()
                .flatten();
        }

        if let Some(done) = outcome {
            return PacketEnd::Report(done);
        }
        match stop {
            Some(StopReason::Cancelled) => PacketEnd::Cancelled,
            Some(StopReason::OverLimit { used, limit }) => failure(
                FailureClassification::TaskError,
                format!(
                    "task exceeded its memory limit of {}: it used {}, and the agent stopped it",
                    format_bytes(limit),
                    format_bytes(used)
                ),
            ),
            Some(StopReason::AgentOutOfMemory {
                used,
                agent_used,
                agent_total,
            }) => failure(
                FailureClassification::Transient,
                format!(
                    "the agent ran out of memory ({} of {} in use): it stopped this task, its \
                     largest, which used {}",
                    format_bytes(agent_used),
                    format_bytes(agent_total),
                    format_bytes(used)
                ),
            ),
            Some(StopReason::TimeLimit) if started => failure(
                FailureClassification::Timeout,
                format!("task exceeded timeout of {}s", packet.timeout_seconds),
            ),
            Some(StopReason::TimeLimit) => failure(
                FailureClassification::Timeout,
                format!(
                    "task did not start within {}s (its time limit of {}s and {}s to load the \
                     package)",
                    (time_limit + LOAD_ALLOWANCE).as_secs(),
                    packet.timeout_seconds,
                    LOAD_ALLOWANCE.as_secs()
                ),
            ),
            None => {
                let status = match status {
                    Ok(status) => status,
                    Err(e) => {
                        return failure(
                            FailureClassification::Transient,
                            format!("wait for the task's worker process: {e}"),
                        );
                    }
                };
                let oom_killed = matches!(
                    (oom_kills_before, oom_kill_count()),
                    (Some(before), Some(after)) if after > before
                );
                warn!(
                    pid,
                    task = %packet.task_name,
                    status = %status,
                    oom_killed,
                    "the task's worker ended with no result"
                );
                if status.signal() == Some(libc::SIGKILL) && oom_killed {
                    return failure(
                        FailureClassification::Transient,
                        "the agent ran out of memory: the kernel killed the task's worker"
                            .to_string(),
                    );
                }
                let why = match status.signal() {
                    Some(libc::SIGKILL) => "it was killed (signal 9), not by the agent".to_string(),
                    Some(libc::SIGXCPU) => {
                        "it used more CPU time than its limit allows (signal SIGXCPU)".to_string()
                    }
                    Some(signal) => format!("signal {signal}"),
                    None => format!("{status}"),
                };
                failure(
                    FailureClassification::TaskError,
                    format!("the task's worker process ended with no result: {why}"),
                )
            }
        }
    }

    /// Take the dead processes a task left behind out of the process table.
    ///
    /// A process the task started and did not wait for becomes a child of
    /// the agent when its worker exits, where the agent is the first process
    /// of its container. Once killed it stays in the process table until its
    /// parent takes its status, and each entry counts against the pod's
    /// process limit. Only children in the worker's process group are taken:
    /// no other worker, and no process an inline task started. Where another
    /// process adopts orphans, the agent has no such child and this returns
    /// at once.
    async fn reap_group(group: u32) {
        if group == 0 {
            return;
        }
        for _ in 0..25 {
            // SAFETY: `waitpid` with a negative pid takes a dead child of
            // that process group; no status is wanted.
            let got = unsafe {
                libc::waitpid(-(group as libc::pid_t), std::ptr::null_mut(), libc::WNOHANG)
            };
            match got {
                // One taken: look for the next.
                n if n > 0 => {}
                // Children of the group are still on their way out.
                0 => tokio::time::sleep(Duration::from_millis(20)).await,
                // No child in that group (left).
                _ => break,
            }
        }
    }

    /// A name for a worker directory: unique in this agent.
    fn uuid_like() -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        format!(
            "{}-{}-{}",
            std::process::id(),
            nanos,
            NEXT.fetch_add(1, Ordering::Relaxed)
        )
    }
}

#[cfg(unix)]
pub(crate) use spawn::run_in_worker;

// ────────────────────────────────────────────────────────────────────
// The worker side
// ────────────────────────────────────────────────────────────────────

/// The scheduling priority of a worker (`nice`). The agent stays at 0, so a
/// task that uses all the CPU does not hold back the agent's heartbeat.
#[cfg(unix)]
const WORKER_NICE: libc::c_int = 5;

/// The entry of the agent binary in worker mode. Reads one job from stdin,
/// runs it, writes the outcome to the result pipe, and exits.
#[cfg(unix)]
pub(crate) fn worker_main() -> ! {
    use std::io::{BufRead, Read, Write};
    use std::os::fd::FromRawFd;

    fn fail(what: &str) -> ! {
        eprintln!("cloacina-agent worker: {what}");
        std::process::exit(2)
    }
    let Some(result_fd) = std::env::var(RESULT_FD_ENV)
        .ok()
        .and_then(|v| v.parse::<libc::c_int>().ok())
    else {
        fail("no result pipe: this mode is started by the agent, not by hand")
    };
    // SAFETY: the agent passed this descriptor for the worker's use; nothing
    // else in this process owns it. `fcntl` keeps it out of the processes the
    // task starts.
    let result = unsafe {
        libc::fcntl(result_fd, libc::F_SETFD, libc::FD_CLOEXEC);
        std::fs::File::from_raw_fd(result_fd)
    };

    let mut line = String::new();
    if std::io::stdin().lock().read_line(&mut line).is_err() || line.trim().is_empty() {
        fail("no job on stdin")
    }
    let job: WorkerJob = match serde_json::from_str(&line) {
        Ok(job) => job,
        Err(e) => fail(&format!("the job on stdin does not parse: {e}")),
    };
    drop(line);

    // The kernel picks a worker, not the agent, when the pod is out of
    // memory; and the agent gets the CPU before its workers do.
    #[cfg(target_os = "linux")]
    let _ = std::fs::write("/proc/self/oom_score_adj", "1000");
    // SAFETY: `setpriority` has no memory-safety preconditions.
    unsafe {
        libc::setpriority(libc::PRIO_PROCESS, 0, WORKER_NICE);
    }

    // stdin closes when the agent is gone: end this worker and what the task
    // started.
    std::thread::spawn(|| {
        let mut sink = [0u8; 256];
        let mut stdin = std::io::stdin();
        loop {
            match stdin.read(&mut sink) {
                Ok(0) => break,
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
        // SAFETY: `killpg` has no memory-safety preconditions. The worker
        // leads its own process group.
        unsafe {
            libc::killpg(libc::getpgrp(), libc::SIGKILL);
        }
    });

    let _ = cloacina::cloacina_workflow_plugin::unpack_limit::set_max_unpacked_bytes(
        job.max_unpacked_bytes,
    );
    let write_event = |event: &WorkerEvent| {
        if let Ok(mut text) = serde_json::to_string(event) {
            text.push('\n');
            let mut pipe = &result;
            let _ = pipe.write_all(text.as_bytes());
            let _ = pipe.flush();
        }
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => fail(&format!("build the tokio runtime: {e}")),
    };
    let outcome = runtime.block_on(async {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
        let _ = match job.log_format.parse().unwrap_or(LogFormat::Text) {
            LogFormat::Text => tracing_subscriber::fmt().with_env_filter(filter).try_init(),
            LogFormat::Json => tracing_subscriber::fmt()
                .json()
                .with_env_filter(filter)
                .try_init(),
        };
        cloacina_python::install();
        crate::run_job(job, None, || write_event(&WorkerEvent::Started)).await
    });
    write_event(&WorkerEvent::Done { outcome });
    drop(result);
    // The task is over: leave at once. A thread the task left running does
    // not hold the slot.
    std::process::exit(0)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use cloacina::fleet::{AGENT_PROTOCOL_VERSION, ArtifactRef, FailureClassification};

    /// What a test job does in place of a packaged task.
    #[derive(Clone, Serialize, Deserialize)]
    #[serde(tag = "action", rename_all = "snake_case")]
    pub(crate) enum TestAction {
        /// Return the process id, the arguments, the environment and whether
        /// the secret `name` resolves to `value`.
        Report { secret: Option<(String, String)> },
        /// Take and touch `bytes` of memory, then block for `hold_ms`.
        Allocate { bytes: u64, hold_ms: u64 },
        /// Block the thread for `ms`, as a data job does.
        Block { ms: u64 },
        /// Start `sleep 300`, write its pid to `pid_file`, then block.
        StartChild { pid_file: PathBuf, ms: u64 },
        /// End the process with no result.
        Crash,
    }

    /// Run a test action. Called by `run_job` in place of the package load.
    pub(crate) async fn run_test_action(
        action: TestAction,
        secrets: Option<Arc<dyn cloacina::SecretResolver>>,
    ) -> AgentOutcome {
        let ok = |context: serde_json::Value| AgentOutcome::Success { context };
        match action {
            TestAction::Report { secret } => {
                let secret_ok = match (secret, secrets) {
                    (Some((name, value)), Some(resolver)) => resolver
                        .resolve(&name)
                        .await
                        .is_ok_and(|fields| fields.values().any(|v| *v == value)),
                    _ => false,
                };
                ok(serde_json::json!({
                    "pid": std::process::id(),
                    "args": std::env::args().collect::<Vec<_>>(),
                    "env": std::env::vars().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>(),
                    "secret_ok": secret_ok,
                    "tmpdir": std::env::temp_dir(),
                }))
            }
            TestAction::Allocate { bytes, hold_ms } => {
                let mut block = vec![0u8; bytes as usize];
                for at in (0..block.len()).step_by(4096) {
                    block[at] = 1;
                }
                std::thread::sleep(Duration::from_millis(hold_ms));
                ok(serde_json::json!({ "pid": std::process::id(), "sum": block[0] }))
            }
            TestAction::Block { ms } => {
                std::thread::sleep(Duration::from_millis(ms));
                ok(serde_json::json!({ "pid": std::process::id() }))
            }
            TestAction::StartChild { pid_file, ms } => {
                // Left running on purpose: the test checks that the agent
                // kills what a task started.
                #[allow(clippy::zombie_processes)]
                let child = std::process::Command::new("sleep")
                    .arg("300")
                    .spawn()
                    .expect("start sleep");
                std::fs::write(&pid_file, child.id().to_string()).expect("write the pid");
                std::thread::sleep(Duration::from_millis(ms));
                ok(serde_json::json!({ "pid": std::process::id() }))
            }
            TestAction::Crash => std::process::abort(),
        }
    }

    const CHILD_ENV: &str = "CLOACINA_AGENT_TEST_WORKER";

    /// The worker of the tests: the test binary runs this one test again as
    /// a child, and the test turns into a worker.
    #[cfg(unix)]
    #[test]
    fn worker_child_entry() {
        if std::env::var_os(CHILD_ENV).is_some() {
            worker_main();
        }
    }

    fn tempdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cloacina-agent-worker-test-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Settings whose worker is this test binary.
    pub(crate) fn test_settings(name: &str, watchdog: Arc<Watchdog>) -> WorkerSettings {
        WorkerSettings {
            program: std::env::current_exe().unwrap(),
            args: [
                "--exact",
                "worker::tests::worker_child_entry",
                "--nocapture",
                "--test-threads=1",
            ]
            .map(str::to_string)
            .to_vec(),
            env: vec![(CHILD_ENV.to_string(), "1".to_string())],
            watchdog,
            ..WorkerSettings::unlimited(tempdir(name))
        }
    }

    fn fast_watchdog(agent: AgentMemory) -> Arc<Watchdog> {
        Watchdog::with_interval(agent, Duration::from_millis(50))
    }

    pub(crate) fn job(name: &str, action: TestAction) -> WorkerJob {
        WorkerJob {
            packet: WorkPacket {
                protocol_version: AGENT_PROTOCOL_VERSION,
                task_execution_id: format!("task-{name}"),
                workflow_execution_id: "w1".into(),
                task_name: format!("public::pkg::wf::{name}"),
                attempt: 1,
                context: serde_json::json!({}),
                artifact: ArtifactRef {
                    digest: "none".into(),
                    fetch_url: "/none".into(),
                    build_target_triple: cloacina::fleet::host_target_triple(),
                },
                timeout_seconds: 60,
                tenant_id: None,
                language: Some("rust".into()),
                wrapped_secrets: Vec::new(),
                secret_key_id: None,
                dispatch_id: Some(format!("dispatch-{name}")),
                pool: None,
                cpu_millis: None,
                memory_bytes: None,
                inline: false,
            },
            secrets: HashMap::new(),
            package: PreparedPackage {
                digest: "none".into(),
                library: None,
                source_archive: None,
                vendored_set: None,
                providers: Providers::None,
            },
            work_dir: PathBuf::new(),
            log_format: "text".into(),
            max_unpacked_bytes: 1 << 20,
            test_action: Some(action),
        }
    }

    fn success(end: PacketEnd) -> serde_json::Value {
        match end {
            PacketEnd::Report(AgentOutcome::Success { context }) => context,
            other => panic!("expected a success, got {other:?}"),
        }
    }

    fn failure(end: PacketEnd) -> (FailureClassification, String) {
        match end {
            PacketEnd::Report(AgentOutcome::Failure {
                message,
                classification,
            }) => (classification, message),
            other => panic!("expected a failure, got {other:?}"),
        }
    }

    #[cfg(unix)]
    fn is_alive(pid: u32) -> bool {
        // SAFETY: signal 0 only checks that the process exists.
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }

    /// The pid a `StartChild` job wrote, once it is there.
    #[cfg(unix)]
    async fn read_pid(pid_file: &std::path::Path) -> u32 {
        for _ in 0..200 {
            if let Some(pid) = std::fs::read_to_string(pid_file)
                .ok()
                .and_then(|t| t.trim().parse().ok())
            {
                return pid;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("the job did not write {pid_file:?}");
    }

    // ── A task runs in a worker; an inline task runs in the agent ─────────

    #[cfg(unix)]
    #[tokio::test]
    async fn a_task_runs_in_a_worker_process_and_an_inline_task_in_the_agent() {
        let settings = test_settings("pid", fast_watchdog(AgentMemory::Unknown));
        let here = u64::from(std::process::id());

        let in_worker = success(
            run_here_or_in_worker(
                job("pid-worker", TestAction::Report { secret: None }),
                &settings,
            )
            .await,
        );
        assert_ne!(
            in_worker["pid"].as_u64().unwrap(),
            here,
            "a worker process ran it"
        );

        let mut inline = job("pid-inline", TestAction::Report { secret: None });
        inline.packet.inline = true;
        let in_agent = success(run_here_or_in_worker(inline, &settings).await);
        assert_eq!(
            in_agent["pid"].as_u64().unwrap(),
            here,
            "the agent process ran it"
        );
    }

    // ── A real package: a Python task, loaded and run by the worker ───────

    /// Pack a Python source package with one task, `report`, that puts its
    /// process id in the context. Returns the fetched-package record and the
    /// full name of the task.
    fn python_package(dir: &std::path::Path) -> (PreparedPackage, String) {
        let source = dir.join("t1028-py");
        std::fs::create_dir_all(source.join("workflow/t1028_py")).unwrap();
        std::fs::write(
            source.join("package.toml"),
            "[package]\nname = \"t1028-py\"\nversion = \"0.1.0\"\n\
             interface = \"cloacina-workflow-plugin\"\ninterface_version = 1\n\
             extension = \"cloacina\"\n\n[metadata]\nworkflow_name = \"t1028_py\"\n\
             language = \"python\"\ndescription = \"worker test\"\n\
             entry_module = \"t1028_py.tasks\"\n",
        )
        .unwrap();
        std::fs::write(source.join("workflow/t1028_py/__init__.py"), "").unwrap();
        std::fs::write(
            source.join("workflow/t1028_py/tasks.py"),
            "import os\nimport cloaca\n\n\n@cloaca.task(dependencies=[])\n\
             def report(context):\n    context.set(\"pid\", os.getpid())\n    return context\n",
        )
        .unwrap();
        let archive = dir.join("t1028-py.cloacina");
        fidius_core::package::pack_package(&source, Some(&archive)).unwrap();
        (
            PreparedPackage {
                digest: format!("t1028-py-{}", std::process::id()),
                library: None,
                source_archive: Some(archive),
                vendored_set: None,
                providers: Providers::None,
            },
            "public::t1028-py::t1028_py::report".to_string(),
        )
    }

    fn python_job(name: &str, package: &PreparedPackage, task: &str) -> WorkerJob {
        let mut one = job(name, TestAction::Crash);
        one.test_action = None;
        one.package = package.clone();
        one.packet.language = Some("python".into());
        one.packet.task_name = task.to_string();
        one.max_unpacked_bytes =
            cloacina::cloacina_workflow_plugin::unpack_limit::DEFAULT_MAX_UNPACKED_BYTES;
        one
    }

    /// The whole path with a real package: the worker unpacks a Python
    /// package into a directory of its own, starts the interpreter, imports
    /// the module and runs the task. The same package marked inline runs in
    /// this process.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_python_task_runs_in_a_worker_and_inline_in_the_agent() {
        let settings = test_settings("python", fast_watchdog(AgentMemory::Unknown));
        std::fs::create_dir_all(&settings.work_root).unwrap();
        let (package, task) = python_package(&settings.work_root);
        let here = u64::from(std::process::id());

        let got = success(
            run_here_or_in_worker(python_job("py-worker", &package, &task), &settings).await,
        );
        assert_ne!(
            got["pid"].as_u64().unwrap(),
            here,
            "a worker process ran it"
        );
        // Only the package files are left: the worker's directory is gone.
        let left: Vec<_> = std::fs::read_dir(&settings.work_root)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            left.iter().all(|name| name.starts_with("t1028-py")),
            "{left:?}"
        );

        cloacina_python::install();
        let mut inline = python_job("py-inline", &package, &task);
        inline.packet.inline = true;
        inline.work_dir = settings.work_root.join("t1028-py-inline");
        let got = success(run_here_or_in_worker(inline, &settings).await);
        assert_eq!(
            got["pid"].as_u64().unwrap(),
            here,
            "the agent process ran it"
        );
    }

    // ── Secrets reach the task and are not in arguments or environment ────

    #[cfg(unix)]
    #[tokio::test]
    async fn a_secret_reaches_the_task_and_is_not_in_the_workers_arguments_or_environment() {
        const VALUE: &str = "s3cr3t-value-of-t-1028";
        let settings = test_settings("secret", fast_watchdog(AgentMemory::Unknown));
        let mut with_secret = job(
            "secret",
            TestAction::Report {
                secret: Some(("warehouse".into(), VALUE.into())),
            },
        );
        with_secret.secrets.insert(
            "warehouse".into(),
            [("password".to_string(), VALUE.to_string())].into(),
        );
        let got = success(run_here_or_in_worker(with_secret, &settings).await);
        assert_eq!(got["secret_ok"], true, "the task read its secret");
        let listed = format!("{} {}", got["args"], got["env"]);
        assert!(
            !listed.contains(VALUE),
            "the secret is in args or env: {listed}"
        );
        // The worker's temporary directory is its own, and is gone after.
        let tmp = PathBuf::from(got["tmpdir"].as_str().unwrap());
        assert!(tmp.starts_with(&settings.work_root), "{tmp:?}");
        assert!(!tmp.exists(), "the worker's directory is removed");
    }

    // ── Memory: the task over its limit fails alone ───────────────────────

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn a_task_over_its_memory_limit_fails_alone() {
        let settings = test_settings("memory", fast_watchdog(AgentMemory::Unknown));
        let mut greedy = job(
            "greedy",
            TestAction::Allocate {
                bytes: 600 << 20,
                hold_ms: 20_000,
            },
        );
        greedy.packet.memory_bytes = Some(200 << 20);
        let mut modest = job(
            "modest",
            TestAction::Allocate {
                bytes: 8 << 20,
                hold_ms: 1_500,
            },
        );
        modest.packet.memory_bytes = Some(200 << 20);

        let started = std::time::Instant::now();
        let (greedy_end, modest_end) = tokio::join!(
            run_here_or_in_worker(greedy, &settings),
            run_here_or_in_worker(modest, &settings)
        );
        let (class, message) = failure(greedy_end);
        assert_eq!(class, FailureClassification::TaskError);
        assert!(
            message.contains("exceeded its memory limit of 200Mi"),
            "{message}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "the task was stopped, it did not run out its 20 s"
        );
        success(modest_end);
    }

    /// A task with no request gets its share of the agent as its limit.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn a_task_with_no_request_is_held_to_its_share_of_the_agent() {
        let mut settings = test_settings("share", fast_watchdog(AgentMemory::Unknown));
        // An agent that offers 800Mi over 4 slots: 200Mi a task.
        settings.default_memory_limit = default_memory_share(Some(800 << 20), 4);
        assert_eq!(settings.default_memory_limit, Some(200 << 20));
        let greedy = job(
            "share",
            TestAction::Allocate {
                bytes: 600 << 20,
                hold_ms: 20_000,
            },
        );
        let (_, message) = failure(run_here_or_in_worker(greedy, &settings).await);
        assert!(
            message.contains("exceeded its memory limit of 200Mi"),
            "{message}"
        );
    }

    /// The agent as a whole is out of memory: its largest worker is stopped
    /// with the other error, and the agent goes on.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn when_the_agent_is_out_of_memory_its_largest_worker_is_stopped() {
        // The agent's memory: what this process uses now and 150Mi more.
        let total = crate::memory::own_memory().unwrap() + (150 << 20);
        let settings = test_settings("agent-oom", fast_watchdog(AgentMemory::Sum { total }));
        let large = job(
            "large",
            TestAction::Allocate {
                bytes: 600 << 20,
                hold_ms: 20_000,
            },
        );
        let (class, message) = failure(run_here_or_in_worker(large, &settings).await);
        assert_eq!(class, FailureClassification::Transient);
        assert!(message.contains("the agent ran out of memory"), "{message}");
        assert!(!message.contains("exceeded its memory limit"), "{message}");
        // The agent takes the next task.
        let next = job("after-oom", TestAction::Report { secret: None });
        success(run_here_or_in_worker(next, &settings).await);
    }

    // ── Time limit: the worker is killed, with what it started ────────────

    #[cfg(unix)]
    #[tokio::test]
    async fn a_task_past_its_time_limit_is_killed_before_its_slot_is_free() {
        let settings = test_settings("time", fast_watchdog(AgentMemory::Unknown));
        let pid_file = settings.work_root.join("child.pid");
        std::fs::create_dir_all(&settings.work_root).unwrap();
        let mut slow = job(
            "slow",
            TestAction::StartChild {
                pid_file: pid_file.clone(),
                ms: 60_000,
            },
        );
        slow.packet.timeout_seconds = 1;

        let started = std::time::Instant::now();
        let run = tokio::spawn(async move {
            let end = run_here_or_in_worker(slow, &settings).await;
            (end, settings)
        });
        let grandchild = read_pid(&pid_file).await;
        assert!(is_alive(grandchild), "the task's own child runs");
        let (end, _settings) = run.await.unwrap();
        // `run_here_or_in_worker` is what holds the slot: at its return the
        // worker has exited, and what the task started is gone too.
        let (class, message) = failure(end);
        assert_eq!(class, FailureClassification::Timeout);
        assert_eq!(message, "task exceeded timeout of 1s");
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "killed, not waited out"
        );
        let gone = (0..200).any(|_| {
            let alive = is_alive(grandchild);
            if alive {
                std::thread::sleep(Duration::from_millis(25));
            }
            !alive
        });
        assert!(gone, "the task's child {grandchild} outlived the kill");
    }

    // ── Cancel ────────────────────────────────────────────────────────────

    #[cfg(unix)]
    #[tokio::test]
    async fn a_cancel_stops_the_worker_and_nothing_is_reported() {
        let settings = test_settings("cancel", fast_watchdog(AgentMemory::Unknown));
        let pid_file = settings.work_root.join("child.pid");
        std::fs::create_dir_all(&settings.work_root).unwrap();
        let long = job(
            "cancel",
            TestAction::StartChild {
                pid_file: pid_file.clone(),
                ms: 60_000,
            },
        );
        let started = std::time::Instant::now();
        let run = tokio::spawn(async move { run_here_or_in_worker(long, &settings).await });
        let grandchild = read_pid(&pid_file).await;

        assert!(
            dispatch_registry().cancel("dispatch-cancel"),
            "a worker was told to stop"
        );
        let end = run.await.unwrap();
        assert!(matches!(end, PacketEnd::Cancelled), "{end:?}");
        assert!(started.elapsed() < Duration::from_secs(20));
        let gone = (0..200).any(|_| {
            let alive = is_alive(grandchild);
            if alive {
                std::thread::sleep(Duration::from_millis(25));
            }
            !alive
        });
        assert!(gone, "the task's child {grandchild} outlived the cancel");
    }

    /// A packet that arrives after its cancel is not started.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_dispatch_cancelled_before_it_arrives_is_not_started() {
        let settings = test_settings("early-cancel", fast_watchdog(AgentMemory::Unknown));
        assert!(
            !dispatch_registry().cancel("dispatch-early-cancel"),
            "no worker runs it yet"
        );
        let late = job("early-cancel", TestAction::Report { secret: None });
        let end = run_here_or_in_worker(late, &settings).await;
        assert!(matches!(end, PacketEnd::Cancelled), "{end:?}");
    }

    #[test]
    fn the_registry_forgets_the_oldest_cancelled_dispatches() {
        let mut registry = DispatchRegistry::default();
        for n in 0..=CANCELLED_MEMORY {
            registry.cancel(&format!("d{n}"));
        }
        assert!(!registry.is_cancelled("d0"));
        assert!(registry.is_cancelled("d1"));
        assert!(registry.begin("d1").is_none());
        // A dispatch in work is told one time.
        let rx = registry.begin("live").expect("not cancelled");
        assert!(registry.cancel("live"));
        assert!(!registry.cancel("live"));
        drop(rx);
    }

    // ── A worker that dies fails its own task only ────────────────────────

    #[cfg(unix)]
    #[tokio::test]
    async fn a_worker_that_crashes_fails_its_task_with_a_reason() {
        let settings = test_settings("crash", fast_watchdog(AgentMemory::Unknown));
        let (class, message) =
            failure(run_here_or_in_worker(job("crash", TestAction::Crash), &settings).await);
        assert_eq!(class, FailureClassification::TaskError);
        assert!(
            message.contains("worker process ended with no result"),
            "{message}"
        );
        success(
            run_here_or_in_worker(
                job("after-crash", TestAction::Report { secret: None }),
                &settings,
            )
            .await,
        );
    }

    // ── A worker does not outlive its agent ───────────────────────────────

    /// The agent holds the worker's stdin open. When the agent is gone the
    /// pipe closes, and the worker kills its own process group.
    #[cfg(unix)]
    #[test]
    fn a_worker_ends_when_its_agent_is_gone() {
        use cloacina::packaging::build_child::{
            ChildPolicy, ChildRlimits, ChildStep, build_child_command,
        };
        use std::io::{BufRead, Write};
        use std::os::fd::AsRawFd;
        use std::os::unix::process::{CommandExt, ExitStatusExt};

        let settings = test_settings("orphan", fast_watchdog(AgentMemory::Unknown));
        let (result_read, result_write) = std::io::pipe().unwrap();
        let result_fd = result_write.as_raw_fd();
        let no_limits = ChildRlimits {
            cpu_s: u64::MAX,
            mem_bytes: u64::MAX,
            files: u64::MAX,
            procs: u64::MAX,
        };
        let mut policy = ChildPolicy::worker(no_limits.capped_at_own_limits())
            .env(RESULT_FD_ENV, result_fd.to_string());
        for (key, value) in &settings.env {
            policy = policy.env(key, value);
        }
        let mut cmd = build_child_command(
            settings.program.to_str().unwrap(),
            &policy,
            ChildStep::Build,
        );
        cmd.args(&settings.args).stdin(std::process::Stdio::piped());
        // SAFETY: only `fcntl`, after fork and before exec.
        unsafe {
            cmd.pre_exec(move || {
                let flags = libc::fcntl(result_fd, libc::F_GETFD);
                libc::fcntl(result_fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
                Ok(())
            });
        }
        let mut child = cmd.spawn().unwrap();
        drop(result_write);

        let mut stdin = child.stdin.take().unwrap();
        let mut line =
            serde_json::to_vec(&job("orphan", TestAction::Block { ms: 60_000 })).unwrap();
        line.push(b'\n');
        stdin.write_all(&line).unwrap();
        stdin.flush().unwrap();

        // The task runs.
        let mut first = String::new();
        std::io::BufReader::new(result_read)
            .read_line(&mut first)
            .unwrap();
        assert!(first.contains("started"), "{first}");

        // The agent goes away: its end of the pipe closes.
        let gone_at = std::time::Instant::now();
        drop(stdin);
        let status = child.wait().unwrap();
        assert_eq!(status.signal(), Some(libc::SIGKILL), "{status}");
        assert!(
            gone_at.elapsed() < Duration::from_secs(10),
            "the worker ended at once, not after its 60 s"
        );
    }

    // ── The limits a worker starts with ───────────────────────────────────

    #[cfg(unix)]
    #[test]
    fn the_address_space_ceiling_follows_the_memory_limit() {
        let limits = spawn::worker_rlimits(Some(1 << 30), 60, 8);
        assert_eq!(limits.mem_bytes, (8 << 30) + ADDRESS_SPACE_SLACK);
        assert_eq!(limits.files, WORKER_MAX_OPEN_FILES);
        assert_eq!(limits.procs, WORKER_MAX_PROCESSES);
        assert!(limits.cpu_s >= 60 + LOAD_ALLOWANCE.as_secs() + 60);
        // No limit, or the factor 0: no ceiling of the agent's own.
        assert_eq!(spawn::worker_rlimits(None, 60, 8).mem_bytes, u64::MAX);
        assert_eq!(
            spawn::worker_rlimits(Some(1 << 30), 60, 0).mem_bytes,
            u64::MAX
        );
    }

    #[test]
    fn a_task_request_is_its_limit_and_no_request_gets_the_share() {
        let mut settings = WorkerSettings::unlimited(PathBuf::from("/nonexistent"));
        let mut asked = job("limit", TestAction::Crash).packet;
        assert_eq!(settings.memory_limit_of(&asked), None);
        settings.default_memory_limit = default_memory_share(Some(3 << 30), 4);
        assert_eq!(settings.memory_limit_of(&asked), Some(768 << 20));
        asked.memory_bytes = Some(2 << 30);
        assert_eq!(settings.memory_limit_of(&asked), Some(2 << 30));
        assert_eq!(default_memory_share(None, 4), None);
        assert_eq!(default_memory_share(Some(1 << 30), 0), Some(1 << 30));
    }

    /// The start cost of a worker: process start, runtime start, one result.
    /// Run by hand: `cargo test -p cloacina-agent -- --ignored --nocapture
    /// worker_start_cost`. With `CLOACINA_AGENT_BIN` set to a built agent
    /// binary it measures that binary (the job then ends as a refusal: it has
    /// no package).
    #[cfg(unix)]
    #[tokio::test]
    #[ignore = "a measurement, run by hand"]
    async fn worker_start_cost() {
        let mut settings = test_settings("cost", fast_watchdog(AgentMemory::Unknown));
        let real = std::env::var_os("CLOACINA_AGENT_BIN");
        if let Some(bin) = &real {
            settings.program = PathBuf::from(bin);
            settings.args = vec![WORKER_ARG.to_string()];
            settings.env.clear();
        }
        let rounds = 20;
        let mut times = Vec::new();
        for n in 0..rounds {
            let mut one = job(&format!("cost-{n}"), TestAction::Report { secret: None });
            if real.is_some() {
                one.package.library = Some(PathBuf::from("/nonexistent/library"));
            }
            let at = std::time::Instant::now();
            let end = run_here_or_in_worker(one, &settings).await;
            times.push(at.elapsed());
            assert!(matches!(end, PacketEnd::Report(_)), "{end:?}");
        }
        times.sort();
        println!(
            "worker start cost over {rounds} runs ({}): min {:?}, median {:?}, max {:?}",
            if real.is_some() {
                "agent binary"
            } else {
                "test binary"
            },
            times[0],
            times[rounds / 2],
            times[rounds - 1]
        );

        // A Python task: the same, and the interpreter start, the unpack of a
        // small package and the import of one module with no dependencies.
        std::fs::create_dir_all(&settings.work_root).unwrap();
        let (package, task) = python_package(&settings.work_root);
        let mut python = Vec::new();
        for n in 0..rounds {
            let one = python_job(&format!("cost-py-{n}"), &package, &task);
            let at = std::time::Instant::now();
            let end = run_here_or_in_worker(one, &settings).await;
            python.push(at.elapsed());
            success(end);
        }
        python.sort();
        println!(
            "python task in a worker over {rounds} runs: min {:?}, median {:?}, max {:?}",
            python[0],
            python[rounds / 2],
            python[rounds - 1]
        );
    }
}
