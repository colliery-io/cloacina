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

//! The one place that makes a child process for a package build
//! (CLOACINA-T-1054).
//!
//! A package build starts `cargo` several times: to fetch, to build, to read
//! the dependency graph, to build a provider crate and to run its manifest
//! binary. Some of these run code that the package author wrote. In the
//! compiler service the author is a tenant, and the compiler's own environment
//! holds `DATABASE_URL`. So every such child is made by
//! [`build_child_command`], and no other code in the compiler or in this
//! `packaging` module may call `Command::new` (a test checks the source).
//!
//! A [`ChildPolicy`] says how the child is held:
//!
//! - [`ChildPolicy::confined`] — the compiler's posture. The environment is
//!   cleared. The child gets the names of [`CHILD_ENV_ALLOWLIST`] from the
//!   parent, the variables the policy sets by name, a time limit, resource
//!   limits (Linux) and its own process group, so a time-out kills the group.
//!   A [`ChildStep::Resolve`] child (one that downloads and runs no package
//!   code) also gets the names of [`RESOLVE_ENV_ALLOWLIST`].
//! - [`ChildPolicy::inherit`] — the author's own machine (`cloacinactl`,
//!   tests). The child gets the caller's environment and no limit.
//! - [`ChildPolicy::worker`] — a task worker of a fleet agent
//!   (CLOACINA-T-1028). The child gets the caller's environment, resource
//!   limits (Linux) and its own process group. The agent holds the clock and
//!   kills the group with [`kill_child_group`].
//!
//! This is not a sandbox. A confined child still has the compiler's user id,
//! filesystem and network.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Variables a confined child takes from the parent process. Everything else
/// is dropped.
pub const CHILD_ENV_ALLOWLIST: &[&str] = &[
    // Locate cargo, rustc, cc and friends.
    "PATH",
    // cargo and rustup fall back to ~/.cargo and ~/.rustup through HOME.
    "HOME",
    // Operator-pinned toolchain and registry locations.
    "CARGO_HOME",
    "RUSTUP_HOME",
    "RUSTUP_TOOLCHAIN",
    // Scratch space and terminal niceties.
    "TMPDIR",
    "TERM",
    // Operator override for the debuginfo level of built packages.
    "CARGO_PROFILE_DEV_DEBUG",
];

/// Extra variables for a confined [`ChildStep::Resolve`] child: `cargo fetch`
/// and `cargo metadata` download crates, so they need the operator's proxy,
/// certificate and cargo network settings. No registry token is on the list:
/// registry credentials belong in the cargo home's files.
pub const RESOLVE_ENV_ALLOWLIST: &[&str] = &[
    // Proxies (curl and cargo read both spellings).
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "NO_PROXY",
    "ALL_PROXY",
    "http_proxy",
    "https_proxy",
    "no_proxy",
    "all_proxy",
    // Certificate stores.
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
    // Cargo's own network settings.
    "CARGO_HTTP_PROXY",
    "CARGO_HTTP_CAINFO",
    "CARGO_HTTP_TIMEOUT",
    "CARGO_HTTP_MULTIPLEXING",
    "CARGO_NET_RETRY",
    "CARGO_NET_GIT_FETCH_WITH_CLI",
    "CARGO_NET_OFFLINE",
    "CARGO_REGISTRIES_CRATES_IO_PROTOCOL",
];

/// What a child does, as far as its environment goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildStep {
    /// Resolves and downloads dependencies and runs no package code
    /// (`cargo fetch`, `cargo metadata`).
    Resolve,
    /// Builds or runs package code (`cargo build`, `cargo run`).
    Build,
}

/// Kernel resource ceilings for a confined child, set with `setrlimit` before
/// `exec`. Applied on Linux only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildRlimits {
    /// `RLIMIT_CPU`, CPU-seconds.
    pub cpu_s: u64,
    /// `RLIMIT_AS`, bytes of address space.
    pub mem_bytes: u64,
    /// `RLIMIT_NOFILE`, open file descriptors.
    pub files: u64,
    /// `RLIMIT_NPROC`, processes of the user.
    pub procs: u64,
}

