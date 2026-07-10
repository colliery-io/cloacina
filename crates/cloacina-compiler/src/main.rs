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

//! cloacina-compiler — standalone build service for Cloacina packages.
//!
//! Polls the DB for packages in `build_status = pending`, compiles them, and
//! writes the resulting cdylib bytes back into `workflow_packages.compiled_data`.
//! Reconcilers on `cloacina-server` / `cloacinactl daemon` then load the bytes
//! directly — no runtime toolchain required. See CLOACI-I-0097 + ADR-0004.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;

use cloacina_compiler::{run, BuildRlimits, CompilerConfig};

/// cloacina-compiler — DB-queue-driven build service.
#[derive(Parser)]
#[command(name = "cloacina-compiler")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Cloacina home directory (logs + tmp scratch space).
    #[arg(long, default_value_os_t = default_home())]
    home: PathBuf,

    /// Address to bind the local /health + /v1/status endpoint to.
    #[arg(long, default_value = "127.0.0.1:9000")]
    bind: SocketAddr,

    /// Database URL (overrides CLOACINA_DATABASE_URL / DATABASE_URL env vars).
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,

    /// CLOACI-T-0779: scope this compiler to a single tenant's Postgres schema
    /// for build ISOLATION. When set, the compiler claims and builds ONLY that
    /// tenant's pending packages (separate source, logs, and target dir per
    /// tenant — no cross-tenant leakage). Omit for the default (public) schema.
    /// Run one compiler per tenant, mirroring the tenant-scoped agent fleet.
    #[arg(long, env = "CLOACINA_TENANT_SCHEMA")]
    tenant_schema: Option<String>,

    /// CLOACI-T-0780: run as a PER-TARGET compiler producing cdylibs for this
    /// triple (e.g. "x86_64-linux"). Scan-and-fills `package_artifacts` for
    /// success packages lacking this arch, building natively — run the container
    /// on that arch (e.g. docker `platform: linux/amd64`). Omit for the primary
    /// host compiler (which claims pending rows into `workflow_packages`).
    #[arg(long, env = "CLOACINA_BUILD_TARGET")]
    build_target: Option<String>,

    /// CLOACI-T-0780: restrict the per-target scan to one package name (keeps an
    /// emulated build cheap). Only meaningful with --build-target.
    #[arg(long, env = "CLOACINA_BUILD_TARGET_PACKAGE")]
    build_target_package: Option<String>,

    /// Poll interval for new pending rows (milliseconds).
    #[arg(long, default_value_t = 2000)]
    poll_interval_ms: u64,

    /// Heartbeat interval while building (seconds).
    #[arg(long, default_value_t = 10)]
    heartbeat_interval_s: u64,

    /// Threshold past which a stuck `building` row is reset to `pending`
    /// (seconds). Should be at least 3× heartbeat interval.
    #[arg(long, default_value_t = 60)]
    stale_threshold_s: u64,

    /// Sweeper loop interval (seconds).
    #[arg(long, default_value_t = 30)]
    sweep_interval_s: u64,

    /// Override cargo build flags. Default:
    /// `build --release --lib --frozen --offline`. Repeatable. Setting any
    /// `--cargo-flag` replaces the full default list — include `--frozen`
    /// and `--offline` explicitly if you still want the offline posture.
    /// Closes CLOACI-T-0574 / SEC-06 (network half).
    #[arg(long = "cargo-flag")]
    cargo_flags: Vec<String>,

    /// Shared CARGO_TARGET_DIR for per-package builds. When set, transitive
    /// deps are compiled once and reused across packages — critical for
    /// dev / CI where many small packages get uploaded. Defaults to the
    /// unpacked package's own `target/` when unset.
    #[arg(long)]
    cargo_target_dir: Option<PathBuf>,

    /// Maximum wall-clock time for a single cargo build (seconds). Builds
    /// exceeding this are killed (SIGKILL) and the row is left for the
    /// stale-build sweeper to reset. Default 600s (10 min). Closes
    /// CLOACI-T-0573 / OPS-10.
    #[arg(long, env = "CLOACINA_COMPILER_BUILD_TIMEOUT_S", default_value_t = 600)]
    build_timeout_s: u64,

    /// Override `CARGO_HOME` for the cargo subprocess. Operator-managed:
    /// run `cargo vendor` against a known-good source tree, point this at
    /// the resulting cargo home. The compiler combines this with
    /// `--frozen --offline` so package builds resolve only what the
    /// operator has explicitly allowed. Unset means cargo uses its usual
    /// `~/.cargo`. Closes CLOACI-T-0574 / SEC-06 (network half).
    #[arg(long, env = "CLOACINA_COMPILER_VENDOR_DIR")]
    vendor_dir: Option<PathBuf>,

    /// DEV ESCAPE HATCH (CLOACI-T-0887): local cloacina workspace root. When set,
    /// each build injects `[patch.crates-io]` mapping `<root>/crates/*` to their
    /// paths (RO-bound in the sandbox), so packages that ship crates.io version
    /// deps resolve against the UNPUBLISHED local crates during dev cycles. NOT
    /// for production; only dev/e2e stacks (e.g. the demo compiler) set this.
    #[arg(long, env = "CLOACINA_COMPILER_DEV_WORKSPACE")]
    dev_workspace: Option<PathBuf>,

    /// `RLIMIT_CPU` (CPU-seconds) for the cargo subprocess. Linux-only.
    /// Default matches `--build-timeout-s` as a generous upper bound; the
    /// wall-clock timeout (T-0573) is the real bound. CLOACI-T-0575.
    #[arg(long, env = "CLOACINA_COMPILER_BUILD_RLIMIT_CPU")]
    build_rlimit_cpu: Option<u64>,

    /// `RLIMIT_AS` (virtual address space) for the cargo subprocess. Linux-only.
    /// Accepts plain bytes or human-readable suffixes (`K`, `M`, `G`).
    /// Default 4 GiB. CLOACI-T-0575.
    #[arg(
        long,
        env = "CLOACINA_COMPILER_BUILD_RLIMIT_MEM",
        default_value = "4G",
        value_parser = parse_size
    )]
    build_rlimit_mem: u64,

    /// `RLIMIT_NOFILE` (max open fds) for the cargo subprocess. Linux-only.
    /// Default 1024. CLOACI-T-0575.
    #[arg(
        long,
        env = "CLOACINA_COMPILER_BUILD_RLIMIT_FILES",
        default_value_t = 1024
    )]
    build_rlimit_files: u64,

    /// `RLIMIT_NPROC` (max user processes — bounds fork bombs) for the
    /// cargo subprocess. Linux-only. Default 256. CLOACI-T-0575.
    #[arg(
        long,
        env = "CLOACINA_COMPILER_BUILD_RLIMIT_PROCS",
        default_value_t = 256
    )]
    build_rlimit_procs: u64,

    /// Number of daily-rotated log files to retain on disk. `0` disables
    /// pruning entirely (unbounded — explicit opt-out). Default 14 days.
    /// CLOACI-I-0109 / T-0592.
    #[arg(long, default_value_t = 14)]
    log_retention_days: u64,
}

