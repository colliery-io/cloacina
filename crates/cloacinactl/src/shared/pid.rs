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

//! PID-file read/write/signal helpers used by `daemon stop` and `server stop`.

use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

/// Write the current process PID into `path`, creating the parent directory
/// if needed.
pub fn write(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("failed to create PID file parent dir {}", parent.display())
        })?;
    }
    fs::write(path, std::process::id().to_string())
        .with_context(|| format!("failed to write PID file {}", path.display()))
}

/// Read a PID from `path`. Returns an error if the file is missing or malformed.
pub fn read(path: &Path) -> Result<u32> {
    let s = fs::read_to_string(path)
        .with_context(|| format!("failed to read PID file {}", path.display()))?;
    s.trim()
        .parse::<u32>()
        .with_context(|| format!("PID file {} is malformed", path.display()))
}

/// Non-erroring variant — `None` when the file is absent or unreadable.
pub fn try_read(path: &Path) -> Option<u32> {
    fs::read_to_string(path).ok()?.trim().parse::<u32>().ok()
}

/// Remove the PID file, ignoring "not found" errors.
pub fn remove(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => {
            Err(anyhow::Error::new(e)
                .context(format!("failed to remove PID file {}", path.display())))
        }
    }
}

/// Send SIGTERM (or SIGKILL if `force`) to `pid` and wait up to `timeout` for
/// the process to disappear.
pub fn signal_and_wait(pid: u32, force: bool, timeout: Duration) -> Result<()> {
    let signum = if force {
        libc_signal::SIGKILL
    } else {
        libc_signal::SIGTERM
    };

    // SAFETY: `kill(2)` with a valid signal number has well-defined behavior.
    let rc = unsafe { libc::kill(pid as libc::pid_t, signum) };
    if rc != 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ESRCH) {
            // Process already gone — treat as success.
            return Ok(());
        }
        bail!("failed to signal pid {pid}: {err}");
    }

    if force {
        return Ok(()); // SIGKILL is immediate; no wait.
    }

    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        // SAFETY: `kill(pid, 0)` is the canonical "does this pid exist" probe.
        let rc = unsafe { libc::kill(pid as libc::pid_t, 0) };
        if rc != 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    bail!("pid {pid} did not exit within {timeout:?}")
}

mod libc_signal {
    pub use libc::{SIGKILL, SIGTERM};
}
