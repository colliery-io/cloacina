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

//! The dependencies of a Python package: the lock file, and the optional
//! prebuilt `vendor/` folder (CLOACINA-T-1048, CLOACINA-T-1073).
//!
//! `cloacinactl package build` on a Python package reads
//! `[project].dependencies` of the package's `pyproject.toml` and pins them
//! with `uv pip compile` into `requirements.lock`, with hashes. That is all
//! it does by default: the compiler installs what the lock names when the
//! package is published, for each platform of its settings.
//!
//! With `--vendor` the build also installs the wheels of each target platform
//! into `vendor/<platform>/` with `uv pip install --target`, and records the
//! Python version and the platforms in `[metadata.vendored]` of
//! `package.toml`. A package that ships `vendor/` is used as it is: the
//! compiler downloads nothing for it.
//!
//! `uv pip install --target` is used, and not a download of wheel files that
//! are then unpacked: `uv` has no `pip download` command. The install takes
//! `--python-platform` and `--python-version`, so it writes the files of
//! another platform than the one it runs on, and `--only-binary :all:` makes
//! it refuse a dependency that has no wheel.

use std::path::{Path, PathBuf};
use std::process::Command;

use clap::{Args, ValueEnum};
use cloacina_workflow_plugin::python_env::{
    self, DEFAULT_PLATFORMS, DEFAULT_PYTHON_VERSION, LOCK_FILE, Vendored,
};
use cloacina_workflow_plugin::python_vendor;

use super::manifest;
use crate::shared::error::CliError;

/// The CPU architectures of the official agent image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Arch {
    /// 64-bit x86 (`x86_64-linux`).
    #[value(name = "x86_64")]
    X86_64,
    /// 64-bit ARM (`aarch64-linux`).
    #[value(name = "aarch64")]
    Aarch64,
    /// Both architectures.
    Both,
}

/// The dependency flags of `package build` and `package publish`. They apply
/// to Python packages only.
#[derive(Debug, Clone, Args)]
pub struct VendorArgs {
    /// Python packages: also install the dependencies into
    /// `vendor/<platform>/`, so the package ships them. Without it the build
    /// writes `requirements.lock` only, and the compiler vendors the
    /// dependencies when the package is published.
    #[arg(long, conflicts_with = "no_lock")]
    pub vendor: bool,
    /// Python packages: the Python version the dependencies are locked (and
    /// with `--vendor`, vendored) for.
    #[arg(long, value_name = "MAJOR.MINOR", default_value = DEFAULT_PYTHON_VERSION)]
    pub python_version: String,
    /// Python packages, with `--vendor`: the CPU architecture of the Linux
    /// agents to vendor for.
    #[arg(long, value_enum, default_value_t = Arch::Both, conflicts_with = "platform")]
    pub arch: Arch,
    /// Python packages, with `--vendor`: a platform to vendor for, as
    /// `<arch>-<os>` (for example aarch64-macos). Repeat it for more than
    /// one. It replaces the platforms that `--arch` gives.
    #[arg(long = "platform", value_name = "ARCH-OS", requires = "vendor")]
    pub platform: Vec<String>,
    /// Python packages: use the `requirements.lock` in the package as it is.
    /// Without it the build resolves the versions again.
    #[arg(long)]
    pub locked: bool,
    /// Python packages: read no `pyproject.toml`, write no lock file and do
    /// not touch `vendor/`. For a package whose `vendor/` is filled by hand.
    #[arg(long)]
    pub no_lock: bool,
}

impl Default for VendorArgs {
    fn default() -> Self {
        Self {
            vendor: false,
            python_version: DEFAULT_PYTHON_VERSION.to_string(),
            arch: Arch::Both,
            platform: Vec::new(),
            locked: false,
            no_lock: false,
        }
    }
}

impl VendorArgs {
    /// The platforms to vendor for, sorted, each one time.
    pub fn platforms(&self) -> Result<Vec<String>, CliError> {
        let mut out: Vec<String> = if self.platform.is_empty() {
            match self.arch {
                Arch::X86_64 => vec!["x86_64-linux".to_string()],
                Arch::Aarch64 => vec!["aarch64-linux".to_string()],
                Arch::Both => DEFAULT_PLATFORMS.iter().map(|p| p.to_string()).collect(),
            }
        } else {
            self.platform.clone()
        };
        for p in &out {
            if python_env::uv_platform(p).is_none() {
                return Err(CliError::UserError(format!(
                    "unknown platform \"{p}\". Known platforms: {}",
                    python_env::SUPPORTED_PLATFORMS.join(", ")
                )));
            }
        }
        out.sort();
        out.dedup();
        Ok(out)
    }

    /// The Python version, checked to be `major.minor`.
    pub fn python(&self) -> Result<String, CliError> {
        let (major, minor) =
            python_env::major_minor(&self.python_version).map_err(CliError::UserError)?;
        Ok(format!("{major}.{minor}"))
    }
}

/// What a run of the build's dependency step did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VendorOutcome {
    /// `--no-lock`: nothing was read or written.
    Skipped,
    /// The package declares no dependencies.
    NothingToVendor,
    /// `requirements.lock` holds this many pinned distributions, and
    /// `vendor/` was not filled: the compiler vendors them.
    Locked(usize),
    /// `vendor/` was filled, and this record was written to `package.toml`.
    Vendored(Vendored),
}

