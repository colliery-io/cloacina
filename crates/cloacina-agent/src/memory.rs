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

//! The memory watchdog of the agent (CLOACINA-T-1028).
//!
//! Each task runs in a worker process. The kernel has no limit on the
//! resident memory of one process, and an agent pod cannot make a cgroup for
//! each worker (its cgroup filesystem is read-only). So the agent reads the
//! memory of each worker, with the processes the worker started, on a short
//! interval ([`CHECK_INTERVAL`]) and stops the worker that is over its limit.
//!
//! It also watches the agent as a whole. When the agent and its workers
//! together are close to the agent's memory, it stops the largest worker
//! before the kernel has to pick a victim.
//!
//! What is read:
//!
//! - Linux: `RssAnon` + `RssShmem` of `/proc/<pid>/status` (memory the kernel
//!   cannot drop; pages of a mapped file are not counted), for every process
//!   in the worker's process group or below the worker in the process tree.
//! - macOS (development): the resident size of every process in the worker's
//!   process group.
//! - Other systems: nothing; no task is stopped for memory.
//!
//! Between two reads a task can go over its limit. `setrlimit` on the worker
//! and the kernel's own out-of-memory handling are behind the watchdog.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::mpsc;
use tracing::debug;

/// How often the watchdog reads the memory of the workers.
pub(crate) const CHECK_INTERVAL: Duration = Duration::from_millis(250);

/// The agent stops its largest worker when the agent as a whole uses more
/// than this share of its memory, in percent.
const AGENT_FULL_PERCENT: u64 = 95;

/// Why the agent stops a worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StopReason {
    /// The task used more memory than its limit.
    OverLimit { used: u64, limit: u64 },
    /// The agent as a whole is out of memory, and this worker is the largest.
    AgentOutOfMemory {
        used: u64,
        agent_used: u64,
        agent_total: u64,
    },
    /// The task ran past its time limit.
    TimeLimit,
    /// The server gave up on the dispatch.
    Cancelled,
}

/// Where the memory use of the agent as a whole is read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentMemory {
    /// Not known: the agent has no memory figure. Only per-task limits hold.
    Unknown,
    /// The agent's own cgroup (v2) has a limit: read the cgroup.
    Cgroup { dir: std::path::PathBuf, total: u64 },
    /// `--memory` with no cgroup limit: the agent's own memory plus the
    /// memory of its workers.
    Sum { total: u64 },
}

impl AgentMemory {
    /// The agent's memory in bytes, when it is known.
    pub(crate) fn total(&self) -> Option<u64> {
        match self {
            AgentMemory::Unknown => None,
            AgentMemory::Cgroup { total, .. } | AgentMemory::Sum { total } => Some(*total),
        }
    }

    /// Settle the source at start. `flag` is `--memory` in bytes.
    ///
    /// A cgroup limit wins for the reading, because it is what the kernel
    /// enforces. The total is the smaller of the two when both are known.
    pub(crate) fn detect(flag: Option<u64>) -> Self {
        match (own_cgroup_limit(), flag) {
            (Some((dir, limit)), flag) => AgentMemory::Cgroup {
                dir,
                total: flag.map_or(limit, |f| f.min(limit)),
            },
            (None, Some(total)) => AgentMemory::Sum { total },
            (None, None) => AgentMemory::Unknown,
        }
    }
}

struct Watched {
    limit: Option<u64>,
    stop: mpsc::UnboundedSender<StopReason>,
    stopping: bool,
}

/// The watchdog. One per agent process; tests make their own.
pub(crate) struct Watchdog {
    workers: Mutex<HashMap<u32, Watched>>,
    agent: AgentMemory,
    interval: Duration,
    running: AtomicBool,
}

/// Takes a worker out of the watchdog when it is dropped.
pub(crate) struct WatchGuard {
    watchdog: Arc<Watchdog>,
    pid: u32,
}

impl Drop for WatchGuard {
    fn drop(&mut self) {
        self.watchdog
            .workers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.pid);
    }
}

