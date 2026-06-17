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

//! Build execution: unpack source → cargo (or language-appropriate) build →
//! return the compiled cdylib bytes. Called per-claim from the compiler's
//! main loop.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use cloacina::dal::unified::workflow_registry_storage::UnifiedRegistryStorage;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use cloacina::security::audit;
use cloacina::UniversalUuid;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use tracing::{debug, info, warn};

use crate::config::{BuildRlimits, CompilerConfig};

/// Result of a single build attempt.
pub enum BuildOutcome {
    Success(Vec<u8>),
    Failed(String),
    /// The cargo subprocess exceeded `CompilerConfig::build_timeout` and was
    /// killed. The build row's heartbeat stops and the stale-build sweeper
    /// resets it to `pending` on its next tick — do NOT call
    /// `mark_build_failed` for this outcome (the row is reclaimed, not
    /// terminally failed). CLOACI-T-0573 / OPS-10.
    TimedOut {
        elapsed: Duration,
    },
}

/// Internal build-step error. `cargo_build` returns this so `run_build` can
/// distinguish "build failed cleanly" from "build was killed for taking too
/// long" before lowering to `BuildOutcome`. Carries cargo exit info for
/// T-0576's audit `finished` event.
#[derive(Debug)]
enum BuildError {
    Failed {
        reason: String,
        /// `Some(code)` if cargo exited normally; `None` for pre-spawn
        /// errors (target_dir mkdir, spawn failure) and signal-terminated
        /// cargo exits.
        exit_status: Option<i32>,
        /// `Some("SIGKILL")` etc. when cargo was signal-terminated.
        exit_signal: Option<String>,
    },
    TimedOut {
        elapsed: Duration,
    },
}

impl BuildError {
    /// Pre-spawn failure: no cargo subprocess ever ran, so exit fields
    /// are `None`. Used for filesystem / registry / unpack errors.
    fn internal(reason: impl Into<String>) -> Self {
        Self::Failed {
            reason: reason.into(),
            exit_status: None,
            exit_signal: None,
        }
    }
}

/// Successful cargo invocation. Carries the artifact plus exit info for
/// T-0576's audit `finished` event.
#[derive(Debug)]
struct CargoBuildSuccess {
    artifact: Vec<u8>,
    /// Cargo exits with `Some(0)` on success; we still capture it so the
    /// audit event has a uniform shape.
    exit_status: Option<i32>,
}

/// Execute a build for the given package id.
///
/// Fetches source bytes from the registry, unpacks them, runs the
/// language-appropriate build step, and returns the produced cdylib bytes
/// (empty for pure-Python packages) or an error tail.
pub async fn execute_build(
    registry: &WorkflowRegistryImpl<UnifiedRegistryStorage>,
    package_id: uuid::Uuid,
    config: &CompilerConfig,
) -> BuildOutcome {
    match run_build(registry, package_id, config).await {
        Ok(bytes) => BuildOutcome::Success(bytes),
        Err(BuildError::Failed { reason, .. }) => BuildOutcome::Failed(reason),
        Err(BuildError::TimedOut { elapsed }) => BuildOutcome::TimedOut { elapsed },
    }
}

/// SHA-256 of a file's bytes, hex-encoded. `Ok(None)` if the file doesn't
/// exist. Used by T-0576's audit emit to record source-tree provenance.
fn sha256_hex_if_present(path: &Path) -> std::io::Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    Ok(Some(hex::encode(digest)))
}

/// Translate a signal number to a name for the audit `exit_signal` field.
/// Only covers the signals the compiler actually sees (SIGKILL from
/// timeout, SIGSEGV/SIGABRT from rlimit overshoots, SIGTERM from
/// operator shutdown). Returns the number as a string for unrecognized
/// signals.
#[cfg(unix)]
fn signal_name(num: i32) -> String {
    match num {
        1 => "SIGHUP".into(),
        2 => "SIGINT".into(),
        6 => "SIGABRT".into(),
        9 => "SIGKILL".into(),
        11 => "SIGSEGV".into(),
        15 => "SIGTERM".into(),
        24 => "SIGXCPU".into(),
        other => format!("signal({other})"),
    }
}