/// The `uv` program: `$CLOACINA_UV` when it is set, else `uv` on `PATH`.
pub fn uv_program() -> PathBuf {
    std::env::var_os("CLOACINA_UV")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("uv"))
}

/// Read `[project].dependencies` of `<dir>/pyproject.toml`.
///
/// `Ok(None)`: there is no `pyproject.toml`. `Ok(Some(vec![]))`: the file
/// declares no dependencies.
pub fn read_dependencies(dir: &Path) -> Result<Option<Vec<String>>, CliError> {
    let path = dir.join("pyproject.toml");
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| CliError::UserError(format!("read {}: {e}", path.display())))?;
    python_vendor::pyproject_dependencies(&raw)
        .map(Some)
        .map_err(|e| CliError::UserError(format!("{}: {e}", path.display())))
}

/// What `package validate` and `package pack` check of a Python package's
/// dependencies before an upload: what the compiler would refuse
/// (CLOACINA-T-1073). A package that ships `vendor/` passes as it is.
/// Dependencies with no lock file, a lock file that does not pin one of them,
/// and a lock file that holds more than pins with hashes are errors.
pub fn check_dependencies_ship(dir: &Path, name: &str) -> Result<(), CliError> {
    python_vendor::vendor_plan(name, dir)
        .map(|_| ())
        .map_err(CliError::UserError)
}

/// Lock the dependencies of the Python package in `dir`, and with `--vendor`
/// vendor them. See the module documentation.
pub fn run(dir: &Path, args: &VendorArgs) -> Result<VendorOutcome, CliError> {
    run_with_uv(dir, args, &uv_program())
}

/// [`run`] with the `uv` program given (the tests give a script).
pub fn run_with_uv(dir: &Path, args: &VendorArgs, uv: &Path) -> Result<VendorOutcome, CliError> {
    if args.no_lock {
        println!(
            "--no-lock: {} and vendor/ of {} are left as they are",
            LOCK_FILE,
            dir.display()
        );
        return Ok(VendorOutcome::Skipped);
    }

    // uv runs with the package as its working directory: every path given
    // to it must not depend on the caller's working directory.
    let dir = &std::fs::canonicalize(dir)
        .map_err(|e| CliError::UserError(format!("{}: {e}", dir.display())))?;

    let metadata = manifest::read_metadata(dir)?;
    let previous = metadata.vendored.clone();
    let python = args.python()?;
    let platforms = args.platforms()?;

    let deps = read_dependencies(dir)?;
    let deps = match deps {
        Some(d) if !d.is_empty() => d,
        other => {
            let why = if other.is_none() {
                "no pyproject.toml"
            } else {
                "no [project].dependencies in pyproject.toml"
            };
            println!("{}: {why}. Nothing to lock.", dir.display());
            if let Some(prev) = &previous {
                // The dependencies were removed: remove what an earlier build wrote.
                for p in &prev.platforms {
                    remove_dir(&dir.join("vendor").join(p))?;
                }
                write_vendored_record(dir, None)?;
            }
            return Ok(VendorOutcome::NothingToVendor);
        }
    };

    if let Some(spec) = metadata.requires_python.as_deref() {
        let allowed =
            python_env::requires_python_allows(spec, &python).map_err(CliError::UserError)?;
        if !allowed {
            return Err(CliError::UserError(format!(
                "package.toml requires Python {spec} (requires_python), and the build locks \
                 dependencies for Python {python}. Give `--python-version`, or change \
                 requires_python."
            )));
        }
    }

    if args.vendor {
        refuse_hand_filled_vendor(dir, previous.as_ref())?;
    }

    let lock = dir.join(LOCK_FILE);
    let lock_before = std::fs::read_to_string(&lock).ok();
    if args.locked {
        if !lock.exists() {
            return Err(CliError::UserError(format!(
                "--locked: {} does not exist. Run the build one time without --locked.",
                lock.display()
            )));
        }
        println!("using {LOCK_FILE} as it is (--locked)");
    } else {
        let uv_version = check_uv(uv)?;
        println!(
            "resolving {} dependenc{} for Python {python} ({uv_version})",
            deps.len(),
            if deps.len() == 1 { "y" } else { "ies" }
        );
        let out = Command::new(uv)
            .current_dir(dir)
            .args([
                "pip",
                "compile",
                "pyproject.toml",
                "--universal",
                "--python-version",
                &python,
                "--generate-hashes",
                "--custom-compile-command",
                "cloacinactl package build",
                "--output-file",
                LOCK_FILE,
                "--quiet",
            ])
            .output()
            .map_err(|e| CliError::UserError(format!("failed to run uv: {e}")))?;
        if !out.status.success() {
            return Err(CliError::UserError(format!(
                "the dependencies of pyproject.toml do not resolve for Python {python} \
                 (uv pip compile failed):\n{}",
                String::from_utf8_lossy(&out.stderr).trim_end()
            )));
        }
    }

    if !args.vendor {
        return lock_only(dir, &deps, lock_before.as_deref(), previous.as_ref());
    }
    if args.locked {
        check_uv(uv)?;
    }

    let vendor = dir.join("vendor");
    std::fs::create_dir_all(&vendor)
        .map_err(|e| CliError::UserError(format!("create {}: {e}", vendor.display())))?;

    let mut native = false;
    for platform in &platforms {
        let staging = vendor.join(format!(".{platform}.tmp"));
        remove_dir(&staging)?;
        println!("vendoring for {platform} into vendor/{platform}/");
        let install = python_vendor::uv_install_args(LOCK_FILE, &staging, platform, &python)
            .expect("checked in platforms()");
        let out = Command::new(uv)
            .current_dir(dir)
            .args(install)
            .output()
            .map_err(|e| CliError::UserError(format!("failed to run uv: {e}")))?;
        if !out.status.success() {
            remove_dir(&staging)?;
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            return Err(CliError::UserError(python_vendor::uv_install_error(
                &stderr,
                platform,
                &python,
                "the package index of this machine's uv settings",
            )));
        }
        if !staging.is_dir() {
            return Err(CliError::UserError(format!(
                "uv pip install wrote no files for {platform}"
            )));
        }
        // Console scripts are not importable code, and `.lock` is uv's own
        // lock on the target folder.
        python_vendor::clean_target_dir(&staging);

        let dists = python_env::scan_dists(&staging);
        let native_here = python_vendor::target_dir_is_native(&staging);
        native |= native_here;

        let target = vendor.join(platform);
        remove_dir(&target)?;
        std::fs::rename(&staging, &target).map_err(|e| {
            CliError::UserError(format!("move {} into place: {e}", target.display()))
        })?;
        println!(
            "  {} distribution{}, {}{}",
            dists.len(),
            if dists.len() == 1 { "" } else { "s" },
            human_size(dir_size(&target)),
            if native_here {
                ", with compiled code"
            } else {
                ""
            }
        );
    }

    // A platform of an earlier build that this build does not make.
    if let Some(prev) = &previous {
        for p in &prev.platforms {
            if !platforms.contains(p) {
                remove_dir(&vendor.join(p))?;
            }
        }
    }

    let record = Vendored {
        python_version: python,
        platforms,
        native,
        lock_file: Some(LOCK_FILE.to_string()),
    };
    write_vendored_record(dir, Some(&record))?;
    println!(
        "vendored for Python {} on {} ({}); recorded in package.toml and {LOCK_FILE}",
        record.python_version,
        record.platforms.join(", "),
        if record.native {
            "the package runs on these platforms only"
        } else {
            "pure Python: the package runs on any platform"
        }
    );
    Ok(VendorOutcome::Vendored(record))
}

