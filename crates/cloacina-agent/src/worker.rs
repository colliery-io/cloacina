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
//!
//! A computation-graph firing whose packet is not marked `inline` (the
//! server's `--fleet-graph-firings worker`) runs the same way
//! (CLOACINA-T-1125): its job
//! is a [`GraphJob`], it has the memory share of a task with no request and
//! the packet's time limit, and a cancel names its firing id.
//!
//! A poll of a poll trigger runs the same way (CLOACINA-T-1143): its job is
//! a [`PollJob`], with the memory share of a task with no request and the
//! packet's time limit (the server's `--fleet-poll-timeout-s`).
//!
//! **Warm workers** (CLOACINA-T-1126). After a job a worker can wait, with
//! its package loaded, for the next job of the same [`WarmKey`]: the same
//! kind, package version, tenant, language and memory limit. Each job is a
//! line on stdin and gets its own `started` and `done`. The memory limit,
//! the time limit and the cancel are per job; a kill ends the worker. The
//! agent leaves the worker's `TMPDIR` empty between jobs (it checks it, and
//! empties it when a job wrote there), and does not use a worker again whose
//! job changed its environment or left a process running. At its job cap a
//! worker is replaced by one that loaded the package in the background
//! (CLOACINA-T-1159). See [`WarmPool`].

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use cloacina::fleet::{AgentOutcome, GraphWorkPacket, PollWorkPacket, WorkPacket};
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

/// The CPU seconds one job may use: its time limit and the load allowance,
/// times the host's CPUs, plus 60 s. A worker that may take more jobs sets
/// it again before each one (CLOACINA-T-1126).
pub(crate) fn cpu_allowance(timeout_seconds: u32) -> u64 {
    let cpus = std::thread::available_parallelism().map_or(1, |n| n.get() as u64);
    let wall = u64::from(timeout_seconds.max(1)) + LOAD_ALLOWANCE.as_secs();
    wall.saturating_mul(cpus).saturating_add(60)
}

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
    /// Load the package and stop: the replacement of a warm worker at its
    /// job cap (CLOACINA-T-1159). Such a job has no secrets and no data.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub warm_up: bool,
    /// Tests only: what the job does in place of a packaged task.
    #[cfg(test)]
    #[serde(default)]
    pub test_action: Option<tests::TestAction>,
}

/// The job of one graph firing (CLOACINA-T-1125). A firing carries no
/// secrets: the server sends none for a graph.
#[derive(Serialize, Deserialize)]
pub(crate) struct GraphJob {
    /// The graph packet: the reactor's input cache, as source name → JSON.
    pub packet: GraphWorkPacket,
    pub package: PreparedPackage,
    /// Where a Python package is unpacked, as for a task.
    pub work_dir: PathBuf,
    /// `text` or `json`: the format of the agent's log lines.
    pub log_format: String,
    pub max_unpacked_bytes: u64,
    /// Load the package and stop (CLOACINA-T-1159), as for a task.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub warm_up: bool,
    /// Tests only: what the job does in place of a packaged graph.
    #[cfg(test)]
    #[serde(default)]
    pub test_action: Option<tests::TestAction>,
}

/// The job of one poll of a poll trigger (CLOACINA-T-1143). A poll carries
/// no secrets.
#[derive(Serialize, Deserialize)]
pub(crate) struct PollJob {
    pub packet: PollWorkPacket,
    pub package: PreparedPackage,
    /// Where a Python package is unpacked, as for a task.
    pub work_dir: PathBuf,
    /// `text` or `json`: the format of the agent's log lines.
    pub log_format: String,
    pub max_unpacked_bytes: u64,
    /// Tests only: what the job does in place of a packaged trigger.
    #[cfg(test)]
    #[serde(default)]
    pub test_action: Option<tests::TestAction>,
}

/// What a worker gets on stdin: one task, one graph firing, or one poll.
#[derive(Serialize, Deserialize)]
#[serde(tag = "job", rename_all = "snake_case")]
pub(crate) enum WorkerInput {
    Task(WorkerJob),
    Graph(GraphJob),
    Poll(PollJob),
}

/// What the agent needs to hold a worker to its limits.
struct JobInfo {
    /// The id a cancel names: the dispatch id of a task, the firing id of a
    /// firing, the poll id of a poll.
    stop_id: Option<String>,
    timeout_seconds: u32,
    /// The memory request. A firing and a poll have none.
    memory_request: Option<u64>,
    /// The task, graph or trigger name, for the log.
    name: String,
    /// "task", "graph firing" or "poll", for the error texts.
    noun: &'static str,
}

impl WorkerInput {
    fn info(&self) -> JobInfo {
        match self {
            WorkerInput::Task(job) => JobInfo {
                stop_id: job.packet.dispatch_id.clone(),
                timeout_seconds: job.packet.timeout_seconds,
                memory_request: job.packet.memory_bytes,
                name: job.packet.task_name.clone(),
                noun: "task",
            },
            WorkerInput::Graph(job) => JobInfo {
                stop_id: Some(job.packet.firing_id.clone()),
                timeout_seconds: job.packet.timeout_seconds,
                memory_request: None,
                name: job.packet.graph_name.clone(),
                noun: "graph firing",
            },
            WorkerInput::Poll(job) => JobInfo {
                stop_id: Some(job.packet.poll_id.clone()),
                timeout_seconds: job.packet.timeout_seconds,
                memory_request: None,
                name: job.packet.trigger_name.clone(),
                noun: "poll",
            },
        }
    }

    /// Give the job its own directory and the agent's settings.
    fn set_run(&mut self, work_dir: PathBuf, log_format: String, max_unpacked_bytes: u64) {
        let (dir, format, max) = match self {
            WorkerInput::Task(job) => (
                &mut job.work_dir,
                &mut job.log_format,
                &mut job.max_unpacked_bytes,
            ),
            WorkerInput::Graph(job) => (
                &mut job.work_dir,
                &mut job.log_format,
                &mut job.max_unpacked_bytes,
            ),
            WorkerInput::Poll(job) => (
                &mut job.work_dir,
                &mut job.log_format,
                &mut job.max_unpacked_bytes,
            ),
        };
        *dir = work_dir;
        *format = log_format;
        *max = max_unpacked_bytes;
    }

    fn max_unpacked_bytes(&self) -> u64 {
        match self {
            WorkerInput::Task(job) => job.max_unpacked_bytes,
            WorkerInput::Graph(job) => job.max_unpacked_bytes,
            WorkerInput::Poll(job) => job.max_unpacked_bytes,
        }
    }

    fn log_format(&self) -> &str {
        match self {
            WorkerInput::Task(job) => &job.log_format,
            WorkerInput::Graph(job) => &job.log_format,
            WorkerInput::Poll(job) => &job.log_format,
        }
    }

    /// A load-only copy of this job (CLOACINA-T-1159), for the worker that
    /// replaces one at its job cap: the same package, key and settings, and
    /// no secrets and no data of the job (no task context, no graph cache).
    /// `None` for a poll: a poll's worker is retired at the cap and the next
    /// poll starts a new one, as before. A poll waits for its interval, not
    /// for a hand-over, and a poll job may consume what it reads.
    fn warm_up(&self) -> Option<WorkerInput> {
        Some(match self {
            WorkerInput::Task(job) => {
                let mut packet = job.packet.clone();
                packet.context = serde_json::json!({});
                packet.wrapped_secrets = Vec::new();
                packet.secret_key_id = None;
                WorkerInput::Task(WorkerJob {
                    packet,
                    secrets: HashMap::new(),
                    package: job.package.clone(),
                    work_dir: job.work_dir.clone(),
                    log_format: job.log_format.clone(),
                    max_unpacked_bytes: job.max_unpacked_bytes,
                    warm_up: true,
                    #[cfg(test)]
                    test_action: job.test_action.clone(),
                })
            }
            WorkerInput::Graph(job) => {
                let mut packet = job.packet.clone();
                packet.cache = HashMap::new();
                packet.wrapped_secrets = Vec::new();
                packet.secret_key_id = None;
                WorkerInput::Graph(GraphJob {
                    packet,
                    package: job.package.clone(),
                    work_dir: job.work_dir.clone(),
                    log_format: job.log_format.clone(),
                    max_unpacked_bytes: job.max_unpacked_bytes,
                    warm_up: true,
                    #[cfg(test)]
                    test_action: job.test_action.clone(),
                })
            }
            WorkerInput::Poll(_) => return None,
        })
    }

    /// The jobs a warm worker of this job may take next (CLOACINA-T-1126):
    /// the same kind, package version, tenant, language and memory limit.
    /// A poll's key is of kind `poll` (CLOACINA-T-1143): a poll never takes
    /// the warm worker of a task or a firing of the same build.
    fn warm_key(&self, memory_limit: Option<u64>) -> WarmKey {
        match self {
            WorkerInput::Task(job) => WarmKey {
                kind: "task",
                digest: job.package.digest.clone(),
                // The tenant of the packet, and the tenant of the task's
                // namespace, which the load registers the tasks under.
                tenant: format!(
                    "{}|{}",
                    job.packet.tenant_id.as_deref().unwrap_or(""),
                    cloacina::parse_namespace(&job.packet.task_name)
                        .map(|ns| ns.tenant_id)
                        .unwrap_or_default()
                ),
                language: job.packet.language.clone().unwrap_or_default(),
                memory_limit,
            },
            WorkerInput::Graph(job) => WarmKey {
                kind: "graph",
                digest: job.package.digest.clone(),
                tenant: job.packet.tenant_id.clone().unwrap_or_default(),
                language: job.packet.language.clone().unwrap_or_default(),
                memory_limit,
            },
            WorkerInput::Poll(job) => WarmKey {
                kind: "poll",
                digest: job.package.digest.clone(),
                tenant: job.packet.tenant_id.clone().unwrap_or_default(),
                language: job.packet.language.clone().unwrap_or_default(),
                memory_limit,
            },
        }
    }
}

/// What a warm worker is kept for (CLOACINA-T-1126): it takes only jobs of
/// an equal key. Never another tenant's, package's or version's.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct WarmKey {
    kind: &'static str,
    /// The package version: a new version is a new digest.
    digest: String,
    tenant: String,
    language: String,
    /// The address-space ceiling of the worker follows it.
    memory_limit: Option<u64>,
}

/// How an agent keeps warm workers (CLOACINA-T-1126).
#[derive(Clone, Debug)]
pub(crate) struct WarmConfig {
    /// The most idle warm workers the agent keeps. 0: none; each job gets a
    /// new worker and the worker ends with it.
    pub max_idle: usize,
    /// A warm worker idle for longer is ended.
    pub idle_timeout: Duration,
    /// A worker that has run this many jobs is replaced: a new worker of
    /// its key loads the package, then the old one ends (CLOACINA-T-1159).
    pub max_jobs: u32,
    /// A worker that holds more than this share of its jobs' memory limit
    /// after a job, in percent, is ended.
    pub max_memory_percent: u64,
}

/// The defaults of the agent's flags.
pub(crate) const DEFAULT_WARM_IDLE_SECONDS: u64 = 60;
pub(crate) const DEFAULT_WARM_MAX_JOBS: u32 = 100;
pub(crate) const DEFAULT_WARM_MAX_MEMORY_PERCENT: u64 = 50;

impl WarmConfig {
    /// No warm workers: one worker per job.
    pub(crate) fn off() -> Self {
        Self {
            max_idle: 0,
            idle_timeout: Duration::from_secs(DEFAULT_WARM_IDLE_SECONDS),
            max_jobs: DEFAULT_WARM_MAX_JOBS,
            max_memory_percent: DEFAULT_WARM_MAX_MEMORY_PERCENT,
        }
    }

    fn enabled(&self) -> bool {
        self.max_idle > 0 && self.max_jobs > 1
    }
}

/// The warm workers of an agent and their counts (CLOACINA-T-1126).
pub(crate) struct WarmPool {
    pub(crate) config: WarmConfig,
    #[cfg(unix)]
    idle: std::sync::Mutex<Vec<spawn::Worker>>,
    counts: std::sync::Mutex<cloacina::fleet::AgentWorkerCounts>,
    sweeper: std::sync::atomic::AtomicBool,
}

impl WarmPool {
    pub(crate) fn new(config: WarmConfig) -> Arc<Self> {
        Arc::new(Self {
            config,
            #[cfg(unix)]
            idle: std::sync::Mutex::new(Vec::new()),
            counts: std::sync::Mutex::new(Default::default()),
            sweeper: std::sync::atomic::AtomicBool::new(false),
        })
    }

    fn count(&self, change: impl FnOnce(&mut cloacina::fleet::AgentWorkerCounts)) {
        change(&mut self.counts.lock().unwrap_or_else(|e| e.into_inner()));
    }

    fn count_retired(&self, reason: &str) {
        self.count(|c| *c.retired.entry(reason.to_string()).or_default() += 1);
    }

