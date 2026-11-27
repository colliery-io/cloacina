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

//! The compiler vendors a Python package's dependencies (CLOACINA-T-1073).
//!
//! A Python package that ships no `vendor/` folder and has a
//! `requirements.lock` gets its dependencies here: for each platform of
//! [`PythonVendorConfig::platforms`], `uv pip install` puts the wheels the
//! lock names into a folder, and the folder is packed as one archive. The
//! build stores one archive per platform; a server or an agent unpacks the
//! one of its platform into `vendor/<platform>/` when it loads the package.
//!
//! - A package that ships `vendor/` is left as it is: no `uv` runs.
//! - Wheels only. `uv` runs with `--only-binary :all:`, so it runs no code of
//!   a dependency, and a dependency with no wheel fails the build by name.
//! - The lock file is checked before `uv` reads it: pinned versions with
//!   hashes, and nothing that names a file or a server.
//! - `uv` is a build child like `cargo`: it is made by
//!   `cloacina::packaging::build_child::build_child_command` with the
//!   compiler's confined policy, so it gets a cleared environment, the time
//!   limit and the resource limits.
//! - The wheels come from a package index, or from a wheel folder or a `uv`
//!   cache in the compiler image ([`WheelSource`]).

use std::path::{Path, PathBuf};
use std::process::Stdio;

use cloacina::packaging::build_child::{
    ChildPolicy, ChildStep, build_child_command, kill_child_group,
};
use cloacina_workflow_plugin::python_env::{self, Vendored};
use cloacina_workflow_plugin::python_vendor::{self, VendorPlan, WheelSource};

/// How this compiler vendors the dependencies of Python packages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonVendorConfig {
    /// The `uv` program (`--uv`): a name on `PATH`, or a path.
    pub uv: PathBuf,
    /// The Python version the wheels are for (`--python-version`), as
    /// `major.minor`. It must be the Python of the servers and agents that
    /// load the packages.
    pub python_version: String,
    /// The platforms to vendor for (`--python-platform`), as `<arch>-<os>`.
    pub platforms: Vec<String>,
    /// Where the wheels come from.
    pub source: WheelSource,
}

impl Default for PythonVendorConfig {
    fn default() -> Self {
        Self {
            uv: PathBuf::from("uv"),
            python_version: python_env::DEFAULT_PYTHON_VERSION.to_string(),
            platforms: python_env::DEFAULT_PLATFORMS
                .iter()
                .map(|p| p.to_string())
                .collect(),
            source: WheelSource::default(),
        }
    }
}

impl PythonVendorConfig {
    /// Make the settings usable, or say which one is not: the version becomes
    /// `major.minor`, the platforms are sorted, each one time.
    pub fn checked(mut self) -> Result<Self, String> {
        let (major, minor) = python_env::major_minor(&self.python_version)
            .map_err(|e| format!("--python-version: {e}"))?;
        self.python_version = format!("{major}.{minor}");
        if self.platforms.is_empty() {
            return Err("--python-platform: give at least one platform".to_string());
        }
        for p in &self.platforms {
            if python_env::uv_platform(p).is_none() {
                return Err(format!(
                    "--python-platform: unknown platform \"{p}\". Known platforms: {}",
                    python_env::SUPPORTED_PLATFORMS.join(", ")
                ));
            }
        }
        self.platforms.sort();
        self.platforms.dedup();
        self.source.check()?;
        Ok(self)
    }
}

/// The vendored dependencies of one platform.
#[derive(Debug)]
pub(crate) struct VendoredSet {
    pub platform: String,
    /// The archive to store.
    pub archive: Vec<u8>,
    /// True when a wheel holds compiled code.
    pub native: bool,
    pub unpacked_bytes: u64,
    pub files: u64,
}

/// What [`vendor_package`] did.
#[derive(Debug)]
pub(crate) enum VendorOutcome {
    /// The package ships `vendor/`: it is used as it is.
    Prebuilt,
    /// The package has no dependencies.
    NoDependencies,
    /// One set per platform of the settings.
    Vendored(Vec<VendoredSet>),
}

/// The longest error text of `uv` that goes into a build error.
const MAX_ERR: usize = 16 * 1024;