impl Watchdog {
    pub(crate) fn new(agent: AgentMemory) -> Arc<Self> {
        Self::with_interval(agent, CHECK_INTERVAL)
    }

    pub(crate) fn with_interval(agent: AgentMemory, interval: Duration) -> Arc<Self> {
        Arc::new(Self {
            workers: Mutex::new(HashMap::new()),
            agent,
            interval,
            running: AtomicBool::new(false),
        })
    }

    /// Watch the worker `pid` (the leader of its process group). When it must
    /// stop, the reason is sent on `stop`, one time. Must be called inside a
    /// tokio runtime: the first call starts the loop.
    pub(crate) fn watch(
        self: &Arc<Self>,
        pid: u32,
        limit: Option<u64>,
        stop: mpsc::UnboundedSender<StopReason>,
    ) -> WatchGuard {
        self.workers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(
                pid,
                Watched {
                    limit,
                    stop,
                    stopping: false,
                },
            );
        if !self.running.swap(true, Ordering::SeqCst) {
            let watchdog = Arc::downgrade(self);
            let interval = self.interval;
            tokio::spawn(async move {
                let mut ticker = tokio::time::interval(interval);
                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    ticker.tick().await;
                    let Some(watchdog) = watchdog.upgrade() else {
                        break;
                    };
                    watchdog.check().await;
                }
            });
        }
        WatchGuard {
            watchdog: self.clone(),
            pid,
        }
    }

    /// One round: read, compare, send the stops.
    async fn check(&self) {
        let pids: Vec<u32> = {
            let workers = self.workers.lock().unwrap_or_else(|e| e.into_inner());
            workers.keys().copied().collect()
        };
        if pids.is_empty() {
            return;
        }
        let agent = self.agent.clone();
        let read = tokio::task::spawn_blocking(move || {
            let used = worker_memory(&pids);
            let agent_used = match &agent {
                AgentMemory::Unknown => None,
                AgentMemory::Cgroup { dir, .. } => cgroup_used(dir),
                AgentMemory::Sum { .. } => {
                    Some(own_memory().unwrap_or(0) + used.values().sum::<u64>())
                }
            };
            (used, agent_used)
        })
        .await;
        let Ok((used, agent_used)) = read else {
            return;
        };

        let mut workers = self.workers.lock().unwrap_or_else(|e| e.into_inner());
        for (pid, watched) in workers.iter_mut() {
            let (Some(limit), Some(&now)) = (watched.limit, used.get(pid)) else {
                continue;
            };
            if now > limit && !watched.stopping {
                watched.stopping = true;
                let _ = watched
                    .stop
                    .send(StopReason::OverLimit { used: now, limit });
            }
        }
        // The agent as a whole. One victim at a time: while a worker is on
        // its way out, its memory is still counted.
        let (Some(agent_total), Some(agent_used)) = (self.agent.total(), agent_used) else {
            return;
        };
        if agent_used.saturating_mul(100) <= agent_total.saturating_mul(AGENT_FULL_PERCENT)
            || workers.values().any(|w| w.stopping)
        {
            return;
        }
        let largest = workers
            .iter_mut()
            .filter_map(|(pid, w)| used.get(pid).map(|&now| (now, w)))
            .max_by_key(|(now, _)| *now);
        match largest {
            Some((now, watched)) => {
                watched.stopping = true;
                let _ = watched.stop.send(StopReason::AgentOutOfMemory {
                    used: now,
                    agent_used,
                    agent_total,
                });
            }
            None => debug!(
                agent_used,
                agent_total, "the agent is close to its memory and no worker can be read"
            ),
        }
    }
}

/// Bytes as a short binary quantity: `512Mi`, `1.5Gi`.
pub(crate) fn format_bytes(bytes: u64) -> String {
    const UNITS: [(&str, u64); 3] = [("Gi", 1 << 30), ("Mi", 1 << 20), ("Ki", 1 << 10)];
    for (unit, size) in UNITS {
        if bytes >= size {
            let tenths = (bytes as u128 * 10 / size as u128) as u64;
            return if tenths.is_multiple_of(10) {
                format!("{}{unit}", tenths / 10)
            } else {
                format!("{}.{}{unit}", tenths / 10, tenths % 10)
            };
        }
    }
    format!("{bytes}B")
}

