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

//! Build-process sandbox (CLOACI-I-0105, SEC-06/OPS-07).
//!
//! `cargo build` runs attacker-controlled code (`build.rs`, proc-macros).
//! Phase 1 (I-0104) capped COST (rlimits, `--frozen --offline`, curated
//! vendor registry); this phase isolates the PROCESS via a fail-closed
//! ladder selected by `CLOACINA_COMPILER_SANDBOX`:
//!
//! - **`required`** — builds run under bwrap (level 1) or the compiler
//!   REFUSES AT BOOT. The multi-tenant posture.
//! - **`preferred`** — best available level, downgrades logged loudly.
//! - **`off`** — no sandbox (dev laptops); logged loudly at boot.
//!
//! **Level 1 — bwrap**: PID/net/user/... namespaces (`--unshare-all`, so NO
//! network), `--clearenv` (a build.rs cannot read `DATABASE_URL`), RO binds
//! for the toolchain + curated registry, writable binds ONLY for the build
//! dir + shared target cache, tmpfs `/tmp`, `--die-with-parent`.
//!
//! **Level 2 — landlock** (containers without userns, kernel >=5.13):
//! kernel FS ACLs applied in `pre_exec` — RO everything, RW only the build
//! dir + target cache. No namespace isolation; env is still scrubbed by the
//! caller. Phase 1 rlimits apply at every level.
//!
//! Every build's audit row records the ACHIEVED level + a hash of the
//! sandbox configuration, so forensics can prove what contained a given
//! build.

use std::path::{Path, PathBuf};

/// Operator-selected sandbox mode (`CLOACINA_COMPILER_SANDBOX`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxMode {
    Required,
    Preferred,
    Off,
}

impl SandboxMode {
    pub fn from_env() -> Result<Self, String> {
        match std::env::var("CLOACINA_COMPILER_SANDBOX")
            .unwrap_or_else(|_| "preferred".to_string())
            .to_ascii_lowercase()
            .as_str()
        {
            "required" => Ok(Self::Required),
            "preferred" => Ok(Self::Preferred),
            "off" => Ok(Self::Off),
            other => Err(format!(
                "CLOACINA_COMPILER_SANDBOX must be required|preferred|off, got '{other}'"
            )),
        }
    }
}

/// The isolation level a build actually ran under (audited per build).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxLevel {
    /// bwrap namespaces + clearenv + RO mounts + no network.
    Bwrap,
    /// landlock FS ACLs (+ Phase 1 rlimits); no namespace isolation.
    Landlock,
    /// No process sandbox (Phase 1 rlimits/offline only).
    None,
}

impl SandboxLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Bwrap => "bwrap",
            Self::Landlock => "landlock",
            Self::None => "none",
        }
    }
}

/// Boot-time probe result: the level every build will use.
#[derive(Debug, Clone)]
pub struct SandboxPlan {
    pub mode: SandboxMode,
    pub level: SandboxLevel,
}

/// Probe the host for the best available level and reconcile with the mode.
/// `Required` without bwrap is a LOUD boot failure — never a silent
/// downgrade (the REQ-008 pattern).
pub fn probe(mode: SandboxMode) -> Result<SandboxPlan, String> {
    if mode == SandboxMode::Off {
        tracing::warn!(
            "CLOACINA_COMPILER_SANDBOX=off — builds run UNSANDBOXED \
             (dev-only posture; do not use with untrusted packages)"
        );
        return Ok(SandboxPlan {
            mode,
            level: SandboxLevel::None,
        });
    }

    let bwrap = bwrap_usable();
    if bwrap {
        tracing::info!("compiler sandbox: bwrap available — builds run at level 1 (namespaced)");
        return Ok(SandboxPlan {
            mode,
            level: SandboxLevel::Bwrap,
        });
    }

    if mode == SandboxMode::Required {
        return Err(
            "CLOACINA_COMPILER_SANDBOX=required but bwrap is unusable here \
             (missing binary, or user namespaces blocked — in Docker, set the \
             documented security_opt so unprivileged userns work). Refusing to \
             start rather than build untrusted packages unsandboxed."
                .to_string(),
        );
    }

    if landlock_usable() {
        tracing::warn!(
            "compiler sandbox: bwrap unusable — DOWNGRADED to level 2 (landlock \
             FS ACLs, no namespace isolation). Set the container security_opt \
             for full isolation."
        );
        return Ok(SandboxPlan {
            mode,
            level: SandboxLevel::Landlock,
        });
    }

    tracing::warn!(
        "compiler sandbox: neither bwrap nor landlock usable — builds run \
         UNSANDBOXED (Phase 1 rlimits/offline only). Use \
         CLOACINA_COMPILER_SANDBOX=required to make this a hard failure."
    );
    Ok(SandboxPlan {
        mode,
        level: SandboxLevel::None,
    })
}