/// Vendor the dependencies of the unpacked Python package in `source_dir`.
///
/// `work` is a scratch folder of this build, on the same file system.
/// `policy` is the compiler's confined child policy. An `Err` is the build
/// error, as the publisher reads it.
pub(crate) async fn vendor_package(
    package: &str,
    version: &str,
    requires_python: Option<&str>,
    source_dir: &Path,
    work: &Path,
    config: &PythonVendorConfig,
    policy: &ChildPolicy,
) -> Result<VendorOutcome, String> {
    let pins = match python_vendor::vendor_plan(package, source_dir)? {
        VendorPlan::Prebuilt => return Ok(VendorOutcome::Prebuilt),
        VendorPlan::NoDependencies => return Ok(VendorOutcome::NoDependencies),
        VendorPlan::FromLock(pins) => pins,
    };
    let python = config.python_version.as_str();

    if let Some(spec) = requires_python
        && !python_env::requires_python_allows(spec, python)
            .map_err(|e| format!("package `{package}`: {e}"))?
    {
        return Err(format!(
            "package `{package}` requires Python {spec} (requires_python), and this compiler \
             vendors dependencies for Python {python} (--python-version)"
        ));
    }

    tracing::info!(
        package,
        version,
        pins = pins.len(),
        python,
        platforms = ?config.platforms,
        source = %config.source.describe(),
        "vendoring Python dependencies from the lock file"
    );

    let mut policy = policy.clone();
    for (key, value) in config.source.env() {
        policy = policy.env(key, value);
    }

    let mut sets = Vec::new();
    for platform in &config.platforms {
        let target = work.join(format!("py-vendor-{platform}"));
        run_uv_install(source_dir, &target, platform, config, &policy).await?;

        let (package, version, platform, python, work) = (
            package.to_string(),
            version.to_string(),
            platform.clone(),
            python.to_string(),
            work.to_path_buf(),
        );
        let set = tokio::task::spawn_blocking(move || {
            pack_set(&package, &version, &platform, &python, &target, &work)
        })
        .await
        .map_err(|e| format!("packing the vendored dependencies panicked: {e}"))??;
        tracing::info!(
            platform = %set.platform,
            unpacked_bytes = set.unpacked_bytes,
            files = set.files,
            archive_bytes = set.archive.len(),
            native = set.native,
            "vendored Python dependencies"
        );
        sets.push(set);
    }
    Ok(VendorOutcome::Vendored(sets))
}

/// Run `uv pip install` for one platform, as a confined build child.
async fn run_uv_install(
    source_dir: &Path,
    target: &Path,
    platform: &str,
    config: &PythonVendorConfig,
    policy: &ChildPolicy,
) -> Result<(), String> {
    let python = config.python_version.as_str();
    let args = python_vendor::uv_install_args(python_env::LOCK_FILE, target, platform, python)
        .ok_or_else(|| format!("unknown platform \"{platform}\""))?;

    // `Resolve`: uv downloads and runs no code of the package or of a
    // dependency (wheels only), so it also gets the proxy and certificate
    // names of the compiler's environment.
    let uv = config.uv.to_string_lossy();
    let mut cmd =
        tokio::process::Command::from(build_child_command(&uv, policy, ChildStep::Resolve));
    cmd.kill_on_drop(true)
        .args(args)
        .args(config.source.args())
        .current_dir(source_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            format!(
                "this compiler cannot vendor Python dependencies: `{uv}` was not found. The \
                 compiler image must hold uv (set --uv or CLOACINA_UV to its path). To publish \
                 without it, ship the dependencies in the package: `cloacinactl package build \
                 --vendor`"
            )
        } else {
            format!("failed to start `{uv}`: {e}")
        }
    })?;
    let pid = child.id();
    let waited = match policy.timeout() {
        Some(limit) => match tokio::time::timeout(limit, child.wait_with_output()).await {
            Ok(out) => out,
            Err(_) => {
                if let Some(pid) = pid {
                    kill_child_group(pid);
                }
                return Err(format!(
                    "vendoring for {platform} (Python {python}) timed out after {limit:?} \
                     (--build-timeout-s)"
                ));
            }
        },
        None => child.wait_with_output().await,
    };
    let out = waited.map_err(|e| format!("waiting for `{uv}` failed: {e}"))?;
    if !out.status.success() {
        let mut stderr = String::from_utf8_lossy(&out.stderr).to_string();
        if stderr.len() > MAX_ERR {
            let mut cut = MAX_ERR;
            while !stderr.is_char_boundary(cut) {
                cut -= 1;
            }
            stderr.truncate(cut);
        }
        return Err(python_vendor::uv_install_error(
            &stderr,
            platform,
            python,
            &config.source.describe(),
        ));
    }
    if !target.is_dir() {
        return Err(format!("uv wrote no files for {platform}"));
    }
    Ok(())
}