async fn run_build(
    registry: &WorkflowRegistryImpl<UnifiedRegistryStorage>,
    package_id: uuid::Uuid,
    config: &CompilerConfig,
) -> Result<Vec<u8>, BuildError> {
    let build_started_at = std::time::Instant::now();
    let build_claim_id = UniversalUuid(package_id);

    let (meta, source_bytes) = registry
        .get_source_for_build(package_id)
        .await
        .map_err(|e| BuildError::internal(format!("failed to load source for {package_id}: {e}")))?
        .ok_or_else(|| {
            BuildError::internal(format!(
                "package {package_id} disappeared between claim and build"
            ))
        })?;

    let tmp_root = config.tmp_root_or_default();
    std::fs::create_dir_all(&tmp_root).map_err(|e| {
        BuildError::internal(format!(
            "failed to ensure tmp_root {}: {e}",
            tmp_root.display()
        ))
    })?;

    let work = TempDir::new_in(&tmp_root)
        .map_err(|e| BuildError::internal(format!("failed to create build tmpdir: {e}")))?;

    let archive_path = work.path().join("pkg.cloacina");
    std::fs::write(&archive_path, &source_bytes)
        .map_err(|e| BuildError::internal(format!("failed to stage archive: {e}")))?;

    let extract_dir = work.path().join("source");
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| BuildError::internal(format!("failed to prepare extract dir: {e}")))?;

    let archive_path_inner = archive_path.clone();
    let extract_dir_inner = extract_dir.clone();
    let source_dir = tokio::task::spawn_blocking(move || {
        fidius_core::package::unpack_package(&archive_path_inner, &extract_dir_inner)
    })
    .await
    .map_err(|e| BuildError::internal(format!("unpack task panicked: {e}")))?
    .map_err(|e| BuildError::internal(format!("fidius_core::unpack_package failed: {e}")))?;

    let manifest = load_manifest(&source_dir).map_err(BuildError::internal)?;
    let language = manifest_language(&manifest);

    info!(
        %package_id,
        package_name = %meta.package_name,
        version = %meta.version,
        language = %language,
        "starting build"
    );

    // CLOACI-T-0576: hash Cargo.toml / Cargo.lock for the audit `started`
    // event so operators have content-addressable provenance for every
    // build. Missing files are tolerated (Python packages have no
    // Cargo.toml; some packages have no Cargo.lock).
    let cargo_toml_hash = sha256_hex_if_present(&source_dir.join("Cargo.toml"))
        .map_err(|e| BuildError::internal(format!("failed to hash Cargo.toml: {e}")))?
        .unwrap_or_else(|| "<absent>".to_string());
    let cargo_lock_hash = sha256_hex_if_present(&source_dir.join("Cargo.lock"))
        .map_err(|e| BuildError::internal(format!("failed to hash Cargo.lock: {e}")))?;

    audit::log_compiler_build_started(
        build_claim_id,
        &meta.package_name,
        &meta.version,
        &cargo_toml_hash,
        cargo_lock_hash.as_deref(),
        config.compiler_instance_id,
    );

    let result = match language.as_str() {
        "python" => {
            debug!("pure-Python package: skipping cargo build, using empty artifact");
            Ok(CargoBuildSuccess {
                artifact: Vec::new(),
                exit_status: Some(0),
            })
        }
        _ => cargo_build(package_id, &source_dir, config).await,
    };

    // Single emit-finished point: classify outcome, compute wall-clock,
    // call the audit fn, then return the original Result up the stack.
    let wall_clock_ms = build_started_at
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let final_result: Result<Vec<u8>, BuildError> = match result {
        Ok(success) => {
            audit::log_compiler_build_finished(
                build_claim_id,
                &meta.package_name,
                &meta.version,
                &cargo_toml_hash,
                cargo_lock_hash.as_deref(),
                config.compiler_instance_id,
                "success",
                success.exit_status,
                None,
                wall_clock_ms,
                None,
            );
            info!(
                %package_id,
                artifact_bytes = success.artifact.len(),
                "build succeeded"
            );
            Ok(success.artifact)
        }
        Err(BuildError::Failed {
            reason,
            exit_status,
            exit_signal,
        }) => {
            audit::log_compiler_build_finished(
                build_claim_id,
                &meta.package_name,
                &meta.version,
                &cargo_toml_hash,
                cargo_lock_hash.as_deref(),
                config.compiler_instance_id,
                "failed",
                exit_status,
                exit_signal.as_deref(),
                wall_clock_ms,
                Some(&reason),
            );
            Err(BuildError::Failed {
                reason,
                exit_status,
                exit_signal,
            })
        }
        Err(BuildError::TimedOut { elapsed }) => {
            audit::log_compiler_build_finished(
                build_claim_id,
                &meta.package_name,
                &meta.version,
                &cargo_toml_hash,
                cargo_lock_hash.as_deref(),
                config.compiler_instance_id,
                "timeout_killed",
                None,
                Some("SIGKILL"),
                wall_clock_ms,
                Some(&format!(
                    "cargo build exceeded build_timeout after {}s",
                    elapsed.as_secs()
                )),
            );
            Err(BuildError::TimedOut { elapsed })
        }
    };
    final_result
}