// ────────────────────────────────────────────────────────────────────
// Reading memory
// ────────────────────────────────────────────────────────────────────

/// The memory of each worker in `pids` with the processes it started, in
/// bytes. A worker that cannot be read (it has exited) is left out.
#[cfg(target_os = "linux")]
pub(crate) fn worker_memory(pids: &[u32]) -> HashMap<u32, u64> {
    // One pass over /proc: the parent and the process group of each process.
    let mut procs: HashMap<u32, (u32, u32)> = HashMap::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return HashMap::new();
    };
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
            continue;
        };
        let Some((state, parent, group)) = parse_stat(&stat) else {
            continue;
        };
        // A dead process holds no memory.
        if state == 'Z' {
            continue;
        }
        procs.insert(pid, (parent, group));
    }
    let mut used = HashMap::new();
    for &worker in pids {
        if !procs.contains_key(&worker) {
            continue;
        }
        let mut total = 0u64;
        for (&pid, &(_, group)) in &procs {
            if group == worker || descends_from(&procs, pid, worker) {
                total += process_memory(pid).unwrap_or(0);
            }
        }
        used.insert(worker, total);
    }
    used
}

/// The state, the parent pid and the process group of a `/proc/<pid>/stat`
/// line. The command name is in brackets and can hold spaces and brackets,
/// so the fields are counted from the last `)`.
#[cfg(any(target_os = "linux", test))]
fn parse_stat(stat: &str) -> Option<(char, u32, u32)> {
    let rest = &stat[stat.rfind(')')? + 1..];
    let mut fields = rest.split_whitespace();
    let state = fields.next()?.chars().next()?;
    let parent = fields.next()?.parse().ok()?;
    let group = fields.next()?.parse().ok()?;
    Some((state, parent, group))
}

/// Whether `pid` is `ancestor` or below it in the process tree.
#[cfg(any(target_os = "linux", test))]
fn descends_from(procs: &HashMap<u32, (u32, u32)>, pid: u32, ancestor: u32) -> bool {
    let mut at = pid;
    // A process tree is not deeper than the number of processes.
    for _ in 0..=procs.len() {
        if at == ancestor {
            return true;
        }
        match procs.get(&at) {
            Some(&(parent, _)) if parent != 0 && parent != at => at = parent,
            _ => return false,
        }
    }
    false
}

/// `RssAnon` + `RssShmem` of a `/proc/<pid>/status` text, in bytes; `VmRSS`
/// on a kernel that does not split it.
#[cfg(any(target_os = "linux", test))]
fn parse_status_memory(status: &str) -> Option<u64> {
    let field = |name: &str| -> Option<u64> {
        let line = status.lines().find(|l| l.starts_with(name))?;
        let kb: u64 = line[name.len()..].split_whitespace().next()?.parse().ok()?;
        Some(kb * 1024)
    };
    match (field("RssAnon:"), field("RssShmem:")) {
        (Some(anon), shmem) => Some(anon + shmem.unwrap_or(0)),
        (None, _) => field("VmRSS:"),
    }
}

#[cfg(target_os = "linux")]
fn process_memory(pid: u32) -> Option<u64> {
    parse_status_memory(&std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?)
}

#[cfg(target_os = "macos")]
pub(crate) fn worker_memory(pids: &[u32]) -> HashMap<u32, u64> {
    let mut out = HashMap::new();
    for &worker in pids {
        let mut members = [0 as libc::pid_t; 4096];
        // SAFETY: the buffer is valid for the size given, in bytes.
        let count = unsafe {
            libc::proc_listpgrppids(
                worker as libc::pid_t,
                members.as_mut_ptr().cast(),
                std::mem::size_of_val(&members) as libc::c_int,
            )
        };
        if count <= 0 {
            continue;
        }
        let count = (count as usize).min(members.len());
        let mut total = 0u64;
        let mut seen = false;
        for &pid in &members[..count] {
            if pid <= 0 {
                continue;
            }
            if let Some(bytes) = process_memory(pid as u32) {
                total += bytes;
                seen = true;
            }
        }
        if seen {
            out.insert(worker, total);
        }
    }
    out
}