impl ChildRlimits {
    /// The same limits, each lowered to the calling process's own hard limit
    /// where that is lower. A process without privileges cannot give a child
    /// a hard limit above its own: `setrlimit` fails and the child does not
    /// start. `u64::MAX` in a field therefore means "the caller's own limit".
    /// Not Linux: unchanged (the limits are not applied there).
    pub fn capped_at_own_limits(self) -> Self {
        #[cfg(target_os = "linux")]
        {
            let cap = |resource: libc::__rlimit_resource_t, val: u64| -> u64 {
                let mut own = libc::rlimit {
                    rlim_cur: 0,
                    rlim_max: 0,
                };
                // SAFETY: `getrlimit` writes one `rlimit` to the pointer.
                if unsafe { libc::getrlimit(resource, &mut own) } != 0 {
                    return val;
                }
                val.min(own.rlim_max)
            };
            Self {
                cpu_s: cap(libc::RLIMIT_CPU, self.cpu_s),
                mem_bytes: cap(libc::RLIMIT_AS, self.mem_bytes),
                files: cap(libc::RLIMIT_NOFILE, self.files),
                procs: cap(libc::RLIMIT_NPROC, self.procs),
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            self
        }
    }
}

/// How a build child is held. See the module docs.
#[derive(Debug, Clone)]
pub struct ChildPolicy {
    confined: bool,
    own_group: bool,
    env: Vec<(OsString, OsString)>,
    timeout: Option<Duration>,
    rlimits: Option<ChildRlimits>,
}

impl ChildPolicy {
    /// The caller's own code on the caller's own machine: the child gets the
    /// whole environment, no time limit and no resource limit.
    pub fn inherit() -> Self {
        Self {
            confined: false,
            own_group: false,
            env: Vec::new(),
            timeout: None,
            rlimits: None,
        }
    }

    /// Code of another party (a tenant's package in the compiler service):
    /// cleared environment plus the allowlist, `timeout` as the wall-clock
    /// limit, `rlimits` as the kernel ceilings, an own process group.
    pub fn confined(timeout: Duration, rlimits: ChildRlimits) -> Self {
        Self {
            confined: true,
            own_group: true,
            env: Vec::new(),
            timeout: Some(timeout),
            rlimits: Some(rlimits),
        }
    }

    /// A task worker of a fleet agent (CLOACINA-T-1028): the caller's
    /// environment and stdio, `rlimits` as the kernel ceilings, an own
    /// process group. No time limit here: the agent holds the task's clock
    /// and kills the group with [`kill_child_group`].
    ///
    /// The environment is kept because a task reads it (a cloud identity, a
    /// proxy, a time zone), and the agent takes its own key out of its
    /// environment at start.
    pub fn worker(rlimits: ChildRlimits) -> Self {
        Self {
            confined: false,
            own_group: true,
            env: Vec::new(),
            timeout: None,
            rlimits: Some(rlimits),
        }
    }

    /// Give the child one more variable, by name. A later value for the same
    /// name wins.
    pub fn env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.env
            .push((key.as_ref().to_os_string(), value.as_ref().to_os_string()));
        self
    }

    /// True for a [`ChildPolicy::confined`] policy.
    pub fn is_confined(&self) -> bool {
        self.confined
    }

    /// The wall-clock limit, if any.
    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    /// The value a [`ChildStep::Build`] child sees for `key`: the value the
    /// policy sets, else the parent's value when the child inherits that name.
    /// Used to find where a child wrote (`CARGO_TARGET_DIR`).
    pub fn env_value(&self, key: &str) -> Option<OsString> {
        if let Some((_, v)) = self.env.iter().rev().find(|(k, _)| k == key) {
            return Some(v.clone());
        }
        if self.confined && !CHILD_ENV_ALLOWLIST.contains(&key) {
            return None;
        }
        std::env::var_os(key)
    }
}

