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

use anyhow::{Result, bail};
use std::net::SocketAddr;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

use crate::GlobalOpts;
use crate::commands::config::{self, CloacinaConfig, CompilerSection};
use crate::shared::pid;

/// Compiler settings that `compiler start` forwards to `cloacina-compiler`
/// (CLOACINA-T-1106): the unpack limit and the Python settings of
/// CLOACINA-T-1073. Each one is a flag here, the compiler's own env var, and
/// a key of `[compiler]` in `config.toml`. The order is: flag, env var,
/// `config.toml`. A setting that none of them gives is not forwarded, and the
/// compiler uses its own default.
#[derive(clap::Args, Debug, Default, Clone)]
pub struct CompilerSettingArgs {
    /// The largest size, in bytes, that a package archive may unpack to.
    /// Overrides `[compiler].max_unpacked_bytes`.
    #[arg(long, env = "CLOACINA_MAX_UNPACKED_BYTES")]
    pub max_unpacked_bytes: Option<u64>,

    /// Python packages: a platform to vendor dependencies for, as
    /// `<arch>-<os>`. Repeat it, or give a comma-separated list. Overrides
    /// `[compiler].python_platforms`.
    #[arg(
        long = "python-platform",
        env = "CLOACINA_COMPILER_PYTHON_PLATFORMS",
        value_name = "ARCH-OS",
        value_delimiter = ','
    )]
    pub python_platform: Vec<String>,

    /// Python packages: the Python version to vendor for, `major.minor`.
    /// Overrides `[compiler].python_version`.
    #[arg(
        long,
        env = "CLOACINA_COMPILER_PYTHON_VERSION",
        value_name = "MAJOR.MINOR"
    )]
    pub python_version: Option<String>,

    /// Python packages: the package index that wheels are downloaded from.
    /// Overrides `[compiler].python_index_url`. The URL can hold a password,
    /// so it goes to the compiler as an env var, not as an argument.
    #[arg(
        long,
        env = "CLOACINA_COMPILER_PYTHON_INDEX_URL",
        value_name = "URL",
        hide_env_values = true
    )]
    pub python_index_url: Option<String>,

    /// Python packages: a folder of wheel files. Overrides
    /// `[compiler].python_wheel_dir`.
    #[arg(long, env = "CLOACINA_COMPILER_PYTHON_WHEEL_DIR", value_name = "DIR")]
    pub python_wheel_dir: Option<String>,

    /// Python packages: the cache folder of `uv`. Overrides
    /// `[compiler].python_uv_cache_dir`.
    #[arg(
        long,
        env = "CLOACINA_COMPILER_PYTHON_UV_CACHE_DIR",
        value_name = "DIR"
    )]
    pub python_uv_cache_dir: Option<String>,

    /// Python packages: vendor with no network. Or
    /// `[compiler].python_offline = true`.
    #[arg(long, env = "CLOACINA_COMPILER_PYTHON_OFFLINE")]
    pub python_offline: bool,

    /// Python packages: the `uv` program. Overrides `[compiler].uv`.
    #[arg(long, env = "CLOACINA_UV", value_name = "PATH")]
    pub uv: Option<String>,

    // CLOACINA-T-1124: the other flags of `cloacina-compiler`, but its two
    // time limits.
    /// Build only this tenant's packages (one compiler per tenant). Overrides
    /// `[compiler].tenant_schema`.
    #[arg(long, env = "CLOACINA_TENANT_SCHEMA", value_name = "SCHEMA")]
    pub tenant_schema: Option<String>,

    /// Run as a per-target compiler that builds for this triple. Overrides
    /// `[compiler].build_target`.
    #[arg(long, env = "CLOACINA_BUILD_TARGET", value_name = "TRIPLE")]
    pub build_target: Option<String>,

    /// With `--build-target`: build only this package. Overrides
    /// `[compiler].build_target_package`.
    #[arg(long, env = "CLOACINA_BUILD_TARGET_PACKAGE", value_name = "NAME")]
    pub build_target_package: Option<String>,

    /// A cargo flag added to the default list. Repeat it; write a value that
    /// starts with `-` as `--cargo-flag=<value>`. Overrides
    /// `[compiler].cargo_flags`.
    #[arg(
        long = "cargo-flag",
        value_name = "FLAG",
        conflicts_with = "cargo_flags_replace"
    )]
    pub cargo_flag: Vec<String>,

    /// The whole cargo flag list, defaults replaced (it starts with the
    /// cargo subcommand, usually `build`). Repeat it. Overrides
    /// `[compiler].cargo_flags_replace`.
    #[arg(long = "cargo-flags-replace", value_name = "FLAG")]
    pub cargo_flags_replace: Vec<String>,

    /// A shared cargo target folder for package builds. Overrides
    /// `[compiler].cargo_target_dir`.
    #[arg(long, value_name = "DIR")]
    pub cargo_target_dir: Option<String>,

    /// `CARGO_HOME` of the cargo build: a vendored cargo home. Overrides
    /// `[compiler].vendor_dir`.
    #[arg(long, env = "CLOACINA_COMPILER_VENDOR_DIR", value_name = "DIR")]
    pub vendor_dir: Option<String>,

    /// Dev only: a local cloacina workspace that builds patch to. Overrides
    /// `[compiler].dev_workspace`.
    #[arg(long, env = "CLOACINA_COMPILER_DEV_WORKSPACE", value_name = "DIR")]
    pub dev_workspace: Option<String>,

    /// `RLIMIT_CPU` of the cargo build, CPU-seconds (Linux). Overrides
    /// `[compiler].build_rlimit_cpu`.
    #[arg(
        long,
        env = "CLOACINA_COMPILER_BUILD_RLIMIT_CPU",
        value_name = "SECONDS"
    )]
    pub build_rlimit_cpu: Option<u64>,

    /// `RLIMIT_AS` of the cargo build: bytes, or a size with `K`, `M` or `G`
    /// (Linux). Overrides `[compiler].build_rlimit_mem`.
    #[arg(long, env = "CLOACINA_COMPILER_BUILD_RLIMIT_MEM", value_name = "SIZE")]
    pub build_rlimit_mem: Option<String>,

    /// `RLIMIT_NOFILE` of the cargo build (Linux). Overrides
    /// `[compiler].build_rlimit_files`.
    #[arg(long, env = "CLOACINA_COMPILER_BUILD_RLIMIT_FILES", value_name = "N")]
    pub build_rlimit_files: Option<u64>,

    /// `RLIMIT_NPROC` of the cargo build (Linux). Overrides
    /// `[compiler].build_rlimit_procs`.
    #[arg(long, env = "CLOACINA_COMPILER_BUILD_RLIMIT_PROCS", value_name = "N")]
    pub build_rlimit_procs: Option<u64>,

    /// Daily log files to keep; `0` keeps all. Overrides
    /// `[compiler].log_retention_days`.
    #[arg(long, value_name = "DAYS")]
    pub log_retention_days: Option<u64>,
}