#[cfg(target_os = "macos")]
fn process_memory(pid: u32) -> Option<u64> {
    // SAFETY: `proc_taskinfo` is plain data; zero is a valid value.
    let mut info: libc::proc_taskinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::proc_taskinfo>() as libc::c_int;
    // SAFETY: the buffer is valid for `size` bytes.
    let got = unsafe {
        libc::proc_pidinfo(
            pid as libc::c_int,
            libc::PROC_PIDTASKINFO,
            0,
            (&mut info as *mut libc::proc_taskinfo).cast(),
            size,
        )
    };
    (got == size).then_some(info.pti_resident_size)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub(crate) fn worker_memory(_pids: &[u32]) -> HashMap<u32, u64> {
    HashMap::new()
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn process_memory(_pid: u32) -> Option<u64> {
    None
}

/// The memory of the agent process itself, in bytes.
pub(crate) fn own_memory() -> Option<u64> {
    process_memory(std::process::id())
}

// ────────────────────────────────────────────────────────────────────
// The agent's own cgroup (Linux, cgroup v2)
// ────────────────────────────────────────────────────────────────────

/// The directory of the agent's own cgroup (v2), from `/proc/self/cgroup`.
#[cfg(target_os = "linux")]
fn own_cgroup_dir() -> Option<std::path::PathBuf> {
    let text = std::fs::read_to_string("/proc/self/cgroup").ok()?;
    let path = text.lines().find_map(|l| l.strip_prefix("0::"))?;
    let dir = std::path::Path::new("/sys/fs/cgroup").join(path.trim_start_matches('/'));
    dir.join("cgroup.controllers").exists().then_some(dir)
}

/// The agent's own cgroup and its memory limit, when it has a limit.
#[cfg(target_os = "linux")]
fn own_cgroup_limit() -> Option<(std::path::PathBuf, u64)> {
    let dir = own_cgroup_dir()?;
    let max = std::fs::read_to_string(dir.join("memory.max")).ok()?;
    let limit = max.trim().parse::<u64>().ok()?;
    Some((dir, limit))
}

#[cfg(not(target_os = "linux"))]
fn own_cgroup_limit() -> Option<(std::path::PathBuf, u64)> {
    None
}

/// What the cgroup uses and the kernel cannot drop: `memory.current` less the
/// page cache (`active_file` + `inactive_file` of `memory.stat`).
fn cgroup_used(dir: &std::path::Path) -> Option<u64> {
    let current: u64 = std::fs::read_to_string(dir.join("memory.current"))
        .ok()?
        .trim()
        .parse()
        .ok()?;
    let stat = std::fs::read_to_string(dir.join("memory.stat")).unwrap_or_default();
    Some(current.saturating_sub(page_cache_of(&stat)))
}

fn page_cache_of(memory_stat: &str) -> u64 {
    memory_stat
        .lines()
        .filter_map(|l| {
            let (name, value) = l.split_once(' ')?;
            matches!(name, "active_file" | "inactive_file")
                .then(|| value.trim().parse::<u64>().ok())?
        })
        .sum()
}

/// How many times the kernel killed a process of the agent's cgroup for
/// memory (`oom_kill` of `memory.events`). `None` when it cannot be read.
pub(crate) fn oom_kill_count() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let text = std::fs::read_to_string(own_cgroup_dir()?.join("memory.events")).ok()?;
        parse_oom_kill(&text)
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(any(target_os = "linux", test))]
fn parse_oom_kill(memory_events: &str) -> Option<u64> {
    memory_events
        .lines()
        .find_map(|l| l.strip_prefix("oom_kill "))?
        .trim()
        .parse()
        .ok()
}

/// Whether the agent can make a child cgroup, with the reason when it cannot.
/// The agent does not use one; the answer is logged at start so an operator
/// knows what the pod allows.
pub(crate) fn probe_child_cgroup() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let dir = own_cgroup_dir().ok_or("the agent is not in a cgroup v2 hierarchy")?;
        let probe = dir.join(format!("cloacina-probe-{}", std::process::id()));
        std::fs::create_dir(&probe).map_err(|e| format!("{}: {e}", dir.display()))?;
        if let Err(e) = std::fs::remove_dir(&probe) {
            tracing::warn!(path = %probe.display(), error = %e, "could not remove the probe cgroup");
        }
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err("cgroups are a Linux feature".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_are_written_as_binary_quantities() {
        assert_eq!(format_bytes(512), "512B");
        assert_eq!(format_bytes(64 << 20), "64Mi");
        assert_eq!(format_bytes(1 << 30), "1Gi");
        assert_eq!(format_bytes((1 << 30) + (512 << 20)), "1.5Gi");
        assert_eq!(format_bytes(300 * 1000 * 1000), "286.1Mi");
    }

    #[test]
    fn stat_line_is_read_past_a_command_name_with_brackets() {
        let line = "4242 (my (odd) name) S 17 4000 4000 0 -1 4194560 100";
        assert_eq!(parse_stat(line), Some(('S', 17, 4000)));
        assert_eq!(
            parse_stat("77 (sleep) Z 1 4000 4000 0 -1 0"),
            Some(('Z', 1, 4000))
        );
        assert_eq!(parse_stat("garbage"), None);
    }

    #[test]
    fn a_process_is_found_below_its_ancestor() {
        // 1 -> 10 (worker) -> 11 -> 12 ; 20 is a sibling of the worker.
        let procs: HashMap<u32, (u32, u32)> = [
            (1, (0, 1)),
            (10, (1, 10)),
            (11, (10, 10)),
            // 12 left the process group (setsid) and is still below 10.
            (12, (11, 12)),
            (20, (1, 20)),
        ]
        .into_iter()
        .collect();
        assert!(descends_from(&procs, 12, 10));
        assert!(descends_from(&procs, 10, 10));
        assert!(!descends_from(&procs, 20, 10));
        assert!(!descends_from(&procs, 1, 10));
    }

    #[test]
    fn status_memory_counts_anonymous_and_shared_and_not_file_pages() {
        let status = "Name:\tworker\nVmRSS:\t  900000 kB\nRssAnon:\t  200000 kB\n\
                      RssFile:\t  650000 kB\nRssShmem:\t   50000 kB\n";
        assert_eq!(parse_status_memory(status), Some(250_000 * 1024));
        // An older kernel gives the total only.
        assert_eq!(
            parse_status_memory("VmRSS:\t  900000 kB\n"),
            Some(900_000 * 1024)
        );
        assert_eq!(parse_status_memory("Name:\tkthread\n"), None);
    }

    #[test]
    fn cgroup_files_are_read() {
        assert_eq!(
            page_cache_of("anon 100\nfile 900\nactive_file 300\ninactive_file 500\nshmem 100\n"),
            800
        );
        assert_eq!(
            parse_oom_kill("low 0\nhigh 0\nmax 3\noom 1\noom_kill 2\n"),
            Some(2)
        );
        assert_eq!(parse_oom_kill("low 0\n"), None);
    }

    #[test]
    fn the_agent_memory_source_is_settled_from_the_flag() {
        // On a host with no cgroup limit the flag is the total; with a limit
        // the smaller of the two is.
        match AgentMemory::detect(Some(1 << 30)) {
            AgentMemory::Sum { total } => assert_eq!(total, 1 << 30),
            AgentMemory::Cgroup { total, .. } => assert!(total <= 1 << 30),
            AgentMemory::Unknown => panic!("the flag gives a total"),
        }
        assert_eq!(AgentMemory::Unknown.total(), None);
    }

    /// The reader finds this process: it leads no group of its own in a test
    /// run, so the check is on the single-process read.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn the_memory_of_this_process_can_be_read() {
        let bytes = own_memory().expect("own memory");
        assert!(bytes > 1 << 20, "{bytes}");
    }
}