/// The end of a build without `--vendor`: the lock file is written, and
/// `vendor/` is not filled.
///
/// - The lock must be one the compiler accepts: pins with hashes only, and a
///   pin for each dependency of `pyproject.toml`.
/// - Folders that an earlier `--vendor` build wrote are kept when the lock
///   did not change: they hold what it names, and the package ships them. When
///   the lock changed they hold other versions, and the build stops.
fn lock_only(
    dir: &Path,
    deps: &[String],
    lock_before: Option<&str>,
    previous: Option<&Vendored>,
) -> Result<VendorOutcome, CliError> {
    let lock = dir.join(LOCK_FILE);
    let text = std::fs::read_to_string(&lock)
        .map_err(|e| CliError::UserError(format!("read {}: {e}", lock.display())))?;

    if let Some(prev) = previous {
        if lock_before != Some(text.as_str()) {
            return Err(CliError::UserError(format!(
                "{LOCK_FILE} changed, and vendor/ holds the dependencies of the lock before \
                 it (for {}). Run the build with --vendor to fill vendor/ again, or remove \
                 vendor/ and the [metadata.vendored] table of package.toml to let the compiler \
                 vendor the dependencies.",
                prev.platforms.join(", ")
            )));
        }
        println!(
            "{LOCK_FILE} did not change: vendor/ (for {}) is kept, and the package ships it",
            prev.platforms.join(", ")
        );
        return Ok(VendorOutcome::Vendored(prev.clone()));
    }

    if dir.join("vendor").is_dir() {
        println!("vendor/ is not written by this command: the package ships it as it is");
    }

    let pins = python_vendor::check_lock_file(&text).map_err(|e| {
        CliError::UserError(format!(
            "{e}\nThe compiler vendors only from such a lock file. A dependency it cannot \
             name this way (a URL, a path) ships in the package: run the build with --vendor."
        ))
    })?;
    let missing = python_vendor::dependencies_not_locked(deps, &pins);
    if !missing.is_empty() {
        return Err(CliError::UserError(format!(
            "{LOCK_FILE} does not pin {}, which pyproject.toml declares. Run the build \
             without --locked.",
            missing.join(", ")
        )));
    }
    println!(
        "wrote {LOCK_FILE}: {} pinned distribution{}. vendor/ is not filled: the compiler \
         vendors them when the package is published. To ship them in the package, pass --vendor.",
        pins.len(),
        if pins.len() == 1 { "" } else { "s" }
    );
    Ok(VendorOutcome::Locked(pins.len()))
}