/// What `compiler start` adds to the `cloacina-compiler` command for the
/// settings in [`CompilerSettingArgs`].
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ForwardedCompilerSettings {
    /// Arguments, in the order they are added.
    pub args: Vec<String>,
    /// Env vars set on the compiler process.
    pub env: Vec<(&'static str, String)>,
}

/// Resolve each setting (flag or env var first, then `[compiler]` in
/// `config.toml`) and give what to forward. A setting with no value gives
/// nothing. Cargo flags added to the defaults and a full replacement of them
/// cannot both be given (CLOACINA-T-1124).
pub(crate) fn forwarded_compiler_settings(
    cli: &CompilerSettingArgs,
    config: &CompilerSection,
) -> Result<ForwardedCompilerSettings> {
    let list = |cli: &Vec<String>, config: &Vec<String>| {
        if cli.is_empty() {
            config.clone()
        } else {
            cli.clone()
        }
    };
    let cargo_flags = list(&cli.cargo_flag, &config.cargo_flags);
    let cargo_flags_replace = list(&cli.cargo_flags_replace, &config.cargo_flags_replace);
    if !cargo_flags.is_empty() && !cargo_flags_replace.is_empty() {
        bail!(
            "cargo flags are given both as additions (--cargo-flag / [compiler].cargo_flags) \
             and as a full replacement (--cargo-flags-replace / \
             [compiler].cargo_flags_replace); give one of them"
        );
    }

    let mut out = ForwardedCompilerSettings::default();
    let not_blank = |t: &String| !t.trim().is_empty();
    let text = |cli: &Option<String>, config: &Option<String>| {
        cli.clone()
            .filter(not_blank)
            .or_else(|| config.clone().filter(not_blank))
    };
    let mut arg = |flag: &str, value: Option<String>| {
        if let Some(value) = value {
            out.args.push(flag.to_string());
            out.args.push(value);
        }
    };

    arg(
        "--max-unpacked-bytes",
        cli.max_unpacked_bytes
            .or(config.max_unpacked_bytes)
            .map(|n| n.to_string()),
    );
    let platforms = if cli.python_platform.is_empty() {
        &config.python_platforms
    } else {
        &cli.python_platform
    };
    arg(
        "--python-platform",
        (!platforms.is_empty()).then(|| platforms.join(",")),
    );
    arg(
        "--python-version",
        text(&cli.python_version, &config.python_version),
    );
    arg(
        "--python-wheel-dir",
        text(&cli.python_wheel_dir, &config.python_wheel_dir),
    );
    arg(
        "--python-uv-cache-dir",
        text(&cli.python_uv_cache_dir, &config.python_uv_cache_dir),
    );
    arg("--uv", text(&cli.uv, &config.uv));

    // CLOACINA-T-1124.
    let number = |cli: Option<u64>, config: Option<u64>| cli.or(config).map(|n| n.to_string());
    arg(
        "--tenant-schema",
        text(&cli.tenant_schema, &config.tenant_schema),
    );
    arg(
        "--build-target",
        text(&cli.build_target, &config.build_target),
    );
    arg(
        "--build-target-package",
        text(&cli.build_target_package, &config.build_target_package),
    );
    arg(
        "--cargo-target-dir",
        text(&cli.cargo_target_dir, &config.cargo_target_dir),
    );
    arg("--vendor-dir", text(&cli.vendor_dir, &config.vendor_dir));
    arg(
        "--dev-workspace",
        text(&cli.dev_workspace, &config.dev_workspace),
    );
    arg(
        "--build-rlimit-cpu",
        number(cli.build_rlimit_cpu, config.build_rlimit_cpu),
    );
    arg(
        "--build-rlimit-mem",
        text(&cli.build_rlimit_mem, &config.build_rlimit_mem),
    );
    arg(
        "--build-rlimit-files",
        number(cli.build_rlimit_files, config.build_rlimit_files),
    );
    arg(
        "--build-rlimit-procs",
        number(cli.build_rlimit_procs, config.build_rlimit_procs),
    );
    arg(
        "--log-retention-days",
        number(cli.log_retention_days, config.log_retention_days),
    );
    // `--flag=value`: a cargo flag starts with `-`, and as a separate
    // argument the compiler would read it as a flag of its own.
    for flag in &cargo_flags {
        out.args.push(format!("--cargo-flag={flag}"));
    }
    for flag in &cargo_flags_replace {
        out.args.push(format!("--cargo-flags-replace={flag}"));
    }

    if cli.python_offline || config.python_offline == Some(true) {
        out.args.push("--python-offline".to_string());
    }

    // The index URL can hold a password: an env var of the compiler process,
    // so it is not in the process list.
    if let Some(url) = text(&cli.python_index_url, &config.python_index_url) {
        out.env.push(("CLOACINA_COMPILER_PYTHON_INDEX_URL", url));
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub async fn run(
    globals: &GlobalOpts,
    bind: SocketAddr,
    metrics_bind: Option<SocketAddr>,
    database_url: Option<String>,
    poll_interval_ms: Option<u64>,
    heartbeat_interval_s: Option<u64>,
    stale_threshold_s: Option<u64>,
    sweep_interval_s: Option<u64>,
    settings: CompilerSettingArgs,
) -> Result<()> {
    let config_path = globals.home.join("config.toml");
    let db_url = config::resolve_database_url(database_url.as_deref(), &config_path)?;
    let forwarded =
        forwarded_compiler_settings(&settings, &CloacinaConfig::load(&config_path).compiler)?;

    // Write PID file before exec — we're the process about to become compiler.
    pid::write(&globals.home.join("compiler.pid"))?;

    let mut cmd = compiler_command(&globals.home, bind, metrics_bind, &db_url);
    if globals.verbose {
        cmd.arg("--verbose");
    }
    if let Some(v) = poll_interval_ms {
        cmd.arg("--poll-interval-ms").arg(v.to_string());
    }
    if let Some(v) = heartbeat_interval_s {
        cmd.arg("--heartbeat-interval-s").arg(v.to_string());
    }
    if let Some(v) = stale_threshold_s {
        cmd.arg("--stale-threshold-s").arg(v.to_string());
    }
    if let Some(v) = sweep_interval_s {
        cmd.arg("--sweep-interval-s").arg(v.to_string());
    }
    cmd.args(&forwarded.args);
    cmd.envs(forwarded.env.iter().map(|(name, value)| (*name, value)));

    // Replaces the current process. Only returns on error.
    let err = cmd.exec();
    bail!("failed to exec cloacina-compiler: {err}");
}

/// The base `cloacina-compiler` command line.
fn compiler_command(
    home: &Path,
    bind: SocketAddr,
    metrics_bind: Option<SocketAddr>,
    db_url: &str,
) -> Command {
    let mut cmd = Command::new("cloacina-compiler");
    cmd.arg("--home")
        .arg(home)
        .arg("--bind")
        .arg(bind.to_string())
        .arg("--database-url")
        .arg(db_url);
    // CLOACINA-T-1039: passed on only when set, so the compiler's own default
    // (`/metrics` on `--bind`) is as before.
    if let Some(addr) = metrics_bind {
        cmd.arg("--metrics-bind").arg(addr.to_string());
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(metrics_bind: Option<SocketAddr>) -> Vec<String> {
        compiler_command(
            Path::new("/tmp/cloacina-home"),
            "127.0.0.1:9000".parse().unwrap(),
            metrics_bind,
            "postgres://u:p@h:5432/d",
        )
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect()
    }

    #[test]
    fn metrics_bind_is_passed_to_the_compiler_when_set() {
        let args = args(Some("127.0.0.1:9091".parse().unwrap()));
        let at = args.iter().position(|a| a == "--metrics-bind").unwrap();
        assert_eq!(args[at + 1], "127.0.0.1:9091");
    }

    #[test]
    fn metrics_bind_is_left_out_when_not_set() {
        assert!(!args(None).iter().any(|a| a == "--metrics-bind"));
    }

    fn section(toml: &str) -> CompilerSection {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, toml).unwrap();
        CloacinaConfig::load(&path).compiler
    }

    const FULL_CONFIG: &str = r#"
[compiler]
max_unpacked_bytes = 1073741824
python_platforms = ["x86_64-linux"]
python_version = "3.12"
python_index_url = "https://robin:pw@pypi.example.com/simple"
python_wheel_dir = "/opt/wheels"
python_uv_cache_dir = "/tmp/uv-cache"
python_offline = true
uv = "/usr/local/bin/uv"
"#;

    #[test]
    fn nothing_set_forwards_nothing() {
        let forwarded = forwarded_compiler_settings(
            &CompilerSettingArgs::default(),
            &CompilerSection::default(),
        )
        .unwrap();
        assert_eq!(forwarded, ForwardedCompilerSettings::default());
    }

    /// CLOACINA-T-1106: each `[compiler]` key is forwarded as its flag; the
    /// index URL as an env var only.
    #[test]
    fn config_toml_compiler_settings_are_forwarded() {
        let forwarded =
            forwarded_compiler_settings(&CompilerSettingArgs::default(), &section(FULL_CONFIG))
                .unwrap();
        assert_eq!(
            forwarded.args,
            [
                "--max-unpacked-bytes",
                "1073741824",
                "--python-platform",
                "x86_64-linux",
                "--python-version",
                "3.12",
                "--python-wheel-dir",
                "/opt/wheels",
                "--python-uv-cache-dir",
                "/tmp/uv-cache",
                "--uv",
                "/usr/local/bin/uv",
                "--python-offline",
            ]
        );
        assert_eq!(
            forwarded.env,
            [(
                "CLOACINA_COMPILER_PYTHON_INDEX_URL",
                "https://robin:pw@pypi.example.com/simple".to_string()
            )]
        );
        assert!(!forwarded.args.iter().any(|a| a.contains("pypi.example")));
    }

    #[test]
    fn a_compiler_flag_wins_over_config_toml() {
        let cli = CompilerSettingArgs {
            python_platform: vec!["aarch64-linux".to_string(), "x86_64-linux".to_string()],
            python_version: Some("3.11".to_string()),
            max_unpacked_bytes: Some(1),
            ..CompilerSettingArgs::default()
        };
        let forwarded = forwarded_compiler_settings(&cli, &section(FULL_CONFIG)).unwrap();
        let value_of = |flag: &str| {
            let at = forwarded.args.iter().position(|a| a == flag).unwrap();
            forwarded.args[at + 1].clone()
        };
        assert_eq!(value_of("--python-platform"), "aarch64-linux,x86_64-linux");
        assert_eq!(value_of("--python-version"), "3.11");
        assert_eq!(value_of("--max-unpacked-bytes"), "1");
        assert_eq!(value_of("--uv"), "/usr/local/bin/uv");
    }

    #[test]
    fn python_offline_false_in_config_forwards_nothing() {
        let forwarded = forwarded_compiler_settings(
            &CompilerSettingArgs::default(),
            &section("[compiler]\npython_offline = false\n"),
        )
        .unwrap();
        assert!(forwarded.args.is_empty());
    }

    const BUILD_CONFIG: &str = r#"
[compiler]
tenant_schema = "team_a"
build_target = "x86_64-linux"
build_target_package = "etl"
cargo_flags = ["--features", "fast"]
cargo_target_dir = "/var/cache/cargo-target"
vendor_dir = "/opt/cargo-home"
dev_workspace = "/src/cloacina"
build_rlimit_cpu = 900
build_rlimit_mem = "8G"
build_rlimit_files = 2048
build_rlimit_procs = 512
log_retention_days = 7
"#;

    /// CLOACINA-T-1124: each of the other compiler keys is forwarded once, as
    /// its flag; cargo flags as `--cargo-flag=<value>`.
    #[test]
    fn config_toml_build_settings_are_forwarded() {
        let forwarded =
            forwarded_compiler_settings(&CompilerSettingArgs::default(), &section(BUILD_CONFIG))
                .unwrap();
        assert_eq!(
            forwarded.args,
            [
                "--tenant-schema",
                "team_a",
                "--build-target",
                "x86_64-linux",
                "--build-target-package",
                "etl",
                "--cargo-target-dir",
                "/var/cache/cargo-target",
                "--vendor-dir",
                "/opt/cargo-home",
                "--dev-workspace",
                "/src/cloacina",
                "--build-rlimit-cpu",
                "900",
                "--build-rlimit-mem",
                "8G",
                "--build-rlimit-files",
                "2048",
                "--build-rlimit-procs",
                "512",
                "--log-retention-days",
                "7",
                "--cargo-flag=--features",
                "--cargo-flag=fast",
            ]
        );
        assert!(forwarded.env.is_empty());
    }

    /// The forwarded cargo flags parse on the compiler's side as values, a
    /// value that starts with `-` too.
    #[test]
    fn forwarded_cargo_flags_reach_the_compiler_as_values() {
        #[derive(clap::Parser)]
        struct Compiler {
            #[arg(long = "cargo-flag")]
            cargo_flags: Vec<String>,
        }
        let forwarded =
            forwarded_compiler_settings(&CompilerSettingArgs::default(), &section(BUILD_CONFIG))
                .unwrap();
        let cargo: Vec<&String> = forwarded
            .args
            .iter()
            .filter(|a| a.starts_with("--cargo-flag="))
            .collect();
        let parsed = <Compiler as clap::Parser>::try_parse_from(
            std::iter::once(&"cloacina-compiler".to_string()).chain(cargo),
        )
        .unwrap();
        assert_eq!(parsed.cargo_flags, ["--features", "fast"]);
    }

    #[test]
    fn a_build_flag_wins_over_config_toml() {
        let cli = CompilerSettingArgs {
            tenant_schema: Some("team_b".to_string()),
            build_rlimit_mem: Some("2G".to_string()),
            log_retention_days: Some(0),
            cargo_flag: vec!["--locked".to_string()],
            ..CompilerSettingArgs::default()
        };
        let forwarded = forwarded_compiler_settings(&cli, &section(BUILD_CONFIG)).unwrap();
        let value_of = |flag: &str| {
            let at = forwarded.args.iter().position(|a| a == flag).unwrap();
            forwarded.args[at + 1].clone()
        };
        assert_eq!(value_of("--tenant-schema"), "team_b");
        assert_eq!(value_of("--build-rlimit-mem"), "2G");
        assert_eq!(value_of("--log-retention-days"), "0");
        assert_eq!(value_of("--vendor-dir"), "/opt/cargo-home");
        let cargo: Vec<&String> = forwarded
            .args
            .iter()
            .filter(|a| a.starts_with("--cargo-flag"))
            .collect();
        assert_eq!(cargo, ["--cargo-flag=--locked"]);
        assert_eq!(
            forwarded
                .args
                .iter()
                .filter(|a| *a == "--tenant-schema")
                .count(),
            1
        );
    }

    /// Cargo flag additions and a full replacement cannot both be given: by
    /// flags, by keys, or one of each.
    #[test]
    fn cargo_flags_and_their_replacement_conflict() {
        let replace = CompilerSettingArgs {
            cargo_flags_replace: vec!["build".to_string(), "--lib".to_string()],
            ..CompilerSettingArgs::default()
        };
        let err = forwarded_compiler_settings(&replace, &section(BUILD_CONFIG)).unwrap_err();
        assert!(err.to_string().contains("--cargo-flags-replace"), "{err}");
        assert!(
            forwarded_compiler_settings(
                &CompilerSettingArgs::default(),
                &section(
                    "[compiler]\ncargo_flags = [\"--locked\"]\ncargo_flags_replace = [\"build\"]\n"
                ),
            )
            .is_err()
        );
        // The replacement alone is forwarded.
        let forwarded = forwarded_compiler_settings(&replace, &CompilerSection::default()).unwrap();
        assert_eq!(
            forwarded.args,
            ["--cargo-flags-replace=build", "--cargo-flags-replace=--lib"]
        );
        // On the command line clap refuses both.
        use clap::Args;
        let cmd = CompilerSettingArgs::augment_args(clap::Command::new("start"));
        assert!(
            cmd.try_get_matches_from([
                "start",
                "--cargo-flag=--locked",
                "--cargo-flags-replace=build"
            ])
            .is_err()
        );
    }

    /// The flags of `cloacina-compiler` that `compiler start` does not take:
    /// its two time limits, by Dylan's decision (CLOACINA-T-1124 note n-1).
    /// Set them on `cloacina-compiler` itself or through their env var.
    const NOT_ON_COMPILER_START: [&str; 2] = [
        "--build-timeout-s",
        // CLOACINA-T-1074, as `--build-timeout-s`.
        "--describe-timeout-s",
    ];

    /// CLOACINA-T-1106: each flag of `cloacina-compiler` is a flag of
    /// `cloacinactl compiler start` or in [`NOT_ON_COMPILER_START`]. A new
    /// compiler flag fails this test until it is put in one of them.
    #[test]
    fn compiler_start_takes_every_compiler_flag_but_the_listed_ones() {
        use clap::Args;
        let compiler = crate::shared::flag_list::long_flags(
            include_str!("../../../../cloacina-compiler/src/main.rs"),
            "Cli",
        );
        assert!(compiler.len() > 30, "the compiler's flags were not read");
        let cmd = super::super::CompilerCmd::augment_args(clap::Command::new("compiler"));
        let start = cmd.find_subcommand("start").unwrap();
        let mut taken: std::collections::BTreeSet<String> = start
            .get_arguments()
            .filter_map(|a| a.get_long().map(|l| format!("--{l}")))
            .collect();
        taken.insert("--home".to_string());
        taken.insert("--verbose".to_string());
        let left_out: std::collections::BTreeSet<String> = NOT_ON_COMPILER_START
            .iter()
            .map(|f| f.to_string())
            .collect();

        let missing: Vec<&String> = compiler
            .iter()
            .filter(|f| !taken.contains(*f) && !left_out.contains(*f))
            .collect();
        assert!(
            missing.is_empty(),
            "flags of cloacina-compiler that `compiler start` neither takes nor lists: {missing:?}"
        );
        let unknown: Vec<&String> = taken.difference(&compiler).collect();
        assert!(unknown.is_empty(), "not compiler flags: {unknown:?}");
        let stale: Vec<&String> = left_out.difference(&compiler).collect();
        assert!(stale.is_empty(), "listed but not compiler flags: {stale:?}");
        let both: Vec<&String> = left_out.intersection(&taken).collect();
        assert!(both.is_empty(), "taken and listed: {both:?}");
    }
}
