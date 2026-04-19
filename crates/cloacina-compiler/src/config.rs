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

/// Runtime configuration for the compiler service.
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    pub home: PathBuf,
    pub bind: SocketAddr,
    pub database_url: String,
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

    /// Root directory for unpacking source during builds. Defaults to
    /// `$home/build-tmp` when unset.
    pub tmp_root: Option<PathBuf>,

    /// Shared `CARGO_TARGET_DIR` for per-package builds. When set, every
    /// unpacked package compiles against the same cargo target cache, so
    /// transitive deps (cloacina-workflow, cloacina-macros, …) only compile
    /// once. Falls back to the unpacked dir's default `target/` when unset.
    pub cargo_target_dir: Option<PathBuf>,
}

impl CompilerConfig {
    /// Resolve the effective tmp-root — uses `$home/build-tmp` when unset.
    pub fn tmp_root_or_default(&self) -> PathBuf {
        self.tmp_root
            .clone()
            .unwrap_or_else(|| self.home.join("build-tmp"))
    }
}