/// Pack the filled folder of one platform, and check that a loader will
/// unpack it: the archive goes through the unpack limits of a package.
fn pack_set(
    package: &str,
    version: &str,
    platform: &str,
    python: &str,
    target: &Path,
    work: &Path,
) -> Result<VendoredSet, String> {
    python_vendor::clean_target_dir(target);
    let native = python_vendor::target_dir_is_native(target);
    let (unpacked_bytes, files) = python_vendor::dir_size_and_files(target);

    let record = Vendored {
        python_version: python.to_string(),
        platforms: vec![platform.to_string()],
        native,
        lock_file: Some(python_env::LOCK_FILE.to_string()),
    };
    let pack_dir = work.join(format!("py-pack-{platform}"));
    std::fs::create_dir_all(&pack_dir)
        .map_err(|e| format!("create {}: {e}", pack_dir.display()))?;
    let out = work.join(format!("py-vendor-{platform}.set"));
    python_vendor::pack_vendor_artifact(&pack_dir, package, version, &record, target, &out)?;

    let check_dir = work.join(format!("py-check-{platform}"));
    std::fs::create_dir_all(&check_dir)
        .map_err(|e| format!("create {}: {e}", check_dir.display()))?;
    let limits = cloacina_workflow_plugin::unpack_limit::unpack_options();
    let checked = python_vendor::read_vendor_artifact(&out, &check_dir, &limits);
    let _ = std::fs::remove_dir_all(&check_dir);
    let _ = std::fs::remove_dir_all(&pack_dir);
    if let Err(e) = checked {
        return Err(format!(
            "the dependencies of package `{package}` for {platform} are {} in {files} files, \
             and a server or an agent would refuse to unpack them: {e}. Use fewer or smaller \
             dependencies, put the heavy ones in the agent image, or raise \
             --max-unpacked-bytes on the compiler, the server and the agents",
            human_size(unpacked_bytes)
        ));
    }

    let archive = std::fs::read(&out).map_err(|e| format!("read {}: {e}", out.display()))?;
    let _ = std::fs::remove_file(&out);
    Ok(VendoredSet {
        platform: platform.to_string(),
        archive,
        native,
        unpacked_bytes,
        files,
    })
}

fn human_size(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    if bytes as f64 >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB)
    } else {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use cloacina::packaging::build_child::ChildRlimits;
    use std::fs;
    use std::time::Duration;
    use tempfile::TempDir;

    const HASH: &str = "4721f391ed90541fddacab5acf947aa0d3dc7d27b2e1e8eda2be8970586c3274";

    fn lock() -> String {
        format!("six==1.17.0 \\\n    --hash=sha256:{HASH}\n")
    }

    /// A Python package source. `deps`: the list inside
    /// `[project].dependencies`. `lock`: the text of `requirements.lock`.
    fn write_package(dir: &Path, deps: Option<&str>, lock: Option<&str>, vendor: bool) {
        fs::create_dir_all(dir.join("workflow/demo")).unwrap();
        fs::write(dir.join("workflow/demo/__init__.py"), "").unwrap();
        fs::write(
            dir.join("package.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[metadata]\nlanguage = \"python\"\n",
        )
        .unwrap();
        if let Some(deps) = deps {
            fs::write(
                dir.join("pyproject.toml"),
                format!(
                    "[project]\nname = \"demo\"\nversion = \"0.1.0\"\ndependencies = [{deps}]\n"
                ),
            )
            .unwrap();
        }
        if let Some(lock) = lock {
            fs::write(dir.join("requirements.lock"), lock).unwrap();
        }
        if vendor {
            fs::create_dir_all(dir.join("vendor/x86_64-linux")).unwrap();
            fs::write(dir.join("vendor/x86_64-linux/six.py"), "SHIPPED = 1\n").unwrap();
        }
    }

    /// A stand-in for `uv`. It appends its arguments to `<dir>/uv.log` and
    /// its environment to `<dir>/uv.env`, then does what `mode` says:
    ///
    /// - `ok`: write a pure-Python tree into `--target`;
    /// - `native`: the same with a compiled file;
    /// - `no-wheel`, `absent`, `hash`: fail with the text `uv` 0.9.21 prints
    ///   for a dependency with no wheel, one that is not in the wheel folder,
    ///   and a wheel whose hash is not in the lock;
    /// - `offline-only`: `ok` when `--offline --no-index --find-links` are
    ///   given, else fail as a machine with no network does;
    /// - `sleep`: never finish.
    fn fake_uv(dir: &Path, mode: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let script = dir.join(format!("fake-uv-{mode}"));
        let body = format!(
            r##"#!/bin/sh
echo "$@" >> "{dir}/uv.log"
env >> "{dir}/uv.env"
mode="{mode}"
target=""; platform=""; offline=no; noindex=no; links=""
while [ $# -gt 0 ]; do
  case "$1" in
    --target) target="$2"; shift ;;
    --python-platform) platform="$2"; shift ;;
    --find-links) links="$2"; shift ;;
    --offline) offline=yes ;;
    --no-index) noindex=yes ;;
  esac
  shift
