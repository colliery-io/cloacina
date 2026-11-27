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

//! What the compiler and `cloacinactl` share to vendor a Python package's
//! dependencies from its lock file (CLOACINA-T-1073).
//!
//! - The check of `requirements.lock`: the compiler gives the file to `uv`,
//!   and the file comes from a tenant. It may hold pinned versions with
//!   hashes and nothing else.
//! - The arguments of `uv pip install`, and the reading of its errors, so the
//!   publisher's build and the compiler install the same way and name a
//!   failing distribution the same way.
//! - The vendored set as an artifact: one archive per platform, in the format
//!   of a package archive, that the compiler stores and a loader unpacks into
//!   `vendor/<platform>/`.
//!
//! This module starts no process. The caller runs `uv`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::python_env::{self, Vendored};

// ---------------------------------------------------------------------------
// The lock file
// ---------------------------------------------------------------------------

/// One pinned distribution of a lock file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockPin {
    /// The distribution name, normalized (`PyYAML` → `pyyaml`).
    pub name: String,
    /// The pinned version.
    pub version: String,
    /// The line of the pin, from 1.
    pub line: usize,
}

/// A distribution name as package indexes compare it: lower case, with each
/// run of `-`, `_` and `.` as one `-`.
pub fn normalize_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut sep = false;
    for c in name.trim().chars() {
        if matches!(c, '-' | '_' | '.') {
            sep = true;
        } else {
            if sep && !out.is_empty() {
                out.push('-');
            }
            sep = false;
            out.push(c.to_ascii_lowercase());
        }
    }
    out
}

fn is_name(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && text
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
}

fn is_version(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '!' | '+' | '_' | '-'))
}

fn is_hash(text: &str) -> bool {
    match text.split_once(':') {
        Some((algo, hex)) => {
            matches!(algo, "sha256" | "sha384" | "sha512")
                && hex.len() >= 64
                && hex.chars().all(|c| c.is_ascii_hexdigit())
        }
        None => false,
    }
}

/// Check a lock file and read its pins.
///
/// The file may hold, and nothing else:
///
/// - comment lines and empty lines;
/// - `name==version`, with an optional `; marker`;
/// - `--hash=<sha256|sha384|sha512>:<hex>` for the pin above it (or on the
///   pin's own line).
///
/// Every pin must have a hash. An option line (`-r`, `--index-url`,
/// `--find-links`, …), a URL, a path or a version range is an error that
/// names the line: such a line would make `uv` read a file or a server that
/// the lock's author chose.
pub fn check_lock_file(text: &str) -> Result<Vec<LockPin>, String> {
    const HOW: &str = "The lock file may hold only pinned versions (`name==version`) with \
                       `--hash` lines. Write it with `cloacinactl package build`.";
    let mut pins: Vec<LockPin> = Vec::new();
    let mut hashed: Vec<bool> = Vec::new();

    for (index, raw) in text.lines().enumerate() {
        let line_no = index + 1;
        let mut line = raw.trim();
        if let Some(rest) = line.strip_suffix('\\') {
            line = rest.trim_end();
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // The requirement (if any), then `--hash=` words.
        let (head, hashes) = match line.find("--hash=") {
            Some(at) => (line[..at].trim(), &line[at..]),
            None => (line, ""),
        };

        if !head.is_empty() {
            if head.starts_with('-') {
                let option = head.split([' ', '=']).next().unwrap_or(head);
                return Err(format!(
                    "{} line {line_no}: the option `{option}` is not allowed. {HOW}",
                    python_env::LOCK_FILE
                ));
            }
            // An inline comment after the requirement.
            let head = match head.find(" #") {
                Some(at) => head[..at].trim(),
                None => head,
            };
            let requirement = head.split(';').next().unwrap_or("").trim();
            let pin = requirement
                .split_once("==")
                .map(|(n, v)| (n.trim(), v.trim()))
                .filter(|(n, v)| is_name(n) && is_version(v));
            let Some((name, version)) = pin else {
                return Err(format!(
                    "{} line {line_no}: `{requirement}` is not a pinned version. {HOW}",
                    python_env::LOCK_FILE
                ));
            };
            pins.push(LockPin {
                name: normalize_name(name),
                version: version.to_string(),
                line: line_no,
            });
            hashed.push(false);
        }

        for word in hashes.split_whitespace() {
            let value = word.strip_prefix("--hash=").filter(|v| is_hash(v));
            if value.is_none() {
                return Err(format!(
                    "{} line {line_no}: `{word}` is not a `--hash=sha256:<hex>` value. {HOW}",
                    python_env::LOCK_FILE
                ));
            }
            match hashed.last_mut() {
                Some(h) => *h = true,
                None => {
                    return Err(format!(
                        "{} line {line_no}: a `--hash` with no pinned version above it. {HOW}",
                        python_env::LOCK_FILE
                    ));
                }
            }
        }
    }

    if let Some(at) = hashed.iter().position(|h| !h) {
        let pin = &pins[at];
        return Err(format!(
            "{} line {}: `{}=={}` has no `--hash`. {HOW}",
            python_env::LOCK_FILE,
            pin.line,
            pin.name,
            pin.version
        ));
    }
    Ok(pins)
}

/// The dependencies of `pyproject.toml` that the lock file does not pin, by
/// normalized name. A dependency with an environment marker (`; …`) is not
/// checked: the marker can leave it out of the lock.
pub fn dependencies_not_locked(dependencies: &[String], pins: &[LockPin]) -> Vec<String> {
    let mut missing = Vec::new();
    for dep in dependencies {
        if dep.contains(';') {
            continue;
        }
        let name: String = dep
            .trim()
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
            .collect();
        if name.is_empty() {
            continue;
        }
        let name = normalize_name(&name);
        if !pins.iter().any(|p| p.name == name) && !missing.contains(&name) {
            missing.push(name);
        }
    }
    missing
}

/// Read `[project].dependencies` of the text of a `pyproject.toml`: the
/// requirement strings, trimmed, without empty ones. A file with no such key
/// gives an empty list.
pub fn pyproject_dependencies(pyproject: &str) -> Result<Vec<String>, String> {
    const SHAPE: &str = "pyproject.toml: [project].dependencies must be a list of requirement \
                         strings";
    let doc: toml::Value =
        toml::from_str(pyproject).map_err(|e| format!("invalid pyproject.toml: {e}"))?;
    let deps = match doc.get("project").and_then(|p| p.get("dependencies")) {
        None => return Ok(Vec::new()),
        Some(d) => d,
    };
    let list = deps.as_array().ok_or_else(|| SHAPE.to_string())?;
    let mut out = Vec::new();
    for item in list {
        let s = item.as_str().ok_or_else(|| SHAPE.to_string())?;
        if !s.trim().is_empty() {
            out.push(s.trim().to_string());
        }
    }
    Ok(out)
}

/// What a Python package source asks of a build that can vendor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VendorPlan {
    /// The package ships a `vendor/` folder: it is used as it is.
    Prebuilt,
    /// The package declares no dependencies: there is nothing to vendor.
    NoDependencies,
    /// Install these pins, from the lock file, for each platform.
    FromLock(Vec<LockPin>),
}