/// Parse a byte-size string with an optional `K`/`M`/`G` suffix (base-1024).
/// Examples: `4096`, `512K`, `4G`. Suffixes are case-insensitive.
fn parse_size(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty size".to_string());
    }
    let (num_str, multiplier) = match s.chars().last().unwrap() {
        c if c.is_ascii_digit() => (s, 1u64),
        'K' | 'k' => (&s[..s.len() - 1], 1024u64),
        'M' | 'm' => (&s[..s.len() - 1], 1024 * 1024),
        'G' | 'g' => (&s[..s.len() - 1], 1024 * 1024 * 1024),
        other => return Err(format!("unrecognized size suffix '{other}'")),
    };
    let n: u64 = num_str
        .trim()
        .parse()
        .map_err(|e: std::num::ParseIntError| format!("invalid number '{num_str}': {e}"))?;
    n.checked_mul(multiplier)
        .ok_or_else(|| format!("size '{s}' overflows u64"))
}

fn default_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cloacina")
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Default flags include `--frozen --offline` so package builds resolve
    // only against the operator's vendor dir. Operators with a curated
    // online registry can override via repeated `--cargo-flag`. See
    // CLOACI-T-0574 / SEC-06.
    let cargo_flags = if cli.cargo_flags.is_empty() {
        vec![
            "build".to_string(),
            "--release".to_string(),
            "--lib".to_string(),
            "--frozen".to_string(),
            "--offline".to_string(),
        ]
    } else {
        cli.cargo_flags
    };

    if cli.build_timeout_s == 0 {
        anyhow::bail!("--build-timeout-s must be greater than zero");
    }

    // CLOACI-I-0105: probe the sandbox ladder ONCE at boot. `required`
    // without bwrap is a hard startup failure — never a silent downgrade.
    let sandbox_mode =
        cloacina_compiler::sandbox::SandboxMode::from_env().map_err(|e| anyhow::anyhow!(e))?;
    let sandbox_plan =
        cloacina_compiler::sandbox::probe(sandbox_mode).map_err(|e| anyhow::anyhow!(e))?;

    let config = CompilerConfig {
        sandbox_level: sandbox_plan.level,
        home: cli.home,
        bind: cli.bind,
        database_url: cli.database_url,
        tenant_schema: cli.tenant_schema,
        build_target: cli.build_target,
        build_target_package: cli.build_target_package,
        verbose: cli.verbose,
        poll_interval: Duration::from_millis(cli.poll_interval_ms),
        heartbeat_interval: Duration::from_secs(cli.heartbeat_interval_s),
        stale_threshold: Duration::from_secs(cli.stale_threshold_s),
        sweep_interval: Duration::from_secs(cli.sweep_interval_s),
        cargo_flags,
        tmp_root: None,
        cargo_target_dir: cli.cargo_target_dir,
        build_timeout: Duration::from_secs(cli.build_timeout_s),
        vendor_dir: cli.vendor_dir,
        dev_workspace: cli.dev_workspace,
        build_rlimits: BuildRlimits {
            // RLIMIT_CPU default tracks the wall-clock timeout: T-0573 is
            // the real bound; this is a generous upper ceiling.
            cpu_s: cli.build_rlimit_cpu.unwrap_or(cli.build_timeout_s),
            mem_bytes: cli.build_rlimit_mem,
            files: cli.build_rlimit_files,
            procs: cli.build_rlimit_procs,
        },
        // CLOACI-T-0576: stamp every compiler.build.* audit event with a
        // process-unique id so operators can correlate builds back to a
        // specific compiler instance (e.g. when chasing flake or kill
        // events across a worker pool).
        compiler_instance_id: cloacina::UniversalUuid::new_v4(),
        log_retention_days: cli.log_retention_days,
    };

    run(config).await
}