done
case "$mode" in
  sleep) sleep 600 ;;
  no-wheel)
    echo "  x No solution found when resolving dependencies:" >&2
    echo "  Because six==1.17.0 has no wheels with a matching platform tag (e.g.," >&2
    echo "      \`manylinux_2_28_aarch64\`) and you require six==1.17.0, we can conclude" >&2
    exit 1 ;;
  absent)
    echo "  x No solution found when resolving dependencies:" >&2
    echo "  Because six was not found in the provided package locations and" >&2
    echo "      you require six==1.17.0, we can conclude that your requirements are" >&2
    exit 1 ;;
  hash)
    echo "  x Failed to download \`six==1.17.0\`" >&2
    echo "  Hash mismatch for \`six==1.17.0\`" >&2
    echo "      Expected: sha256:00  Computed: sha256:47" >&2
    exit 1 ;;
  offline-only)
    if [ "$offline" != yes ] || [ "$noindex" != yes ] || [ ! -d "$links" ]; then
      echo "error: Failed to fetch: \`https://pypi.org/simple/six/\`" >&2
      echo "  Caused by: dns error: failed to lookup address information" >&2
      exit 2
    fi ;;
esac
mkdir -p "$target/six-1.17.0.dist-info" "$target/bin"
echo "VALUE = '$platform'" > "$target/six.py"
echo "#!/bin/sh" > "$target/bin/tool"
: > "$target/.lock"
printf 'Metadata-Version: 2.1\nName: six\nVersion: 1.17.0\n' > "$target/six-1.17.0.dist-info/METADATA"
printf 'six.py,sha256=x,1\n' > "$target/six-1.17.0.dist-info/RECORD"
if [ "$mode" = native ]; then
  printf 'Tag: cp311-cp311-%s\n' "$platform" > "$target/six-1.17.0.dist-info/WHEEL"
  : > "$target/_six.cpython-311.so"
else
  printf 'Tag: py3-none-any\n' > "$target/six-1.17.0.dist-info/WHEEL"