/// Make the command for one build child. This is the only `Command::new` for
/// package builds: see the module docs.
///
/// The caller adds arguments, the working directory and stdio, then runs it
/// with [`run_build_child`] (blocking) or turns it into a
/// `tokio::process::Command` and enforces [`ChildPolicy::timeout`] itself,
/// killing with [`kill_child_group`].
pub fn build_child_command(program: &str, policy: &ChildPolicy, step: ChildStep) -> Command {
    let mut cmd = Command::new(program);
    if policy.confined {
        cmd.env_clear();
        for key in CHILD_ENV_ALLOWLIST {
            if let Some(val) = std::env::var_os(key) {
                cmd.env(key, val);
            }
        }
        if step == ChildStep::Resolve {
            for key in RESOLVE_ENV_ALLOWLIST {
                if let Some(val) = std::env::var_os(key) {
                    cmd.env(key, val);
                }
            }
        }
        cmd.stdin(Stdio::null());
    }
    #[cfg(unix)]
    if policy.own_group {
        use std::os::unix::process::CommandExt;
        // Own process group: a time-out kills cargo and what it started
        // (rustc, build scripts, a provider's manifest binary) together. For
        // a task worker: the worker and what the task started.
        cmd.process_group(0);
    }
    for (key, val) in &policy.env {
        cmd.env(key, val);
    }
    if let Some(rlimits) = &policy.rlimits {
        apply_rlimits(&mut cmd, rlimits);
    }
    cmd
}

/// Install a `pre_exec` hook that calls `setrlimit` for each limit. The kernel
/// then enforces the ceiling on the child and on what it starts.
///
/// The closure runs after fork and before exec. It calls only `setrlimit`,
/// which is async-signal-safe; it does not allocate. On failure the spawn
/// fails.
#[cfg(target_os = "linux")]
fn apply_rlimits(cmd: &mut Command, rlimits: &ChildRlimits) {
    use std::os::unix::process::CommandExt;

    let ChildRlimits {
        cpu_s,
        mem_bytes,
        files,
        procs,
    } = *rlimits;

    // SAFETY: the closure runs post-fork, pre-exec. It calls only
    // `libc::setrlimit`, async-signal-safe per POSIX. No allocation, no
    // locks, no Rust-side panic paths.
    unsafe {
        cmd.pre_exec(move || {
            let set = |resource: libc::__rlimit_resource_t, val: u64| -> std::io::Result<()> {
                let rlim = libc::rlimit {
                    rlim_cur: val,
                    rlim_max: val,
                };
                if libc::setrlimit(resource, &rlim) != 0 {
                    Err(std::io::Error::last_os_error())
                } else {
                    Ok(())
                }
            };
            set(libc::RLIMIT_CPU, cpu_s)?;
            set(libc::RLIMIT_AS, mem_bytes)?;
            set(libc::RLIMIT_NOFILE, files)?;
            set(libc::RLIMIT_NPROC, procs)?;
            Ok(())
        });
    }
}

/// Not Linux: the limits are carried but not applied. The server and the
/// compiler are deployed on Linux; other systems are development hosts.
#[cfg(not(target_os = "linux"))]
fn apply_rlimits(_cmd: &mut Command, _rlimits: &ChildRlimits) {}

/// SIGKILL the process group of a child. `pid` is the child's pid, which is
/// also its group id ([`build_child_command`] makes it a group leader). Call
/// it only for a child of a confined or a worker policy: a child of
/// [`ChildPolicy::inherit`] is in the caller's own group.
#[cfg(unix)]
pub fn kill_child_group(pid: u32) {
    if pid == 0 {
        return;
    }
    // SAFETY: `killpg` has no memory-safety preconditions.
    unsafe {
        libc::killpg(pid as libc::pid_t, libc::SIGKILL);
    }
}

/// Not Unix: no process groups; the caller kills the child itself.
#[cfg(not(unix))]
pub fn kill_child_group(_pid: u32) {}