/// `uv --version`, or an error that says how to get `uv`.
fn check_uv(uv: &Path) -> Result<String, CliError> {
    match Command::new(uv).arg("--version").output() {
        Ok(out) if out.status.success() => {
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        }
        Ok(out) => Err(CliError::UserError(format!(
            "`{} --version` failed: {}",
            uv.display(),
            String::from_utf8_lossy(&out.stderr).trim_end()
        ))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(CliError::UserError(format!(
            "`{}` was not found. `package build` locks the dependencies of a Python package \
             with uv. Install it (https://docs.astral.sh/uv/):\n    \
             curl -LsSf https://astral.sh/uv/install.sh | sh\n\
             then run the command again. Set CLOACINA_UV to the path of uv when it is not on \
             PATH. To write no lock file and leave vendor/ as it is, pass --no-lock.",
            uv.display()
        ))),
        Err(e) => Err(CliError::UserError(format!(
            "failed to run `{}`: {e}",
            uv.display()
        ))),
    }
}

/// Refuse to write into a `vendor/` folder that holds files this command did
/// not write.
fn refuse_hand_filled_vendor(dir: &Path, previous: Option<&Vendored>) -> Result<(), CliError> {
    let vendor = dir.join("vendor");
    let entries = match std::fs::read_dir(&vendor) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let ours = python_env::uv_platform(&name).is_some()
            || previous.is_some_and(|p| p.platforms.contains(&name))
            || name.starts_with('.');
        if !ours {
            return Err(CliError::UserError(format!(
                "{} holds `{name}`, which this command did not write. `package build --vendor` \
                 fills vendor/<platform>/ from pyproject.toml. Remove the files of vendor/ and \
                 declare the dependencies in pyproject.toml, or run the build without --vendor \
                 to keep vendor/ as it is.",
                vendor.display()
            )));
        }
    }
    Ok(())
}

/// Write (or with `None`, remove) the `[metadata.vendored]` table of
/// `<dir>/package.toml`. The rest of the file keeps its text and comments.
pub fn write_vendored_record(dir: &Path, record: Option<&Vendored>) -> Result<(), CliError> {
    let path = dir.join("package.toml");
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| CliError::UserError(format!("read {}: {e}", path.display())))?;
    let updated = set_vendored_table(&raw, record)?;
    if updated != raw {
        // The result must still be a manifest.
        let doc: toml::Value = toml::from_str(&updated)
            .map_err(|e| CliError::UserError(format!("package.toml after the update: {e}")))?;
        let inline = doc
            .get("metadata")
            .and_then(|m| m.get("vendored"))
            .is_some();
        if inline != record.is_some() {
            return Err(CliError::UserError(
                "package.toml declares `vendored` in a form this command cannot rewrite. \
                 Write it as a `[metadata.vendored]` table."
                    .to_string(),
            ));
        }
        std::fs::write(&path, updated)
            .map_err(|e| CliError::UserError(format!("write {}: {e}", path.display())))?;
    }
    Ok(())
}

/// The comment line above the table that [`write_vendored_record`] writes.
const RECORD_COMMENT: &str =
    "# Written by `cloacinactl package build`: what is in vendor/. Do not edit.";

/// The text of `package.toml` with its `[metadata.vendored]` table replaced by
/// `record` (removed with `None`).
fn set_vendored_table(raw: &str, record: Option<&Vendored>) -> Result<String, CliError> {
    let mut kept: Vec<&str> = Vec::new();
    let mut in_table = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            let header = trimmed
                .split('#')
                .next()
                .unwrap_or("")
                .trim()
                .replace(' ', "");
            in_table = header == "[metadata.vendored]";
        }
        if !in_table && trimmed != RECORD_COMMENT {
            kept.push(line);
        }
    }
    while kept.last().is_some_and(|l| l.trim().is_empty()) {
        kept.pop();
    }
    let mut out = kept.join("\n");
    out.push('\n');
    if let Some(record) = record {
        let body = toml::to_string(record)
            .map_err(|e| CliError::UserError(format!("serialize [metadata.vendored]: {e}")))?;
        out.push('\n');
        out.push_str(RECORD_COMMENT);
        out.push_str("\n[metadata.vendored]\n");
        out.push_str(&body);
    }
    Ok(out)
}

fn remove_dir(path: &Path) -> Result<(), CliError> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(CliError::UserError(format!(
            "remove {}: {e}",
            path.display()
        ))),
    }
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            match entry.metadata() {
                Ok(m) if m.is_dir() => total += dir_size(&p),
                Ok(m) => total += m.len(),
                Err(_) => {}
            }
        }
    }
    total
}