fn load_manifest(source_dir: &Path) -> Result<toml::Value, String> {
    let manifest_path = source_dir.join("package.toml");
    let raw = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("failed to read {}: {e}", manifest_path.display()))?;
    let value: toml::Value = toml::from_str(&raw)
        .map_err(|e| format!("failed to parse {}: {e}", manifest_path.display()))?;
    Ok(value)
}

fn manifest_language(manifest: &toml::Value) -> String {
    // `language` is canonically a `[metadata]` field — that's what the upload
    // schema accepts and what every example/fixture sets. (CLOACI-T-0666: this
    // previously read `[package].language`, which is never populated, so every
    // packaged-Python build wrongly fell through to cargo and failed.) Keep a
    // `[package]` fallback for resilience against older hand-authored manifests.
    manifest
        .get("metadata")
        .and_then(|m| m.get("language"))
        .or_else(|| manifest.get("package").and_then(|p| p.get("language")))
        .and_then(|v| v.as_str())
        .unwrap_or("rust")
        .to_ascii_lowercase()
}

/// Classify a non-zero cargo exit into an operator-actionable error message
/// for the common offline / vendor-missing failure modes. Returns `None` if
/// the stderr doesn't match a known offline-failure shape — the caller
/// falls back to the raw stderr tail.
///
/// Patterns covered (cargo's exact wording can drift across releases; we
/// match conservatively on stable substrings):
///
/// 1. **Missing crates.** `error: no matching package named \`<name>\` found`
///    appears once per missing crate. We aggregate names into a single
///    "dependencies not available offline" message naming each.
/// 2. **Missing Cargo.lock under `--frozen`.** Builds with no lockfile
///    error with `the lock file ... needs to be generated but --frozen
///    was specified`. The operator needs the package author to commit
///    `Cargo.lock`.
/// 3. **Git source offline.** Builds whose deps are git-sourced and not
///    in the vendor cache error with `failed to load source for
///    dependency` + an "offline" mention. Operator vendors the dep.
///
/// CLOACI-T-0574 / SEC-06.
fn classify_offline_failure(stderr: &str) -> Option<String> {
    // 1. Missing-crate aggregation.
    let needle = "no matching package named `";
    let mut missing: Vec<String> = Vec::new();
    let mut cursor = stderr;
    while let Some(start) = cursor.find(needle) {
        let after = &cursor[start + needle.len()..];
        if let Some(end) = after.find('`') {
            let name = after[..end].to_string();
            if !missing.contains(&name) {
                missing.push(name);
            }
            cursor = &after[end..];
        } else {
            break;
        }
    }
    if !missing.is_empty() {
        return Some(format!(
            "dependencies not available offline: {}. \
             Operator workflow: `cargo vendor` against a source tree that \
             includes these crates, then point --vendor-dir at the \
             resulting cargo home.",
            missing.join(", ")
        ));
    }

    // 2. Missing Cargo.lock under --frozen.
    if stderr.contains("lock file")
        && stderr.contains("needs to be generated but --frozen was specified")
    {
        return Some(
            "Cargo.lock is missing from the package source; --frozen requires \
             it. Commit `Cargo.lock` to the package and re-upload."
                .to_string(),
        );
    }

    // 3. Git source unavailable offline.
    if stderr.contains("failed to load source for dependency") && stderr.contains("offline") {
        return Some(
            "Git dependency unavailable offline. Vendor the dependency \
             (`cargo vendor`) and point --vendor-dir at the result."
                .to_string(),
        );
    }

    None
}