fi
exit 0
"##,
            dir = dir.display()
        );
        fs::write(&script, body).unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        script
    }

    fn confined(timeout: Duration) -> ChildPolicy {
        ChildPolicy::confined(
            timeout,
            ChildRlimits {
                cpu_s: 600,
                mem_bytes: 4 * 1024 * 1024 * 1024,
                files: 1024,
                procs: 256,
            },
        )
    }

    fn config(uv: PathBuf) -> PythonVendorConfig {
        PythonVendorConfig {
            uv,
            ..PythonVendorConfig::default()
        }
    }

    struct Fixture {
        tmp: TempDir,
        source: PathBuf,
        work: PathBuf,
    }

    fn fixture(deps: Option<&str>, lock: Option<&str>, vendor: bool) -> Fixture {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("demo-0.1.0");
        let work = tmp.path().join("work");
        fs::create_dir_all(&work).unwrap();
        write_package(&source, deps, lock, vendor);
        Fixture { tmp, source, work }
    }

    async fn vendor(
        f: &Fixture,
        requires_python: Option<&str>,
        config: &PythonVendorConfig,
        policy: &ChildPolicy,
    ) -> Result<VendorOutcome, String> {
        vendor_package(
            "demo",
            "0.1.0",
            requires_python,
            &f.source,
            &f.work,
            config,
            policy,
        )
        .await
    }

    fn uv_log(f: &Fixture) -> String {
        fs::read_to_string(f.tmp.path().join("uv.log")).unwrap_or_default()
    }

    /// Unpack a stored set the way a loader does, and give the staged root.
    fn load_set(f: &Fixture, set: &VendoredSet) -> (Vendored, PathBuf) {
        let root = f.tmp.path().join(format!("staged-{}", set.platform));
        fs::create_dir_all(root.join("workflow")).unwrap();
        let archive = f.tmp.path().join(format!("{}.set", set.platform));
        fs::write(&archive, &set.archive).unwrap();
        let scratch = f.tmp.path().join(format!("scratch-{}", set.platform));
        fs::create_dir_all(&scratch).unwrap();
        let record = python_vendor::unpack_vendor_artifact(
            &archive,
            &root,
            &scratch,
            &python_vendor::UnpackLimits::default(),
        )
        .unwrap();
        (record, root)
    }

    #[tokio::test]
    async fn package_with_a_lock_and_no_vendor_folder_is_vendored_per_platform() {
        let f = fixture(Some("\"six>=1.16\""), Some(&lock()), false);
        let uv = fake_uv(f.tmp.path(), "ok");
        let out = vendor(
            &f,
            Some(">=3.10"),
            &config(uv),
            &confined(Duration::from_secs(30)),
        )
        .await
        .unwrap();
        let VendorOutcome::Vendored(sets) = out else {
            panic!("expected sets, got {out:?}");
        };
        let platforms: Vec<&str> = sets.iter().map(|s| s.platform.as_str()).collect();
        assert_eq!(platforms, vec!["aarch64-linux", "x86_64-linux"]);

        for set in &sets {
            assert!(!set.native);
            assert!(set.files >= 3, "{}", set.files);
            let (record, root) = load_set(&f, set);
            assert_eq!(record.python_version, "3.11");
            assert_eq!(record.platforms, vec![set.platform.clone()]);
            assert!(!record.native);
            let dir = root.join("vendor").join(&set.platform);
            // The files of this platform, and none of uv's own.
            let uv_platform = python_env::uv_platform(&set.platform).unwrap();
            assert_eq!(
                fs::read_to_string(dir.join("six.py")).unwrap(),
                format!("VALUE = '{uv_platform}'\n")
            );
            assert!(!dir.join("bin").exists());
            assert!(!dir.join(".lock").exists());
        }

        // uv installed what the lock names: no resolve, no build, hashes on.
        let log = uv_log(&f);
        assert_eq!(log.lines().count(), 2, "{log}");
        assert!(!log.contains("compile"), "{log}");
        for line in log.lines() {
            assert!(line.starts_with("pip install --requirement requirements.lock --target "));
            for flag in [
                "--python-version 3.11",
                "--only-binary :all:",
                "--require-hashes",
                "--no-deps",
                "--no-config",
                "--no-python-downloads",
            ] {
                assert!(line.contains(flag), "{flag} missing in: {line}");
            }
        }
        assert!(log.contains("--python-platform aarch64-unknown-linux-gnu"));
        assert!(log.contains("--python-platform x86_64-unknown-linux-gnu"));
        // The package source is not changed.
        assert!(!f.source.join("vendor").exists());
    }

    #[tokio::test]
    async fn package_that_ships_vendor_is_left_as_it_is() {
        // The lock file is one the compiler would refuse: it is not read.
        let f = fixture(Some("\"six\""), Some("-r /etc/passwd\n"), true);
        let uv = fake_uv(f.tmp.path(), "ok");
        let out = vendor(&f, None, &config(uv), &confined(Duration::from_secs(30)))
            .await
            .unwrap();
        assert!(matches!(out, VendorOutcome::Prebuilt), "{out:?}");
        assert_eq!(uv_log(&f), "", "uv must not run");
        assert_eq!(
            fs::read_to_string(f.source.join("vendor/x86_64-linux/six.py")).unwrap(),
            "SHIPPED = 1\n"
        );
        assert_eq!(fs::read_dir(&f.work).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn package_with_no_dependencies_needs_no_uv() {
        for (deps, lock) in [(None, None), (Some(""), None), (Some(""), Some("# none\n"))] {
            let f = fixture(deps, lock, false);
            // A uv that does not exist: it is never started.
            let cfg = config(PathBuf::from("/nonexistent/uv"));
            let out = vendor(&f, None, &cfg, &confined(Duration::from_secs(30)))
                .await
                .unwrap();
            assert!(matches!(out, VendorOutcome::NoDependencies), "{out:?}");
        }
    }

    #[tokio::test]
    async fn dependencies_with_no_lock_file_fail_the_build() {
        let f = fixture(Some("\"six>=1.16\""), None, false);
        let uv = fake_uv(f.tmp.path(), "ok");
        let err = vendor(&f, None, &config(uv), &confined(Duration::from_secs(30)))
            .await
            .unwrap_err();
        assert!(err.contains("package `demo`"), "{err}");
        assert!(err.contains("has no requirements.lock"), "{err}");
        assert!(err.contains("cloacinactl package build"), "{err}");
        assert_eq!(uv_log(&f), "", "uv must not run");
    }

    #[tokio::test]
    async fn lock_file_that_names_a_file_or_a_server_never_reaches_uv() {
        for bad in [
            "-r /proc/1/environ\n".to_string(),
            format!("{}--index-url https://example.invalid/simple\n", lock()),
            format!("six @ file:///srv/six.whl --hash=sha256:{HASH}\n"),
        ] {
            let f = fixture(Some("\"six\""), Some(&bad), false);
            let uv = fake_uv(f.tmp.path(), "ok");
            let err = vendor(&f, None, &config(uv), &confined(Duration::from_secs(30)))
                .await
                .unwrap_err();
            assert!(err.contains("requirements.lock line"), "{err}");
            assert_eq!(uv_log(&f), "", "uv must not run for: {bad}");
        }
    }

    #[tokio::test]
    async fn dependency_with_no_wheel_fails_the_build_by_name() {
        let f = fixture(Some("\"six\""), Some(&lock()), false);
        let uv = fake_uv(f.tmp.path(), "no-wheel");
        let err = vendor(&f, None, &config(uv), &confined(Duration::from_secs(30)))
            .await
            .unwrap_err();
        assert!(
            err.starts_with("`six` has no wheel for aarch64-linux with Python 3.11"),
            "{err}"
        );
        assert!(
            err.contains("does not compile a source distribution"),
            "{err}"
        );
        // The build stops at the first platform.
        assert_eq!(uv_log(&f).lines().count(), 1);
    }

    #[tokio::test]
    async fn offline_build_from_a_wheel_folder() {
        let f = fixture(Some("\"six\""), Some(&lock()), false);
        let wheels = f.tmp.path().join("wheels");
        fs::create_dir_all(&wheels).unwrap();
        let uv = fake_uv(f.tmp.path(), "offline-only");

        // With the network settings of an online build, this uv fails.
        let online = config(uv.clone());
        let err = vendor(&f, None, &online, &confined(Duration::from_secs(30)))
            .await
            .unwrap_err();
        assert!(err.contains("dns error"), "{err}");

        // Offline from the folder: no index, no network.
        let offline = PythonVendorConfig {
            source: WheelSource {
                index_url: Some("https://mirror.example.invalid/simple".to_string()),
                wheel_dir: Some(wheels.clone()),
                cache_dir: None,
                offline: true,
            },
            ..config(uv)
        }
        .checked()
        .unwrap();
        let out = vendor(&f, None, &offline, &confined(Duration::from_secs(30)))
            .await
            .unwrap();
        assert!(
            matches!(out, VendorOutcome::Vendored(ref s) if s.len() == 2),
            "{out:?}"
        );
        let log = uv_log(&f);
        let last = log.lines().last().unwrap();
        assert!(last.contains("--offline"), "{last}");
        assert!(last.contains("--no-index"), "{last}");
        assert!(
            last.contains(&format!("--find-links {}", wheels.display())),
            "{last}"
        );
        // The index URL is not given to an offline build from a folder.
        let env = fs::read_to_string(f.tmp.path().join("uv.env")).unwrap();
        assert!(!env.contains("UV_DEFAULT_INDEX"), "{env}");
    }

    #[tokio::test]
    async fn distribution_missing_from_the_wheel_folder_is_named() {
        let f = fixture(Some("\"six\""), Some(&lock()), false);
        let wheels = f.tmp.path().join("wheels");
        fs::create_dir_all(&wheels).unwrap();
        let cfg = PythonVendorConfig {
            source: WheelSource {
                wheel_dir: Some(wheels.clone()),
                offline: true,
                ..WheelSource::default()
            },
            ..config(fake_uv(f.tmp.path(), "absent"))
        };
        let err = vendor(&f, None, &cfg, &confined(Duration::from_secs(30)))
            .await
            .unwrap_err();
        assert!(
            err.starts_with("`six` was not found for aarch64-linux"),
            "{err}"
        );
        assert!(
            err.contains(&format!("the wheel folder {}", wheels.display())),
            "{err}"
        );
        assert!(err.contains("no network"), "{err}");
    }

    #[tokio::test]
    async fn wheel_with_a_hash_the_lock_does_not_list_is_refused() {
        let f = fixture(Some("\"six\""), Some(&lock()), false);
        let uv = fake_uv(f.tmp.path(), "hash");
        let err = vendor(&f, None, &config(uv), &confined(Duration::from_secs(30)))
            .await
            .unwrap_err();
        assert!(err.starts_with("`six`: the wheel"), "{err}");
        assert!(err.contains("requirements.lock does not list"), "{err}");
        assert!(err.contains("Hash mismatch"), "{err}");
    }

    #[tokio::test]
    async fn compiled_code_is_recorded() {
        let f = fixture(Some("\"six\""), Some(&lock()), false);
        let cfg = PythonVendorConfig {
            platforms: vec!["x86_64-linux".to_string()],
            ..config(fake_uv(f.tmp.path(), "native"))
        };
        let out = vendor(&f, None, &cfg, &confined(Duration::from_secs(30)))
            .await
            .unwrap();
        let VendorOutcome::Vendored(sets) = out else {
            panic!("{out:?}");
        };
        assert_eq!(sets.len(), 1);
        assert!(sets[0].native);
        let (record, _) = load_set(&f, &sets[0]);
        assert!(record.native);
    }

    #[tokio::test]
    async fn requires_python_that_excludes_the_compiler_version_fails_before_uv() {
        let f = fixture(Some("\"six\""), Some(&lock()), false);
        let uv = fake_uv(f.tmp.path(), "ok");
        let err = vendor(
            &f,
            Some(">=3.12"),
            &config(uv),
            &confined(Duration::from_secs(30)),
        )
        .await
        .unwrap_err();
        assert!(err.contains("requires Python >=3.12"), "{err}");
        assert!(err.contains("for Python 3.11"), "{err}");
        assert_eq!(uv_log(&f), "");
    }

    #[tokio::test]
    async fn missing_uv_says_what_the_image_needs() {
        let f = fixture(Some("\"six\""), Some(&lock()), false);
        let cfg = config(PathBuf::from("/nonexistent/uv"));
        let err = vendor(&f, None, &cfg, &confined(Duration::from_secs(30)))
            .await
            .unwrap_err();
        assert!(err.contains("`/nonexistent/uv` was not found"), "{err}");
        assert!(err.contains("compiler image must hold uv"), "{err}");
        assert!(err.contains("--vendor"), "{err}");
    }

    /// The `uv` child is a build child: cleared environment, the names the
    /// policy gives, and nothing else of the compiler's own environment.
    #[tokio::test]
    async fn uv_runs_as_a_confined_build_child() {
        let f = fixture(Some("\"six\""), Some(&lock()), false);
        let uv = fake_uv(f.tmp.path(), "ok");
        // SAFETY: a name no other test reads or writes.
        unsafe {
            std::env::set_var(
                "CLOACINA_T1073_COMPILER_SECRET",
                "postgres://user:pw@db/app",
            );
        }
        let cfg = PythonVendorConfig {
            platforms: vec!["x86_64-linux".to_string()],
            source: WheelSource {
                index_url: Some("https://mirror.example.invalid/simple".to_string()),
                ..WheelSource::default()
            },
            ..config(uv)
        };
        vendor(&f, None, &cfg, &confined(Duration::from_secs(30)))
            .await
            .unwrap();

        let env = fs::read_to_string(f.tmp.path().join("uv.env")).unwrap();
        assert!(!env.contains("CLOACINA_T1073_COMPILER_SECRET"), "{env}");
        assert!(!env.contains("postgres://"), "{env}");
        // The index URL is in the environment, not on the command line.
        assert!(
            env.contains("UV_DEFAULT_INDEX=https://mirror.example.invalid/simple"),
            "{env}"
        );
        assert!(!uv_log(&f).contains("mirror.example.invalid"));
        // Every name the child has is on a list.
        use cloacina::packaging::build_child::{CHILD_ENV_ALLOWLIST, RESOLVE_ENV_ALLOWLIST};
        for line in env.lines() {
            let Some((name, _)) = line.split_once('=') else {
                continue;
            };
            let known = CHILD_ENV_ALLOWLIST.contains(&name)
                || RESOLVE_ENV_ALLOWLIST.contains(&name)
                || name == "UV_DEFAULT_INDEX"
                // The shell of the stand-in sets these itself.
                || matches!(name, "PWD" | "OLDPWD" | "SHLVL" | "_" | "__CF_USER_TEXT_ENCODING");
            assert!(known, "the uv child has `{name}` in its environment");
        }
    }

    #[tokio::test]
    async fn uv_past_the_time_limit_is_killed() {
        let f = fixture(Some("\"six\""), Some(&lock()), false);
        let uv = fake_uv(f.tmp.path(), "sleep");
        let started = std::time::Instant::now();
        let err = vendor(&f, None, &config(uv), &confined(Duration::from_millis(400)))
            .await
            .unwrap_err();
        assert!(err.contains("timed out"), "{err}");
        assert!(started.elapsed() < Duration::from_secs(20));
    }

    /// The real `uv` and the real index, with a scratch `uv` cache. Not run by
    /// default: `cargo test -p cloacina-compiler --lib real_uv -- --ignored`.
    #[tokio::test]
    #[ignore = "needs uv on PATH and a reachable package index"]
    async fn real_uv_vendors_from_the_index_then_offline_from_its_cache() {
        let f = fixture(Some("\"six==1.17.0\""), Some(&lock()), false);
        let cache = f.tmp.path().join("uv-cache");
        let online = PythonVendorConfig {
            source: WheelSource {
                cache_dir: Some(cache.clone()),
                ..WheelSource::default()
            },
            ..PythonVendorConfig::default()
        };
        let policy = confined(Duration::from_secs(120));
        let out = vendor(&f, Some(">=3.10"), &online, &policy).await.unwrap();
        let VendorOutcome::Vendored(sets) = out else {
            panic!("{out:?}");
        };
        assert_eq!(sets.len(), 2);
        for set in &sets {
            let (record, root) = load_set(&f, set);
            assert!(!record.native);
            assert!(
                root.join("vendor")
                    .join(&set.platform)
                    .join("six.py")
                    .is_file()
            );
            println!(
                "{}: {} files, {} bytes unpacked, {} bytes stored",
                set.platform,
                set.files,
                set.unpacked_bytes,
                set.archive.len()
            );
        }

        // The same build with no network, from the cache the first one filled.
        let offline = PythonVendorConfig {
            source: WheelSource {
                cache_dir: Some(cache),
                offline: true,
                ..WheelSource::default()
            },
            ..PythonVendorConfig::default()
        };
        let g = fixture(Some("\"six==1.17.0\""), Some(&lock()), false);
        let out = vendor(&g, None, &offline, &policy).await.unwrap();
        assert!(
            matches!(out, VendorOutcome::Vendored(ref s) if s.len() == 2),
            "{out:?}"
        );

        // A hash that is not the wheel's.
        let bad = format!("six==1.17.0 --hash=sha256:{}\n", "0".repeat(64));
        let h = fixture(Some("\"six==1.17.0\""), Some(&bad), false);
        let err = vendor(&h, None, &online, &policy).await.unwrap_err();
        assert!(err.starts_with("`six`: the wheel"), "{err}");
        println!("{err}");

        // A distribution the offline cache does not hold.
        let other = format!("idna==3.10 --hash=sha256:{}\n", "0".repeat(64));
        let i = fixture(Some("\"idna==3.10\""), Some(&other), false);
        let err = vendor(&i, None, &offline, &policy).await.unwrap_err();
        println!("{err}");
        assert!(err.contains("`idna`"), "{err}");
    }

    #[test]
    fn settings_are_checked() {
        let ok = PythonVendorConfig {
            python_version: "3.11.9".to_string(),
            platforms: vec![
                "x86_64-linux".to_string(),
                "aarch64-macos".to_string(),
                "x86_64-linux".to_string(),
            ],
            ..PythonVendorConfig::default()
        }
        .checked()
        .unwrap();
        assert_eq!(ok.python_version, "3.11");
        assert_eq!(ok.platforms, vec!["aarch64-macos", "x86_64-linux"]);

        let default = PythonVendorConfig::default().checked().unwrap();
        assert_eq!(default.python_version, "3.11");
        assert_eq!(default.platforms, vec!["aarch64-linux", "x86_64-linux"]);
        assert_eq!(default.uv, PathBuf::from("uv"));

        let bad = |c: PythonVendorConfig| c.checked().unwrap_err();
        assert!(
            bad(PythonVendorConfig {
                platforms: vec!["linux-arm64".to_string()],
                ..PythonVendorConfig::default()
            })
            .contains("unknown platform")
        );
        assert!(
            bad(PythonVendorConfig {
                platforms: Vec::new(),
                ..PythonVendorConfig::default()
            })
            .contains("at least one platform")
        );
        assert!(
            bad(PythonVendorConfig {
                python_version: "three".to_string(),
                ..PythonVendorConfig::default()
            })
            .contains("--python-version")
        );
        assert!(
            bad(PythonVendorConfig {
                source: WheelSource {
                    offline: true,
                    ..WheelSource::default()
                },
                ..PythonVendorConfig::default()
            })
            .contains("wheel folder or a uv cache")
        );
    }
}