fn human_size(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    if bytes as f64 >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB)
    } else {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// A Python package source with a `pyproject.toml` that has `deps`.
    fn write_package(dir: &Path, deps: Option<&str>, extra_metadata: &str) {
        fs::create_dir_all(dir.join("workflow/demo")).unwrap();
        fs::write(dir.join("workflow/demo/__init__.py"), "").unwrap();
        fs::write(dir.join("workflow/demo/tasks.py"), "import cloaca\n").unwrap();
        fs::write(
            dir.join("package.toml"),
            format!(
                "# the demo package\n[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
                 [metadata]\nworkflow_name = \"demo\" # keep this comment\n{extra_metadata}"
            ),
        )
        .unwrap();
        if let Some(deps) = deps {
            fs::write(
                dir.join("pyproject.toml"),
                format!("[project]\nname = \"demo\"\nversion = \"0.1.0\"\n{deps}\n"),
            )
            .unwrap();
        }
    }

    /// A stand-in for `uv`: a script that writes a fixed lock file and a
    /// fixed tree, and logs its arguments. `native` makes the tree hold a
    /// compiled file. `fail` makes `pip install` fail as `uv` does for a
    /// dependency with no wheel.
    #[cfg(unix)]
    fn fake_uv(dir: &Path, native: bool, fail: bool) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let log = dir.join("uv.log");
        let script = dir.join("fake-uv");
        let body = format!(
            r##"#!/bin/sh
echo "$@" >> "{log}"
if [ "$1" = "--version" ]; then echo "uv 0.0.0-fake"; exit 0; fi
if [ "$2" = "compile" ]; then
  printf 'six==1.17.0 \\\n    --hash=sha256:4721f391ed90541fddacab5acf947aa0d3dc7d27b2e1e8eda2be8970586c3274\n' > requirements.lock
  exit 0
fi
if [ "$2" = "install" ]; then
  if [ "{fail}" = "true" ]; then
    echo "  x No solution found when resolving dependencies:" >&2
    echo "  Because docopt==0.6.2 has no usable wheels and you require" >&2
    echo "      docopt==0.6.2, we can conclude that your requirements are unsatisfiable." >&2
    echo "      hint: Wheels are required for \`docopt\` because building from source is" >&2
    exit 1
  fi
  target=""; platform=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --target) target="$2"; shift ;;
      --python-platform) platform="$2"; shift ;;
    esac
    shift
  done
  mkdir -p "$target/six-1.17.0.dist-info" "$target/bin"
  echo "print('six')" > "$target/six.py"
  echo "#!/bin/sh" > "$target/bin/tool"
  printf 'Metadata-Version: 2.1\nName: six\nVersion: 1.17.0\n' > "$target/six-1.17.0.dist-info/METADATA"
  printf 'six.py,sha256=x,1\n' > "$target/six-1.17.0.dist-info/RECORD"
  if [ "{native}" = "true" ]; then
    printf 'Tag: cp311-cp311-%s\n' "$platform" > "$target/six-1.17.0.dist-info/WHEEL"
    echo "" > "$target/_six.cpython-311.so"
  else
    printf 'Tag: py3-none-any\n' > "$target/six-1.17.0.dist-info/WHEEL"
  fi
  exit 0