/// Decide what to do with the Python package in `dir`.
///
/// - A `vendor/` folder: [`VendorPlan::Prebuilt`]. Nothing else is read.
/// - No dependencies in `pyproject.toml` (or no such file) and no lock file:
///   [`VendorPlan::NoDependencies`].
/// - Dependencies and no lock file: an error. The build does not resolve
///   versions: what runs is what the publisher locked.
/// - A lock file: it is checked ([`check_lock_file`]), and each dependency of
///   `pyproject.toml` must be pinned in it.
pub fn vendor_plan(package: &str, dir: &Path) -> Result<VendorPlan, String> {
    if dir.join("vendor").is_dir() {
        return Ok(VendorPlan::Prebuilt);
    }
    let dependencies = match std::fs::read_to_string(dir.join("pyproject.toml")) {
        Ok(text) => {
            pyproject_dependencies(&text).map_err(|e| format!("package `{package}`: {e}"))?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(format!("package `{package}`: read pyproject.toml: {e}")),
    };
    let lock = match std::fs::read_to_string(dir.join(python_env::LOCK_FILE)) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            return Err(format!(
                "package `{package}`: read {}: {e}",
                python_env::LOCK_FILE
            ));
        }
    };
    let Some(lock) = lock else {
        if dependencies.is_empty() {
            return Ok(VendorPlan::NoDependencies);
        }
        return Err(format!(
            "package `{package}` declares dependencies in pyproject.toml ({}) and has no {} and \
             no vendor/ folder. Run `cloacinactl package build` to write the lock file (or \
             `cloacinactl package build --vendor` to ship the dependencies in the package), \
             then publish again.",
            dependencies.join(", "),
            python_env::LOCK_FILE
        ));
    };
    let pins = check_lock_file(&lock).map_err(|e| format!("package `{package}`: {e}"))?;
    let missing = dependencies_not_locked(&dependencies, &pins);
    if !missing.is_empty() {
        return Err(format!(
            "package `{package}`: {} does not pin {}, which pyproject.toml declares. The lock \
             file is older than pyproject.toml: run `cloacinactl package build` again.",
            python_env::LOCK_FILE,
            missing
                .iter()
                .map(|n| format!("`{n}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if pins.is_empty() {
        return Ok(VendorPlan::NoDependencies);
    }
    Ok(VendorPlan::FromLock(pins))
}

// ---------------------------------------------------------------------------
// uv
// ---------------------------------------------------------------------------

/// The arguments of `uv pip install` that fill `target` with the wheels of
/// `lock_file` for one platform. `None` for a platform `uv` has no name for.
///
/// `--only-binary :all:` refuses a distribution with no wheel, `--no-deps`
/// installs what the lock names and nothing else, and `--require-hashes`
/// refuses a wheel whose hash is not in the lock.
pub fn uv_install_args(
    lock_file: &str,
    target: &Path,
    platform: &str,
    python: &str,
) -> Option<Vec<OsString>> {
    let uv_platform = python_env::uv_platform(platform)?;
    let mut args: Vec<OsString> = ["pip", "install", "--requirement", lock_file, "--target"]
        .iter()
        .map(OsString::from)
        .collect();
    args.push(target.as_os_str().to_os_string());
    for a in [
        "--python-platform",
        uv_platform,
        "--python-version",
        python,
        "--only-binary",
        ":all:",
        "--require-hashes",
        "--no-deps",
        "--link-mode",
        "copy",
        "--quiet",
    ] {
        args.push(OsString::from(a));
    }
    Some(args)
}

/// Where a compiler's `uv` gets wheels.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WheelSource {
    /// The package index. `None`: `uv`'s default (PyPI).
    pub index_url: Option<String>,
    /// A folder of wheel files (`uv --find-links`), for example one baked
    /// into the compiler image.
    pub wheel_dir: Option<PathBuf>,
    /// The `uv` cache folder. `None`: `uv`'s default, under the home folder.
    pub cache_dir: Option<PathBuf>,
    /// Use no network: the wheel folder, or else the cache, must hold every
    /// wheel.
    pub offline: bool,
}

impl WheelSource {
    /// A source that cannot work, as a message; `Ok` otherwise.
    pub fn check(&self) -> Result<(), String> {
        if self.offline && self.wheel_dir.is_none() && self.cache_dir.is_none() {
            return Err(
                "an offline Python build needs a wheel folder or a uv cache folder: without \
                 one there is nowhere to get a wheel from"
                    .to_string(),
            );
        }
        if let Some(dir) = &self.wheel_dir
            && !dir.is_dir()
        {
            return Err(format!(
                "the Python wheel folder {} is not a folder",
                dir.display()
            ));
        }
        Ok(())
    }

    /// The `uv` arguments for this source. With `offline` and a wheel folder,
    /// `--no-index`: only the folder is read.
    ///
    /// `--no-config`: the build runs in the package's folder, and a `uv.toml`
    /// or a `[tool.uv]` table there must not choose the index.
    /// `--no-python-downloads`: `uv` uses the interpreter of the machine.
    pub fn args(&self) -> Vec<OsString> {
        let mut args: Vec<OsString> = vec!["--no-config".into(), "--no-python-downloads".into()];
        if let Some(dir) = &self.cache_dir {
            args.push("--cache-dir".into());
            args.push(dir.as_os_str().to_os_string());
        }
        if let Some(dir) = &self.wheel_dir {
            args.push("--find-links".into());
            args.push(dir.as_os_str().to_os_string());
        }
        if self.offline {
            args.push("--offline".into());
            if self.wheel_dir.is_some() {
                args.push("--no-index".into());
            }
        }
        args
    }

    /// The environment variables for this source: the index URL. It goes in
    /// the environment and not on the command line, because a URL of a
    /// private index can hold a password.
    pub fn env(&self) -> Vec<(&'static str, String)> {
        match (&self.index_url, self.offline && self.wheel_dir.is_some()) {
            (Some(url), false) => vec![("UV_DEFAULT_INDEX", url.clone())],
            _ => Vec::new(),
        }
    }

    /// One line that says where wheels come from, for a log or an error.
    pub fn describe(&self) -> String {
        let mut parts = Vec::new();
        if let Some(dir) = &self.wheel_dir {
            parts.push(format!("the wheel folder {}", dir.display()));
        }
        if self.offline {
            if self.wheel_dir.is_none() {
                match &self.cache_dir {
                    Some(dir) => parts.push(format!("the uv cache {}", dir.display())),
                    None => parts.push("the uv cache".to_string()),
                }
            }
            parts.push("no network".to_string());
        } else {
            parts.push(match &self.index_url {
                Some(_) => "the configured package index".to_string(),
                None => "the default package index (PyPI)".to_string(),
            });
        }
        parts.join(", ")
    }
}

/// Why a `uv pip install` failed, read from its error text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UvFailure {
    /// These distributions have no wheel for the target.
    NoWheel(Vec<String>),
    /// These distributions are not in the wheel source (the index, the wheel
    /// folder or the cache).
    NotInSource(Vec<String>),
    /// The wheel of these distributions has a hash that the lock file does
    /// not list.
    HashMismatch(Vec<String>),
    /// Something else.
    Other,
}

fn push_name(names: &mut Vec<String>, raw: &str) {
    let name = raw
        .trim_matches(|c: char| c == '`' || c == ',' || c == '.' || c == ':')
        .split(['=', '<', '>', '!', '~', '['])
        .next()
        .unwrap_or("")
        .to_string();
    if !name.is_empty() && !names.contains(&name) {
        names.push(name);
    }
}

/// The names between back quotes that follow each `marker` in `flat`.
fn quoted_after(flat: &str, marker: &str, names: &mut Vec<String>) {
    let mut rest = flat;
    while let Some(at) = rest.find(marker) {
        let tail = &rest[at + marker.len()..];
        match tail.find('`') {
            Some(end) => {
                push_name(names, &tail[..end]);
                rest = &tail[end..];
            }
            None => break,
        }
    }
}

/// Classify the error text of a failed `uv pip install`.
pub fn classify_uv_failure(stderr: &str) -> UvFailure {
    let flat = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    let words: Vec<&str> = flat.split(' ').collect();

    // "Hash mismatch for `six==1.17.0`"
    let mut hash = Vec::new();
    quoted_after(&flat, "Hash mismatch for `", &mut hash);
    if !hash.is_empty() {
        return UvFailure::HashMismatch(hash);
    }

    // "Because docopt==0.6.2 has no usable wheels and …"
    // "… pyfoo==1.0 has no wheels with a matching platform tag (…) …"
    // "hint: Wheels are required for `docopt` because building from source is disabled"
    let mut no_wheel = Vec::new();
    for (i, w) in words.iter().enumerate() {
        if *w == "has"
            && i > 0
            && words.get(i + 1) == Some(&"no")
            && matches!(words.get(i + 2), Some(&"usable") | Some(&"wheels"))
        {
            push_name(&mut no_wheel, words[i - 1]);
        }
    }
    quoted_after(&flat, "Wheels are required for `", &mut no_wheel);
    if !no_wheel.is_empty() {
        return UvFailure::NoWheel(no_wheel);
    }

    // "Because pyyaml was not found in the provided package locations and …"
    // "Because pyfoo was not found in the package registry and …"
    let mut absent = Vec::new();
    for (i, w) in words.iter().enumerate() {
        if *w == "was"
            && i > 0
            && words.get(i + 1) == Some(&"not")
            && words.get(i + 2) == Some(&"found")
            && words.get(i + 3) == Some(&"in")
        {
            push_name(&mut absent, words[i - 1]);
        }
    }
    // "Failed to download `pyyaml==6.0.2` … Network connectivity is disabled,
    //  but the requested data wasn't found in the cache for: …"
    if flat.contains("wasn't found in the cache") {
        quoted_after(&flat, "Failed to download `", &mut absent);
        quoted_after(&flat, "Failed to fetch `", &mut absent);
    }
    if !absent.is_empty() {
        return UvFailure::NotInSource(absent);
    }
    UvFailure::Other
}

fn name_list(names: &[String]) -> String {
    names
        .iter()
        .map(|n| format!("`{n}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The error for a failed `uv pip install`: it names the distributions that
/// failed and why, then gives `uv`'s own text. `source` says where the build
/// gets wheels ([`WheelSource::describe`]).
pub fn uv_install_error(stderr: &str, platform: &str, python: &str, source: &str) -> String {
    let detail = stderr.trim_end();
    match classify_uv_failure(stderr) {
        UvFailure::NoWheel(names) => format!(
            "{} {} no wheel for {platform} with Python {python}. A package holds built wheels \
             only: the build does not compile a source distribution. Pin a version that has a \
             wheel for this target, or remove the dependency.\n{detail}",
            name_list(&names),
            if names.len() == 1 { "has" } else { "have" }
        ),
        UvFailure::NotInSource(names) => format!(
            "{} {} not found for {platform} with Python {python} in the wheel source of this \
             build ({source}). Add the missing wheel to the source, or pin a version it \
             holds.\n{detail}",
            name_list(&names),
            if names.len() == 1 { "was" } else { "were" }
        ),
        UvFailure::HashMismatch(names) => format!(
            "{}: the wheel from the wheel source of this build ({source}) has a hash that {} \
             does not list. Nothing was installed.\n{detail}",
            name_list(&names),
            python_env::LOCK_FILE
        ),
        UvFailure::Other => {
            format!("vendoring for {platform} (Python {python}) failed:\n{detail}")
        }
    }
}

/// Remove what `uv pip install --target` writes that is not importable code:
/// the console scripts in `bin/`, and `uv`'s own `.lock` file.
pub fn clean_target_dir(target: &Path) {
    let _ = std::fs::remove_dir_all(target.join("bin"));
    let _ = std::fs::remove_file(target.join(".lock"));
}

/// True when a filled target folder holds compiled code: a wheel built for
/// one platform, or a compiled file.
pub fn target_dir_is_native(target: &Path) -> bool {
    python_env::scan_dists(target).iter().any(|d| d.is_native())
        || !python_env::native_files(target, 1).is_empty()
}

// ---------------------------------------------------------------------------
// The vendored set as an artifact
// ---------------------------------------------------------------------------

/// The start of the `target_triple` of a `package_artifacts` row that holds a
/// vendored set. The rest is the platform. A Rust library's row has the bare
/// triple, so the two never match.
pub const VENDOR_ARTIFACT_PREFIX: &str = "python-vendor:";

/// The `interface` of the manifest inside a vendor artifact.
pub const VENDOR_ARTIFACT_INTERFACE: &str = "cloacina-python-vendor";

/// The `target_triple` of the vendor artifact of `platform`.
pub fn vendor_artifact_key(platform: &str) -> String {
    format!("{VENDOR_ARTIFACT_PREFIX}{platform}")
}

/// The platform of a vendor artifact's `target_triple`, or `None` for a row
/// that is not a vendor artifact.
pub fn vendor_artifact_platform(target_triple: &str) -> Option<&str> {
    target_triple.strip_prefix(VENDOR_ARTIFACT_PREFIX)
}

/// The limits for unpacking a vendor artifact.
pub type UnpackLimits = fidius_core::package::UnpackOptions;

/// Pack a filled target folder as the vendor artifact of one platform.
///
/// `work` is an empty scratch folder on the same file system as `target`.
/// `target` is moved into it. The archive is written to `out`. `record` must
/// name exactly this one platform.
///
/// The archive has the format of a package archive: a `package.toml` whose
/// `[metadata]` is `record`, and `vendor/<platform>/`.
pub fn pack_vendor_artifact(
    work: &Path,
    package: &str,
    version: &str,
    record: &Vendored,
    target: &Path,
    out: &Path,
) -> Result<(), String> {
    let platform = match record.platforms.as_slice() {
        [one] => one.as_str(),
        _ => return Err("a vendor artifact holds one platform".to_string()),
    };
    let root = work.join("artifact");
    let vendor = root.join("vendor");
    std::fs::create_dir_all(&vendor).map_err(|e| format!("create {}: {e}", vendor.display()))?;
    std::fs::rename(target, vendor.join(platform))
        .map_err(|e| format!("move {} into the artifact: {e}", target.display()))?;

    #[derive(serde::Serialize)]
    struct Header<'a> {
        name: String,
        version: &'a str,
        interface: &'a str,
        interface_version: u32,
    }
    #[derive(serde::Serialize)]
    struct Manifest<'a> {
        package: Header<'a>,
        metadata: &'a Vendored,
    }
    let manifest = toml::to_string(&Manifest {
        package: Header {
            name: format!("{package}-vendor-{platform}"),
            version,
            interface: VENDOR_ARTIFACT_INTERFACE,
            interface_version: 1,
        },
        metadata: record,
    })
    .map_err(|e| format!("write the artifact manifest: {e}"))?;
    std::fs::write(root.join("package.toml"), manifest)
        .map_err(|e| format!("write the artifact manifest: {e}"))?;

    fidius_core::package::pack_package(&root, Some(out))
        .map_err(|e| format!("pack the vendored set of {platform}: {e}"))?;
    Ok(())
}

/// Check that an archive is a vendor artifact that `limits` let through, and
/// read its record. `scratch` is a folder the archive is unpacked into; the
/// caller removes it.
pub fn read_vendor_artifact(
    archive: &Path,
    scratch: &Path,
    limits: &UnpackLimits,
) -> Result<(Vendored, PathBuf), String> {
    let dir = fidius_core::package::unpack_package_with_options(archive, scratch, limits)
        .map_err(|e| format!("the vendored set cannot be unpacked: {e}"))?;
    let manifest = fidius_core::package::load_manifest::<Vendored>(&dir)
        .map_err(|e| format!("the vendored set has no readable record: {e}"))?;
    if manifest.package.interface != VENDOR_ARTIFACT_INTERFACE {
        return Err(format!(
            "the archive is not a vendored set (interface `{}`)",
            manifest.package.interface
        ));
    }
    let record = manifest.metadata;
    for platform in &record.platforms {
        // A platform name is one path segment.
        if platform.is_empty()
            || platform.starts_with('.')
            || platform.contains(['/', '\\'])
            || !dir.join("vendor").join(platform).is_dir()
        {
            return Err(format!(
                "the vendored set records the platform `{platform}` and has no such folder"
            ));
        }
    }
    Ok((record, dir))
}

/// Unpack a vendor artifact into a staged package: its `vendor/<platform>/`
/// folders are moved to `<package_root>/vendor/`. Returns the record, which
/// the loader uses as the package's `[metadata.vendored]`.
///
/// A package that ships its own `vendor/` folder gets no artifact: it is an
/// error to have both.
pub fn unpack_vendor_artifact(
    archive: &Path,
    package_root: &Path,
    scratch: &Path,
    limits: &UnpackLimits,
) -> Result<Vendored, String> {
    let vendor = package_root.join("vendor");
    if vendor.exists() {
        return Err(
            "the package ships a vendor/ folder and the registry also holds a vendored set \
             for it"
                .to_string(),
        );
    }
    let (record, dir) = read_vendor_artifact(archive, scratch, limits)?;
    std::fs::create_dir_all(&vendor).map_err(|e| format!("create {}: {e}", vendor.display()))?;
    for platform in &record.platforms {
        let from = dir.join("vendor").join(platform);
        let to = vendor.join(platform);
        if std::fs::rename(&from, &to).is_err() {
            // Another file system: copy.
            copy_dir(&from, &to)?;
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    Ok(record)
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| format!("create {}: {e}", to.display()))?;
    let entries = std::fs::read_dir(from).map_err(|e| format!("read {}: {e}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("read {}: {e}", from.display()))?;
        let (src, dst) = (entry.path(), to.join(entry.file_name()));
        if src.is_dir() {
            copy_dir(&src, &dst)?;
        } else {
            std::fs::copy(&src, &dst).map_err(|e| format!("copy {}: {e}", src.display()))?;
        }
    }
    Ok(())
}

/// The total size and the number of files of a folder.
pub fn dir_size_and_files(path: &Path) -> (u64, u64) {
    let mut total = (0, 0);
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            match entry.metadata() {
                Ok(m) if m.is_dir() => {
                    let (bytes, files) = dir_size_and_files(&entry.path());
                    total.0 += bytes;
                    total.1 += files;
                }
                Ok(m) => {
                    total.0 += m.len();
                    total.1 += 1;
                }
                Err(_) => {}
            }
        }
    }
    total
}

/// True when the text of a lock file pins at least one distribution. A file
/// that does not pass [`check_lock_file`] counts as one that does.
pub fn lock_has_pins(lock: &str) -> bool {
    check_lock_file(lock).map(|p| !p.is_empty()).unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const HASH: &str = "4721f391ed90541fddacab5acf947aa0d3dc7d27b2e1e8eda2be8970586c3274";

    fn uv_lock() -> String {
        format!(
            "# This file was autogenerated by uv via the following command:\n\
             #    cloacinactl package build\n\
             colorama==0.4.6 ; sys_platform == 'win32' \\\n\
             \x20   --hash=sha256:{HASH}\n\
             PyYAML==6.0.2 \\\n\
             \x20   --hash=sha256:{HASH} \\\n\
             \x20   --hash=sha256:{HASH}\n\
             \x20   # via demo (pyproject.toml)\n\
             six==1.17.0 --hash=sha256:{HASH}\n"
        )
    }

    #[test]
    fn lock_written_by_uv_is_read() {
        let pins = check_lock_file(&uv_lock()).unwrap();
        let names: Vec<(&str, &str, usize)> = pins
            .iter()
            .map(|p| (p.name.as_str(), p.version.as_str(), p.line))
            .collect();
        assert_eq!(
            names,
            vec![
                ("colorama", "0.4.6", 3),
                ("pyyaml", "6.0.2", 5),
                ("six", "1.17.0", 9)
            ]
        );
        assert!(check_lock_file("").unwrap().is_empty());
        assert!(check_lock_file("# nothing\n\n").unwrap().is_empty());
    }

    #[test]
    fn lock_with_an_option_line_is_refused_with_its_line() {
        for (line, option) in [
            ("-r /proc/1/environ", "-r"),
            ("--requirement other.txt", "--requirement"),
            ("--index-url https://example.invalid/simple", "--index-url"),
            (
                "--extra-index-url=https://example.invalid/simple",
                "--extra-index-url",
            ),
            ("--find-links /srv/wheels", "--find-links"),
            ("-e .", "-e"),
            ("--no-binary :all:", "--no-binary"),
            ("-c constraints.txt", "-c"),
        ] {
            let text = format!("six==1.17.0 --hash=sha256:{HASH}\n{line}\n");
            let err = check_lock_file(&text).unwrap_err();
            assert!(err.contains("line 2"), "{err}");
            assert!(err.contains(&format!("`{option}`")), "{err}");
            assert!(err.contains("cloacinactl package build"), "{err}");
        }
    }

    #[test]
    fn lock_with_a_url_a_path_or_a_range_is_refused() {
        for line in [
            "six @ https://example.invalid/six-1.17.0-py3-none-any.whl",
            "https://example.invalid/six-1.17.0-py3-none-any.whl",
            "./wheels/six-1.17.0-py3-none-any.whl",
            "/srv/wheels/six-1.17.0-py3-none-any.whl",
            "file:///etc/passwd",
            "six>=1.0",
            "six",
            "six==1.17.0 ; extra == 'x' @ https://example.invalid/x.whl",
            "six[extra]==1.17.0",
            "six==1.*",
        ] {
            let text = format!("{line} --hash=sha256:{HASH}\n");
            // The marker case is a valid pin; every other line is refused.
            let result = check_lock_file(&text);
            if line.starts_with("six==1.17.0 ;") {
                assert!(result.is_ok(), "{line}");
            } else {
                let err = result.unwrap_err();
                assert!(err.contains("line 1"), "{line}: {err}");
                assert!(err.contains("not a pinned version"), "{line}: {err}");
            }
        }
    }

    #[test]
    fn lock_pin_without_a_hash_and_a_bad_hash_are_refused() {
        let err = check_lock_file(&format!(
            "six==1.17.0 --hash=sha256:{HASH}\npyyaml==6.0.2\n"
        ))
        .unwrap_err();
        assert!(err.contains("line 2"), "{err}");
        assert!(err.contains("`pyyaml==6.0.2` has no `--hash`"), "{err}");

        let err = check_lock_file("six==1.17.0 --hash=md5:abcd\n").unwrap_err();
        assert!(
            err.contains("is not a `--hash=sha256:<hex>` value"),
            "{err}"
        );

        let err = check_lock_file(&format!("--hash=sha256:{HASH}\n")).unwrap_err();
        assert!(err.contains("no pinned version above it"), "{err}");
    }

    #[test]
    fn names_are_compared_normalized() {
        assert_eq!(normalize_name("PyYAML"), "pyyaml");
        assert_eq!(normalize_name("typing_extensions"), "typing-extensions");
        assert_eq!(normalize_name("ruamel.yaml"), "ruamel-yaml");
        assert_eq!(normalize_name("a-_.b"), "a-b");

        let pins = check_lock_file(&uv_lock()).unwrap();
        let deps = vec![
            "pyyaml>=6".to_string(),
            "Six".to_string(),
            "requests[socks]>=2.31".to_string(),
            "typing_extensions".to_string(),
            "tomli ; python_version < '3.11'".to_string(),
        ];
        assert_eq!(
            dependencies_not_locked(&deps, &pins),
            vec!["requests".to_string(), "typing-extensions".to_string()]
        );
    }

    #[test]
    fn pyproject_dependencies_are_read() {
        assert_eq!(
            pyproject_dependencies(
                "[project]\nname = \"x\"\ndependencies = [\"requests>=2.31\", \" pyyaml \", \"\"]\n"
            )
            .unwrap(),
            vec!["requests>=2.31".to_string(), "pyyaml".to_string()]
        );
        assert!(
            pyproject_dependencies("[project]\nname = \"x\"\n")
                .unwrap()
                .is_empty()
        );
        assert!(
            pyproject_dependencies("[build-system]\n")
                .unwrap()
                .is_empty()
        );
        assert!(pyproject_dependencies("[project]\ndependencies = \"requests\"\n").is_err());
        assert!(pyproject_dependencies("[project]\ndependencies = [1]\n").is_err());
        assert!(pyproject_dependencies("not toml [").is_err());
    }

    fn package(dir: &Path, deps: Option<&str>, lock: Option<&str>, vendor: bool) {
        fs::create_dir_all(dir.join("workflow")).unwrap();
        if let Some(deps) = deps {
            fs::write(
                dir.join("pyproject.toml"),
                format!("[project]\nname = \"etl\"\ndependencies = [{deps}]\n"),
            )
            .unwrap();
        }
        if let Some(lock) = lock {
            fs::write(dir.join(python_env::LOCK_FILE), lock).unwrap();
        }
        if vendor {
            fs::create_dir_all(dir.join("vendor")).unwrap();
        }
    }

    #[test]
    fn plan_a_package_with_a_vendor_folder_is_prebuilt() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Even with a lock file that would be refused: nothing is read.
        package(tmp.path(), Some("\"six\""), Some("-r /etc/passwd\n"), true);
        assert_eq!(
            vendor_plan("etl", tmp.path()).unwrap(),
            VendorPlan::Prebuilt
        );
    }

    #[test]
    fn plan_no_dependencies_is_nothing_to_vendor() {
        let tmp = tempfile::TempDir::new().unwrap();
        package(tmp.path(), None, None, false);
        assert_eq!(
            vendor_plan("etl", tmp.path()).unwrap(),
            VendorPlan::NoDependencies
        );
        package(tmp.path(), Some(""), Some("# empty\n"), false);
        assert_eq!(
            vendor_plan("etl", tmp.path()).unwrap(),
            VendorPlan::NoDependencies
        );
    }

    #[test]
    fn plan_dependencies_with_no_lock_file_are_refused() {
        let tmp = tempfile::TempDir::new().unwrap();
        package(tmp.path(), Some("\"six>=1\", \"pyyaml\""), None, false);
        let err = vendor_plan("etl", tmp.path()).unwrap_err();
        assert!(err.contains("package `etl`"), "{err}");
        assert!(err.contains("six>=1, pyyaml"), "{err}");
        assert!(err.contains("has no requirements.lock"), "{err}");
        assert!(err.contains("cloacinactl package build"), "{err}");
    }

    #[test]
    fn plan_a_lock_file_gives_its_pins() {
        let tmp = tempfile::TempDir::new().unwrap();
        package(
            tmp.path(),
            Some("\"six>=1\", \"pyyaml\""),
            Some(&uv_lock()),
            false,
        );
        match vendor_plan("etl", tmp.path()).unwrap() {
            VendorPlan::FromLock(pins) => assert_eq!(pins.len(), 3),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn plan_a_lock_older_than_pyproject_is_refused() {
        let tmp = tempfile::TempDir::new().unwrap();
        package(
            tmp.path(),
            Some("\"six\", \"requests\""),
            Some(&uv_lock()),
            false,
        );
        let err = vendor_plan("etl", tmp.path()).unwrap_err();
        assert!(err.contains("does not pin `requests`"), "{err}");
        assert!(
            err.contains("run `cloacinactl package build` again"),
            "{err}"
        );
    }

    #[test]
    fn plan_a_lock_with_an_option_is_refused() {
        let tmp = tempfile::TempDir::new().unwrap();
        package(
            tmp.path(),
            Some("\"six\""),
            Some("-r /proc/1/environ\n"),
            false,
        );
        let err = vendor_plan("etl", tmp.path()).unwrap_err();
        assert!(err.contains("package `etl`"), "{err}");
        assert!(err.contains("the option `-r` is not allowed"), "{err}");
    }

    fn strings(args: &[OsString]) -> Vec<String> {
        args.iter()
            .map(|a| a.to_string_lossy().to_string())
            .collect()
    }

    #[test]
    fn install_arguments_pin_the_target_and_refuse_builds() {
        let args = strings(
            &uv_install_args(
                "requirements.lock",
                Path::new("/t/x"),
                "aarch64-linux",
                "3.11",
            )
            .unwrap(),
        );
        assert_eq!(
            args,
            vec![
                "pip",
                "install",
                "--requirement",
                "requirements.lock",
                "--target",
                "/t/x",
                "--python-platform",
                "aarch64-unknown-linux-gnu",
                "--python-version",
                "3.11",
                "--only-binary",
                ":all:",
                "--require-hashes",
                "--no-deps",
                "--link-mode",
                "copy",
                "--quiet",
            ]
        );
        assert!(uv_install_args("l", Path::new("/t"), "sparc-solaris", "3.11").is_none());
    }

    #[test]
    fn wheel_source_arguments() {
        let online = WheelSource::default();
        assert_eq!(
            strings(&online.args()),
            vec!["--no-config", "--no-python-downloads"]
        );
        assert!(online.env().is_empty());
        assert!(online.check().is_ok());
        assert_eq!(online.describe(), "the default package index (PyPI)");

        let mirror = WheelSource {
            index_url: Some("https://user:secret@mirror.example.invalid/simple".to_string()),
            ..WheelSource::default()
        };
        // The URL is in the environment, never in the arguments or the text.
        assert!(!strings(&mirror.args()).iter().any(|a| a.contains("secret")));
        assert_eq!(
            mirror.env(),
            vec![(
                "UV_DEFAULT_INDEX",
                "https://user:secret@mirror.example.invalid/simple".to_string()
            )]
        );
        assert!(!mirror.describe().contains("secret"));

        let tmp = tempfile::TempDir::new().unwrap();
        let baked = WheelSource {
            index_url: Some("https://mirror.example.invalid/simple".to_string()),
            wheel_dir: Some(tmp.path().to_path_buf()),
            cache_dir: Some(PathBuf::from("/var/cache/uv")),
            offline: true,
        };
        let dir = tmp.path().to_string_lossy().to_string();
        assert_eq!(
            strings(&baked.args()),
            vec![
                "--no-config",
                "--no-python-downloads",
                "--cache-dir",
                "/var/cache/uv",
                "--find-links",
                dir.as_str(),
                "--offline",
                "--no-index",
            ]
        );
        // Offline from a folder: the index is not used at all.
        assert!(baked.env().is_empty());
        assert!(baked.check().is_ok());
        assert!(baked.describe().contains("no network"));

        let cache_only = WheelSource {
            cache_dir: Some(PathBuf::from("/var/cache/uv")),
            offline: true,
            ..WheelSource::default()
        };
        assert_eq!(
            strings(&cache_only.args()),
            vec![
                "--no-config",
                "--no-python-downloads",
                "--cache-dir",
                "/var/cache/uv",
                "--offline"
            ]
        );
        assert!(cache_only.check().is_ok());
    }

    #[test]
    fn wheel_source_that_cannot_work_is_refused() {
        let nowhere = WheelSource {
            offline: true,
            ..WheelSource::default()
        };
        assert!(
            nowhere
                .check()
                .unwrap_err()
                .contains("wheel folder or a uv cache")
        );
        let missing = WheelSource {
            wheel_dir: Some(PathBuf::from("/nonexistent/wheels")),
            ..WheelSource::default()
        };
        assert!(missing.check().unwrap_err().contains("is not a folder"));
    }

    // The texts below are what uv 0.9.21 printed.

    #[test]
    fn failure_no_wheel_is_named() {
        let text = "  × No solution found when resolving dependencies:\n  ╰─▶ Because \
                    pyyaml==6.0.2 has no wheels with a matching platform tag (e.g.,\n      \
                    `manylinux_2_28_aarch64`) and you require pyyaml==6.0.2, we can conclude\n";
        assert_eq!(
            classify_uv_failure(text),
            UvFailure::NoWheel(vec!["pyyaml".to_string()])
        );
        let text = "Because docopt==0.6.2 has no usable wheels and you require docopt==0.6.2, we \
                    can conclude\nhint: Wheels are required for `docopt` because building from \
                    source is disabled";
        assert_eq!(
            classify_uv_failure(text),
            UvFailure::NoWheel(vec!["docopt".to_string()])
        );
        let message = uv_install_error(text, "aarch64-linux", "3.11", "x");
        assert!(message.contains("`docopt` has no wheel for aarch64-linux with Python 3.11"));
        assert!(message.contains("does not compile a source distribution"));
    }

    #[test]
    fn failure_not_in_the_wheel_folder_is_named() {
        let text = "  × No solution found when resolving dependencies:\n  ╰─▶ Because pyyaml was \
                    not found in the provided package locations and\n      you require \
                    pyyaml==6.0.2, we can conclude that your requirements are\n";
        assert_eq!(
            classify_uv_failure(text),
            UvFailure::NotInSource(vec!["pyyaml".to_string()])
        );
        let message = uv_install_error(
            text,
            "x86_64-linux",
            "3.11",
            "the wheel folder /opt/wheels, no network",
        );
        assert!(
            message.starts_with("`pyyaml` was not found for x86_64-linux"),
            "{message}"
        );
        assert!(
            message.contains("the wheel folder /opt/wheels, no network"),
            "{message}"
        );
    }

    #[test]
    fn failure_not_in_the_cache_is_named() {
        let text = "  × Failed to download `pyyaml==6.0.2`\n  ╰─▶ Network connectivity is \
                    disabled, but the\n      requested data wasn't found in the cache for:\n      \
                    `https://files.example.invalid/PyYAML-6.0.2-cp311-cp311-manylinux.whl`\n";
        assert_eq!(
            classify_uv_failure(text),
            UvFailure::NotInSource(vec!["pyyaml".to_string()])
        );
        let text = "  × No solution found when resolving dependencies:\n  ╰─▶ Because idna was \
                    not found in the cache and you require idna==3.10, we\n      can conclude \
                    that your requirements are unsatisfiable.\n";
        assert_eq!(
            classify_uv_failure(text),
            UvFailure::NotInSource(vec!["idna".to_string()])
        );
    }

    #[test]
    fn failure_hash_mismatch_is_named() {
        let text = "  × Failed to download `six==1.17.0`\n  ╰─▶ Hash mismatch for \
                    `six==1.17.0`\n\n      Expected:\n        sha256:00\n\n      Computed:\n        \
                    sha256:47\n";
        assert_eq!(
            classify_uv_failure(text),
            UvFailure::HashMismatch(vec!["six".to_string()])
        );
        let message = uv_install_error(text, "x86_64-linux", "3.11", "the default package index");
        assert!(message.starts_with("`six`: the wheel"), "{message}");
        assert!(
            message.contains("requirements.lock does not list"),
            "{message}"
        );
        assert!(message.contains("Nothing was installed"), "{message}");
    }

    #[test]
    fn failure_of_another_kind_keeps_the_text() {
        assert_eq!(
            classify_uv_failure("error: network unreachable"),
            UvFailure::Other
        );
        let message = uv_install_error("error: network unreachable", "x86_64-linux", "3.11", "x");
        assert!(message.contains("vendoring for x86_64-linux (Python 3.11) failed"));
        assert!(message.contains("network unreachable"));
    }

    fn fill_target(dir: &Path, native: bool) {
        fs::create_dir_all(dir.join("six-1.17.0.dist-info")).unwrap();
        fs::create_dir_all(dir.join("bin")).unwrap();
        fs::write(dir.join("bin/tool"), "#!/bin/sh\n").unwrap();
        fs::write(dir.join(".lock"), "").unwrap();
        fs::write(dir.join("six.py"), "VALUE = 1\n").unwrap();
        fs::write(
            dir.join("six-1.17.0.dist-info/METADATA"),
            "Name: six\nVersion: 1.17.0\n",
        )
        .unwrap();
        fs::write(
            dir.join("six-1.17.0.dist-info/WHEEL"),
            "Tag: py3-none-any\n",
        )
        .unwrap();
        if native {
            fs::write(dir.join("_six.cpython-311-x86_64-linux-gnu.so"), "").unwrap();
        }
    }

    fn record(platform: &str, native: bool) -> Vendored {
        Vendored {
            python_version: "3.11".to_string(),
            platforms: vec![platform.to_string()],
            native,
            lock_file: Some(python_env::LOCK_FILE.to_string()),
        }
    }

    #[test]
    fn target_folder_is_cleaned_and_scanned() {
        let tmp = tempfile::TempDir::new().unwrap();
        fill_target(tmp.path(), false);
        clean_target_dir(tmp.path());
        assert!(!tmp.path().join("bin").exists());
        assert!(!tmp.path().join(".lock").exists());
        assert!(!target_dir_is_native(tmp.path()));
        fill_target(tmp.path(), true);
        assert!(target_dir_is_native(tmp.path()));
    }

    #[test]
    fn artifact_key_is_not_a_bare_platform() {
        assert_eq!(
            vendor_artifact_key("x86_64-linux"),
            "python-vendor:x86_64-linux"
        );
        assert_eq!(
            vendor_artifact_platform("python-vendor:x86_64-linux"),
            Some("x86_64-linux")
        );
        assert_eq!(vendor_artifact_platform("x86_64-linux"), None);
    }

    #[test]
    fn artifact_round_trips_into_a_staged_package() {
        let tmp = tempfile::TempDir::new().unwrap();
        let target = tmp.path().join("target-x");
        fill_target(&target, true);
        clean_target_dir(&target);
        let work = tmp.path().join("work");
        fs::create_dir_all(&work).unwrap();
        let out = tmp.path().join("set.bin");
        pack_vendor_artifact(
            &work,
            "etl",
            "1.2.0",
            &record("x86_64-linux", true),
            &target,
            &out,
        )
        .unwrap();
        assert!(out.metadata().unwrap().len() > 0);

        let root = tmp.path().join("staged/etl-1.2.0");
        fs::create_dir_all(root.join("workflow")).unwrap();
        let scratch = tmp.path().join("scratch");
        fs::create_dir_all(&scratch).unwrap();
        let got = unpack_vendor_artifact(&out, &root, &scratch, &UnpackLimits::default()).unwrap();
        assert_eq!(got, record("x86_64-linux", true));
        assert!(root.join("vendor/x86_64-linux/six.py").is_file());
        assert!(
            root.join("vendor/x86_64-linux/_six.cpython-311-x86_64-linux-gnu.so")
                .is_file()
        );
        // The loader's own choice finds the folder.
        assert_eq!(
            python_env::select_vendor_dir("etl", &root, Some(&got), "x86_64-linux").unwrap(),
            Some(root.join("vendor/x86_64-linux"))
        );
    }

    #[test]
    fn artifact_is_refused_for_a_package_with_its_own_vendor_folder() {
        let tmp = tempfile::TempDir::new().unwrap();
        let target = tmp.path().join("t");
        fill_target(&target, false);
        let work = tmp.path().join("work");
        fs::create_dir_all(&work).unwrap();
        let out = tmp.path().join("set.bin");
        pack_vendor_artifact(
            &work,
            "etl",
            "1.0.0",
            &record("x86_64-linux", false),
            &target,
            &out,
        )
        .unwrap();
        let root = tmp.path().join("staged");
        fs::create_dir_all(root.join("vendor")).unwrap();
        let err =
            unpack_vendor_artifact(&out, &root, tmp.path(), &UnpackLimits::default()).unwrap_err();
        assert!(err.contains("ships a vendor/ folder"), "{err}");
    }

    #[test]
    fn artifact_over_the_unpack_limit_is_refused() {
        let tmp = tempfile::TempDir::new().unwrap();
        let target = tmp.path().join("t");
        fill_target(&target, false);
        fs::write(target.join("big.py"), vec![b'#'; 4096]).unwrap();
        let work = tmp.path().join("work");
        fs::create_dir_all(&work).unwrap();
        let out = tmp.path().join("set.bin");
        pack_vendor_artifact(
            &work,
            "etl",
            "1.0.0",
            &record("x86_64-linux", false),
            &target,
            &out,
        )
        .unwrap();
        let limits = UnpackLimits {
            max_decompressed: 1024,
            ..UnpackLimits::default()
        };
        let err = read_vendor_artifact(&out, tmp.path(), &limits).unwrap_err();
        assert!(err.contains("exceeds limit"), "{err}");
    }

    #[test]
    fn an_archive_that_is_not_a_vendored_set_is_refused() {
        let tmp = tempfile::TempDir::new().unwrap();
        let pkg = tmp.path().join("pkg");
        fs::create_dir_all(pkg.join("vendor/x86_64-linux")).unwrap();
        fs::write(pkg.join("vendor/x86_64-linux/a.py"), "").unwrap();
        fs::write(
            pkg.join("package.toml"),
            "[package]\nname = \"p\"\nversion = \"1\"\ninterface = \"other\"\n\
             interface_version = 1\n\n[metadata]\npython_version = \"3.11\"\n\
             platforms = [\"x86_64-linux\"]\n",
        )
        .unwrap();
        let out = tmp.path().join("p.bin");
        fidius_core::package::pack_package(&pkg, Some(&out)).unwrap();
        let scratch = tmp.path().join("s");
        fs::create_dir_all(&scratch).unwrap();
        let err = read_vendor_artifact(&out, &scratch, &UnpackLimits::default()).unwrap_err();
        assert!(err.contains("not a vendored set"), "{err}");
    }

    #[test]
    fn size_and_files_of_a_folder() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("a/b")).unwrap();
        fs::write(tmp.path().join("a/x"), "12345").unwrap();
        fs::write(tmp.path().join("a/b/y"), "123").unwrap();
        assert_eq!(dir_size_and_files(tmp.path()), (8, 2));
    }

    #[test]
    fn lock_has_pins_reads_the_file() {
        assert!(!lock_has_pins("# nothing\n"));
        assert!(lock_has_pins(&uv_lock()));
        assert!(lock_has_pins("-r other.txt\n"));
    }
}