/// What a finished child left.
#[derive(Debug)]
pub struct ChildOutput {
    /// The exit status.
    pub status: ExitStatus,
    /// Captured stdout (empty when not captured).
    pub stdout: Vec<u8>,
    /// Captured stderr (empty when not captured).
    pub stderr: Vec<u8>,
}

/// Why [`run_build_child`] gave no output.
#[derive(Debug, thiserror::Error)]
pub enum ChildError {
    /// The program could not be started.
    #[error("spawn failed: {0}")]
    Spawn(std::io::Error),
    /// Waiting for the child failed.
    #[error("wait failed: {0}")]
    Wait(std::io::Error),
    /// The child, or a process that kept its output open, outlived the time
    /// limit and was killed.
    #[error("timed out after {0:?}")]
    TimedOut(Duration),
}

/// Run a command made by [`build_child_command`] to its end, on the calling
/// thread, under the policy's time limit.
///
/// `capture` pipes stdout and stderr into the result; without it the child
/// writes to the caller's stdout and stderr. At the time limit the child's
/// process group is killed and [`ChildError::TimedOut`] is returned. The limit
/// also covers the output: a process that outlives the child and holds its
/// pipes cannot keep the caller waiting.
pub fn run_build_child(
    mut cmd: Command,
    policy: &ChildPolicy,
    capture: bool,
) -> Result<ChildOutput, ChildError> {
    if capture {
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    }
    let deadline = policy.timeout.map(|t| (t, Instant::now() + t));
    let mut child = cmd.spawn().map_err(ChildError::Spawn)?;
    let pid = child.id();
    let kill_group = |pid: u32| {
        if policy.own_group {
            kill_child_group(pid);
        }
    };

    // Drain the pipes on threads, so a chatty child never blocks on a full pipe.
    let (tx, rx) = mpsc::channel::<(bool, Vec<u8>)>();
    let mut readers = 0;
    if let Some(mut out) = child.stdout.take() {
        let tx = tx.clone();
        readers += 1;
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = out.read_to_end(&mut buf);
            let _ = tx.send((true, buf));
        });
    }
    if let Some(mut err) = child.stderr.take() {
        let tx = tx.clone();
        readers += 1;
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = err.read_to_end(&mut buf);
            let _ = tx.send((false, buf));
        });
    }
    drop(tx);

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(e) => {
                kill_group(pid);
                let _ = child.kill();
                let _ = child.wait();
                return Err(ChildError::Wait(e));
            }
        }
        if let Some((limit, at)) = deadline
            && Instant::now() >= at
        {
            kill_group(pid);
            let _ = child.kill();
            let _ = child.wait();
            return Err(ChildError::TimedOut(limit));
        }
        std::thread::sleep(Duration::from_millis(10));
    };

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    for _ in 0..readers {
        let got = match deadline {
            Some((limit, at)) => {
                match rx.recv_timeout(at.saturating_duration_since(Instant::now())) {
                    Ok(got) => Some(got),
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        kill_group(pid);
                        return Err(ChildError::TimedOut(limit));
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => None,
                }
            }
            None => rx.recv().ok(),
        };
        match got {
            Some((true, buf)) => stdout = buf,
            Some((false, buf)) => stderr = buf,
            None => break,
        }
    }

    Ok(ChildOutput {
        status,
        stdout,
        stderr,
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use serial_test::serial;

    fn test_rlimits() -> ChildRlimits {
        ChildRlimits {
            cpu_s: 600,
            mem_bytes: 8 * 1024 * 1024 * 1024,
            files: 4096,
            procs: 4096,
        }
    }

    fn confined(timeout: Duration) -> ChildPolicy {
        ChildPolicy::confined(timeout, test_rlimits())
    }

    /// Print the environment of a child made with `policy` for `step`.
    fn child_env(policy: &ChildPolicy, step: ChildStep) -> String {
        let cmd = build_child_command("env", policy, step);
        let out = run_build_child(cmd, policy, true).expect("run `env`");
        assert!(out.status.success(), "`env` exited nonzero");
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn has_var(printed: &str, key: &str) -> bool {
        let prefix = format!("{key}=");
        printed.lines().any(|l| l.starts_with(&prefix))
    }

    /// CLOACINA-T-1054: a confined child sees none of the parent's variables
    /// outside the allowlist, for either step. The parent here holds a
    /// database URL, a marker and a registry token.
    #[test]
    #[serial]
    fn confined_child_sees_none_of_the_parent_secrets() {
        let secrets = [
            ("DATABASE_URL", "postgres://user:secret@host/db"),
            ("CLOACINA_T_1054_MARKER", "parent-only"),
            ("CLOACINA_SECRET_KEK", "kek"),
            ("CARGO_REGISTRY_TOKEN", "token"),
        ];
        for (k, v) in secrets {
            // SAFETY: serial test; no other thread of this test reads the env.
            unsafe { std::env::set_var(k, v) };
        }

        let policy = confined(Duration::from_secs(30));
        for step in [ChildStep::Build, ChildStep::Resolve] {
            let printed = child_env(&policy, step);
            for (k, _) in secrets {
                assert!(
                    !has_var(&printed, k),
                    "{k} reached a {step:?} child:\n{printed}"
                );
            }
            assert!(has_var(&printed, "PATH"), "PATH must pass:\n{printed}");
            // Every name the child has is on a list.
            for line in printed.lines() {
                let Some((name, _)) = line.split_once('=') else {
                    continue;
                };
                assert!(
                    CHILD_ENV_ALLOWLIST.contains(&name) || RESOLVE_ENV_ALLOWLIST.contains(&name),
                    "{name} is in a {step:?} child and on no list:\n{printed}"
                );
            }
        }

        for (k, _) in secrets {
            // SAFETY: as above.
            unsafe { std::env::remove_var(k) };
        }
    }

    /// The network variables reach a resolve child and not a build child.
    #[test]
    #[serial]
    fn network_variables_reach_the_resolve_step_only() {
        // SAFETY: serial test; no other thread of this test reads the env.
        unsafe { std::env::set_var("HTTPS_PROXY", "http://proxy.example:3128") };
        unsafe { std::env::set_var("CARGO_NET_RETRY", "10") };

        let policy = confined(Duration::from_secs(30));
        let resolve = child_env(&policy, ChildStep::Resolve);
        let build = child_env(&policy, ChildStep::Build);

        // SAFETY: as above.
        unsafe { std::env::remove_var("HTTPS_PROXY") };
        unsafe { std::env::remove_var("CARGO_NET_RETRY") };

        assert!(has_var(&resolve, "HTTPS_PROXY"), "{resolve}");
        assert!(has_var(&resolve, "CARGO_NET_RETRY"), "{resolve}");
        assert!(!has_var(&build, "HTTPS_PROXY"), "{build}");
        assert!(!has_var(&build, "CARGO_NET_RETRY"), "{build}");
    }

    /// A variable the policy names reaches the child, and `env_value` tells
    /// the caller the same value.
    #[test]
    #[serial]
    fn policy_variables_reach_the_child_by_name() {
        // SAFETY: serial test; no other thread of this test reads the env.
        unsafe { std::env::set_var("CARGO_TARGET_DIR", "/parent/target") };
        let bare = confined(Duration::from_secs(30));
        let named = bare.clone().env("CARGO_TARGET_DIR", "/policy/target");
        let bare_env = child_env(&bare, ChildStep::Build);
        let named_env = child_env(&named, ChildStep::Build);
        let inherit_value = ChildPolicy::inherit().env_value("CARGO_TARGET_DIR");
        // SAFETY: as above.
        unsafe { std::env::remove_var("CARGO_TARGET_DIR") };

        assert!(!has_var(&bare_env, "CARGO_TARGET_DIR"), "{bare_env}");
        assert_eq!(bare.env_value("CARGO_TARGET_DIR"), None);
        assert!(
            named_env
                .lines()
                .any(|l| l == "CARGO_TARGET_DIR=/policy/target"),
            "{named_env}"
        );
        assert_eq!(
            named.env_value("CARGO_TARGET_DIR"),
            Some(OsString::from("/policy/target"))
        );
        assert_eq!(inherit_value, Some(OsString::from("/parent/target")));
    }

    /// An inherit child keeps the caller's environment (the author's machine).
    #[test]
    #[serial]
    fn inherit_child_keeps_the_caller_environment() {
        // SAFETY: serial test; no other thread of this test reads the env.
        unsafe { std::env::set_var("CLOACINA_T_1054_MARKER", "parent-only") };
        let printed = child_env(&ChildPolicy::inherit(), ChildStep::Build);
        // SAFETY: as above.
        unsafe { std::env::remove_var("CLOACINA_T_1054_MARKER") };
        assert!(has_var(&printed, "CLOACINA_T_1054_MARKER"), "{printed}");
    }

    /// CLOACINA-T-1028: a task worker keeps the caller's environment (a task
    /// reads it), and is the leader of its own process group, so the agent
    /// can kill it with what it started.
    #[test]
    #[serial]
    fn worker_child_keeps_the_environment_and_leads_its_own_group() {
        // SAFETY: serial test; no other thread of this test reads the env.
        unsafe { std::env::set_var("CLOACINA_T_1028_MARKER", "from-the-agent") };
        let policy = ChildPolicy::worker(test_rlimits().capped_at_own_limits());
        let printed = child_env(&policy, ChildStep::Build);
        // SAFETY: as above.
        unsafe { std::env::remove_var("CLOACINA_T_1028_MARKER") };
        assert!(has_var(&printed, "CLOACINA_T_1028_MARKER"), "{printed}");
        assert_eq!(policy.timeout(), None);
        assert!(!policy.is_confined());

        let mut cmd = build_child_command("sh", &policy, ChildStep::Build);
        cmd.args(["-c", "sleep 30 & echo $!; wait"]);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd.spawn().expect("spawn sh");
        let mut out = child.stdout.take().expect("stdout");
        let mut line = String::new();
        std::io::BufRead::read_line(&mut std::io::BufReader::new(&mut out), &mut line)
            .expect("read the sleeper's pid");
        let sleeper: libc::pid_t = line.trim().parse().expect("pid");
        // SAFETY: `getpgid` has no memory-safety preconditions.
        let group = unsafe { libc::getpgid(sleeper) };
        assert_eq!(
            group,
            child.id() as libc::pid_t,
            "the worker leads the group"
        );

        kill_child_group(child.id());
        let _ = child.wait();
        let gone = (0..200).any(|_| {
            // SAFETY: signal 0 only checks that the process exists.
            let alive = unsafe { libc::kill(sleeper, 0) } == 0;
            if alive {
                std::thread::sleep(Duration::from_millis(25));
            }
            !alive
        });
        assert!(gone, "the sleeper {sleeper} outlived the group kill");
    }

    /// A limit above the caller's own hard limit is lowered to it, so the
    /// child can start. `u64::MAX` asks for the caller's own limit.
    #[test]
    fn rlimits_are_capped_at_the_callers_own() {
        let asked = ChildRlimits {
            cpu_s: u64::MAX,
            mem_bytes: u64::MAX,
            files: u64::MAX,
            procs: u64::MAX,
        };
        let capped = asked.capped_at_own_limits();
        let policy = ChildPolicy::worker(capped);
        let cmd = build_child_command("true", &policy, ChildStep::Build);
        let out = run_build_child(cmd, &policy, true).expect("the child starts");
        assert!(out.status.success());
        let small = test_rlimits().capped_at_own_limits();
        assert!(small.files <= 4096 && small.procs <= 4096);
    }

    /// A child that sleeps past the time limit is killed and reported.
    #[test]
    fn child_past_the_time_limit_is_killed() {
        let policy = confined(Duration::from_millis(500));
        let mut cmd = build_child_command("sleep", &policy, ChildStep::Build);
        cmd.arg("30");
        let started = Instant::now();
        let err = run_build_child(cmd, &policy, true).expect_err("must time out");
        let elapsed = started.elapsed();
        assert!(matches!(err, ChildError::TimedOut(_)), "got {err:?}");
        assert!(
            elapsed < Duration::from_secs(10),
            "the kill took {elapsed:?}"
        );
    }

    /// The time limit holds for what the child started: a shell that starts a
    /// sleeper and waits is killed with it, and the pipes close.
    #[cfg(unix)]
    #[test]
    fn time_limit_kills_the_process_group() {
        let policy = confined(Duration::from_millis(500));
        let mut cmd = build_child_command("sh", &policy, ChildStep::Build);
        cmd.args(["-c", "sleep 30 & echo $!; wait"]);
        cmd.stdout(Stdio::piped()).stderr(Stdio::null());
        let mut child = cmd.spawn().expect("spawn sh");
        let mut out = child.stdout.take().expect("stdout");
        let mut line = String::new();
        std::io::BufRead::read_line(&mut std::io::BufReader::new(&mut out), &mut line)
            .expect("read the sleeper's pid");
        let sleeper: libc::pid_t = line.trim().parse().expect("pid");

        kill_child_group(child.id());
        let _ = child.wait();

        // The sleeper is a child of the dead shell; once killed it is reaped
        // by init, and `kill(pid, 0)` fails.
        let gone = (0..200).any(|_| {
            // SAFETY: signal 0 only checks that the process exists.
            let alive = unsafe { libc::kill(sleeper, 0) } == 0;
            if alive {
                std::thread::sleep(Duration::from_millis(25));
            }
            !alive
        });
        assert!(gone, "the sleeper {sleeper} outlived the group kill");
    }

    /// A process that outlives the child and holds its stdout does not keep
    /// the caller past the time limit.
    #[cfg(unix)]
    #[test]
    fn time_limit_covers_a_held_pipe() {
        let policy = confined(Duration::from_millis(500));
        let mut cmd = build_child_command("sh", &policy, ChildStep::Build);
        // The shell exits at once; the sleeper keeps the pipe.
        cmd.args(["-c", "sleep 30 & exit 0"]);
        let started = Instant::now();
        let err = run_build_child(cmd, &policy, true).expect_err("must time out");
        assert!(matches!(err, ChildError::TimedOut(_)), "got {err:?}");
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    /// CLOACINA-T-1054: no code in this `packaging` module starts a process
    /// except through `build_child_command`. A new `Command::new` in a sibling
    /// file fails this test.
    #[test]
    fn packaging_starts_no_process_outside_this_file() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/packaging");
        let hits = command_new_sites(&dir, &["build_child.rs"]);
        assert!(
            hits.is_empty(),
            "process started outside build_child.rs:\n{}",
            hits.join("\n")
        );
    }

    /// Lines under `dir` (recursive, `.rs`) that start a process directly.
    /// Comment lines do not count. Files named in `skip` are not read.
    fn command_new_sites(dir: &std::path::Path, skip: &[&str]) -> Vec<String> {
        let needle = ["Command", "::", "new"].concat();
        let mut hits = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        let mut files = 0;
        while let Some(d) = stack.pop() {
            for entry in std::fs::read_dir(&d).expect("read dir").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !name.ends_with(".rs") || skip.contains(&name) {
                    continue;
                }
                files += 1;
                let text = std::fs::read_to_string(&path).expect("read source");
                for (i, line) in text.lines().enumerate() {
                    if line.trim_start().starts_with("//") {
                        continue;
                    }
                    if line.contains(&needle) {
                        hits.push(format!("{}:{}: {}", path.display(), i + 1, line.trim()));
                    }
                }
            }
        }
        assert!(files > 0, "no source read under {}", dir.display());
        hits
    }
}
