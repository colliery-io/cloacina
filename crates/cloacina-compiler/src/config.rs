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

//! Configuration for cloacina-compiler.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use cloacina::UniversalUuid;

/// Kernel-enforced resource ceilings applied to the cargo subprocess via
/// `setrlimit` in a `pre_exec` hook. Linux-only; on other platforms the
/// values are stored but not applied. Closes CLOACI-T-0575 / SEC-06
/// (kernel-bounded resource cost half).
#[derive(Debug, Clone)]
pub struct BuildRlimits {
    /// `RLIMIT_CPU` — CPU-seconds. Note: this is CPU time, not wall-clock;
    /// the wall-clock bound lives in `CompilerConfig::build_timeout`
    /// (CLOACI-T-0573).
    pub cpu_s: u64,
    /// `RLIMIT_AS` — total virtual address space, bytes.
    pub mem_bytes: u64,
    /// `RLIMIT_NOFILE` — max open file descriptors.
    pub files: u64,
    /// `RLIMIT_NPROC` — max user processes (per-uid). Bounds fork bombs.
    pub procs: u64,
}

/// Runtime configuration for the compiler service.
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    pub home: PathBuf,
    pub bind: SocketAddr,
    pub database_url: String,
    /// CLOACI-T-0779: when set, scope this compiler to a single tenant's Postgres
    /// schema for build isolation (claims/builds only that tenant's packages).
    pub tenant_schema: Option<String>,
    pub verbose: bool,

    /// How often to poll for new pending rows.
    pub poll_interval: Duration,

    /// Heartbeat update interval while a build is in flight.
    pub heartbeat_interval: Duration,

    /// Threshold past which a stuck `building` row is reset to `pending`.
    /// Must be meaningfully larger than `heartbeat_interval` (spec recommends 6×).
    pub stale_threshold: Duration,

    /// How often the sweeper checks for stale rows.
    pub sweep_interval: Duration,

    /// Cargo flags, e.g. `["build", "--release", "--lib"]`.
    pub cargo_flags: Vec<String>,

    /// Maximum wall-clock time for a single `cargo build`. Builds exceeding
    /// this are SIGKILL'd; the row's heartbeat stops and the stale-build
    /// sweeper resets it back to `pending`. Closes CLOACI-T-0573 / OPS-10
    /// and bounds the wall-clock attack surface for SEC-06.
    pub build_timeout: Duration,

    /// Root directory for unpacking source during builds. Defaults to
    /// `$home/build-tmp` when unset.
    pub tmp_root: Option<PathBuf>,

    /// Shared `CARGO_TARGET_DIR` for per-package builds. When set, every
    /// unpacked package compiles against the same cargo target cache, so
    /// transitive deps (cloacina-workflow, cloacina-macros, …) only compile
    /// once. Falls back to the unpacked dir's default `target/` when unset.
    pub cargo_target_dir: Option<PathBuf>,

    /// Override for the cargo subprocess's `CARGO_HOME`. When `Some`, the
    /// cargo child sees only the registry cache + git checkouts under this
    /// dir. Combined with the `--frozen --offline` defaults, this is how
    /// the operator gates which dependencies a package build can resolve.
    /// `None` leaves `CARGO_HOME` untouched (cargo uses `~/.cargo`).
    /// Operator workflow: `cargo vendor` against a known-good source tree,
    /// point `--vendor-dir` at the resulting cargo home. Closes the
    /// network-side of CLOACI-T-0574 / SEC-06.
    pub vendor_dir: Option<PathBuf>,

    /// Kernel-enforced resource ceilings applied via `setrlimit` in a
    /// `pre_exec` hook on Linux. Stored on all platforms but only applied
    /// on Linux (`#[cfg(target_os = "linux")]`). CLOACI-T-0575 / SEC-06.
    pub build_rlimits: BuildRlimits,

    /// Unique identifier for this compiler process. Generated once at
    /// startup and stamped on every `compiler.build.started` /
    /// `compiler.build.finished` audit event so operators can correlate
    /// builds against a specific compiler instance during forensics.
    /// CLOACI-T-0576.
    pub compiler_instance_id: UniversalUuid,

    /// Number of daily-rotated log files to retain on disk. `0` disables
    /// pruning (unbounded). CLOACI-I-0109 / T-0592.
    pub log_retention_days: u64,
}

impl CompilerConfig {
    /// Resolve the effective tmp-root — uses `$home/build-tmp` when unset.
    pub fn tmp_root_or_default(&self) -> PathBuf {
        self.tmp_root
            .clone()
            .unwrap_or_else(|| self.home.join("build-tmp"))
    }
}