// ---------------------------------------------------------------------------
// CLOACI-T-0575: byte-suffix parser for --build-rlimit-mem
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::parse_size;

    #[test]
    fn parse_size_plain_bytes() {
        assert_eq!(parse_size("0").unwrap(), 0);
        assert_eq!(parse_size("4096").unwrap(), 4096);
    }

    #[test]
    fn parse_size_kilo_suffix() {
        assert_eq!(parse_size("1K").unwrap(), 1024);
        assert_eq!(parse_size("512k").unwrap(), 512 * 1024);
    }

    #[test]
    fn parse_size_mega_suffix() {
        assert_eq!(parse_size("1M").unwrap(), 1024 * 1024);
        assert_eq!(parse_size("128m").unwrap(), 128 * 1024 * 1024);
    }

    #[test]
    fn parse_size_giga_suffix() {
        assert_eq!(parse_size("1G").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_size("4g").unwrap(), 4u64 * 1024 * 1024 * 1024);
    }

    #[test]
    fn parse_size_rejects_empty() {
        assert!(parse_size("").is_err());
    }

    #[test]
    fn parse_size_rejects_unknown_suffix() {
        let err = parse_size("4T").unwrap_err();
        assert!(err.contains("'T'"), "error should name the suffix: {err}");
    }

    #[test]
    fn parse_size_rejects_garbage_number() {
        assert!(parse_size("abc").is_err());
        assert!(parse_size("abcG").is_err());
    }

    #[test]
    fn parse_size_overflow() {
        // u64::MAX in GiB: 17_179_869_184 G would overflow.
        let err = parse_size("18446744073709551616G").unwrap_err();
        assert!(
            err.contains("invalid number") || err.contains("overflow"),
            "expected overflow / parse error, got: {err}"
        );
    }
}