async fn cargo_build(
    package_id: uuid::Uuid,
    source_dir: &Path,
    config: &CompilerConfig,
) -> Result<CargoBuildSuccess, BuildError> {
    const MAX_ERR: usize = 64 * 1024;

    let mut cmd = tokio::process::Command::new("cargo");
    cmd.args(&config.cargo_flags)
        .current_dir(source_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Belt-and-suspenders: if the awaited future is dropped (e.g. parent
        // task is cancelled), tokio will SIGKILL the cargo child rather than
        // leak it. We also kill explicitly on timeout, but this protects
        // against unexpected cancellation paths.
        .kill_on_drop(true);
    // Share a cargo target cache across builds so transitive deps
    // (cloacina-workflow, cloacina-macros, tokio, …) compile once and are
    // reused by every subsequent package. Without this, every build is a
    // cold ~120-crate compile.
    if let Some(target_dir) = &config.cargo_target_dir {
        std::fs::create_dir_all(target_dir).map_err(|e| {
            BuildError::internal(format!(
                "failed to create cargo_target_dir {}: {e}",
                target_dir.display()
            ))
        })?;
        cmd.env("CARGO_TARGET_DIR", target_dir);
    }
    // Default the produced cdylibs to line-tables-only debuginfo. These are
    // deployment artifacts the agent/server dlopen — never debugged
    // interactively — so full `debuginfo=2` only buys an LLVM memory blowup
    // (OOMs the build of workflows that pull in the heavy `cloacina` lib),
    // slower builds, and a larger artifact to ship by digest. `line-tables-only`
    // keeps file:line in panic backtraces at a fraction of the cost. Only set
    // when the operator hasn't already pinned it via the environment (the demo
    // compose stack, for instance, can force `0`). CLOACI-I-0124.
    if std::env::var_os("CARGO_PROFILE_DEV_DEBUG").is_none() {
        cmd.env("CARGO_PROFILE_DEV_DEBUG", "line-tables-only");
    }
    // CARGO_HOME override (CLOACI-T-0574). When the operator has pointed
    // --vendor-dir at a curated cargo home (output of `cargo vendor`), the
    // cargo subprocess sees only that registry. Combined with
    // `--frozen --offline` in the default cargo_flags, this gates which
    // dependencies a package build can resolve.
    if let Some(vendor) = &config.vendor_dir {
        cmd.env("CARGO_HOME", vendor);
    }
    // setrlimit hook (CLOACI-T-0575). Linux-only: the kernel enforces the
    // resource ceiling before cargo even starts, so a malicious build.rs
    // cannot exhaust CPU, address space, file descriptors, or proc slots.
    apply_rlimits(&mut cmd, &config.build_rlimits);

    let mut child = cmd
        .spawn()
        .map_err(|e| BuildError::internal(format!("failed to spawn cargo: {e}")))?;

    let started = std::time::Instant::now();
    let timeout = config.build_timeout;

    // Drain stdout + stderr concurrently with the wait. We want stderr available
    // both for the failure-tail message *and* for parsing missing-dep errors
    // (T-0574) out of cargo's structured output.
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let drain_stdout = tokio::spawn(async move {
        let mut buf = Vec::new();
        if let Some(mut s) = stdout {
            let _ = tokio::io::AsyncReadExt::read_to_end(&mut s, &mut buf).await;
        }
        buf
    });
    let drain_stderr = tokio::spawn(async move {
        let mut buf = Vec::new();
        if let Some(mut s) = stderr {
            let _ = tokio::io::AsyncReadExt::read_to_end(&mut s, &mut buf).await;
        }
        buf
    });

    let exit_status: std::process::ExitStatus =
        match tokio::time::timeout(timeout, child.wait()).await {
            Ok(Ok(status)) => status,
            Ok(Err(e)) => {
                return Err(BuildError::internal(format!("cargo wait failed: {e}")));
            }
            Err(_elapsed_err) => {
                // Timed out. Send SIGKILL to the cargo child and reap so we
                // don't leak a zombie. kill_on_drop would handle this on drop,
                // but doing it explicitly lets us reap *before* returning so the
                // post-condition is clean.
                let elapsed = started.elapsed();
                warn!(
                    %package_id,
                    elapsed_s = elapsed.as_secs(),
                    timeout_s = timeout.as_secs(),
                    "cargo build exceeded build_timeout; killing subprocess. \
                     Build row will be left for stale-build sweeper to reset."
                );
                if let Err(e) = child.kill().await {
                    warn!(%package_id, %e, "failed to SIGKILL timed-out cargo child");
                }
                // Reap. wait() after kill is bounded — the kernel has already
                // delivered SIGKILL by this point.
                let _ = child.wait().await;
                // Best-effort drain so the spawn handles don't hang their tasks.
                let _ = drain_stdout.await;
                let _ = drain_stderr.await;
                return Err(BuildError::TimedOut { elapsed });
            }
        };

    // Extract cargo's exit code / signal once for both the audit hand-off
    // and the failure-branch reporting. On Unix, signal-terminated children
    // have `code()` = None and `signal()` = Some(num); normal exits flip
    // that around.
    let cargo_exit_status_code: Option<i32> = exit_status.code();
    #[cfg(unix)]
    let cargo_exit_signal_name: Option<String> = {
        use std::os::unix::process::ExitStatusExt;
        exit_status.signal().map(signal_name)
    };
    #[cfg(not(unix))]
    let cargo_exit_signal_name: Option<String> = None;

    // Collect drained pipes (these complete after the child exits since the
    // kernel closes its write end on exit).
    let _stdout_bytes = drain_stdout.await.unwrap_or_default();
    let stderr_bytes = drain_stderr.await.unwrap_or_default();

    if !exit_status.success() {
        let stderr = String::from_utf8_lossy(&stderr_bytes);
        let tail = if stderr.len() > MAX_ERR {
            let start = stderr.len() - MAX_ERR;
            stderr[start..].to_string()
        } else {
            stderr.to_string()
        };
        warn!(status = ?exit_status, %package_id, "cargo build failed");
        // CLOACI-T-0574: try to classify the failure as an offline/vendor
        // issue first so the operator gets an actionable error instead of
        // a 64 KiB stderr tail.
        let reason = classify_offline_failure(&stderr)
            .unwrap_or_else(|| format!("cargo build failed:\n{tail}"));
        return Err(BuildError::Failed {
            reason,
            exit_status: cargo_exit_status_code,
            exit_signal: cargo_exit_signal_name,
        });
    }

    let target_subdir = profile_for_flags(&config.cargo_flags);
    let target_root = config
        .cargo_target_dir
        .clone()
        .unwrap_or_else(|| source_dir.join("target"));
    let target_dir = target_root.join(target_subdir);
    // With a shared cargo_target_dir, multiple packages' cdylibs coexist.
    // Match on the Cargo.toml [package].name to pick the right one.
    let pkg_name = read_cargo_package_name(source_dir).map_err(BuildError::internal)?;
    let lib_path = find_cdylib(&target_dir, &pkg_name).map_err(BuildError::internal)?;
    let artifact = std::fs::read(&lib_path).map_err(|e| {
        BuildError::internal(format!(
            "failed to read compiled library {}: {e}",
            lib_path.display()
        ))
    })?;

    Ok(CargoBuildSuccess {
        artifact,
        exit_status: cargo_exit_status_code,
    })
}

fn profile_for_flags(flags: &[String]) -> &'static str {
    if flags.iter().any(|f| f == "--release") {
        "release"
    } else {
        "debug"
    }
}

fn find_cdylib(target_dir: &Path, pkg_name: &str) -> Result<PathBuf, String> {
    let ext = if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };
    // Cargo normalizes `-` to `_` in the emitted libfoo.dylib.
    let normalized = pkg_name.replace('-', "_");
    let expected = format!("lib{}.{}", normalized, ext);

    let candidate = target_dir.join(&expected);
    if candidate.exists() {
        return Ok(candidate);
    }

    Err(format!(
        "expected {} in {} (built package: {})",
        expected,
        target_dir.display(),
        pkg_name
    ))
}

