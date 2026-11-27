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
/// nothing.
pub(crate) fn forwarded_compiler_settings(
    cli: &CompilerSettingArgs,
    config: &CompilerSection,
) -> ForwardedCompilerSettings {
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
    if cli.python_offline || config.python_offline == Some(true) {
        out.args.push("--python-offline".to_string());
    }

    // The index URL can hold a password: an env var of the compiler process,
    // so it is not in the process list.
    if let Some(url) = text(&cli.python_index_url, &config.python_index_url) {
        out.env.push(("CLOACINA_COMPILER_PYTHON_INDEX_URL", url));
    }
    out
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
        forwarded_compiler_settings(&settings, &CloacinaConfig::load(&config_path).compiler);

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
        );
        assert_eq!(forwarded, ForwardedCompilerSettings::default());
    }

    /// CLOACINA-T-1106: each `[compiler]` key is forwarded as its flag; the
    /// index URL as an env var only.
    #[test]
    fn config_toml_compiler_settings_are_forwarded() {
        let forwarded =
            forwarded_compiler_settings(&CompilerSettingArgs::default(), &section(FULL_CONFIG));
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
        let forwarded = forwarded_compiler_settings(&cli, &section(FULL_CONFIG));
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
        );
        assert!(forwarded.args.is_empty());
    }

    /// The flags of `cloacina-compiler` that `compiler start` does not take
    /// (CLOACINA-T-1106). Set them on `cloacina-compiler` itself, or through
    /// their env var where they have one.
    const NOT_ON_COMPILER_START: [&str; 14] = [
        "--build-rlimit-cpu",
        "--build-rlimit-files",
        "--build-rlimit-mem",
        "--build-rlimit-procs",
        "--build-target",
        "--build-target-package",
        "--build-timeout-s",
        "--cargo-flag",
        "--cargo-flags-replace",
        "--cargo-target-dir",
        "--dev-workspace",
        "--log-retention-days",
        "--tenant-schema",
        "--vendor-dir",
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