    /// The counts since the last call, for the heartbeat. `None`: nothing
    /// happened.
    pub(crate) fn take_counts(&self) -> Option<cloacina::fleet::AgentWorkerCounts> {
        let taken = std::mem::take(&mut *self.counts.lock().unwrap_or_else(|e| e.into_inner()));
        (!taken.is_empty()).then_some(taken)
    }

    /// Put back counts the server did not take.
    pub(crate) fn restore_counts(&self, counts: &cloacina::fleet::AgentWorkerCounts) {
        self.count(|c| c.add(counts));
    }

    /// How many idle warm workers the agent holds.
    #[cfg(all(unix, test))]
    pub(crate) fn idle_len(&self) -> usize {
        self.idle.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// The process ids of the idle warm workers, and their memory now.
    #[cfg(all(unix, test))]
    pub(crate) fn idle_memory(&self) -> Vec<(u32, Option<u64>)> {
        let pids: Vec<u32> = self
            .idle
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|w| w.pid())
            .collect();
        let used = crate::memory::worker_memory(&pids);
        pids.into_iter()
            .map(|pid| (pid, used.get(&pid).copied()))
            .collect()
    }
}

/// What a worker writes on its result pipe, one JSON object per line.
#[derive(Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub(crate) enum WorkerEvent {
    /// The package is loaded and the task starts now.
    Started,
    /// The task ended. `reusable`: the job left the worker as it found it
    /// (the same environment, no process of its own still running), so the
    /// worker may take the next job of the same key (CLOACINA-T-1126).
    Done {
        outcome: AgentOutcome,
        #[serde(default)]
        reusable: bool,
    },
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
#[derive(Clone)]
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
    /// The warm workers (CLOACINA-T-1126).
    pub warm: Arc<WarmPool>,
}

impl WorkerSettings {
    /// Workers of this binary with no memory figure: no limit by default, and
    /// no warm workers.
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
            warm: WarmPool::new(WarmConfig::off()),
        }
    }

    /// The memory limit of a job: its request, or its share of the agent. A
    /// graph firing has no request and gets the share (CLOACINA-T-1125).
    pub(crate) fn memory_limit_for(&self, request: Option<u64>) -> Option<u64> {
        request.or(self.default_memory_limit)
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
    /// Dispatches the agent stopped itself at the end of its drain
    /// (CLOACINA-T-1029): their work is handed back to the server.
    handed_back: HashSet<String>,
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

    /// The agent stops: stop every worker that still runs, and remember that
    /// its work is handed back, not cancelled by the server
    /// (CLOACINA-T-1029). Returns how many were told to stop.
    pub(crate) fn hand_back_all(&mut self) -> usize {
        let ids: Vec<String> = self.running.keys().cloned().collect();
        self.hand_back_ids(&ids)
    }

    /// [`hand_back_all`](Self::hand_back_all) for the dispatches `ids` only.
    pub(crate) fn hand_back_ids(&mut self, ids: &[String]) -> usize {
        let mut stopped = 0;
        for id in ids {
            if let Some(tx) = self.running.remove(id) {
                self.handed_back.insert(id.clone());
                if tx.send(()).is_ok() {
                    stopped += 1;
                }
            }
        }
        stopped
    }

    /// Whether a worker runs for the dispatch now.
    #[cfg(test)]
    pub(crate) fn is_running(&self, dispatch_id: &str) -> bool {
        self.running.contains_key(dispatch_id)
    }

    /// Whether the agent handed this dispatch back; forgets it.
    pub(crate) fn take_handed_back(&mut self, dispatch_id: &str) -> bool {
        self.handed_back.remove(dispatch_id)
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
        run_in_worker(WorkerInput::Task(job), settings).await
    }
    #[cfg(not(unix))]
    {
        let _ = settings;
        unreachable!("worker processes need a Unix system")
    }
}

/// Run a graph firing (CLOACINA-T-1125): in this process when the packet is
/// marked `inline`, else in a worker process, with the limits of a task.
pub(crate) async fn run_graph_here_or_in_worker(
    job: GraphJob,
    settings: &WorkerSettings,
) -> PacketEnd {
    if job.packet.inline || !cfg!(unix) {
        let limit = Duration::from_secs(u64::from(job.packet.timeout_seconds.max(1)));
        return PacketEnd::Report(crate::run_graph_job(job, Some(limit), || {}).await);
    }
    #[cfg(unix)]
    {
        run_in_worker(WorkerInput::Graph(job), settings).await
    }
    #[cfg(not(unix))]
    {
        let _ = settings;
        unreachable!("worker processes need a Unix system")
    }
}