fi
exit 2
"##,
            log = log.display()
        );
        fs::write(&script, body).unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        script
    }

    #[test]
    fn reads_dependencies_from_pyproject() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(read_dependencies(tmp.path()).unwrap(), None);

        fs::write(
            tmp.path().join("pyproject.toml"),
            "[project]\nname = \"x\"\ndependencies = [\"requests>=2.31\", \" pyyaml \", \"\"]\n",
        )
        .unwrap();
        assert_eq!(
            read_dependencies(tmp.path()).unwrap(),
            Some(vec!["requests>=2.31".to_string(), "pyyaml".to_string()])
        );

        fs::write(
            tmp.path().join("pyproject.toml"),
            "[project]\nname = \"x\"\n",
        )
        .unwrap();
        assert_eq!(read_dependencies(tmp.path()).unwrap(), Some(vec![]));

        fs::write(tmp.path().join("pyproject.toml"), "[build-system]\n").unwrap();
        assert_eq!(read_dependencies(tmp.path()).unwrap(), Some(vec![]));
    }

    #[test]
    fn rejects_dependencies_that_are_not_a_list_of_strings() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("pyproject.toml"),
            "[project]\ndependencies = \"requests\"\n",
        )
        .unwrap();
        assert!(read_dependencies(tmp.path()).is_err());
        fs::write(
            tmp.path().join("pyproject.toml"),
            "[project]\ndependencies = [1]\n",
        )
        .unwrap();
        assert!(read_dependencies(tmp.path()).is_err());
        fs::write(tmp.path().join("pyproject.toml"), "not toml [").unwrap();
        assert!(read_dependencies(tmp.path()).is_err());
    }

    #[test]
    fn default_target_is_the_agent_image() {
        let args = VendorArgs::default();
        assert_eq!(args.python().unwrap(), "3.11");
        assert_eq!(
            args.platforms().unwrap(),
            vec!["aarch64-linux".to_string(), "x86_64-linux".to_string()]
        );
    }

    #[test]
    fn arch_and_platform_choose_the_targets() {
        let mut args = VendorArgs {
            arch: Arch::Aarch64,
            ..VendorArgs::default()
        };
        assert_eq!(args.platforms().unwrap(), vec!["aarch64-linux".to_string()]);
        args.arch = Arch::X86_64;
        assert_eq!(args.platforms().unwrap(), vec!["x86_64-linux".to_string()]);

        args.platform = vec![
            "x86_64-linux".to_string(),
            "aarch64-macos".to_string(),
            "x86_64-linux".to_string(),
        ];
        assert_eq!(
            args.platforms().unwrap(),
            vec!["aarch64-macos".to_string(), "x86_64-linux".to_string()]
        );

        args.platform = vec!["linux-arm64".to_string()];
        let err = format!("{:?}", args.platforms().unwrap_err());
        assert!(err.contains("linux-arm64"), "{err}");
        assert!(err.contains("aarch64-linux"), "{err}");
    }

    #[test]
    fn python_version_must_be_major_minor() {
        let mut args = VendorArgs {
            python_version: "3.12.4".to_string(),
            ..VendorArgs::default()
        };
        assert_eq!(args.python().unwrap(), "3.12");
        args.python_version = "3".to_string();
        assert!(args.python().is_err());
    }

    #[test]
    fn record_is_written_and_replaced_and_removed() {
        let raw = "# top\n[package]\nname = \"demo\"\n\n[metadata]\nworkflow_name = \"demo\" # c\n";
        let record = Vendored {
            python_version: "3.11".to_string(),
            platforms: vec!["aarch64-linux".to_string(), "x86_64-linux".to_string()],
            native: true,
            lock_file: Some(LOCK_FILE.to_string()),
        };
        let one = set_vendored_table(raw, Some(&record)).unwrap();
        assert!(one.starts_with(raw.trim_end()), "{one}");
        let doc: toml::Value = toml::from_str(&one).unwrap();
        let back: Vendored = doc["metadata"]["vendored"].clone().try_into().unwrap();
        assert_eq!(back, record);

        // A second write replaces the table: it is there one time.
        let other = Vendored {
            platforms: vec!["x86_64-linux".to_string()],
            native: false,
            ..record.clone()
        };
        let two = set_vendored_table(&one, Some(&other)).unwrap();
        assert_eq!(two.matches("[metadata.vendored]").count(), 1);
        assert_eq!(two.matches(RECORD_COMMENT).count(), 1);
        let doc: toml::Value = toml::from_str(&two).unwrap();
        let back: Vendored = doc["metadata"]["vendored"].clone().try_into().unwrap();
        assert_eq!(back, other);

        // A table that follows the record is kept.
        let with_tail = format!("{two}\n[metadata.providers]\nredis = \"0.1\"\n");
        let three = set_vendored_table(&with_tail, None).unwrap();
        assert!(!three.contains("vendored"), "{three}");
        assert!(three.contains("[metadata.providers]"), "{three}");
        assert!(three.contains("# c"), "{three}");
    }

    /// `--vendor` with the default targets.
    fn vendor_args() -> VendorArgs {
        VendorArgs {
            vendor: true,
            ..VendorArgs::default()
        }
    }

    #[test]
    fn no_lock_flag_reads_nothing() {
        let tmp = TempDir::new().unwrap();
        let args = VendorArgs {
            no_lock: true,
            ..VendorArgs::default()
        };
        // No package.toml and no uv: the flag returns first.
        let out = run_with_uv(tmp.path(), &args, Path::new("/nonexistent/uv")).unwrap();
        assert_eq!(out, VendorOutcome::Skipped);
    }

    #[test]
    fn package_with_no_dependencies_needs_no_uv() {
        let tmp = TempDir::new().unwrap();
        write_package(tmp.path(), None, "");
        let out = run_with_uv(
            tmp.path(),
            &VendorArgs::default(),
            Path::new("/nonexistent/uv"),
        )
        .unwrap();
        assert_eq!(out, VendorOutcome::NothingToVendor);
        assert!(!tmp.path().join("vendor").exists());

        write_package(tmp.path(), Some("dependencies = []"), "");
        let out = run_with_uv(
            tmp.path(),
            &VendorArgs::default(),
            Path::new("/nonexistent/uv"),
        )
        .unwrap();
        assert_eq!(out, VendorOutcome::NothingToVendor);
    }

    #[test]
    fn missing_uv_says_how_to_get_it() {
        let tmp = TempDir::new().unwrap();
        write_package(tmp.path(), Some("dependencies = [\"six\"]"), "");
        let err = run_with_uv(
            tmp.path(),
            &VendorArgs::default(),
            Path::new("/nonexistent/uv"),
        )
        .unwrap_err();
        let text = format!("{err:?}");
        assert!(text.contains("was not found"), "{text}");
        assert!(text.contains("astral.sh/uv/install.sh"), "{text}");
        assert!(text.contains("--no-lock"), "{text}");
    }

    #[cfg(unix)]
    #[test]
    fn vendors_each_platform_and_records_it() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("demo");
        write_package(&pkg, Some("dependencies = [\"six\"]"), "");
        let uv = fake_uv(tmp.path(), false, false);

        let out = run_with_uv(&pkg, &vendor_args(), &uv).unwrap();
        let record = match out {
            VendorOutcome::Vendored(r) => r,
            other => panic!("unexpected outcome {other:?}"),
        };
        assert_eq!(record.python_version, "3.11");
        assert_eq!(record.platforms, vec!["aarch64-linux", "x86_64-linux"]);
        assert!(!record.native);
        assert_eq!(record.lock_file.as_deref(), Some("requirements.lock"));

        for p in ["aarch64-linux", "x86_64-linux"] {
            assert!(pkg.join("vendor").join(p).join("six.py").exists());
            // Console scripts are dropped.
            assert!(!pkg.join("vendor").join(p).join("bin").exists());
            assert!(!pkg.join("vendor").join(format!(".{p}.tmp")).exists());
        }
        assert!(pkg.join("requirements.lock").exists());

        // The manifest carries the record and still resolves.
        let meta = manifest::read_metadata(&pkg).unwrap();
        assert_eq!(meta.vendored, Some(record));
        let text = fs::read_to_string(pkg.join("package.toml")).unwrap();
        assert!(text.contains("# keep this comment"), "{text}");

        // The commands uv got.
        let log = fs::read_to_string(tmp.path().join("uv.log")).unwrap();
        assert!(log.contains("pip compile pyproject.toml --universal --python-version 3.11"));
        assert!(log.contains("--python-platform aarch64-unknown-linux-gnu"));
        assert!(log.contains("--python-platform x86_64-unknown-linux-gnu"));
        assert!(log.contains("--only-binary :all:"));
        assert!(log.contains("--require-hashes"));
    }

    #[cfg(unix)]
    #[test]
    fn native_wheels_are_recorded_and_a_dropped_platform_is_removed() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("demo");
        write_package(&pkg, Some("dependencies = [\"six\"]"), "");
        let uv = fake_uv(tmp.path(), true, false);

        run_with_uv(&pkg, &vendor_args(), &uv).unwrap();
        assert!(pkg.join("vendor/aarch64-linux").is_dir());

        let args = VendorArgs {
            arch: Arch::X86_64,
            python_version: "3.12".to_string(),
            ..vendor_args()
        };
        let out = run_with_uv(&pkg, &args, &uv).unwrap();
        let record = match out {
            VendorOutcome::Vendored(r) => r,
            other => panic!("unexpected outcome {other:?}"),
        };
        assert!(record.native);
        assert_eq!(record.python_version, "3.12");
        assert_eq!(record.platforms, vec!["x86_64-linux"]);
        assert!(!pkg.join("vendor/aarch64-linux").exists());
        assert!(pkg.join("vendor/x86_64-linux").is_dir());
        let text = fs::read_to_string(pkg.join("package.toml")).unwrap();
        assert_eq!(text.matches("[metadata.vendored]").count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn locked_uses_the_lock_file_and_does_not_resolve() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("demo");
        write_package(&pkg, Some("dependencies = [\"six\"]"), "");
        let uv = fake_uv(tmp.path(), false, false);
        let args = VendorArgs {
            locked: true,
            ..vendor_args()
        };

        let err = format!("{:?}", run_with_uv(&pkg, &args, &uv).unwrap_err());
        assert!(err.contains("requirements.lock"), "{err}");

        fs::write(pkg.join("requirements.lock"), "six==1.16.0\n").unwrap();
        run_with_uv(&pkg, &args, &uv).unwrap();
        let log = fs::read_to_string(tmp.path().join("uv.log")).unwrap();
        assert!(!log.contains("pip compile"), "{log}");
        assert_eq!(
            fs::read_to_string(pkg.join("requirements.lock")).unwrap(),
            "six==1.16.0\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn dependency_with_no_wheel_is_named() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("demo");
        write_package(&pkg, Some("dependencies = [\"docopt==0.6.2\"]"), "");
        let uv = fake_uv(tmp.path(), false, true);
        let err = format!("{:?}", run_with_uv(&pkg, &vendor_args(), &uv).unwrap_err());
        assert!(
            err.contains("`docopt` has no wheel for aarch64-linux"),
            "{err}"
        );
        assert!(err.contains("Python 3.11"), "{err}");
        // Nothing half-written, and no record.
        assert!(!pkg.join("vendor/aarch64-linux").exists());
        assert!(!pkg.join("vendor/.aarch64-linux.tmp").exists());
        assert!(manifest::read_metadata(&pkg).unwrap().vendored.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn hand_filled_vendor_is_not_overwritten() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("demo");
        write_package(&pkg, Some("dependencies = [\"six\"]"), "");
        fs::create_dir_all(pkg.join("vendor/requests")).unwrap();
        let uv = fake_uv(tmp.path(), false, false);
        let err = format!("{:?}", run_with_uv(&pkg, &vendor_args(), &uv).unwrap_err());
        assert!(err.contains("`requests`"), "{err}");
        assert!(err.contains("without --vendor"), "{err}");
        assert!(pkg.join("vendor/requests").is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn requires_python_that_excludes_the_target_fails_before_uv() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("demo");
        write_package(
            &pkg,
            Some("dependencies = [\"six\"]"),
            "requires_python = \">=3.12\"\n",
        );
        let uv = fake_uv(tmp.path(), false, false);
        let err = format!(
            "{:?}",
            run_with_uv(&pkg, &VendorArgs::default(), &uv).unwrap_err()
        );
        assert!(err.contains(">=3.12"), "{err}");
        assert!(err.contains("3.11"), "{err}");
        assert!(!tmp.path().join("uv.log").exists());
    }

    #[cfg(unix)]
    #[test]
    fn removed_dependencies_remove_the_record_and_its_folders() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("demo");
        write_package(&pkg, Some("dependencies = [\"six\"]"), "");
        let uv = fake_uv(tmp.path(), false, false);
        run_with_uv(&pkg, &vendor_args(), &uv).unwrap();

        fs::write(
            pkg.join("pyproject.toml"),
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let out = run_with_uv(&pkg, &vendor_args(), &uv).unwrap();
        assert_eq!(out, VendorOutcome::NothingToVendor);
        assert!(!pkg.join("vendor/x86_64-linux").exists());
        assert!(manifest::read_metadata(&pkg).unwrap().vendored.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn default_build_writes_the_lock_and_does_not_fill_vendor() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("demo");
        write_package(&pkg, Some("dependencies = [\"six\"]"), "");
        let uv = fake_uv(tmp.path(), false, false);

        let out = run_with_uv(&pkg, &VendorArgs::default(), &uv).unwrap();
        assert_eq!(out, VendorOutcome::Locked(1));
        assert!(pkg.join("requirements.lock").exists());
        assert!(!pkg.join("vendor").exists());
        assert!(manifest::read_metadata(&pkg).unwrap().vendored.is_none());

        // uv resolved, and installed nothing.
        let log = fs::read_to_string(tmp.path().join("uv.log")).unwrap();
        assert!(log.contains("pip compile pyproject.toml --universal --python-version 3.11"));
        assert!(log.contains("--generate-hashes"), "{log}");
        assert!(!log.contains("pip install"), "{log}");

        // What the build wrote is what the compiler accepts.
        check_dependencies_ship(&pkg, "demo").unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn default_build_keeps_a_prebuilt_vendor_while_the_lock_is_the_same() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("demo");
        write_package(&pkg, Some("dependencies = [\"six\"]"), "");
        let uv = fake_uv(tmp.path(), false, false);
        let built = run_with_uv(&pkg, &vendor_args(), &uv).unwrap();

        // `publish` after `build --vendor`: same lock, vendor/ stays.
        let again = run_with_uv(&pkg, &VendorArgs::default(), &uv).unwrap();
        assert_eq!(again, built);
        assert!(pkg.join("vendor/x86_64-linux/six.py").exists());
        assert!(manifest::read_metadata(&pkg).unwrap().vendored.is_some());

        // The lock changes under it: the folders hold other versions.
        fs::write(
            pkg.join("requirements.lock"),
            "six==1.16.0 --hash=sha256:00\n",
        )
        .unwrap();
        let err = format!(
            "{:?}",
            run_with_uv(&pkg, &VendorArgs::default(), &uv).unwrap_err()
        );
        assert!(err.contains("requirements.lock changed"), "{err}");
        assert!(err.contains("--vendor"), "{err}");
        assert!(pkg.join("vendor/x86_64-linux/six.py").exists());
    }

    #[cfg(unix)]
    #[test]
    fn default_build_leaves_a_hand_filled_vendor_alone() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("demo");
        write_package(&pkg, Some("dependencies = [\"six\"]"), "");
        fs::create_dir_all(pkg.join("vendor/requests")).unwrap();
        let uv = fake_uv(tmp.path(), false, false);
        let out = run_with_uv(&pkg, &VendorArgs::default(), &uv).unwrap();
        assert_eq!(out, VendorOutcome::Locked(1));
        assert!(pkg.join("vendor/requests").is_dir());
        check_dependencies_ship(&pkg, "demo").unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn default_build_refuses_a_lock_the_compiler_would_refuse() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("demo");
        write_package(&pkg, Some("dependencies = [\"six\"]"), "");
        let uv = fake_uv(tmp.path(), false, false);
        let args = VendorArgs {
            locked: true,
            ..VendorArgs::default()
        };
        fs::write(
            pkg.join("requirements.lock"),
            "six @ https://example.invalid/six-1.17.0-py3-none-any.whl\n",
        )
        .unwrap();
        let err = format!("{:?}", run_with_uv(&pkg, &args, &uv).unwrap_err());
        assert!(err.contains("line 1"), "{err}");
        assert!(err.contains("--vendor"), "{err}");

        // A lock that leaves out a declared dependency.
        fs::write(
            pkg.join("requirements.lock"),
            "idna==3.10 --hash=sha256:4721f391ed90541fddacab5acf947aa0d3dc7d27b2e1e8eda2be8970586c3274\n",
        )
        .unwrap();
        let err = format!("{:?}", run_with_uv(&pkg, &args, &uv).unwrap_err());
        assert!(err.contains("does not pin six"), "{err}");
    }

    #[test]
    fn dependencies_with_no_lock_and_no_vendor_do_not_ship() {
        let tmp = TempDir::new().unwrap();
        let pkg = tmp.path().join("demo");
        write_package(&pkg, Some("dependencies = [\"six\"]"), "");
        let err = format!("{:?}", check_dependencies_ship(&pkg, "demo").unwrap_err());
        assert!(err.contains("package `demo`"), "{err}");
        assert!(err.contains("has no requirements.lock"), "{err}");

        // A vendor/ folder ships them.
        fs::create_dir_all(pkg.join("vendor")).unwrap();
        check_dependencies_ship(&pkg, "demo").unwrap();

        // No dependencies: nothing to ship.
        let bare = tmp.path().join("bare");
        write_package(&bare, None, "");
        check_dependencies_ship(&bare, "demo").unwrap();
    }
}