/// bwrap is usable iff the binary exists AND it can actually create its
/// namespaces here (Docker's default seccomp blocks unprivileged userns —
/// the probe catches that, not just missing binaries).
fn bwrap_usable() -> bool {
    // REPRESENTATIVE probe (CLOACI-T-0855): exercise the SAME namespace +
    // mount shape a real build uses — a bare `--ro-bind / /` passes in
    // containers where `--proc` mounting or `--unshare-net` actually fails,
    // giving a false-positive that would then break every build. This runs
    // the exact critical bits: unshare net/user/ipc/uts/cgroup, RO-bind
    // /proc (a fresh --proc mount needs a new PID ns + caps a default
    // container lacks), tmpfs, --clearenv.
    std::process::Command::new("bwrap")
        .args([
            "--unshare-user",
            "--unshare-net",
            "--unshare-ipc",
            "--unshare-uts",
            "--unshare-cgroup",
            "--clearenv",
            "--ro-bind",
            "/proc",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
            "--ro-bind",
            "/usr",
            "/usr",
            "true",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn landlock_usable() -> bool {
    use landlock::{Access, AccessFs, Ruleset, RulesetAttr, ABI};
    Ruleset::default()
        .handle_access(AccessFs::from_all(ABI::V1))
        .and_then(|r| r.create())
        .is_ok()
}

#[cfg(not(target_os = "linux"))]
fn landlock_usable() -> bool {
    false
}

/// The filesystem surface one sandboxed build sees.
pub struct BuildMounts<'a> {
    /// The staged package source — the ONLY writable project dir.
    pub source_dir: &'a Path,
    /// Shared cargo target cache (writable — cost optimization from Phase 1).
    pub target_dir: Option<&'a Path>,
    /// Curated vendor registry / CARGO_HOME (read-only at level 1).
    pub vendor_dir: Option<&'a Path>,
    /// DEV ESCAPE HATCH (CLOACI-T-0887): the local workspace `crates/` dir a
    /// `--dev-workspace` build injects into `[patch.crates-io]`. Bound READ-ONLY
    /// so version-dep packages resolve against the unpublished local crates
    /// while crates aren't on crates.io yet. NOT for production — production
    /// packages ship deps via crates.io.
    pub patch_crates_dir: Option<&'a Path>,
}

/// Environment the sandboxed cargo receives. At level 1 the environment is
/// CLEARED and rebuilt from exactly this list — `DATABASE_URL` and friends
/// never cross into attacker code.
pub fn build_env(mounts: &BuildMounts<'_>) -> Vec<(String, String)> {
    let mut env = vec![
        (
            "PATH".to_string(),
            std::env::var("PATH")
                .unwrap_or_else(|_| "/usr/local/cargo/bin:/usr/local/bin:/usr/bin:/bin".into()),
        ),
        // build.rs writes to HOME land in the (contained) build dir.
        (
            "HOME".to_string(),
            mounts.source_dir.to_string_lossy().to_string(),
        ),
    ];
    for var in ["RUSTUP_HOME", "CARGO_HOME"] {
        if let Ok(v) = std::env::var(var) {
            env.push((var.to_string(), v));
        }
    }
    // The shared target cache + debuginfo default must survive the env scrub
    // (they were previously plain `cmd.env` calls in cargo_build).
    if let Some(target) = mounts.target_dir {
        env.push((
            "CARGO_TARGET_DIR".to_string(),
            target.to_string_lossy().to_string(),
        ));
    }
    env.push((
        "CARGO_PROFILE_DEV_DEBUG".to_string(),
        std::env::var("CARGO_PROFILE_DEV_DEBUG").unwrap_or_else(|_| "line-tables-only".into()),
    ));
    // Vendored CARGO_HOME (Phase 1) overrides the toolchain default.
    if let Some(vendor) = mounts.vendor_dir {
        env.retain(|(k, _)| k != "CARGO_HOME");
        env.push((
            "CARGO_HOME".to_string(),
            vendor.to_string_lossy().to_string(),
        ));
    }
    env
}

/// Compose the command for a sandboxed `cargo` invocation at `level`.
/// Returns the program + leading args; the caller appends cargo's own args
/// and the (already-scrubbed) environment from [`build_env`].
pub fn wrap_command(level: SandboxLevel, mounts: &BuildMounts<'_>) -> (String, Vec<String>) {
    match level {
        SandboxLevel::Bwrap => {
            // Per-namespace unshares (NOT --unshare-all): a fresh --proc mount
            // needs a new PID ns + caps a default container lacks (CLOACI-T-0855
            // real-container proof). --unshare-net is the security-critical
            // one; the container's /proc is already PID-isolated by Docker, so
            // RO-bind it rather than mounting a new procfs.
            let mut args: Vec<String> = vec![
                "--unshare-user".into(),
                "--unshare-net".into(),
                "--unshare-ipc".into(),
                "--unshare-uts".into(),
                "--unshare-cgroup".into(),
                "--die-with-parent".into(),
                "--clearenv".into(),
                "--ro-bind".into(),
                "/proc".into(),
                "/proc".into(),
                "--dev".into(),
                "/dev".into(),
                "--tmpfs".into(),
                "/tmp".into(),
            ];
            // Toolchain + system libraries, read-only. Bind only what exists.
            for ro in ["/usr", "/lib", "/lib64", "/bin", "/sbin", "/etc"] {
                if Path::new(ro).exists() {
                    args.extend(["--ro-bind".into(), ro.into(), ro.into()]);
                }
            }
            // Rust toolchain homes (the rust images put them under /usr/local,
            // already covered by /usr; bind explicitly when elsewhere).
            for var in ["RUSTUP_HOME", "CARGO_HOME"] {
                if let Ok(v) = std::env::var(var) {
                    if !v.starts_with("/usr") && Path::new(&v).exists() {
                        args.extend(["--ro-bind".into(), v.clone(), v]);
                    }
                }
            }
            // Curated vendor registry: read-only.
            if let Some(vendor) = mounts.vendor_dir {
                let v = vendor.to_string_lossy().to_string();
                args.extend(["--ro-bind".into(), v.clone(), v]);
            }
            // DEV hatch (CLOACI-T-0887): the local workspace crates the injected
            // `[patch.crates-io]` points at — read-only.
            if let Some(crates) = mounts.patch_crates_dir {
                let c = crates.to_string_lossy().to_string();
                args.extend(["--ro-bind".into(), c.clone(), c]);
            }
            // Writable surfaces: the staged source + the shared target cache.
            let src = mounts.source_dir.to_string_lossy().to_string();
            args.extend(["--bind".into(), src.clone(), src.clone()]);
            if let Some(target) = mounts.target_dir {
                let t = target.to_string_lossy().to_string();
                args.extend(["--bind".into(), t.clone(), t]);
            }
            args.extend(["--chdir".into(), src]);
            // Environment: cleared above; rebuilt explicitly.
            for (k, v) in build_env(mounts) {
                args.extend(["--setenv".into(), k, v]);
            }
            args.push("cargo".into());
            ("bwrap".to_string(), args)
        }
        SandboxLevel::Landlock | SandboxLevel::None => ("cargo".to_string(), Vec::new()),
    }
}

/// Apply the level-2 landlock ruleset to a command (Linux): RO+execute on
/// the system paths, RW only on the build dir + target cache. A best-effort
/// no-op on kernels without landlock (the probe already told the operator).
#[cfg(target_os = "linux")]
pub fn apply_landlock(
    cmd: &mut tokio::process::Command,
    source_dir: PathBuf,
    target_dir: Option<PathBuf>,
    vendor_dir: Option<PathBuf>,
    patch_crates_dir: Option<PathBuf>,
) {
    use landlock::{
        Access, AccessFs, PathBeneath, PathFd, Ruleset, RulesetAttr, RulesetCreatedAttr, ABI,
    };
    unsafe {
        cmd.pre_exec(move || {
            let abi = ABI::V1;
            let mut ruleset = Ruleset::default()
                .handle_access(AccessFs::from_all(abi))
                .and_then(|r| r.create())
                .map_err(|e| std::io::Error::other(format!("landlock create: {e}")))?;
            let ro = AccessFs::from_read(abi);
            let rw = AccessFs::from_all(abi);
            for p in [
                "/usr", "/lib", "/lib64", "/bin", "/sbin", "/etc", "/proc", "/dev", "/tmp",
            ] {
                if let Ok(fd) = PathFd::new(p) {
                    ruleset = ruleset
                        .add_rule(PathBeneath::new(fd, ro))
                        .map_err(|e| std::io::Error::other(format!("landlock rule: {e}")))?;
                }
            }
            for ro_dir in [&vendor_dir, &patch_crates_dir] {
                if let Some(d) = ro_dir {
                    if let Ok(fd) = PathFd::new(d) {
                        ruleset = ruleset
                            .add_rule(PathBeneath::new(fd, ro))
                            .map_err(|e| std::io::Error::other(format!("landlock rule: {e}")))?;
                    }
                }
            }
            let mut rw_paths = vec![source_dir.clone()];
            if let Some(t) = &target_dir {
                rw_paths.push(t.clone());
            }
            // /tmp must stay writable for rustc temp files.
            rw_paths.push(PathBuf::from("/tmp"));
            for p in rw_paths {
                if let Ok(fd) = PathFd::new(&p) {
                    ruleset = ruleset
                        .add_rule(PathBeneath::new(fd, rw))
                        .map_err(|e| std::io::Error::other(format!("landlock rule: {e}")))?;
                }
            }
            ruleset
                .restrict_self()
                .map_err(|e| std::io::Error::other(format!("landlock restrict: {e}")))?;
            Ok(())
        });
    }
}

#[cfg(not(target_os = "linux"))]
pub fn apply_landlock(
    _cmd: &mut tokio::process::Command,
    _source_dir: PathBuf,
    _target_dir: Option<PathBuf>,
    _vendor_dir: Option<PathBuf>,
    _patch_crates_dir: Option<PathBuf>,
) {
}

/// Stable hash of the sandbox configuration for the audit trail.
pub fn config_hash(level: SandboxLevel, mounts: &BuildMounts<'_>) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    level.as_str().hash(&mut h);
    mounts.source_dir.hash(&mut h);
    mounts.target_dir.hash(&mut h);
    mounts.vendor_dir.hash(&mut h);
    mounts.patch_crates_dir.hash(&mut h);
    format!("{:016x}", h.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn mounts<'a>(src: &'a Path, vendor: &'a Path) -> BuildMounts<'a> {
        BuildMounts {
            source_dir: src,
            target_dir: None,
            vendor_dir: Some(vendor),
            patch_crates_dir: None,
        }
    }

    /// The bwrap composition is the isolation contract (CLOACI-T-0853): full
    /// namespaces, cleared env, and the curated mount set — proven without
    /// needing bwrap installed.
    #[test]
    fn bwrap_command_enforces_isolation_contract() {
        let src = PathBuf::from("/staged/pkg");
        let vendor = PathBuf::from("/curated/registry");
        let (prog, args) = wrap_command(SandboxLevel::Bwrap, &mounts(&src, &vendor));
        assert_eq!(prog, "bwrap");

        // No network (the security-critical unshare) + per-namespace
        // isolation + die-with-parent. (CLOACI-T-0855: --unshare-all + a
        // fresh --proc mount fails unprivileged in containers; the real
        // config unshares per-namespace and RO-binds /proc.)
        assert!(args.iter().any(|a| a == "--unshare-net"));
        assert!(args.iter().any(|a| a == "--unshare-user"));
        assert!(args.iter().any(|a| a == "--die-with-parent"));
        // /proc is RO-bound, not a fresh mount.
        assert!(args.join(" ").contains("--ro-bind /proc /proc"));
        // Env is CLEARED before the explicit rebuild — a build.rs cannot read
        // DATABASE_URL et al.
        assert!(args.iter().any(|a| a == "--clearenv"));

        let joined = args.join(" ");
        // The staged source is bound WRITABLE; the curated registry RO.
        assert!(joined.contains("--bind /staged/pkg /staged/pkg"));
        assert!(joined.contains("--ro-bind /curated/registry /curated/registry"));
        // The command ends by launching cargo INSIDE the sandbox.
        assert_eq!(args.last().map(String::as_str), Some("cargo"));

        // ADVERSARIAL: no host path outside the staged source is writable —
        // scan every `--bind` (RW) target and assert it's the source (or the
        // target cache, absent here).
        for w in args.windows(2) {
            if w[0] == "--bind" {
                assert_eq!(
                    w[1], "/staged/pkg",
                    "unexpected writable host bind: {}",
                    w[1]
                );
            }
        }
    }

    /// The sandboxed env is an ALLOWLIST (CLOACI-T-0853): even with a secret
    /// in the parent process, only the curated keys cross the boundary.
    #[test]
    fn build_env_is_an_allowlist_not_a_denylist() {
        unsafe {
            std::env::set_var("CLOACINA_TEST_SECRET_DATABASE_URL", "postgres://leak");
        }
        let src = PathBuf::from("/staged/pkg");
        let vendor = PathBuf::from("/curated/registry");
        let env = build_env(&mounts(&src, &vendor));
        let keys: Vec<&str> = env.iter().map(|(k, _)| k.as_str()).collect();
        assert!(!keys.contains(&"CLOACINA_TEST_SECRET_DATABASE_URL"));
        // HOME is redirected into the contained build dir so build.rs writes
        // to $HOME stay inside the sandbox.
        let home = env
            .iter()
            .find(|(k, _)| k == "HOME")
            .map(|(_, v)| v.as_str());
        assert_eq!(home, Some("/staged/pkg"));
        // The vendored registry wins CARGO_HOME (Phase 1 curation).
        let cargo_home = env
            .iter()
            .find(|(k, _)| k == "CARGO_HOME")
            .map(|(_, v)| v.as_str());
        assert_eq!(cargo_home, Some("/curated/registry"));
        unsafe {
            std::env::remove_var("CLOACINA_TEST_SECRET_DATABASE_URL");
        }
    }

    /// non-bwrap levels do not wrap the command.
    #[test]
    fn non_bwrap_levels_run_cargo_directly() {
        let src = PathBuf::from("/s");
        let vendor = PathBuf::from("/v");
        for level in [SandboxLevel::Landlock, SandboxLevel::None] {
            let (prog, args) = wrap_command(level, &mounts(&src, &vendor));
            assert_eq!(prog, "cargo");
            assert!(args.is_empty());
        }
    }

    /// END-TO-END ADVERSARIAL (CLOACI-T-0855): where bwrap actually works
    /// (Linux CI with userns), a build attempting to WRITE outside its staged
    /// dir is denied — the RO host mount holds. Skips (not fails) where bwrap
    /// is unusable (macOS dev, userns-blocked containers) so the suite stays
    /// green everywhere; CI runs it for real.
    #[test]
    fn bwrap_denies_host_write() {
        let plan = probe(SandboxMode::Preferred).expect("probe");
        if plan.level != SandboxLevel::Bwrap {
            eprintln!(
                "skipping: bwrap not usable here (level={})",
                plan.level.as_str()
            );
            return;
        }
        let tmp = std::env::temp_dir().join("cloacina-sbx-adv");
        let _ = std::fs::create_dir_all(&tmp);
        let (prog, mut args) = wrap_command(SandboxLevel::Bwrap, &mounts(&tmp, &tmp));
        // Replace the trailing `cargo` with a shell that tries to write /etc.
        assert_eq!(args.pop().as_deref(), Some("cargo"));
        args.extend([
            "/bin/sh".into(),
            "-c".into(),
            "echo x > /etc/cloacina_probe".into(),
        ]);
        let status = std::process::Command::new(prog)
            .args(&args)
            .status()
            .expect("run bwrap");
        assert!(
            !status.success(),
            "write to /etc must be denied inside the sandbox"
        );
        assert!(
            !std::path::Path::new("/etc/cloacina_probe").exists(),
            "the probe file must not exist on the host"
        );
    }
}