/// Run a poll of a poll trigger (CLOACINA-T-1143) in a worker process, with
/// the limits of a firing: the agent's default memory share and the packet's
/// time limit. Without worker processes (not Unix) it runs in this process.
pub(crate) async fn run_poll_in_worker(job: PollJob, settings: &WorkerSettings) -> PacketEnd {
    if !cfg!(unix) {
        let limit = Duration::from_secs(u64::from(job.packet.timeout_seconds.max(1)));
        return PacketEnd::Report(crate::run_poll_job(job, Some(limit), || {}).await);
    }
    #[cfg(unix)]
    {
        run_in_worker(WorkerInput::Poll(job), settings).await
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

    /// The `setrlimit` ceilings of one worker.
    pub(crate) fn worker_rlimits(
        memory_limit: Option<u64>,
        timeout_seconds: u32,
        address_space_factor: u64,
    ) -> ChildRlimits {
        ChildRlimits {
            cpu_s: cpu_allowance(timeout_seconds),
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
    pub(crate) async fn run_in_worker(
        mut input: WorkerInput,
        settings: &WorkerSettings,
    ) -> PacketEnd {
        let info = input.info();
        let mut cancel = match info.stop_id.as_deref() {
            Some(id) => match dispatch_registry().begin(id) {
                Some(rx) => Some(rx),
                None => return PacketEnd::Cancelled,
            },
            None => None,
        };
        let end = supervise(&mut input, &info, settings, &mut cancel).await;
        if let Some(id) = info.stop_id.as_deref() {
            dispatch_registry().end(id);
        }
        end
    }

    /// A worker process of the agent. With warm workers on, it can take more
    /// than one job of its key, one at a time (CLOACINA-T-1126).
    pub(crate) struct Worker {
        child: tokio::process::Child,
        pid: u32,
        /// The worker reads its jobs here; when it closes, the worker ends.
        stdin: Option<tokio::process::ChildStdin>,
        events: mpsc::UnboundedReceiver<WorkerEvent>,
        /// The worker's own directory: the unpacked Python package, and
        /// `tmp`, its `TMPDIR`. Removed when the worker is retired.
        work_dir: PathBuf,
        tmp_dir: PathBuf,
        /// The jobs it may take next. `None`: it ends with its job.
        key: Option<WarmKey>,
        /// The jobs it was given.
        jobs: u32,
        /// It loaded its package with a load-only job, and has run no job
        /// since (CLOACINA-T-1159).
        loaded: bool,
        /// Past its job cap: the worker that replaces it, as it loads.
        replacement: Option<Arc<std::sync::atomic::AtomicU8>>,
        idle_since: Instant,
        /// While idle: the watch that counts its memory for the agent, and
        /// where the watchdog says it killed it.
        idle_watch: Option<crate::memory::WatchGuard>,
        idle_stop: Option<mpsc::UnboundedReceiver<StopReason>>,
    }

    /// How one job on a worker ended.
    enum JobEnd {
        /// The worker gave a result. `reusable`: it may take another job;
        /// `stopped`: the agent killed it (the result came first).
        Done {
            outcome: AgentOutcome,
            reusable: bool,
            stopped: bool,
        },
        /// The worker exited with no result, or the agent stopped it.
        Ended(PacketEnd),
        /// A warm worker that could not take the job: it died while idle.
        NotTaken,
    }

    async fn supervise(
        input: &mut WorkerInput,
        info: &JobInfo,
        settings: &WorkerSettings,
        cancel: &mut Option<oneshot::Receiver<()>>,
    ) -> PacketEnd {
        let memory_limit = settings.memory_limit_for(info.memory_request);
        let pool = &settings.warm;
        let key = pool.config.enabled().then(|| input.warm_key(memory_limit));
        // A warm worker of the key, else a new one.
        let mut warm = match &key {
            Some(key) => pool.take(key, memory_limit).await,
            None => None,
        };
        loop {
            let (mut worker, is_warm) = match warm.take() {
                Some(worker) => (worker, true),
                None => match start_worker(info, settings, memory_limit, key.clone()).await {
                    Ok(worker) => (worker, false),
                    Err(end) => return end,
                },
            };
            let end = run_on(&mut worker, input, info, settings, memory_limit, cancel).await;
            if !matches!(end, JobEnd::NotTaken) {
                pool.count(|c| {
                    if is_warm {
                        c.warm_starts += 1;
                    } else {
                        c.cold_starts += 1;
                    }
                });
            }
            match end {
                // It died while idle: a new worker takes the job.
                JobEnd::NotTaken => {
                    pool.count_retired("dead");
                    retire(worker).await;
                }
                JobEnd::Done {
                    outcome,
                    reusable,
                    stopped,
                } => {
                    if worker.key.is_none() || stopped {
                        retire(worker).await;
                    } else if !reusable {
                        debug!(
                            pid = worker.pid,
                            job = %info.name,
                            "the job changed the environment or left a process running; \
                             the worker is not used again"
                        );
                        pool.count_retired("dirty");
                        retire(worker).await;
                    } else {
                        pool.give_back(worker, memory_limit, settings, input, info)
                            .await;
                    }
                    return PacketEnd::Report(outcome);
                }
                JobEnd::Ended(end) => {
                    retire(worker).await;
                    return end;
                }
            }
        }
    }

    /// End a worker: kill its process group, wait for it, take what it left
    /// out of the process table, and remove its directory.
    async fn retire(mut worker: Worker) {
        worker.idle_watch = None;
        kill_child_group(worker.pid);
        worker.stdin = None;
        let _ = worker.child.wait().await;
        reap_group(worker.pid).await;
        let _ = tokio::fs::remove_dir_all(&worker.work_dir).await;
    }

    /// Whether the worker holds more than `percent` of `limit`.
    async fn over_memory(pid: u32, limit: Option<u64>, percent: u64) -> bool {
        let Some(limit) = limit else {
            return false;
        };
        let used = tokio::task::spawn_blocking(move || {
            crate::memory::worker_memory(&[pid]).get(&pid).copied()
        })
        .await
        .ok()
        .flatten();
        used.is_some_and(|used| used.saturating_mul(100) > limit.saturating_mul(percent))
    }

    /// The states of the replacement of a worker at its job cap.
    const REPLACEMENT_LOADING: u8 = 0;
    const REPLACEMENT_READY: u8 = 1;
    const REPLACEMENT_FAILED: u8 = 2;

    /// Whether `tmp` is a directory of its own (not a link) that its owner
    /// may use, with nothing in it: one `lstat` and one read of the
    /// directory, on this thread (CLOACINA-T-1159).
    pub(crate) fn tmp_is_clean(tmp: &std::path::Path) -> bool {
        use std::os::unix::fs::PermissionsExt;
        std::fs::symlink_metadata(tmp)
            .is_ok_and(|m| m.file_type().is_dir() && m.permissions().mode() & 0o700 == 0o700)
            && std::fs::read_dir(tmp).is_ok_and(|mut entries| entries.next().is_none())
    }

    /// Leave a worker's `TMPDIR` an empty directory, for the next job
    /// (CLOACINA-T-1126). A job that wrote nothing there costs one check: the
    /// directory stays as it is. Anything else (a file in it, a link or a
    /// file in its place, a directory the agent cannot read) is removed and
    /// the directory made again. The path stays: Python keeps the one it
    /// found. `false`: it could not be made empty; the worker must go.
    pub(crate) async fn reset_tmp(tmp: &std::path::Path) -> bool {
        if tmp_is_clean(tmp) {
            return true;
        }
        let tmp = tmp.to_path_buf();
        tokio::task::spawn_blocking(move || {
            match std::fs::symlink_metadata(&tmp) {
                Ok(meta) if meta.file_type().is_dir() => {
                    let _ = std::fs::remove_dir_all(&tmp);
                }
                Ok(_) => {
                    let _ = std::fs::remove_file(&tmp);
                }
                Err(_) => {}
            }
            let _ = std::fs::create_dir_all(&tmp);
            tmp_is_clean(&tmp)
        })
        .await
        .unwrap_or(false)
    }

    impl Worker {
        #[cfg(test)]
        pub(crate) fn pid(&self) -> u32 {
            self.pid
        }

        /// Why an idle worker must not take another job, from what is known
        /// without a read of its memory.
        fn unfit(&mut self, idle_timeout: Duration) -> Option<&'static str> {
            // The watchdog killed it for the agent's memory: it may not have
            // exited yet.
            if self
                .idle_stop
                .as_mut()
                .is_some_and(|stop| stop.try_recv().is_ok())
            {
                return Some("pressure");
            }
            if !matches!(self.child.try_wait(), Ok(None)) {
                return Some("dead");
            }
            if self.idle_since.elapsed() > idle_timeout {
                return Some("idle");
            }
            None
        }
    }

    impl WarmPool {
        /// An idle warm worker of `key`, the one used last. The workers of
        /// the key that are unfit are retired on the way.
        async fn take(
            self: &Arc<Self>,
            key: &WarmKey,
            memory_limit: Option<u64>,
        ) -> Option<Worker> {
            loop {
                let found = {
                    let mut idle = self.idle.lock().unwrap_or_else(|e| e.into_inner());
                    idle.iter()
                        .rposition(|w| w.key.as_ref() == Some(key))
                        .map(|at| idle.remove(at))
                };
                let mut worker = found?;
                let reason = match worker.unfit(self.config.idle_timeout) {
                    Some(reason) => Some(reason),
                    None if over_memory(
                        worker.pid,
                        memory_limit,
                        self.config.max_memory_percent,
                    )
                    .await =>
                    {
                        Some("memory")
                    }
                    None => None,
                };
                match reason {
                    None => return Some(worker),
                    Some(reason) => {
                        self.count_retired(reason);
                        retire(worker).await;
                    }
                }
            }
        }

        /// A worker whose job ended well and left it clean: keep it for the
        /// next job of its key, or retire it.
        ///
        /// At its job cap (CLOACINA-T-1159) a worker is not retired at once:
        /// a new worker of its key loads the package in the background, and
        /// the old one takes the jobs of its key until the new one is idle.
        /// So no job of the key waits for a cold start. If the new worker
        /// fails to load, the old one is retired on its next return.
        async fn give_back(
            self: &Arc<Self>,
            mut worker: Worker,
            memory_limit: Option<u64>,
            settings: &WorkerSettings,
            input: &WorkerInput,
            info: &JobInfo,
        ) {
            if over_memory(worker.pid, memory_limit, self.config.max_memory_percent).await {
                self.count_retired("memory");
                retire(worker).await;
                return;
            }
            if worker.jobs >= self.config.max_jobs {
                let state = worker
                    .replacement
                    .as_ref()
                    .map(|s| s.load(std::sync::atomic::Ordering::SeqCst));
                match state {
                    None => match input.warm_up() {
                        Some(job) => {
                            worker.replacement = Some(self.start_replacement(
                                settings,
                                input,
                                job,
                                info,
                                memory_limit,
                                worker.pid,
                            ));
                        }
                        // A poll: retired at the cap, as before.
                        None => {
                            self.count_retired("max_jobs");
                            retire(worker).await;
                            return;
                        }
                    },
                    Some(REPLACEMENT_LOADING) => {}
                    // Replaced, or the replacement failed: the old worker
                    // goes. Its job is done and it runs nothing: the report
                    // does not wait for its end.
                    Some(_) => {
                        self.count_retired("max_jobs");
                        tokio::spawn(retire(worker));
                        return;
                    }
                }
            }
            // Nothing a job wrote to its temporary directory is there for
            // the next one.
            if !reset_tmp(&worker.tmp_dir).await {
                self.count_retired("dirty");
                retire(worker).await;
                return;
            }
            self.park(worker, &settings.watchdog, None).await;
        }

        /// Put a worker on the idle list, watched for the agent's memory.
        /// `replaces`: the pid of an idle worker it takes the place of, and
        /// the state to set to [`REPLACEMENT_READY`] in the same step.
        async fn park(
            self: &Arc<Self>,
            mut worker: Worker,
            watchdog: &Arc<Watchdog>,
            replaces: Option<(u32, &std::sync::atomic::AtomicU8)>,
        ) {
            let (stop_tx, stop_rx) = mpsc::unbounded_channel();
            worker.idle_watch = Some(watchdog.watch_idle(worker.pid, stop_tx));
            worker.idle_stop = Some(stop_rx);
            worker.idle_since = Instant::now();
            let (replaced, evicted) = {
                let mut idle = self.idle.lock().unwrap_or_else(|e| e.into_inner());
                let replaced = replaces.and_then(|(pid, state)| {
                    state.store(REPLACEMENT_READY, std::sync::atomic::Ordering::SeqCst);
                    idle.iter()
                        .position(|w| w.pid == pid)
                        .map(|at| idle.remove(at))
                });
                idle.push(worker);
                let evicted = (idle.len() > self.config.max_idle).then(|| idle.remove(0));
                (replaced, evicted)
            };
            if let Some(old) = replaced {
                self.count_retired("max_jobs");
                retire(old).await;
            }
            if let Some(oldest) = evicted {
                self.count_retired("pool_full");
                retire(oldest).await;
            }
            self.start_sweeper();
        }

        /// Start the worker that replaces the worker `old_pid` at its job
        /// cap (CLOACINA-T-1159): a new process of the same key loads the
        /// package with a load-only job ([`WorkerInput::warm_up`]: no
        /// secrets, no job data), then takes the old one's place on the idle
        /// list. It holds no slot; while it loads, the watchdog holds it to
        /// the key's memory limit, and the load allowance bounds its time.
        fn start_replacement(
            self: &Arc<Self>,
            settings: &WorkerSettings,
            input: &WorkerInput,
            mut job: WorkerInput,
            info: &JobInfo,
            memory_limit: Option<u64>,
            old_pid: u32,
        ) -> Arc<std::sync::atomic::AtomicU8> {
            let state = Arc::new(std::sync::atomic::AtomicU8::new(REPLACEMENT_LOADING));
            let pool = self.clone();
            let settings = settings.clone();
            let info = JobInfo {
                stop_id: None,
                timeout_seconds: info.timeout_seconds,
                memory_request: info.memory_request,
                name: info.name.clone(),
                noun: info.noun,
            };
            let key = input.warm_key(memory_limit);
            let shared = state.clone();
            tokio::spawn(async move {
                let loaded = pool
                    .load_replacement(
                        &settings,
                        &mut job,
                        &info,
                        memory_limit,
                        key,
                        old_pid,
                        &shared,
                    )
                    .await;
                if !loaded {
                    shared.store(REPLACEMENT_FAILED, std::sync::atomic::Ordering::SeqCst);
                }
            });
            state
        }

        #[allow(clippy::too_many_arguments)]
        async fn load_replacement(
            self: &Arc<Self>,
            settings: &WorkerSettings,
            job: &mut WorkerInput,
            info: &JobInfo,
            memory_limit: Option<u64>,
            key: WarmKey,
            old_pid: u32,
            state: &std::sync::atomic::AtomicU8,
        ) -> bool {
            let mut worker = match start_worker(info, settings, memory_limit, Some(key)).await {
                Ok(worker) => worker,
                Err(_) => return false,
            };
            let mut no_cancel = None;
            let end = run_on(
                &mut worker,
                job,
                info,
                settings,
                memory_limit,
                &mut no_cancel,
            )
            .await;
            let loaded = matches!(
                end,
                JobEnd::Done {
                    outcome: AgentOutcome::Success { .. },
                    reusable: true,
                    stopped: false,
                }
            );
            if !loaded || !reset_tmp(&worker.tmp_dir).await {
                debug!(
                    pid = worker.pid,
                    job = %info.name,
                    "the replacement of a warm worker did not load its package"
                );
                retire(worker).await;
                return false;
            }
            worker.jobs = 0;
            worker.loaded = true;
            self.park(worker, &settings.watchdog, Some((old_pid, state)))
                .await;
            true
        }

        /// Retire idle workers past their idle time, and dead ones, on a
        /// timer. One per pool, started with its first idle worker.
        fn start_sweeper(self: &Arc<Self>) {
            if self.sweeper.swap(true, std::sync::atomic::Ordering::SeqCst) {
                return;
            }
            let pool = Arc::downgrade(self);
            let every = (self.config.idle_timeout / 4)
                .clamp(Duration::from_millis(50), Duration::from_secs(5));
            tokio::spawn(async move {
                let mut ticker = tokio::time::interval(every);
                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    ticker.tick().await;
                    let Some(pool) = pool.upgrade() else {
                        break;
                    };
                    pool.sweep().await;
                }
            });
        }

        async fn sweep(&self) {
            let gone: Vec<(Worker, &'static str)> = {
                let mut idle = self.idle.lock().unwrap_or_else(|e| e.into_inner());
                let mut gone = Vec::new();
                let mut at = 0;
                while at < idle.len() {
                    match idle[at].unfit(self.config.idle_timeout) {
                        Some(reason) => gone.push((idle.remove(at), reason)),
                        None => at += 1,
                    }
                }
                gone
            };
            for (worker, reason) in gone {
                debug!(pid = worker.pid, reason, "retiring a warm worker");
                self.count_retired(reason);
                retire(worker).await;
            }
        }
    }

    /// Start a worker process. `key`: it may take more jobs of this key.
    async fn start_worker(
        info: &JobInfo,
        settings: &WorkerSettings,
        memory_limit: Option<u64>,
        key: Option<WarmKey>,
    ) -> Result<Worker, PacketEnd> {
        let noun = info.noun;
        // A directory of its own: the Python package is unpacked there, and
        // the worker's TMPDIR is in it. Removed when the worker is retired.
        let work_dir = settings.work_root.join(uuid_like());
        let tmp_dir = work_dir.join("tmp");
        if let Err(e) = tokio::fs::create_dir_all(&tmp_dir).await {
            return Err(failure(
                FailureClassification::Transient,
                format!("create the worker directory {work_dir:?}: {e}"),
            ));
        }

        let (result_read, result_write) = match std::io::pipe() {
            Ok(pipe) => pipe,
            Err(e) => {
                return Err(failure(
                    FailureClassification::Transient,
                    format!("open the result pipe of the worker: {e}"),
                ));
            }
        };
        let result_fd = result_write.as_raw_fd();
        let mut rlimits = worker_rlimits(
            memory_limit,
            info.timeout_seconds,
            settings.address_space_factor,
        );
        // CPU seconds count for the life of the process. A worker that may
        // take more jobs gets no ceiling here; it sets its own soft limit
        // before each job (`limit_cpu_for_job`).
        if key.is_some() {
            rlimits.cpu_s = u64::MAX;
        }
        let rlimits = rlimits.capped_at_own_limits();
        let mut policy = ChildPolicy::worker(rlimits)
            .env(RESULT_FD_ENV, result_fd.to_string())
            .env("TMPDIR", &tmp_dir);
        for (name, value) in &settings.env {
            policy = policy.env(name, value);
        }
        let Some(program) = settings.program.to_str() else {
            return Err(failure(
                FailureClassification::Transient,
                "the path of the agent binary is not UTF-8".to_string(),
            ));
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

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                return Err(failure(
                    FailureClassification::Transient,
                    format!("start the {noun}'s worker process: {e}"),
                ));
            }
        };
        drop(result_write);
        let pid = child.id().unwrap_or(0);
        // The jobs go in on stdin. The pipe stays open for the life of the
        // worker: the worker reads its end to learn that the agent is gone.
        let stdin = child.stdin.take();

        // The worker's events, read on a blocking thread for its life.
        let (event_tx, events) = mpsc::unbounded_channel::<WorkerEvent>();
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
        Ok(Worker {
            child,
            pid,
            stdin,
            events,
            work_dir,
            tmp_dir,
            key,
            jobs: 0,
            loaded: false,
            replacement: None,
            idle_since: Instant::now(),
            idle_watch: None,
            idle_stop: None,
        })
    }

    /// Run one job on a worker and hold it to the job's limits. Returns when
    /// the worker gave its result, or when it has exited.
    async fn run_on(
        worker: &mut Worker,
        input: &mut WorkerInput,
        info: &JobInfo,
        settings: &WorkerSettings,
        memory_limit: Option<u64>,
        cancel: &mut Option<oneshot::Receiver<()>>,
    ) -> JobEnd {
        let noun = info.noun;
        let pid = worker.pid;
        let time_limit = Duration::from_secs(u64::from(info.timeout_seconds.max(1)));
        input.set_run(
            worker.work_dir.clone(),
            settings.log_format.to_string(),
            settings.max_unpacked_bytes,
        );
        let mut line = match serde_json::to_vec(&*input) {
            Ok(line) => line,
            Err(e) => {
                return JobEnd::Ended(failure(
                    FailureClassification::Validation,
                    format!("serialize the worker job: {e}"),
                ));
            }
        };
        line.push(b'\n');

        // The idle watch ends; the job's watch, with its limit, starts.
        worker.idle_watch = None;
        worker.idle_stop = None;
        let (stop_tx, mut stop_rx) = mpsc::unbounded_channel::<StopReason>();
        let _watch = settings.watchdog.watch(pid, memory_limit, stop_tx);

        let taken = match worker.stdin.as_mut() {
            Some(pipe) => match pipe.write_all(&line).await {
                Ok(()) => pipe.flush().await.is_ok(),
                Err(e) => {
                    debug!(pid, error = %e, "the worker did not take its job");
                    false
                }
            },
            None => false,
        };
        // The line holds the secrets: do not keep it.
        line.iter_mut().for_each(|b| *b = 0);
        drop(line);
        if !taken && (worker.jobs > 0 || worker.loaded) {
            return JobEnd::NotTaken;
        }
        worker.jobs += 1;
        worker.loaded = false;

        let oom_kills_before = oom_kill_count();
        let job_start = Instant::now();
        let hard_deadline = job_start + time_limit + LOAD_ALLOWANCE;
        let mut deadline = hard_deadline;
        let mut started = false;
        let mut stop: Option<StopReason> = None;
        let mut events_open = true;
        let status = loop {
            tokio::select! {
                status = worker.child.wait() => break status,
                event = worker.events.recv(), if events_open => match event {
                    Some(WorkerEvent::Started) => {
                        started = true;
                        deadline = (Instant::now() + time_limit).min(hard_deadline);
                    }
                    // The result stands, even when the agent was stopping
                    // the worker at the same time.
                    Some(WorkerEvent::Done { outcome, reusable }) => {
                        return JobEnd::Done {
                            outcome,
                            reusable,
                            stopped: stop.is_some(),
                        };
                    }
                    None => events_open = false,
                },
                Some(reason) = stop_rx.recv(), if stop.is_none() => {
                    info!(
                        pid,
                        job = %info.name,
                        kind = noun,
                        reason = ?reason,
                        "stopping the worker"
                    );
                    stop = Some(reason);
                    kill_child_group(pid);
                }
                _ = tokio::time::sleep_until(deadline.into()), if stop.is_none() => {
                    info!(
                        pid,
                        job = %info.name,
                        kind = noun,
                        limit_s = info.timeout_seconds,
                        "past its time limit; stopping the worker"
                    );
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
                            job = %info.name,
                            kind = noun,
                            dispatch_id = ?info.stop_id,
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
        worker.stdin = None;
        reap_group(pid).await;

        // A result written just before the exit may still be on its way.
        if stop.is_none() && events_open {
            let events = &mut worker.events;
            let drain = async {
                while let Some(event) = events.recv().await {
                    if let WorkerEvent::Done { outcome, .. } = event {
                        return Some(outcome);
                    }
                }
                None
            };
            if let Some(outcome) = tokio::time::timeout(Duration::from_secs(2), drain)
                .await
                .ok()
                .flatten()
            {
                return JobEnd::Done {
                    outcome,
                    reusable: false,
                    stopped: true,
                };
            }
        }
        JobEnd::Ended(match stop {
            Some(StopReason::Cancelled) => PacketEnd::Cancelled,
            Some(StopReason::OverLimit { used, limit }) => failure(
                FailureClassification::TaskError,
                format!(
                    "{noun} exceeded its memory limit of {}: it used {}, and the agent stopped it",
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
                    "the agent ran out of memory ({} of {} in use): it stopped this {noun}, its \
                     largest, which used {}",
                    format_bytes(agent_used),
                    format_bytes(agent_total),
                    format_bytes(used)
                ),
            ),
            Some(StopReason::TimeLimit) if started => failure(
                FailureClassification::Timeout,
                format!("{noun} exceeded timeout of {}s", info.timeout_seconds),
            ),
            Some(StopReason::TimeLimit) => failure(
                FailureClassification::Timeout,
                format!(
                    "{noun} did not start within {}s (its time limit of {}s and {}s to load the \
                     package)",
                    (time_limit + LOAD_ALLOWANCE).as_secs(),
                    info.timeout_seconds,
                    LOAD_ALLOWANCE.as_secs()
                ),
            ),
            None => {
                let status = match status {
                    Ok(status) => status,
                    Err(e) => {
                        return JobEnd::Ended(failure(
                            FailureClassification::Transient,
                            format!("wait for the {noun}'s worker process: {e}"),
                        ));
                    }
                };
                let oom_killed = matches!(
                    (oom_kills_before, oom_kill_count()),
                    (Some(before), Some(after)) if after > before
                );
                warn!(
                    pid,
                    job = %info.name,
                    kind = noun,
                    status = %status,
                    oom_killed,
                    "the worker ended with no result"
                );
                if status.signal() == Some(libc::SIGKILL) && oom_killed {
                    return JobEnd::Ended(failure(
                        FailureClassification::Transient,
                        format!(
                            "the agent ran out of memory: the kernel killed the {noun}'s worker"
                        ),
                    ));
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
                    format!("the {noun}'s worker process ended with no result: {why}"),
                )
            }
        })
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

/// The entry of the agent binary in worker mode. Reads a job from stdin,
/// runs it, writes the outcome to the result pipe, and waits for the next
/// job; the agent sends one only of the same key (CLOACINA-T-1126). Ends
/// when stdin closes.
#[cfg(unix)]
pub(crate) fn worker_main() -> ! {
    use std::io::{BufRead, Write};
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

    // The jobs come on stdin, one line each, read on a thread of their own.
    // stdin closes when the agent is gone, or when the agent retires this
    // worker: end this worker and what its jobs started.
    let (job_tx, job_rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut stdin = std::io::stdin().lock();
        loop {
            let mut line = String::new();
            match stdin.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if job_tx.send(line).is_err() {
                        break;
                    }
                }
            }
        }
        // SAFETY: `killpg` has no memory-safety preconditions. The worker
        // leads its own process group.
        unsafe {
            libc::killpg(libc::getpgrp(), libc::SIGKILL);
        }
    });

    // The kernel picks a worker, not the agent, when the pod is out of
    // memory; and the agent gets the CPU before its workers do.
    #[cfg(target_os = "linux")]
    let _ = std::fs::write("/proc/self/oom_score_adj", "1000");
    // SAFETY: `setpriority` has no memory-safety preconditions.
    unsafe {
        libc::setpriority(libc::PRIO_PROCESS, 0, WORKER_NICE);
    }

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
    let mut first = true;
    loop {
        let Ok(line) = job_rx.recv() else {
            // The reader is ending this process.
            std::process::exit(0)
        };
        let parsed = serde_json::from_str::<WorkerInput>(&line);
        // The line holds the job's secrets: do not keep it.
        let mut bytes = line.into_bytes();
        bytes.iter_mut().for_each(|b| *b = 0);
        drop(bytes);
        let job = match parsed {
            Ok(job) => job,
            Err(e) => fail(&format!("the job on stdin does not parse: {e}")),
        };
        if first {
            first = false;
            let _ = cloacina::cloacina_workflow_plugin::unpack_limit::set_max_unpacked_bytes(
                job.max_unpacked_bytes(),
            );
            let log_format = job.log_format().parse().unwrap_or(LogFormat::Text);
            runtime.block_on(async {
                let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
                let filter = tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
                let _ = match log_format {
                    LogFormat::Text => tracing_subscriber::fmt().with_env_filter(filter).try_init(),
                    LogFormat::Json => tracing_subscriber::fmt()
                        .json()
                        .with_env_filter(filter)
                        .try_init(),
                };
                cloacina_python::install();
            });
        }
        #[cfg(target_os = "linux")]
        limit_cpu_for_job(cpu_allowance(job.info().timeout_seconds));

        // CLOACINA-T-1126: what a job could leave for the next one.
        let env_before = environment();
        let outcome = runtime.block_on(async {
            let started = || write_event(&WorkerEvent::Started);
            match job {
                WorkerInput::Task(job) => crate::run_job(job, None, started).await,
                WorkerInput::Graph(job) => crate::run_graph_job(job, None, started).await,
                WorkerInput::Poll(job) => crate::run_poll_job(job, None, started).await,
            }
        });
        let reusable = environment() == env_before && !has_live_children();
        write_event(&WorkerEvent::Done { outcome, reusable });
    }
}

/// The environment of this process, to compare before and after a job.
#[cfg(unix)]
fn environment() -> BTreeMap<std::ffi::OsString, std::ffi::OsString> {
    std::env::vars_os().collect()
}

/// Whether a process this worker started is still running. Children that
/// ended are taken out of the process table on the way.
#[cfg(unix)]
fn has_live_children() -> bool {
    loop {
        // SAFETY: `waitpid` for any child, no status wanted, no blocking.
        let got = unsafe { libc::waitpid(-1, std::ptr::null_mut(), libc::WNOHANG) };
        match got {
            // One that ended: look again.
            n if n > 0 => {}
            // Children, none of which has ended.
            0 => return true,
            // No children.
            _ => return false,
        }
    }
}

/// Set the soft CPU-time limit of this worker to what it used so far plus
/// the allowance of one job (CLOACINA-T-1126). The hard limit stays.
#[cfg(target_os = "linux")]
fn limit_cpu_for_job(allowance_s: u64) {
    // SAFETY: `getrusage` and `getrlimit` write one struct each to the
    // pointer; `setrlimit` reads one.
    unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        if libc::getrusage(libc::RUSAGE_SELF, &mut usage) != 0 {
            return;
        }
        let used = (usage.ru_utime.tv_sec.max(0) + usage.ru_stime.tv_sec.max(0)) as u64 + 1;
        let mut own = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if libc::getrlimit(libc::RLIMIT_CPU, &mut own) != 0 {
            return;
        }
        let soft = used.saturating_add(allowance_s).min(own.rlim_max);
        let limit = libc::rlimit {
            rlim_cur: soft,
            rlim_max: own.rlim_max,
        };
        libc::setrlimit(libc::RLIMIT_CPU, &limit);
    }
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
        /// Write a file named `file` in the temporary directory.
        WriteTemp { file: String },
        /// Set the environment variable `name` to `value`.
        SetEnv { name: String, value: String },
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
                    "tmp_files": std::fs::read_dir(std::env::temp_dir())
                        .map(|entries| entries
                            .flatten()
                            .map(|e| e.file_name().to_string_lossy().into_owned())
                            .collect::<Vec<_>>())
                        .unwrap_or_default(),
                }))
            }
            TestAction::WriteTemp { file } => {
                std::fs::write(std::env::temp_dir().join(file), "left by a job").unwrap();
                ok(serde_json::json!({ "pid": std::process::id() }))
            }
            TestAction::SetEnv { name, value } => {
                // SAFETY: a test job; no other thread of the worker reads
                // the environment while it runs.
                unsafe { std::env::set_var(name, value) };
                ok(serde_json::json!({ "pid": std::process::id() }))
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
            warm_up: false,
            test_action: Some(action),
        }
    }

    /// A graph firing that does `action` in place of a packaged graph.
    pub(crate) fn graph_job(name: &str, action: Option<TestAction>) -> GraphJob {
        GraphJob {
            packet: GraphWorkPacket {
                protocol_version: AGENT_PROTOCOL_VERSION,
                firing_id: format!("firing-{name}"),
                graph_name: format!("{name}_rx"),
                cache: HashMap::new(),
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
                inline: false,
            },
            package: PreparedPackage {
                digest: "none".into(),
                library: None,
                source_archive: None,
                vendored_set: None,
                providers: Providers::None,
            },
            work_dir: PathBuf::new(),
            log_format: "text".into(),
            max_unpacked_bytes:
                cloacina::cloacina_workflow_plugin::unpack_limit::DEFAULT_MAX_UNPACKED_BYTES,
            warm_up: false,
            test_action: action,
        }
    }

    /// Pack a Python graph package: a passthrough accumulator `<name>_src`,
    /// a reactor `<name>_rx`, and a graph whose one node writes its process
    /// id to the file named by the `pid_file` field of its input.
    pub(crate) fn python_graph_package(dir: &std::path::Path, name: &str) -> PreparedPackage {
        let source = dir.join(format!("{name}-src"));
        let module = source.join("workflow").join(name);
        std::fs::create_dir_all(&module).unwrap();
        std::fs::write(
            source.join("package.toml"),
            format!(
                "[package]\nname = \"{pkg}\"\nversion = \"0.1.0\"\n\
                 interface = \"cloacina-workflow-plugin\"\ninterface_version = 1\n\
                 extension = \"cloacina\"\n\n[metadata]\ngraph_name = \"{name}_graph\"\n\
                 language = \"python\"\ndescription = \"worker test\"\n\
                 entry_module = \"{name}.graph\"\nreaction_mode = \"when_any\"\n\
                 input_strategy = \"latest\"\n",
                pkg = name.replace('_', "-"),
            ),
        )
        .unwrap();
        std::fs::write(module.join("__init__.py"), "").unwrap();
        std::fs::write(
            module.join("graph.py"),
            format!(
                "import os\nimport cloaca\n\n\n@cloaca.passthrough_accumulator\n\
                 def {name}_src(event):\n    return event\n\n\n\
                 @cloaca.reactor(name=\"{name}_rx\", accumulators=[\"{name}_src\"], mode=\"when_any\")\n\
                 class _Reactor:\n    pass\n\n\n\
                 with cloaca.ComputationGraphBuilder(\n    \"{name}_graph\",\n    reactor=_Reactor,\n\
                 \x20   graph={{\"record\": {{\"inputs\": [\"{name}_src\"]}}}},\n) as builder:\n\n\
                 \x20   @cloaca.node\n    def record({name}_src):\n\
                 \x20       with open({name}_src[\"pid_file\"], \"w\") as f:\n\
                 \x20           f.write(str(os.getpid()))\n\
                 \x20       return {{\"pid\": os.getpid()}}\n"
            ),
        )
        .unwrap();
        let archive = dir.join(format!("{name}.cloacina"));
        fidius_core::package::pack_package(&source, Some(&archive)).unwrap();
        PreparedPackage {
            digest: format!("{name}-{}", std::process::id()),
            library: None,
            source_archive: Some(archive),
            vendored_set: None,
            providers: Providers::None,
        }
    }

    /// A firing of the Python graph package `graph` (the `name` given to
    /// [`python_graph_package`]) that writes its pid to `pid_file`.
    pub(crate) fn python_graph_job(
        job_name: &str,
        graph: &str,
        package: &PreparedPackage,
        pid_file: &std::path::Path,
    ) -> GraphJob {
        let mut one = graph_job(job_name, None);
        one.package = package.clone();
        one.packet.language = Some("python".into());
        one.packet.graph_name = format!("{graph}_rx");
        one.packet.cache.insert(
            format!("{graph}_src"),
            serde_json::json!({ "pid_file": pid_file }).to_string(),
        );
        one
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

    // ── Warm workers (CLOACINA-T-1126) ────────────────────────────────────

    fn warm_config() -> WarmConfig {
        WarmConfig {
            max_idle: 4,
            idle_timeout: Duration::from_secs(60),
            max_jobs: 100,
            max_memory_percent: 50,
        }
    }

    /// Settings with warm workers on.
    fn warm_settings(name: &str, config: WarmConfig) -> WorkerSettings {
        WorkerSettings {
            warm: WarmPool::new(config),
            ..test_settings(name, fast_watchdog(AgentMemory::Unknown))
        }
    }

    fn pid_of(end: PacketEnd) -> u64 {
        success(end)["pid"].as_u64().expect("a pid")
    }

    fn counts(settings: &WorkerSettings) -> cloacina::fleet::AgentWorkerCounts {
        let counts = settings.warm.take_counts().unwrap_or_default();
        settings.warm.restore_counts(&counts);
        counts
    }

    /// The second job of one package version reuses the worker, which has
    /// the package loaded, and is faster.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_second_job_of_the_same_package_reuses_the_warm_worker_and_is_faster() {
        let settings = warm_settings("warm-reuse", warm_config());
        std::fs::create_dir_all(&settings.work_root).unwrap();
        let (package, task) = python_package(&settings.work_root);

        let at = std::time::Instant::now();
        let first = run_here_or_in_worker(python_job("warm-1", &package, &task), &settings).await;
        let cold = at.elapsed();
        let at = std::time::Instant::now();
        let second = run_here_or_in_worker(python_job("warm-2", &package, &task), &settings).await;
        let warm = at.elapsed();
        println!("python task: cold worker {cold:?}, warm worker {warm:?}");

        let (first, second) = (pid_of(first), pid_of(second));
        assert_ne!(first, u64::from(std::process::id()), "a worker ran it");
        assert_eq!(first, second, "the second job ran in the same worker");
        assert!(
            warm < cold,
            "warm {warm:?} is not faster than cold {cold:?}"
        );
        let counts = counts(&settings);
        assert_eq!((counts.cold_starts, counts.warm_starts), (1, 1));
        assert_eq!(settings.warm.idle_len(), 1);
    }

    /// Another package, version, tenant, memory limit or kind of job never
    /// takes the warm worker.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_job_of_another_package_version_or_tenant_never_takes_the_warm_worker() {
        // Room for every key below, so that none is pushed out.
        let settings = warm_settings(
            "warm-keys",
            WarmConfig {
                max_idle: 8,
                ..warm_config()
            },
        );
        let report = || TestAction::Report { secret: None };
        let base = pid_of(run_here_or_in_worker(job("k-base", report()), &settings).await);

        let mut other_version = job("k-version", report());
        other_version.package.digest = "another-digest".into();
        let mut other_tenant = job("k-tenant", report());
        other_tenant.packet.tenant_id = Some("acme".into());
        let mut other_tenant_ns = job("k-tenant-ns", report());
        other_tenant_ns.packet.task_name = "acme::pkg::wf::k".into();
        let mut other_limit = job("k-limit", report());
        other_limit.packet.memory_bytes = Some(300 << 20);
        for (what, one) in [
            ("version", other_version),
            ("tenant", other_tenant),
            ("namespace tenant", other_tenant_ns),
            ("memory limit", other_limit),
        ] {
            let pid = pid_of(run_here_or_in_worker(one, &settings).await);
            assert_ne!(pid, base, "a job of another {what} took the warm worker");
        }
        let graph = pid_of(
            run_graph_here_or_in_worker(graph_job("k_graph", Some(report())), &settings).await,
        );
        assert_ne!(graph, base, "a graph firing took a task's warm worker");

        let again = pid_of(run_here_or_in_worker(job("k-again", report()), &settings).await);
        assert_eq!(again, base, "a job of the same key takes it");
    }

    /// A job over its memory limit is killed with its worker; the next job
    /// of the key gets a fresh one.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn a_job_over_its_memory_limit_ends_the_warm_worker() {
        let settings = warm_settings("warm-memory", warm_config());
        let limited = |name: &str, action: TestAction| {
            let mut one = job(name, action);
            one.packet.memory_bytes = Some(200 << 20);
            one
        };
        let first = pid_of(
            run_here_or_in_worker(
                limited("m-1", TestAction::Report { secret: None }),
                &settings,
            )
            .await,
        );
        let (class, message) = failure(
            run_here_or_in_worker(
                limited(
                    "m-2",
                    TestAction::Allocate {
                        bytes: 600 << 20,
                        hold_ms: 20_000,
                    },
                ),
                &settings,
            )
            .await,
        );
        assert_eq!(class, FailureClassification::TaskError);
        assert!(
            message.contains("exceeded its memory limit of 200Mi"),
            "{message}"
        );
        assert_eq!(counts(&settings).warm_starts, 1, "the warm worker took it");
        assert!(
            !is_alive(first as u32),
            "the warm worker was killed with it"
        );
        let next = pid_of(
            run_here_or_in_worker(
                limited("m-3", TestAction::Report { secret: None }),
                &settings,
            )
            .await,
        );
        assert_ne!(next, first, "the next job got a fresh worker");
    }

    /// A time-limit kill and a cancel end the warm worker.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_time_limit_kill_and_a_cancel_end_the_warm_worker() {
        let settings = Arc::new(warm_settings("warm-kill", warm_config()));
        let report = || TestAction::Report { secret: None };
        let first = pid_of(run_here_or_in_worker(job("t-1", report()), &settings).await);
        let mut slow = job("t-2", TestAction::Block { ms: 60_000 });
        slow.packet.timeout_seconds = 1;
        let (class, _) = failure(run_here_or_in_worker(slow, &settings).await);
        assert_eq!(class, FailureClassification::Timeout);
        assert!(
            !is_alive(first as u32),
            "the time limit ended the warm worker"
        );

        let second = pid_of(run_here_or_in_worker(job("t-3", report()), &settings).await);
        assert_ne!(second, first);
        let long = job("t-4", TestAction::Block { ms: 60_000 });
        let run = {
            let settings = settings.clone();
            tokio::spawn(async move { run_here_or_in_worker(long, &settings).await })
        };
        // The cancel is for a job that runs: wait until it is registered.
        let mut told = false;
        for _ in 0..200 {
            if dispatch_registry().running.contains_key("dispatch-t-4") {
                told = dispatch_registry().cancel("dispatch-t-4");
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(told, "the cancel reached the job");
        assert!(matches!(run.await.unwrap(), PacketEnd::Cancelled));
        assert!(!is_alive(second as u32), "the cancel ended the warm worker");
        let third = pid_of(run_here_or_in_worker(job("t-5", report()), &settings).await);
        assert_ne!(third, second);
        assert_eq!(counts(&settings).warm_starts, 2);
    }

    /// Nothing a job writes to its temporary directory or sets in its
    /// environment is seen by the next job; a job that leaves a process
    /// running retires its worker.
    #[cfg(unix)]
    #[tokio::test]
    async fn the_next_job_does_not_see_the_temp_files_environment_or_processes_of_the_last() {
        let settings = warm_settings("warm-clean", warm_config());
        let report = || TestAction::Report { secret: None };
        let first = pid_of(
            run_here_or_in_worker(
                job(
                    "c-1",
                    TestAction::WriteTemp {
                        file: "left-over.txt".into(),
                    },
                ),
                &settings,
            )
            .await,
        );
        let seen = success(run_here_or_in_worker(job("c-2", report()), &settings).await);
        assert_eq!(seen["pid"].as_u64(), Some(first), "the same worker");
        assert_eq!(seen["tmp_files"], serde_json::json!([]), "{seen}");

        let set = pid_of(
            run_here_or_in_worker(
                job(
                    "c-3",
                    TestAction::SetEnv {
                        name: "T1126_LEFT_BY_A_JOB".into(),
                        value: "yes".into(),
                    },
                ),
                &settings,
            )
            .await,
        );
        assert_eq!(set, first, "the same worker ran it");
        let after = success(run_here_or_in_worker(job("c-4", report()), &settings).await);
        assert_ne!(after["pid"].as_u64(), Some(first), "the worker was retired");
        assert!(
            !after["env"].to_string().contains("T1126_LEFT_BY_A_JOB"),
            "{after}"
        );

        let child_pid = settings.work_root.join("child.pid");
        let left = pid_of(
            run_here_or_in_worker(
                job(
                    "c-5",
                    TestAction::StartChild {
                        pid_file: child_pid.clone(),
                        ms: 0,
                    },
                ),
                &settings,
            )
            .await,
        );
        let next = pid_of(run_here_or_in_worker(job("c-6", report()), &settings).await);
        assert_ne!(next, left, "a job that left a process retired its worker");
        let grandchild = read_pid(&child_pid).await;
        let gone = (0..200).any(|_| {
            let alive = is_alive(grandchild);
            if alive {
                std::thread::sleep(Duration::from_millis(25));
            }
            !alive
        });
        assert!(gone, "the process the job left was killed with its worker");
        assert_eq!(counts(&settings).retired.get("dirty"), Some(&2));
    }

    /// A warm worker is retired after its idle time, and after its job cap.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_warm_worker_is_retired_after_its_idle_time_and_its_job_cap() {
        let settings = warm_settings(
            "warm-retire",
            WarmConfig {
                idle_timeout: Duration::from_millis(300),
                max_jobs: 2,
                ..warm_config()
            },
        );
        let report = || TestAction::Report { secret: None };
        let first = pid_of(run_here_or_in_worker(job("r-1", report()), &settings).await);
        assert_eq!(settings.warm.idle_len(), 1);
        tokio::time::sleep(Duration::from_millis(1_000)).await;
        assert_eq!(settings.warm.idle_len(), 0, "retired when idle");
        assert!(!is_alive(first as u32), "the idle worker has ended");

        let second = pid_of(run_here_or_in_worker(job("r-2", report()), &settings).await);
        assert_ne!(second, first);
        let third = pid_of(run_here_or_in_worker(job("r-3", report()), &settings).await);
        assert_eq!(third, second, "its second job");
        // CLOACINA-T-1159: at its cap the worker is replaced by one that
        // loaded in the background; then the old one is retired.
        let replaced = wait_for(|| {
            let idle = settings.warm.idle_memory();
            idle.len() == 1 && idle[0].0 as u64 != third && !is_alive(third as u32)
        })
        .await;
        assert!(replaced, "replaced and retired at its job cap");
        let fourth = pid_of(run_here_or_in_worker(job("r-4", report()), &settings).await);
        assert_ne!(fourth, third);
        let counts = counts(&settings);
        assert_eq!(counts.retired.get("idle"), Some(&1));
        assert_eq!(counts.retired.get("max_jobs"), Some(&1));
        assert_eq!(
            (counts.cold_starts, counts.warm_starts),
            (2, 2),
            "the job after the cap ran on the loaded replacement"
        );
    }

    /// Wait up to 10 s for `done`.
    #[cfg(unix)]
    async fn wait_for(mut done: impl FnMut() -> bool) -> bool {
        for _ in 0..400 {
            if done() {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        false
    }

    /// CLOACINA-T-1159: a run of jobs over the job cap pays no cold start
    /// after the first: the old worker takes jobs until its replacement has
    /// loaded, then the replacement takes them. Each old worker ends.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_worker_at_its_job_cap_is_replaced_without_a_cold_start() {
        let settings = warm_settings(
            "warm-cap",
            WarmConfig {
                max_jobs: 3,
                ..warm_config()
            },
        );
        let report = || TestAction::Report { secret: None };
        let mut pids = Vec::new();
        for n in 0..40 {
            pids.push(pid_of(
                run_here_or_in_worker(job(&format!("cap-{n}"), report()), &settings).await,
            ));
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let mut seen: Vec<u64> = Vec::new();
        for pid in &pids {
            if seen.last() != Some(pid) {
                assert!(!seen.contains(pid), "a retired worker came back: {pids:?}");
                seen.push(*pid);
            }
        }
        assert!(seen.len() >= 2, "the worker was replaced: {pids:?}");
        let counts = counts(&settings);
        assert_eq!(
            counts.cold_starts, 1,
            "no cold start after the first: {pids:?}"
        );
        assert_eq!(counts.warm_starts, 39);
        let last = *seen.last().unwrap();
        let ended = wait_for(|| {
            seen.iter()
                .filter(|p| **p != last)
                .all(|p| !is_alive(*p as u32))
        })
        .await;
        assert!(ended, "every replaced worker ended");
    }

    /// CLOACINA-T-1159: the load-only job of a replacement carries no
    /// secrets and no data of the job it follows.
    #[test]
    fn the_load_only_job_has_no_secrets_and_no_job_data() {
        const VALUE: &str = "s3cr3t-value-of-t-1159";
        let mut task = job("w-task", TestAction::Report { secret: None });
        task.secrets.insert(
            "warehouse".into(),
            [("password".to_string(), VALUE.to_string())].into(),
        );
        task.packet.context = serde_json::json!({ "input": VALUE });
        let mut graph = graph_job("w_graph", None);
        graph.packet.cache.insert("src".into(), VALUE.into());
        for input in [WorkerInput::Task(task), WorkerInput::Graph(graph)] {
            let warm = input.warm_up().expect("a load-only job");
            let line = serde_json::to_string(&warm).unwrap();
            assert!(!line.contains(VALUE), "{line}");
            assert!(line.contains("\"warm_up\":true"), "{line}");
            assert_eq!(
                warm.warm_key(Some(1)),
                input.warm_key(Some(1)),
                "the same key"
            );
        }
        // An ordinary job line does not name the field.
        let plain = serde_json::to_string(&WorkerInput::Task(job("w-plain", TestAction::Crash)));
        assert!(!plain.unwrap().contains("warm_up"));
        // A poll has none: its worker is retired at the cap.
        assert!(
            WorkerInput::Poll(poll_job("w_poll", None))
                .warm_up()
                .is_none()
        );
    }

    /// CLOACINA-T-1159: the temp check keeps an empty directory as it is,
    /// and empties or makes again anything else.
    #[cfg(unix)]
    #[tokio::test]
    async fn the_temp_check_keeps_an_empty_directory_and_empties_anything_else() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let root = tempdir("tmp-check");
        let tmp = root.join("tmp");
        std::fs::create_dir_all(&tmp).unwrap();
        let ino = std::fs::metadata(&tmp).unwrap().ino();
        assert!(spawn::reset_tmp(&tmp).await);
        assert_eq!(
            std::fs::metadata(&tmp).unwrap().ino(),
            ino,
            "left as it was"
        );

        std::fs::create_dir_all(tmp.join("sub")).unwrap();
        std::fs::write(tmp.join("sub").join("left.txt"), "x").unwrap();
        assert!(!spawn::tmp_is_clean(&tmp));
        assert!(spawn::reset_tmp(&tmp).await);
        assert_eq!(std::fs::read_dir(&tmp).unwrap().count(), 0, "emptied");

        // A link in its place, to a directory with a file in it.
        let elsewhere = root.join("elsewhere");
        std::fs::create_dir_all(&elsewhere).unwrap();
        std::fs::write(elsewhere.join("kept.txt"), "x").unwrap();
        std::fs::remove_dir(&tmp).unwrap();
        std::os::unix::fs::symlink(&elsewhere, &tmp).unwrap();
        assert!(!spawn::tmp_is_clean(&tmp));
        assert!(spawn::reset_tmp(&tmp).await);
        let meta = std::fs::symlink_metadata(&tmp).unwrap();
        assert!(meta.file_type().is_dir(), "a directory again, not a link");
        assert!(
            elsewhere.join("kept.txt").exists(),
            "the link's target is not touched"
        );

        // An empty directory its owner may not write to.
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o500)).unwrap();
        assert!(!spawn::tmp_is_clean(&tmp));
        assert!(spawn::reset_tmp(&tmp).await);
        assert!(spawn::tmp_is_clean(&tmp));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// CLOACINA-T-1159: a job that wrote nothing to its temporary directory
    /// leaves the same directory for the next one; a job that wrote leaves
    /// an empty one.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_job_that_wrote_nothing_keeps_its_temp_directory() {
        use std::os::unix::fs::MetadataExt;
        let settings = warm_settings("warm-tmp-kept", warm_config());
        let report = || TestAction::Report { secret: None };
        let first = success(run_here_or_in_worker(job("tk-1", report()), &settings).await);
        let tmp = PathBuf::from(first["tmpdir"].as_str().unwrap());
        let ino = std::fs::metadata(&tmp).unwrap().ino();
        let second = success(run_here_or_in_worker(job("tk-2", report()), &settings).await);
        assert_eq!(second["pid"], first["pid"], "the same worker");
        assert_eq!(
            std::fs::metadata(&tmp).unwrap().ino(),
            ino,
            "not made again"
        );
        let wrote = pid_of(
            run_here_or_in_worker(
                job(
                    "tk-3",
                    TestAction::WriteTemp {
                        file: "left.txt".into(),
                    },
                ),
                &settings,
            )
            .await,
        );
        assert_eq!(Some(wrote), first["pid"].as_u64());
        let after = success(run_here_or_in_worker(job("tk-4", report()), &settings).await);
        assert_eq!(after["pid"], first["pid"]);
        assert_eq!(after["tmp_files"], serde_json::json!([]), "{after}");
    }

    /// The secrets of one job are not readable by the next job of the
    /// worker.
    #[cfg(unix)]
    #[tokio::test]
    async fn the_secrets_of_one_job_are_not_readable_by_the_next() {
        const VALUE: &str = "s3cr3t-value-of-t-1126";
        let settings = warm_settings("warm-secret", warm_config());
        let ask = || TestAction::Report {
            secret: Some(("warehouse".into(), VALUE.into())),
        };
        let mut with_secret = job("s-1", ask());
        with_secret.secrets.insert(
            "warehouse".into(),
            [("password".to_string(), VALUE.to_string())].into(),
        );
        let first = success(run_here_or_in_worker(with_secret, &settings).await);
        assert_eq!(first["secret_ok"], true, "the first job read its secret");
        let second = success(run_here_or_in_worker(job("s-2", ask()), &settings).await);
        assert_eq!(second["pid"], first["pid"], "the same worker");
        assert_eq!(second["secret_ok"], false, "the next job cannot read it");
        assert!(!second.to_string().contains(VALUE), "{second}");
    }

    /// A worker that holds more than its share of the limit after a job is
    /// retired.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn a_worker_that_grew_past_its_share_of_the_limit_is_retired() {
        let settings = warm_settings(
            "warm-grew",
            WarmConfig {
                max_memory_percent: 1,
                ..warm_config()
            },
        );
        let mut grow = job("g-1", TestAction::Report { secret: None });
        // Over 1% of 100Mi after any job. On Linux only anonymous and shared
        // memory counts, which stays under 10Mi for the test binary.
        grow.packet.memory_bytes = Some(100 << 20);
        let first = pid_of(run_here_or_in_worker(grow, &settings).await);
        assert_eq!(settings.warm.idle_len(), 0, "retired for its memory");
        assert!(!is_alive(first as u32));
        assert_eq!(counts(&settings).retired.get("memory"), Some(&1));
    }

    /// With warm workers off, each job gets a worker of its own.
    #[cfg(unix)]
    #[tokio::test]
    async fn with_warm_workers_off_each_job_gets_a_new_worker() {
        let settings = test_settings("warm-off", fast_watchdog(AgentMemory::Unknown));
        let report = || TestAction::Report { secret: None };
        let first = pid_of(run_here_or_in_worker(job("o-1", report()), &settings).await);
        let second = pid_of(run_here_or_in_worker(job("o-2", report()), &settings).await);
        assert_ne!(first, second);
        assert!(!is_alive(first as u32));
        assert_eq!(counts(&settings).cold_starts, 2);
    }

    // ── Graph firings (CLOACINA-T-1125) ───────────────────────────────────

    /// A firing has no memory request: it is held to the share of the agent.
    /// The one over it fails alone; a firing beside it completes.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn a_graph_firing_over_its_memory_limit_fails_alone() {
        let mut settings = test_settings("graph-memory", fast_watchdog(AgentMemory::Unknown));
        settings.default_memory_limit = Some(200 << 20);
        let greedy = graph_job(
            "graph_greedy",
            Some(TestAction::Allocate {
                bytes: 600 << 20,
                hold_ms: 20_000,
            }),
        );
        let modest = graph_job(
            "graph_modest",
            Some(TestAction::Allocate {
                bytes: 8 << 20,
                hold_ms: 1_500,
            }),
        );
        let started = std::time::Instant::now();
        let (greedy_end, modest_end) = tokio::join!(
            run_graph_here_or_in_worker(greedy, &settings),
            run_graph_here_or_in_worker(modest, &settings)
        );
        let (class, message) = failure(greedy_end);
        assert_eq!(class, FailureClassification::TaskError);
        assert!(
            message.contains("graph firing exceeded its memory limit of 200Mi"),
            "{message}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "stopped, not run out"
        );
        let modest = success(modest_end);
        assert_ne!(
            modest["pid"].as_u64().unwrap(),
            u64::from(std::process::id()),
            "the neighbour ran in a worker process"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_graph_firing_past_its_time_limit_is_killed_before_its_slot_is_free() {
        let settings = test_settings("graph-time", fast_watchdog(AgentMemory::Unknown));
        std::fs::create_dir_all(&settings.work_root).unwrap();
        let pid_file = settings.work_root.join("child.pid");
        let mut slow = graph_job(
            "graph_slow",
            Some(TestAction::StartChild {
                pid_file: pid_file.clone(),
                ms: 60_000,
            }),
        );
        slow.packet.timeout_seconds = 1;
        let started = std::time::Instant::now();
        let run = tokio::spawn(async move {
            let end = run_graph_here_or_in_worker(slow, &settings).await;
            (end, settings)
        });
        let grandchild = read_pid(&pid_file).await;
        let (end, _settings) = run.await.unwrap();
        let (class, message) = failure(end);
        assert_eq!(class, FailureClassification::Timeout);
        assert_eq!(message, "graph firing exceeded timeout of 1s");
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
        assert!(gone, "the firing's child {grandchild} outlived the kill");
    }

    /// A cancel names the firing id; the worker is stopped and nothing is
    /// reported. A firing cancelled before it arrives is not started.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_cancel_stops_a_graph_firing_and_nothing_is_reported() {
        let settings = test_settings("graph-cancel", fast_watchdog(AgentMemory::Unknown));
        std::fs::create_dir_all(&settings.work_root).unwrap();
        let pid_file = settings.work_root.join("child.pid");
        let long = graph_job(
            "graph_cancel",
            Some(TestAction::StartChild {
                pid_file: pid_file.clone(),
                ms: 60_000,
            }),
        );
        let started = std::time::Instant::now();
        let run = tokio::spawn(async move { run_graph_here_or_in_worker(long, &settings).await });
        let grandchild = read_pid(&pid_file).await;
        assert!(
            dispatch_registry().cancel("firing-graph_cancel"),
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
        assert!(gone, "the firing's child {grandchild} outlived the cancel");

        let settings = test_settings("graph-early-cancel", fast_watchdog(AgentMemory::Unknown));
        dispatch_registry().cancel("firing-graph_early");
        let late = graph_job("graph_early", Some(TestAction::Report { secret: None }));
        let end = run_graph_here_or_in_worker(late, &settings).await;
        assert!(matches!(end, PacketEnd::Cancelled), "{end:?}");
    }

    /// A real Python graph package: the worker imports it and runs the
    /// firing in a process of its own. The same firing marked inline runs
    /// in this process. The outcome is the one the agent reported before.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_python_graph_firing_runs_in_a_worker_and_inline_in_the_agent() {
        let settings = test_settings("py-graph", fast_watchdog(AgentMemory::Unknown));
        std::fs::create_dir_all(&settings.work_root).unwrap();
        let package = python_graph_package(&settings.work_root, "t1125_pid");
        let here = std::process::id();

        let worker_pid = settings.work_root.join("worker.pid");
        let end = run_graph_here_or_in_worker(
            python_graph_job("py_graph_worker", "t1125_pid", &package, &worker_pid),
            &settings,
        )
        .await;
        assert_eq!(success(end), serde_json::json!({ "outputs": [] }));
        let pid: u32 = std::fs::read_to_string(&worker_pid)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert_ne!(pid, here, "a worker process ran the firing");

        cloacina_python::install();
        let inline_pid = settings.work_root.join("inline.pid");
        let mut inline = python_graph_job("py_graph_inline", "t1125_pid", &package, &inline_pid);
        inline.packet.inline = true;
        inline.work_dir = settings.work_root.join("t1125-inline");
        let end = run_graph_here_or_in_worker(inline, &settings).await;
        assert_eq!(success(end), serde_json::json!({ "outputs": [] }));
        let pid: u32 = std::fs::read_to_string(&inline_pid)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert_eq!(pid, here, "the agent process ran the inline firing");
    }

    /// A Rust graph firing in a worker gives the outputs it gives in the
    /// agent process, and the time of each. Run by hand with a built graph
    /// library: `CLOACINA_AGENT_TEST_GRAPH_LIB=<path> cargo test -p
    /// cloacina-agent -- --ignored --nocapture rust_graph_firing`. The
    /// library is a graph package whose reactor `CLOACINA_AGENT_TEST_GRAPH_RX`
    /// names, fired with the cache in `CLOACINA_AGENT_TEST_GRAPH_CACHE` (JSON
    /// object, source name → event JSON).
    #[cfg(unix)]
    #[tokio::test]
    #[ignore = "needs a built graph library, run by hand"]
    async fn rust_graph_firing_in_a_worker_and_in_the_agent() {
        let library = PathBuf::from(
            std::env::var_os("CLOACINA_AGENT_TEST_GRAPH_LIB")
                .expect("CLOACINA_AGENT_TEST_GRAPH_LIB"),
        );
        let reactor = std::env::var("CLOACINA_AGENT_TEST_GRAPH_RX").unwrap_or_default();
        let cache: HashMap<String, serde_json::Value> = serde_json::from_str(
            &std::env::var("CLOACINA_AGENT_TEST_GRAPH_CACHE").unwrap_or_else(|_| "{}".into()),
        )
        .unwrap();
        let settings = test_settings("rust-graph", fast_watchdog(AgentMemory::Unknown));
        let firing = |name: &str, inline: bool| {
            let mut one = graph_job(name, None);
            one.package.digest = format!("rust-graph-{}", std::process::id());
            one.package.library = Some(library.clone());
            one.packet.graph_name = reactor.clone();
            one.packet.cache = cache
                .iter()
                .map(|(k, v)| (k.clone(), v.to_string()))
                .collect();
            one.packet.inline = inline;
            one
        };
        let rounds = 20;
        let mut times = [Vec::new(), Vec::new()];
        let mut outputs = [None, None];
        for (slot, inline) in [(0, false), (1, true)] {
            for n in 0..rounds {
                let at = std::time::Instant::now();
                let end = run_graph_here_or_in_worker(
                    firing(&format!("rg-{slot}-{n}"), inline),
                    &settings,
                )
                .await;
                times[slot].push(at.elapsed());
                outputs[slot] = Some(success(end));
            }
            times[slot].sort();
        }
        assert_eq!(
            outputs[0], outputs[1],
            "the same outputs in a worker and inline"
        );
        println!(
            "rust graph firing outputs: {}",
            outputs[0].as_ref().unwrap()
        );
        for (slot, label) in [(0, "in a worker"), (1, "in the agent process")] {
            println!(
                "rust graph firing {label} over {rounds} runs: min {:?}, median {:?}, max {:?}",
                times[slot][0],
                times[slot][rounds / 2],
                times[slot][rounds - 1]
            );
        }
    }

    // ── Polls (CLOACINA-T-1143) ───────────────────────────────────────────

    /// A poll that does `action` in place of a packaged trigger.
    pub(crate) fn poll_job(name: &str, action: Option<TestAction>) -> PollJob {
        PollJob {
            packet: PollWorkPacket {
                protocol_version: AGENT_PROTOCOL_VERSION,
                poll_id: format!("poll-{name}"),
                trigger_name: format!("{name}_poll"),
                workflow_name: "wf".into(),
                package_name: "pkg".into(),
                artifact: ArtifactRef {
                    digest: "none".into(),
                    fetch_url: "/none".into(),
                    build_target_triple: cloacina::fleet::host_target_triple(),
                },
                timeout_seconds: 30,
                tenant_id: None,
                language: Some("rust".into()),
                not_after_unix_ms: None,
            },
            package: PreparedPackage {
                digest: "none".into(),
                library: None,
                source_archive: None,
                vendored_set: None,
                providers: Providers::None,
            },
            work_dir: PathBuf::new(),
            log_format: "text".into(),
            max_unpacked_bytes:
                cloacina::cloacina_workflow_plugin::unpack_limit::DEFAULT_MAX_UNPACKED_BYTES,
            test_action: action,
        }
    }

    /// Pack a Python package with a workflow `t1143_py` and two poll
    /// triggers: `t1143_fire` fires with the process id as `polled_by`, and
    /// `t1143_skip` skips.
    pub(crate) fn python_poll_package(dir: &std::path::Path) -> PreparedPackage {
        let source = dir.join("t1143-py");
        std::fs::create_dir_all(source.join("workflow/t1143_py")).unwrap();
        std::fs::write(
            source.join("package.toml"),
            "[package]\nname = \"t1143-py\"\nversion = \"0.1.0\"\n\
             interface = \"cloacina-workflow-plugin\"\ninterface_version = 1\n\
             extension = \"cloacina\"\n\n[metadata]\nworkflow_name = \"t1143_py\"\n\
             language = \"python\"\ndescription = \"worker test\"\n\
             entry_module = \"t1143_py.tasks\"\n",
        )
        .unwrap();
        std::fs::write(source.join("workflow/t1143_py/__init__.py"), "").unwrap();
        std::fs::write(
            source.join("workflow/t1143_py/tasks.py"),
            "import os\nimport cloaca\n\n\n@cloaca.task(dependencies=[])\n\
             def report(context):\n    return context\n\n\n\
             @cloaca.trigger(on=\"t1143_py\", poll_interval=\"5s\")\n\
             def t1143_fire():\n    \
             return cloaca.TriggerResult.fire(cloaca.Context({\"polled_by\": os.getpid()}))\n\n\n\
             @cloaca.trigger(on=\"t1143_py\", poll_interval=\"5s\")\n\
             def t1143_skip():\n    return cloaca.TriggerResult.skip()\n",
        )
        .unwrap();
        let archive = dir.join("t1143-py.cloacina");
        fidius_core::package::pack_package(&source, Some(&archive)).unwrap();
        PreparedPackage {
            digest: format!("t1143-py-{}", std::process::id()),
            library: None,
            source_archive: Some(archive),
            vendored_set: None,
            providers: Providers::None,
        }
    }

    /// A poll of `trigger` in the package of [`python_poll_package`].
    fn python_poll_job(name: &str, package: &PreparedPackage, trigger: &str) -> PollJob {
        let mut one = poll_job(name, None);
        one.package = package.clone();
        one.packet.language = Some("python".into());
        one.packet.package_name = "t1143-py".into();
        one.packet.workflow_name = "t1143_py".into();
        one.packet.trigger_name = trigger.to_string();
        one
    }

    /// The answer as the server reads it from the report.
    fn poll_answer(end: PacketEnd) -> cloacina::cloacina_workflow::TriggerResult {
        match end {
            PacketEnd::Report(outcome) => {
                match cloacina::fleet::poll_result_from_outcome(outcome) {
                    Ok(answer) => answer,
                    Err(_) => panic!("not a poll answer"),
                }
            }
            PacketEnd::Cancelled => panic!("the poll was cancelled"),
        }
    }

    /// A poll has no memory request: it is held to the share of the agent.
    /// The one over it fails alone; a poll beside it completes.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[tokio::test]
    async fn a_poll_over_its_memory_limit_fails_alone() {
        let mut settings = test_settings("poll-memory", fast_watchdog(AgentMemory::Unknown));
        settings.default_memory_limit = Some(200 << 20);
        let greedy = poll_job(
            "poll_greedy",
            Some(TestAction::Allocate {
                bytes: 600 << 20,
                hold_ms: 20_000,
            }),
        );
        let modest = poll_job(
            "poll_modest",
            Some(TestAction::Allocate {
                bytes: 8 << 20,
                hold_ms: 1_500,
            }),
        );
        let started = std::time::Instant::now();
        let (greedy_end, modest_end) = tokio::join!(
            run_poll_in_worker(greedy, &settings),
            run_poll_in_worker(modest, &settings)
        );
        let (class, message) = failure(greedy_end);
        assert_eq!(class, FailureClassification::TaskError);
        assert!(
            message.contains("poll exceeded its memory limit of 200Mi"),
            "{message}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "stopped, not run out"
        );
        let modest = success(modest_end);
        assert_ne!(
            modest["pid"].as_u64().unwrap(),
            u64::from(std::process::id()),
            "the neighbour ran in a worker process"
        );
    }

    /// A poll past its time limit is killed with what it started, before
    /// its slot is free.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_poll_past_its_time_limit_is_killed_with_its_process_group() {
        let settings = test_settings("poll-time", fast_watchdog(AgentMemory::Unknown));
        std::fs::create_dir_all(&settings.work_root).unwrap();
        let pid_file = settings.work_root.join("child.pid");
        let mut slow = poll_job(
            "poll_slow",
            Some(TestAction::StartChild {
                pid_file: pid_file.clone(),
                ms: 60_000,
            }),
        );
        slow.packet.timeout_seconds = 1;
        let started = std::time::Instant::now();
        let run = tokio::spawn(async move { run_poll_in_worker(slow, &settings).await });
        let grandchild = read_pid(&pid_file).await;
        let (class, message) = failure(run.await.unwrap());
        assert_eq!(class, FailureClassification::Timeout);
        assert_eq!(message, "poll exceeded timeout of 1s");
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
        assert!(gone, "the poll's child {grandchild} outlived the kill");
    }

    /// A real Python poll trigger runs in a worker; the answers reach the
    /// server as before: fire with the trigger's context, and skip. The
    /// same poll in the agent process gives the same answer.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_python_poll_runs_in_a_worker_and_answers_as_before() {
        let settings = test_settings("py-poll", fast_watchdog(AgentMemory::Unknown));
        std::fs::create_dir_all(&settings.work_root).unwrap();
        let package = python_poll_package(&settings.work_root);
        let here = std::process::id();

        let fired = poll_answer(
            run_poll_in_worker(
                python_poll_job("py_poll_fire", &package, "t1143_fire"),
                &settings,
            )
            .await,
        );
        let ctx = fired
            .into_context()
            .expect("fire with the trigger's context");
        let polled_by = ctx.get("polled_by").and_then(|v| v.as_u64()).unwrap();
        assert_ne!(polled_by, u64::from(here), "a worker process ran the poll");

        let skipped = poll_answer(
            run_poll_in_worker(
                python_poll_job("py_poll_skip", &package, "t1143_skip"),
                &settings,
            )
            .await,
        );
        assert!(!skipped.should_fire(), "skip");

        let missing = run_poll_in_worker(
            python_poll_job("py_poll_missing", &package, "t1143_none"),
            &settings,
        )
        .await;
        let (class, message) = failure(missing);
        assert_eq!(class, FailureClassification::Validation);
        assert!(
            message.contains("has no poll trigger 't1143_none'"),
            "{message}"
        );

        // In the agent process: the same answer, from this process.
        cloacina_python::install();
        let mut inline = python_poll_job("py_poll_inline", &package, "t1143_fire");
        inline.work_dir = settings.work_root.join("t1143-inline");
        let fired = crate::run_poll_job(inline, Some(Duration::from_secs(30)), || {}).await;
        let ctx = cloacina::fleet::poll_result_from_outcome(fired)
            .ok()
            .and_then(|r| r.into_context())
            .expect("fire with the trigger's context");
        assert_eq!(
            ctx.get("polled_by").and_then(|v| v.as_u64()),
            Some(u64::from(here))
        );
    }

    /// A second poll of one build reuses the warm worker, under a key of
    /// kind `poll`: a task of the same build does not take it, nor a poll of
    /// another tenant.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_second_poll_reuses_the_warm_worker_and_a_task_does_not() {
        let settings = warm_settings("warm-poll", warm_config());
        let report = || TestAction::Report { secret: None };
        let first = pid_of(run_poll_in_worker(poll_job("wp_1", Some(report())), &settings).await);
        let second = pid_of(run_poll_in_worker(poll_job("wp_2", Some(report())), &settings).await);
        assert_ne!(first, u64::from(std::process::id()), "a worker ran it");
        assert_eq!(first, second, "the second poll ran in the same worker");

        // A task of the same digest, tenant and language: another kind.
        let task = pid_of(run_here_or_in_worker(job("wp-task", report()), &settings).await);
        assert_ne!(task, first, "a task took a poll's warm worker");
        let mut other_tenant = poll_job("wp_tenant", Some(report()));
        other_tenant.packet.tenant_id = Some("acme".into());
        let tenant = pid_of(run_poll_in_worker(other_tenant, &settings).await);
        assert_ne!(
            tenant, first,
            "a poll of another tenant took the warm worker"
        );

        let again = pid_of(run_poll_in_worker(poll_job("wp_3", Some(report())), &settings).await);
        assert_eq!(again, first, "a poll of the same key takes it");
        let counts = counts(&settings);
        assert_eq!((counts.cold_starts, counts.warm_starts), (3, 2));
    }

    /// The cost of a poll in a worker, cold and warm, against the agent
    /// process (CLOACINA-T-1143). Run by hand: `cargo test -p cloacina-agent
    /// -- --ignored --nocapture poll_cost`. A Rust poll runs when
    /// `CLOACINA_AGENT_TEST_POLL_LIB` names a built library with a poll
    /// trigger and `CLOACINA_AGENT_TEST_POLL_TRIGGER` its name.
    #[cfg(unix)]
    #[tokio::test]
    #[ignore = "a measurement, run by hand"]
    async fn poll_cost() {
        let rounds = 20;
        let cold = test_settings("poll-cost-cold", fast_watchdog(AgentMemory::Unknown));
        let warm = warm_settings("poll-cost-warm", warm_config());
        std::fs::create_dir_all(&cold.work_root).unwrap();
        std::fs::create_dir_all(&warm.work_root).unwrap();
        cloacina_python::install();

        type MakePoll = Box<dyn Fn(&str) -> PollJob>;
        let mut kinds: Vec<(&str, MakePoll)> = Vec::new();
        let package = python_poll_package(&cold.work_root);
        kinds.push((
            "python",
            Box::new(move |name| python_poll_job(name, &package, "t1143_fire")),
        ));
        if let (Some(lib), Ok(trigger)) = (
            std::env::var_os("CLOACINA_AGENT_TEST_POLL_LIB"),
            std::env::var("CLOACINA_AGENT_TEST_POLL_TRIGGER"),
        ) {
            let lib = PathBuf::from(lib);
            kinds.push((
                "rust",
                Box::new(move |name| {
                    let mut one = poll_job(name, None);
                    one.package.digest = format!("pc-rust-{}", std::process::id());
                    one.package.library = Some(lib.clone());
                    one.packet.trigger_name = trigger.clone();
                    one
                }),
            ));
        }
        for (kind, make) in &kinds {
            measure(
                &format!("{kind} poll, a new worker each poll"),
                rounds,
                |n| run_poll_in_worker(make(&format!("pc-cold-{kind}-{n}")), &cold),
            )
            .await;
            measure(&format!("{kind} poll, warm worker"), rounds, |n| {
                run_poll_in_worker(make(&format!("pc-warm-{kind}-{n}")), &warm)
            })
            .await;
            if let Some((pid, used)) = warm.warm.idle_memory().pop() {
                println!(
                    "IDLE {kind} poll: warm worker {pid} holds {}",
                    used.map_or("?".to_string(), crate::memory::format_bytes)
                );
            }
            measure(&format!("{kind} poll, in the agent process"), rounds, |n| {
                let mut one = make(&format!("pc-inline-{kind}-{n}"));
                one.work_dir = cold.work_root.join(format!("pc-inline-{kind}"));
                async move {
                    PacketEnd::Report(
                        crate::run_poll_job(one, Some(Duration::from_secs(30)), || {}).await,
                    )
                }
            })
            .await;
        }
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
        let mut line = serde_json::to_vec(&WorkerInput::Task(job(
            "orphan",
            TestAction::Block { ms: 60_000 },
        )))
        .unwrap();
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
        assert_eq!(settings.memory_limit_for(asked.memory_bytes), None);
        settings.default_memory_limit = default_memory_share(Some(3 << 30), 4);
        assert_eq!(
            settings.memory_limit_for(asked.memory_bytes),
            Some(768 << 20)
        );
        asked.memory_bytes = Some(2 << 30);
        assert_eq!(settings.memory_limit_for(asked.memory_bytes), Some(2 << 30));
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

    /// Pack a Python package whose one task, `report`, runs after an import
    /// of a set of standard-library modules (`asyncio`, `decimal`, `email`,
    /// `http.client`, `json`, `logging`, `sqlite3`, `xml.etree`, ...). No
    /// fixture of the repository has a third-party data stack.
    fn python_heavy_package(dir: &std::path::Path) -> (PreparedPackage, String) {
        let source = dir.join("t1126-heavy");
        std::fs::create_dir_all(source.join("workflow/t1126_heavy")).unwrap();
        std::fs::write(
            source.join("package.toml"),
            "[package]\nname = \"t1126-heavy\"\nversion = \"0.1.0\"\n\
             interface = \"cloacina-workflow-plugin\"\ninterface_version = 1\n\
             extension = \"cloacina\"\n\n[metadata]\nworkflow_name = \"t1126_heavy\"\n\
             language = \"python\"\ndescription = \"worker test\"\n\
             entry_module = \"t1126_heavy.tasks\"\n",
        )
        .unwrap();
        std::fs::write(source.join("workflow/t1126_heavy/__init__.py"), "").unwrap();
        std::fs::write(
            source.join("workflow/t1126_heavy/tasks.py"),
            "import os\nimport asyncio, csv, dataclasses, datetime, decimal, email.mime.text\n\
             import fractions, http.client, json, logging.handlers, sqlite3, statistics\n\
             import urllib.request, uuid, xml.etree.ElementTree, zipfile\nimport cloaca\n\n\n\
             @cloaca.task(dependencies=[])\n\
             def report(context):\n    context.set(\"pid\", os.getpid())\n    return context\n",
        )
        .unwrap();
        let archive = dir.join("t1126-heavy.cloacina");
        fidius_core::package::pack_package(&source, Some(&archive)).unwrap();
        (
            PreparedPackage {
                digest: format!("t1126-heavy-{}", std::process::id()),
                library: None,
                source_archive: Some(archive),
                vendored_set: None,
                providers: Providers::None,
            },
            "public::t1126-heavy::t1126_heavy::report".to_string(),
        )
    }

    /// Run `one` `rounds` times and print the min, median and max.
    async fn measure<F, Fut>(label: &str, rounds: usize, mut one: F) -> Duration
    where
        F: FnMut(usize) -> Fut,
        Fut: std::future::Future<Output = PacketEnd>,
    {
        let mut times = Vec::new();
        for n in 0..rounds {
            let at = std::time::Instant::now();
            let end = one(n).await;
            times.push(at.elapsed());
            success(end);
        }
        times.sort();
        println!(
            "MEASURE {label} over {rounds} runs: min {:?}, median {:?}, max {:?}",
            times[0],
            times[rounds / 2],
            times[rounds - 1]
        );
        times[rounds / 2]
    }

    /// The cost of a job in a worker for each kind of package
    /// (CLOACINA-T-1126). Run by hand: `cargo test -p cloacina-agent --
    /// --ignored --nocapture job_cost_by_kind`. A Rust task runs when
    /// `CLOACINA_AGENT_TEST_TASK_LIB` names a built workflow library and
    /// `CLOACINA_AGENT_TEST_TASK` the full name of one of its tasks; a Rust
    /// graph firing when `CLOACINA_AGENT_TEST_GRAPH_LIB` and
    /// `CLOACINA_AGENT_TEST_GRAPH_RX` are set (see
    /// `rust_graph_firing_in_a_worker_and_in_the_agent`).
    #[cfg(unix)]
    #[tokio::test]
    #[ignore = "a measurement, run by hand"]
    async fn job_cost_by_kind() {
        // `CLOACINA_AGENT_TEST_WARM=1`: with warm workers (CLOACINA-T-1126).
        let warm = std::env::var_os("CLOACINA_AGENT_TEST_WARM").is_some();
        let settings = if warm {
            warm_settings("kind-cost", warm_config())
        } else {
            test_settings("kind-cost", fast_watchdog(AgentMemory::Unknown))
        };
        std::fs::create_dir_all(&settings.work_root).unwrap();
        let rounds = 20;
        // The worker used last is the last one in the pool.
        let idle_memory = |label: &str| {
            if let Some((pid, used)) = settings.warm.idle_memory().pop() {
                println!(
                    "IDLE {label}: warm worker {pid} holds {}",
                    used.map_or("?".to_string(), crate::memory::format_bytes)
                );
            }
        };

        if let (Some(lib), Ok(task)) = (
            std::env::var_os("CLOACINA_AGENT_TEST_TASK_LIB"),
            std::env::var("CLOACINA_AGENT_TEST_TASK"),
        ) {
            let lib = PathBuf::from(lib);
            measure("rust task in a worker", rounds, |n| {
                let mut one = job(&format!("kc-rust-{n}"), TestAction::Crash);
                one.test_action = None;
                one.packet.task_name = task.clone();
                one.package.digest = format!("kc-rust-{}", std::process::id());
                one.package.library = Some(lib.clone());
                run_here_or_in_worker(one, &settings)
            })
            .await;
            idle_memory("rust task");
        }

        if let Some(lib) = std::env::var_os("CLOACINA_AGENT_TEST_GRAPH_LIB") {
            let lib = PathBuf::from(lib);
            let reactor = std::env::var("CLOACINA_AGENT_TEST_GRAPH_RX").unwrap_or_default();
            let cache: HashMap<String, serde_json::Value> = serde_json::from_str(
                &std::env::var("CLOACINA_AGENT_TEST_GRAPH_CACHE").unwrap_or_else(|_| "{}".into()),
            )
            .unwrap();
            measure("rust graph firing in a worker", rounds, |n| {
                let mut one = graph_job(&format!("kc-graph-{n}"), None);
                one.package.digest = format!("kc-graph-{}", std::process::id());
                one.package.library = Some(lib.clone());
                one.packet.graph_name = reactor.clone();
                one.packet.cache = cache
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_string()))
                    .collect();
                run_graph_here_or_in_worker(one, &settings)
            })
            .await;
            idle_memory("rust graph");
        }

        let (small, small_task) = python_package(&settings.work_root);
        measure("python task (one module) in a worker", rounds, |n| {
            run_here_or_in_worker(
                python_job(&format!("kc-py-{n}"), &small, &small_task),
                &settings,
            )
        })
        .await;
        idle_memory("python task (one module)");

        let (heavy, heavy_task) = python_heavy_package(&settings.work_root);
        measure("python task (stdlib imports) in a worker", rounds, |n| {
            run_here_or_in_worker(
                python_job(&format!("kc-heavy-{n}"), &heavy, &heavy_task),
                &settings,
            )
        })
        .await;
        idle_memory("python task (stdlib imports)");

        let graph = python_graph_package(&settings.work_root, "t1126_cost");
        let pid_file = settings.work_root.join("cost.pid");
        measure("python graph firing in a worker", rounds, |n| {
            run_graph_here_or_in_worker(
                python_graph_job(&format!("kc-pyg-{n}"), "t1126_cost", &graph, &pid_file),
                &settings,
            )
        })
        .await;
        idle_memory("python graph");
    }

    /// CLOACINA-T-1159: the hand-over of a warm worker, and the p99 that the
    /// job cap gives. 300 empty firings on one key, then 300 that each write
    /// a temp file, with the default cap of 100 jobs. Run by hand:
    /// `cargo test -p cloacina-agent -- --ignored --nocapture warm_hand_over_cost`.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[ignore = "a measurement, run by hand"]
    async fn warm_hand_over_cost() {
        fn stats(label: &str, mut times: Vec<Duration>, cold: u64) {
            times.sort();
            let n = times.len();
            println!(
                "T1159 {label} (n={n}): min {:?}, p50 {:?}, p90 {:?}, p99 {:?}, max {:?}, \
                 cold starts after the first {cold}",
                times[0],
                times[n / 2],
                times[n * 9 / 10],
                times[(n * 99 / 100).min(n - 1)],
                times[n - 1],
            );
        }
        let rounds = 300;
        for (label, write) in [("empty job", false), ("job that writes a temp file", true)] {
            let settings = warm_settings(&format!("t1159-{write}"), warm_config());
            let one = |n: usize| {
                let action = if write {
                    TestAction::WriteTemp {
                        file: format!("f-{n}"),
                    }
                } else {
                    TestAction::Block { ms: 0 }
                };
                let mut one = graph_job(&format!("h-{write}-{n}"), Some(action));
                one.package.digest = format!("t1159-{write}");
                one
            };
            // The first job starts the worker; it is not counted.
            success(run_graph_here_or_in_worker(one(usize::MAX), &settings).await);
            let mut times = Vec::with_capacity(rounds);
            for n in 0..rounds {
                let at = std::time::Instant::now();
                let end = run_graph_here_or_in_worker(one(n), &settings).await;
                times.push(at.elapsed());
                success(end);
                // A gap between firings, as a reactor has.
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
            stats(label, times, counts(&settings).cold_starts - 1);
        }
    }
}