fn read_cargo_package_name(source_dir: &Path) -> Result<String, String> {
    let cargo_toml = source_dir.join("Cargo.toml");
    let raw = std::fs::read_to_string(&cargo_toml)
        .map_err(|e| format!("failed to read {}: {e}", cargo_toml.display()))?;
    let value: toml::Value = toml::from_str(&raw)
        .map_err(|e| format!("failed to parse {}: {e}", cargo_toml.display()))?;
    value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("[package].name missing in {}", cargo_toml.display()))
}

// ---------------------------------------------------------------------------
// CLOACI-T-0575: setrlimit hook on the cargo subprocess (Linux only)
// ---------------------------------------------------------------------------

/// Install a `pre_exec` hook on the cargo `Command` that calls `setrlimit`
/// for each configured limit. On Linux this is kernel-enforced; on other
/// targets this is a no-op (operator's deployment posture is Linux per
/// CLOACI-A-0005, but the codebase still builds on macOS for dev).
///
/// The `pre_exec` closure runs after fork but before exec. It must only
/// call async-signal-safe functions — `setrlimit` qualifies. No
/// allocation, no panics; on failure we `libc::_exit(1)` so the parent
/// observes the child died before doing useful work.
#[cfg(target_os = "linux")]
fn apply_rlimits(cmd: &mut tokio::process::Command, rlimits: &BuildRlimits) {
    use std::os::unix::process::CommandExt;

    let cpu_s = rlimits.cpu_s;
    let mem_bytes = rlimits.mem_bytes;
    let files = rlimits.files;
    let procs = rlimits.procs;

    // SAFETY: the closure runs post-fork, pre-exec. It calls only
    // `libc::setrlimit` and `libc::_exit`, both async-signal-safe per
    // POSIX. No allocation, no locks, no Rust-side panic paths.
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

/// Non-Linux fallback: rlimits stored on `CompilerConfig` but not applied.
/// macOS/Windows are dev-host only per CLOACI-A-0005.
#[cfg(not(target_os = "linux"))]
fn apply_rlimits(_cmd: &mut tokio::process::Command, _rlimits: &BuildRlimits) {
    // No-op. The non-Linux startup warning is emitted once from `run()`
    // (lib.rs); per-build logging here would be noisy.
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use std::time::Instant;

    /// Build a minimal cargo package whose `build.rs` sleeps for `sleep_secs`
    /// seconds. The library itself is an empty cdylib; the slow component is
    /// the build script. Returns the package directory.
    fn synthetic_sleeper_package(work: &Path, sleep_secs: u64) -> PathBuf {
        let pkg = work.join("t0573-sleeper");
        std::fs::create_dir_all(pkg.join("src")).expect("mkdir src");
        std::fs::write(
            pkg.join("Cargo.toml"),
            "[package]\n\
             name = \"t0573-sleeper\"\n\
             version = \"0.0.0\"\n\
             edition = \"2021\"\n\
             \n\
             [lib]\n\
             crate-type = [\"cdylib\"]\n",
        )
        .expect("write Cargo.toml");
        std::fs::write(pkg.join("src/lib.rs"), "").expect("write src/lib.rs");
        std::fs::write(
            pkg.join("build.rs"),
            format!(
                "fn main() {{\n    \
                     std::thread::sleep(std::time::Duration::from_secs({sleep_secs}));\n\
                 }}\n"
            ),
        )
        .expect("write build.rs");
        pkg
    }

    fn test_config(home: &Path, build_timeout: Duration) -> CompilerConfig {
        CompilerConfig {
            home: home.to_path_buf(),
            bind: "127.0.0.1:0".parse::<SocketAddr>().expect("parse bind"),
            database_url: "unused-in-this-test".to_string(),
            verbose: false,
            poll_interval: Duration::from_secs(1),
            heartbeat_interval: Duration::from_secs(5),
            stale_threshold: Duration::from_secs(30),
            sweep_interval: Duration::from_secs(15),
            cargo_flags: vec!["build".to_string()],
            tmp_root: None,
            cargo_target_dir: Some(home.join("target")),
            build_timeout,
            vendor_dir: None,
            // Generous rlimits for the regular tests so a normal cargo
            // build never trips them; rlimit-specific tests override the
            // relevant field explicitly.
            build_rlimits: BuildRlimits {
                cpu_s: 600,
                mem_bytes: 8 * 1024 * 1024 * 1024,
                files: 4096,
                procs: 4096,
            },
            compiler_instance_id: UniversalUuid::new_v4(),
            log_retention_days: 7,
        }
    }

    /// CLOACI-T-0573: a build whose `build.rs` exceeds `--build-timeout-s`
    /// returns `BuildError::TimedOut` within a bounded wall-clock window.
    ///
    /// We set `build_timeout = 5s` and `build.rs` sleeps for 60s. Whether
    /// the timeout fires during cargo's startup, during `build.rs` compile,
    /// or while `build.rs` is sleeping, the contract is the same: cargo is
    /// SIGKILL'd, drains complete, and `TimedOut` surfaces — bounded.
    #[tokio::test]
    async fn cargo_build_returns_timed_out_when_build_rs_sleeps_past_timeout() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let package = synthetic_sleeper_package(tmp.path(), 60);
        let config = test_config(tmp.path(), Duration::from_secs(5));

        let started = Instant::now();
        let result = cargo_build(uuid::Uuid::new_v4(), &package, &config).await;
        let elapsed = started.elapsed();

        // Wall-clock must be bounded by build_timeout + a tolerance for the
        // SIGKILL + reap + drain steps. If the kill path is wrong (e.g. we
        // leak the child or fail to reap), this assertion catches it.
        assert!(
            elapsed < Duration::from_secs(20),
            "cargo_build hung past the configured 5s timeout (elapsed = {elapsed:?})"
        );

        match result {
            Err(BuildError::TimedOut { elapsed: reported }) => {
                // Reported elapsed should be in the same ballpark as the
                // configured timeout — Instant::elapsed inside cargo_build
                // measures from spawn to the timeout firing.
                assert!(
                    reported >= Duration::from_secs(4) && reported < Duration::from_secs(20),
                    "reported elapsed {reported:?} not in [4s, 20s)"
                );
            }
            other => panic!("expected BuildError::TimedOut, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // CLOACI-T-0574: offline-failure classifier
    // -----------------------------------------------------------------------

    #[test]
    fn classify_offline_failure_extracts_single_missing_crate() {
        let stderr = "\
error: no matching package named `unobtanium` found\n\
location searched: registry `crates-io`\n\
required by package `t0574-victim v0.0.0 (/tmp/t0574)`\n";
        let msg = classify_offline_failure(stderr).expect("should classify");
        assert!(
            msg.contains("unobtanium"),
            "classifier output should name the missing crate, got: {msg}"
        );
        assert!(
            msg.starts_with("dependencies not available offline:"),
            "should use the canonical missing-deps preamble, got: {msg}"
        );
    }

    #[test]
    fn classify_offline_failure_aggregates_multiple_missing_crates() {
        let stderr = "\
error: no matching package named `alpha` found\n\
required by package `victim v0.0.0`\n\
error: no matching package named `beta` found\n\
required by package `victim v0.0.0`\n";
        let msg = classify_offline_failure(stderr).expect("should classify");
        assert!(msg.contains("alpha"), "should name alpha, got: {msg}");
        assert!(msg.contains("beta"), "should name beta, got: {msg}");
    }

    #[test]
    fn classify_offline_failure_dedupes_repeated_missing_crate_mentions() {
        let stderr = "\
error: no matching package named `alpha` found\n\
error: no matching package named `alpha` found\n";
        let msg = classify_offline_failure(stderr).expect("should classify");
        // The crate should appear exactly once in the joined output (`, alpha`
        // would indicate a duplicate after the first).
        let occurrences = msg.matches("alpha").count();
        assert_eq!(occurrences, 1, "alpha should appear once, got: {msg}");
    }

    #[test]
    fn classify_offline_failure_recognizes_missing_lockfile() {
        let stderr = "\
error: the lock file /tmp/t0574/Cargo.lock needs to be generated but --frozen was specified\n";
        let msg = classify_offline_failure(stderr).expect("should classify");
        assert!(
            msg.contains("Cargo.lock"),
            "should mention Cargo.lock, got: {msg}"
        );
        assert!(
            msg.contains("--frozen"),
            "should mention --frozen, got: {msg}"
        );
    }

    #[test]
    fn classify_offline_failure_recognizes_git_dep_offline() {
        let stderr = "\
error: failed to load source for dependency `mygit`\n\
Caused by:\n  attempting to make an HTTP request, but --offline was specified\n";
        let msg = classify_offline_failure(stderr).expect("should classify");
        assert!(
            msg.to_lowercase().contains("git"),
            "should mention git, got: {msg}"
        );
        assert!(
            msg.contains("--vendor-dir"),
            "should point at --vendor-dir, got: {msg}"
        );
    }

    #[test]
    fn classify_offline_failure_returns_none_for_unrelated_stderr() {
        // A regular compile error should not be misclassified as an
        // offline-failure — the caller falls back to the raw stderr tail.
        let stderr = "error[E0425]: cannot find value `foo` in this scope\n";
        assert!(classify_offline_failure(stderr).is_none());
    }

    // -----------------------------------------------------------------------
    // CLOACI-T-0575: setrlimit kills a build that overshoots its limits
    // -----------------------------------------------------------------------

    /// A `build.rs` that tries to allocate 8 GiB. Under RLIMIT_AS=128 MiB,
    /// the allocation aborts the build.rs process; cargo observes the
    /// build-script failure and exits non-zero.
    #[cfg(target_os = "linux")]
    fn synthetic_memory_hog_package(work: &Path) -> PathBuf {
        let pkg = work.join("t0575-memhog");
        std::fs::create_dir_all(pkg.join("src")).expect("mkdir src");
        std::fs::write(
            pkg.join("Cargo.toml"),
            "[package]\n\
             name = \"t0575-memhog\"\n\
             version = \"0.0.0\"\n\
             edition = \"2021\"\n\
             \n\
             [lib]\n\
             crate-type = [\"cdylib\"]\n",
        )
        .expect("write Cargo.toml");
        std::fs::write(pkg.join("src/lib.rs"), "").expect("write src/lib.rs");
        std::fs::write(
            pkg.join("build.rs"),
            "fn main() {\n    \
                 // Try to reserve 8 GiB. With RLIMIT_AS well below this,\n    \
                 // the allocation aborts the process before main returns.\n    \
                 let big: Vec<u8> = vec![0u8; 8 * 1024 * 1024 * 1024];\n    \
                 // Touch a byte so the optimizer can't elide the alloc.\n    \
                 std::hint::black_box(&big[0]);\n\
             }\n",
        )
        .expect("write build.rs");
        pkg
    }

    /// CLOACI-T-0575 (Linux only): a build whose `build.rs` allocates past
    /// `RLIMIT_AS` is killed by the kernel; `cargo_build` surfaces it as
    /// `BuildError::Failed` (not `TimedOut` — the wall-clock budget was
    /// never approached).
    ///
    /// We squeeze RLIMIT_AS to 128 MiB. The build script attempts an 8 GiB
    /// allocation; on Linux the kernel refuses, glibc aborts, cargo exits
    /// non-zero.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn cargo_build_fails_when_build_rs_overshoots_rlimit_as() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let package = synthetic_memory_hog_package(tmp.path());
        let mut config = test_config(tmp.path(), Duration::from_secs(60));
        // 128 MiB — well under the 8 GiB the build.rs wants. Keep CPU and
        // other limits generous so we only test the AS dimension.
        config.build_rlimits.mem_bytes = 128 * 1024 * 1024;

        let started = std::time::Instant::now();
        let result = cargo_build(uuid::Uuid::new_v4(), &package, &config).await;
        let elapsed = started.elapsed();

        // The build should fail fast — well under the 60s wall-clock
        // budget. If we exceed 30s the test is suspicious (kernel didn't
        // enforce the limit, or pre_exec isn't actually wired).
        assert!(
            elapsed < Duration::from_secs(30),
            "build did not fail fast under RLIMIT_AS (elapsed = {elapsed:?})"
        );

        match result {
            Err(BuildError::Failed { .. }) => { /* expected */ }
            Err(BuildError::TimedOut { .. }) => {
                panic!("RLIMIT_AS overshoot reported as TimedOut; kernel didn't enforce")
            }
            Ok(_) => panic!("RLIMIT_AS=128MiB should have aborted an 8 GiB allocation"),
        }
    }
}
