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

//! The Python environment of a package (CLOACINA-T-1048): the platforms its
//! vendored dependencies were built for, the Python version, and the checks
//! that `cloacinactl`, the server and the loader share.
//!
//! All of it is pure: it reads `package.toml` values and the files of a
//! `vendor/` folder. It runs no Python.
//!
//! A platform is named `<arch>-<os>`, as an agent reports its own
//! (`x86_64-linux`, `aarch64-linux`, `aarch64-macos`). `cloacinactl package
//! build` writes one folder per platform, `vendor/<platform>/`, and records
//! them in `[metadata.vendored]`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The Python version of the official agent image (`docker/Dockerfile.agent`).
pub const DEFAULT_PYTHON_VERSION: &str = "3.11";

/// The platforms the official agent image is built for.
pub const DEFAULT_PLATFORMS: [&str; 2] = ["aarch64-linux", "x86_64-linux"];

/// The lock file `cloacinactl package build` writes in the package root.
pub const LOCK_FILE: &str = "requirements.lock";

/// `[metadata.vendored]` of `package.toml`: what `cloacinactl package build`
/// put in `vendor/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vendored {
    /// The Python version the dependencies were resolved for (`"3.11"`).
    pub python_version: String,
    /// The platforms that have a folder `vendor/<platform>/`.
    pub platforms: Vec<String>,
    /// True when a folder holds compiled code (a wheel built for one
    /// platform). False when every dependency is pure Python.
    #[serde(default)]
    pub native: bool,
    /// The lock file with the pinned versions and hashes, relative to the
    /// package root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lock_file: Option<String>,
}

impl Vendored {
    /// The platforms this package can run on. `None` means any platform:
    /// every vendored dependency is pure Python.
    pub fn runnable_platforms(&self) -> Option<Vec<String>> {
        if self.native {
            Some(self.platforms.clone())
        } else {
            None
        }
    }
}

/// The platforms a package can run on, from its `[metadata.vendored]` record.
/// `None` means any platform: there is no record (nothing vendored by the
/// build), or the vendored dependencies are pure Python.
pub fn runnable_platforms(vendored: Option<&Vendored>) -> Option<Vec<String>> {
    vendored.and_then(Vendored::runnable_platforms)
}

/// The platform of this process, `<arch>-<os>`.
pub fn host_platform() -> String {
    format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS)
}

/// The value of `uv --python-platform` for a platform, or `None` when the
/// platform is not one the build can vendor for.
pub fn uv_platform(platform: &str) -> Option<&'static str> {
    match platform {
        "x86_64-linux" => Some("x86_64-unknown-linux-gnu"),
        "aarch64-linux" => Some("aarch64-unknown-linux-gnu"),
        "x86_64-macos" => Some("x86_64-apple-darwin"),
        "aarch64-macos" => Some("aarch64-apple-darwin"),
        "x86_64-windows" => Some("x86_64-pc-windows-msvc"),
        _ => None,
    }
}

/// The platforms the build can vendor for.
pub const SUPPORTED_PLATFORMS: [&str; 5] = [
    "aarch64-linux",
    "x86_64-linux",
    "aarch64-macos",
    "x86_64-macos",
    "x86_64-windows",
];

// ---------------------------------------------------------------------------
// Python versions and `requires_python`
// ---------------------------------------------------------------------------

/// The numeric release of a version string: `"3.11.9"` → `[3, 11, 9]`,
/// `"3.13.0rc1"` → `[3, 13, 0]`.
pub fn parse_version(text: &str) -> Result<Vec<u64>, String> {
    let text = text.trim();
    let mut out = Vec::new();
    for part in text.split('.') {
        let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return Err(format!("\"{text}\" is not a version"));
        }
        out.push(
            digits
                .parse::<u64>()
                .map_err(|_| format!("\"{text}\" is not a version"))?,
        );
        if digits.len() != part.len() {
            // A suffix such as `rc1` ends the release part.
            break;
        }
    }
    if out.is_empty() {
        return Err(format!("\"{text}\" is not a version"));
    }
    Ok(out)
}

/// `major.minor` of a version string.
pub fn major_minor(text: &str) -> Result<(u64, u64), String> {
    let v = parse_version(text)?;
    match v.as_slice() {
        [major, minor, ..] => Ok((*major, *minor)),
        _ => Err(format!(
            "\"{}\" is not a Python version: give major.minor, as in 3.11",
            text.trim()
        )),
    }
}

fn cmp_release(a: &[u64], b: &[u64]) -> std::cmp::Ordering {
    let n = a.len().max(b.len());
    for i in 0..n {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

/// Whether a `requires_python` specifier (`">=3.10"`, `">=3.10,<3.13"`,
/// `"~=3.11"`, `"==3.11.*"`) allows `version`.
///
/// `version` is a full version (`3.11.9`) or `major.minor` (`3.11`). With
/// `major.minor` the patch number is not known: a bound with a patch number
/// is compared on `major.minor` only, so `>=3.11.4` allows `3.11`.
pub fn requires_python_allows(spec: &str, version: &str) -> Result<bool, String> {
    use std::cmp::Ordering::*;

    let candidate = parse_version(version)?;
    let coarse = candidate.len() <= 2;

    for clause in spec.split(',') {
        let clause = clause.trim();
        if clause.is_empty() {
            continue;
        }
        let (op, rest) = ["===", "~=", "==", "!=", ">=", "<=", ">", "<"]
            .iter()
            .find_map(|op| clause.strip_prefix(op).map(|rest| (*op, rest.trim())))
            .ok_or_else(|| {
                format!("requires_python \"{spec}\": \"{clause}\" has no comparison operator")
            })?;
        let (rest, wildcard) = match rest.strip_suffix(".*") {
            Some(r) => (r, true),
            None => (rest, false),
        };
        let mut bound =
            parse_version(rest).map_err(|e| format!("requires_python \"{spec}\": {e}"))?;
        let bound_len = bound.len();
        let bound_was_fine = bound_len > 2;
        if coarse && bound_was_fine {
            bound.truncate(2);
        }
        let prefix_eq =
            |n: usize| (0..n).all(|i| candidate.get(i).copied().unwrap_or(0) == bound[i]);
        let ord = cmp_release(&candidate, &bound);

        let ok = match op {
            "==" | "===" => {
                if wildcard {
                    prefix_eq(bound.len())
                } else {
                    ord == Equal
                }
            }
            "!=" => {
                if coarse && bound_was_fine {
                    true
                } else if wildcard {
                    !prefix_eq(bound.len())
                } else {
                    ord != Equal
                }
            }
            ">=" => ord != Less,
            "<=" => ord != Greater,
            // With major.minor only, a later patch release can be above the bound.
            ">" => {
                if coarse {
                    ord != Less
                } else {
                    ord == Greater
                }
            }
            "<" => {
                if coarse && bound_was_fine {
                    ord != Greater
                } else {
                    ord == Less
                }
            }
            "~=" => {
                if bound_len < 2 {
                    return Err(format!(
                        "requires_python \"{spec}\": \"~=\" needs at least two version numbers"
                    ));
                }
                // `~=3.11.2` is `>=3.11.2, ==3.11.*`; `~=3.11` is `>=3.11, ==3.*`.
                ord != Less && prefix_eq((bound_len - 1).min(bound.len()))
            }
            _ => unreachable!(),
        };
        if !ok {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Check a Python package against the Python that will run it.
///
/// - `requires_python` must allow `python`.
/// - The Python version the dependencies were vendored for must be the same
///   `major.minor` as `python`.
///
/// `python` is a full version or `major.minor`. `runner` names what runs it
/// in the error (`"this server"`, `"this agent"`, `"the build target"`).
pub fn check_python(
    package: &str,
    requires_python: Option<&str>,
    vendored: Option<&Vendored>,
    python: &str,
    runner: &str,
) -> Result<(), String> {
    if let Some(spec) = requires_python
        && !requires_python_allows(spec, python)?
    {
        return Err(format!(
            "package `{package}` requires Python {spec} (requires_python), and {runner} \
             runs Python {python}"
        ));
    }
    if let Some(v) = vendored {
        let recorded = major_minor(&v.python_version)
            .map_err(|e| format!("package `{package}`: [metadata.vendored].python_version: {e}"))?;
        let running = major_minor(python)?;
        if recorded != running {
            return Err(format!(
                "package `{package}` has dependencies vendored for Python {}, and {runner} runs \
                 Python {python}. Build it again with `cloacinactl package build \
                 --python-version {}.{}`",
                v.python_version, running.0, running.1
            ));
        }
        if let Some(spec) = requires_python
            && !requires_python_allows(spec, &v.python_version)?
        {
            return Err(format!(
                "package `{package}` requires Python {spec} (requires_python), and its \
                 dependencies are vendored for Python {}",
                v.python_version
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The files of a vendor folder
// ---------------------------------------------------------------------------

/// One installed distribution of a vendor folder, read from its `.dist-info`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendoredDist {
    /// The distribution name (`pyarrow`).
    pub name: String,
    /// Its version (`18.1.0`).
    pub version: String,
    /// The platform part of each wheel tag (`any`, `manylinux_2_28_x86_64`).
    pub platform_tags: Vec<String>,
    /// The importable top-level modules it installed. A top-level folder with
    /// no `__init__.py` (a namespace package, shared between distributions) is
    /// not listed.
    pub top_level: Vec<String>,
}

impl VendoredDist {
    /// True when the wheel holds compiled code for one platform.
    pub fn is_native(&self) -> bool {
        self.platform_tags.iter().any(|t| t != "any")
    }
}

/// Whether a wheel platform tag (`manylinux_2_28_x86_64`, `macosx_11_0_arm64`,
/// `win_amd64`, `any`) runs on `platform` (`x86_64-linux`).
pub fn wheel_tag_runs_on(tag: &str, platform: &str) -> bool {
    if tag == "any" {
        return true;
    }
    let (arch, os) = match platform.split_once('-') {
        Some(p) => p,
        None => return false,
    };
    let os_ok = match os {
        "linux" => tag.contains("linux"),
        "macos" => tag.starts_with("macosx"),
        "windows" => tag.starts_with("win"),
        _ => false,
    };
    let arch_ok = match arch {
        "x86_64" => {
            tag.ends_with("x86_64")
                || tag.ends_with("amd64")
                || tag.ends_with("universal2")
                || tag.ends_with("intel")
        }
        "aarch64" => {
            tag.ends_with("aarch64") || tag.ends_with("arm64") || tag.ends_with("universal2")
        }
        _ => false,
    };
    os_ok && arch_ok
}

/// Read the distributions of a vendor folder (each `*.dist-info` in it).
/// A folder that is missing or has no `.dist-info` gives an empty list.
pub fn scan_dists(dir: &Path) -> Vec<VendoredDist> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return out,
    };
    let mut infos: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .map(|n| n.to_string_lossy().ends_with(".dist-info"))
                    .unwrap_or(false)
        })
        .collect();
    infos.sort();

    for info in infos {
        let metadata = std::fs::read_to_string(info.join("METADATA")).unwrap_or_default();
        let mut name = String::new();
        let mut version = String::new();
        for line in metadata.lines() {
            if line.is_empty() {
                break; // end of the headers
            }
            if let Some(v) = line.strip_prefix("Name:") {
                name = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("Version:") {
                version = v.trim().to_string();
            }
        }
        if name.is_empty() {
            // `name-1.2.3.dist-info`
            let stem = info
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let stem = stem.trim_end_matches(".dist-info");
            match stem.rsplit_once('-') {
                Some((n, v)) => {
                    name = n.to_string();
                    if version.is_empty() {
                        version = v.to_string();
                    }
                }
                None => name = stem.to_string(),
            }
        }

        let wheel = std::fs::read_to_string(info.join("WHEEL")).unwrap_or_default();
        let mut platform_tags: Vec<String> = wheel
            .lines()
            .filter_map(|l| l.strip_prefix("Tag:"))
            .filter_map(|t| t.trim().splitn(3, '-').nth(2).map(str::to_string))
            // A compressed tag set: `manylinux_2_17_x86_64.manylinux2014_x86_64`.
            .flat_map(|t| t.split('.').map(str::to_string).collect::<Vec<_>>())
            .collect();
        platform_tags.sort();
        platform_tags.dedup();

        let record = std::fs::read_to_string(info.join("RECORD")).unwrap_or_default();
        let mut top_level: Vec<String> = Vec::new();
        for line in record.lines() {
            let path = line
                .split(',')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"');
            let mut parts = path.split('/');
            let first = match parts.next() {
                Some(f) if !f.is_empty() && f != ".." => f,
                _ => continue,
            };
            let nested = parts.next().is_some();
            let module = if nested {
                if first.ends_with(".dist-info") || first.ends_with(".data") || first == "bin" {
                    continue;
                }
                // A namespace package has no `__init__.py`: more than one
                // distribution can put files in it.
                if !dir.join(first).join("__init__.py").exists() {
                    continue;
                }
                first.to_string()
            } else {
                match module_of_file(first) {
                    Some(m) => m,
                    None => continue,
                }
            };
            if !top_level.contains(&module) {
                top_level.push(module);
            }
        }
        top_level.sort();

        out.push(VendoredDist {
            name,
            version,
            platform_tags,
            top_level,
        });
    }
    out
}

/// The module name of a top-level file: `six.py` → `six`,
/// `_cffi.cpython-311-x86_64-linux-gnu.so` → `_cffi`. `None` for other files.
fn module_of_file(file: &str) -> Option<String> {
    let (stem, ext) = file.rsplit_once('.')?;
    match ext {
        "py" => Some(stem.to_string()),
        "so" | "pyd" | "dylib" => Some(stem.split('.').next().unwrap_or(stem).to_string()),
        _ => None,
    }
}

/// The top-level modules of a vendor folder, with the distribution and the
/// version that provides each: `"yaml"` → `("PyYAML", "6.0.2")`.
pub fn vendored_modules(dir: &Path) -> BTreeMap<String, (String, String)> {
    let mut out = BTreeMap::new();
    for dist in scan_dists(dir) {
        for module in &dist.top_level {
            out.insert(module.clone(), (dist.name.clone(), dist.version.clone()));
        }
    }
    out
}

/// The compiled files (`.so`, `.pyd`, `.dylib`) of a folder, relative to it.
/// At most `limit` are returned.
pub fn native_files(dir: &Path, limit: usize) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>, limit: usize) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        paths.sort();
        for path in paths {
            if out.len() >= limit {
                return;
            }
            if path.is_dir() {
                walk(root, &path, out, limit);
            } else if is_native_file(&path.to_string_lossy()) {
                out.push(
                    path.strip_prefix(root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string(),
                );
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out, limit);
    out
}

fn is_native_file(name: &str) -> bool {
    let file = name.rsplit('/').next().unwrap_or(name);
    file.ends_with(".so")
        || file.ends_with(".pyd")
        || file.ends_with(".dylib")
        || file.contains(".so.")
}

/// Whether the name of a compiled file says it was built for a platform other
/// than `platform`: `x.cpython-311-x86_64-linux-gnu.so` on `aarch64-linux`.
/// A name with no platform in it (`x.abi3.so`) gives false.
fn native_file_is_foreign(name: &str, platform: &str) -> bool {
    let file = name.rsplit('/').next().unwrap_or(name);
    let (arch, os) = match platform.split_once('-') {
        Some(p) => p,
        None => return false,
    };
    let other_os = match os {
        "linux" => file.contains("darwin") || file.ends_with(".pyd") || file.ends_with(".dylib"),
        "macos" => file.contains("linux") || file.ends_with(".pyd"),
        "windows" => file.contains("linux") || file.contains("darwin"),
        _ => false,
    };
    let other_arch = match arch {
        "x86_64" => file.contains("aarch64") || file.contains("arm64"),
        "aarch64" => file.contains("x86_64") || file.contains("amd64"),
        _ => false,
    };
    other_os || other_arch
}

/// Choose the folder of vendored dependencies that `platform` loads.
///
/// - `Ok(None)`: the package has no `vendor/` folder.
/// - `vendor/<platform>/` when it exists.
/// - With a `[metadata.vendored]` record and no folder for `platform`: a
///   folder of another platform when no dependency has compiled code; else an
///   error.
/// - With no record (a `vendor/` folder filled by hand): `vendor/` itself. A
///   dependency in it with compiled code for another platform is an error.
pub fn select_vendor_dir(
    package: &str,
    package_root: &Path,
    vendored: Option<&Vendored>,
    platform: &str,
) -> Result<Option<PathBuf>, String> {
    let vendor = package_root.join("vendor");
    if !vendor.is_dir() {
        return match vendored {
            Some(v) if !v.platforms.is_empty() => Err(format!(
                "package `{package}` records vendored dependencies for {} in package.toml, \
                 and has no vendor/ folder",
                v.platforms.join(", ")
            )),
            _ => Ok(None),
        };
    }

    let own = vendor.join(platform);
    if own.is_dir() {
        return Ok(Some(own));
    }

    if let Some(v) = vendored {
        let present: Vec<&String> = v
            .platforms
            .iter()
            .filter(|p| vendor.join(p.as_str()).is_dir())
            .collect();
        if present.is_empty() {
            return Err(format!(
                "package `{package}` records vendored dependencies for {} in package.toml, \
                 and vendor/ has none of these folders",
                v.platforms.join(", ")
            ));
        }
        let native = v.native
            || present
                .iter()
                .any(|p| !native_files(&vendor.join(p.as_str()), 1).is_empty());
        if native {
            return Err(format!(
                "package `{package}` has native dependencies vendored for {}, and none for \
                 this platform ({platform}). Build it again with `cloacinactl package build \
                 --platform {platform}` (add the other platforms it runs on)",
                v.platforms.join(", ")
            ));
        }
        return Ok(Some(vendor.join(present[0].as_str())));
    }

    // A vendor/ folder filled by hand.
    for dist in scan_dists(&vendor) {
        if dist.is_native()
            && !dist
                .platform_tags
                .iter()
                .any(|t| wheel_tag_runs_on(t, platform))
        {
            return Err(format!(
                "package `{package}` vendors `{}` {} built for {}, and this platform is \
                 {platform}. Fill vendor/ with `cloacinactl package build --platform {platform}`",
                dist.name,
                dist.version,
                dist.platform_tags.join(", ")
            ));
        }
    }
    if let Some(file) = native_files(&vendor, 10_000)
        .into_iter()
        .find(|f| native_file_is_foreign(f, platform))
    {
        return Err(format!(
            "package `{package}` vendors a compiled file for another platform \
             (vendor/{file}), and this platform is {platform}. Fill vendor/ with \
             `cloacinactl package build --platform {platform}`"
        ));
    }
    Ok(Some(vendor))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_dist(dir: &Path, name: &str, version: &str, tag: &str, files: &[&str]) {
        let info = dir.join(format!("{name}-{version}.dist-info"));
        fs::create_dir_all(&info).unwrap();
        fs::write(
            info.join("METADATA"),
            format!("Metadata-Version: 2.1\nName: {name}\nVersion: {version}\n\nbody\n"),
        )
        .unwrap();
        fs::write(
            info.join("WHEEL"),
            format!("Wheel-Version: 1.0\nTag: {tag}\n"),
        )
        .unwrap();
        let mut record = String::new();
        for f in files {
            let path = dir.join(f);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, b"").unwrap();
            record.push_str(&format!("{f},sha256=x,0\n"));
        }
        record.push_str(&format!("{name}-{version}.dist-info/RECORD,,\n"));
        fs::write(info.join("RECORD"), record).unwrap();
    }

    fn record(platforms: &[&str], native: bool) -> Vendored {
        Vendored {
            python_version: "3.11".to_string(),
            platforms: platforms.iter().map(|p| p.to_string()).collect(),
            native,
            lock_file: Some(LOCK_FILE.to_string()),
        }
    }

    #[test]
    fn versions_parse() {
        assert_eq!(parse_version("3.11.9").unwrap(), vec![3, 11, 9]);
        assert_eq!(parse_version("3.13.0rc1").unwrap(), vec![3, 13, 0]);
        assert_eq!(major_minor("3.11").unwrap(), (3, 11));
        assert!(major_minor("3").is_err());
        assert!(parse_version("three").is_err());
    }

    #[test]
    fn requires_python_bounds() {
        assert!(requires_python_allows(">=3.10", "3.11.9").unwrap());
        assert!(!requires_python_allows(">=3.12", "3.11.9").unwrap());
        assert!(requires_python_allows(">=3.10,<3.13", "3.12.1").unwrap());
        assert!(!requires_python_allows(">=3.10,<3.13", "3.13.0").unwrap());
        assert!(requires_python_allows("==3.11.*", "3.11.4").unwrap());
        assert!(!requires_python_allows("==3.11.*", "3.12.0").unwrap());
        assert!(requires_python_allows("~=3.11", "3.12.0").unwrap());
        assert!(!requires_python_allows("~=3.11", "4.0.0").unwrap());
        assert!(requires_python_allows("~=3.11.2", "3.11.9").unwrap());
        assert!(!requires_python_allows("~=3.11.2", "3.12.0").unwrap());
        assert!(!requires_python_allows("!=3.11.*", "3.11.2").unwrap());
        assert!(requires_python_allows(">3.10", "3.11").unwrap());
        assert!(requires_python_allows("<4", "3.11.9").unwrap());
        assert!(requires_python_allows(" >= 3.10 , < 4 ", "3.11").unwrap());
    }

    #[test]
    fn requires_python_with_major_minor_only_ignores_the_patch_of_a_bound() {
        assert!(requires_python_allows(">=3.11.4", "3.11").unwrap());
        assert!(!requires_python_allows(">=3.12.1", "3.11").unwrap());
        assert!(requires_python_allows("<3.11.4", "3.11").unwrap());
        assert!(requires_python_allows("!=3.11.2", "3.11").unwrap());
        assert!(requires_python_allows("~=3.11.2", "3.11").unwrap());
        assert!(!requires_python_allows("~=3.11.2", "3.12").unwrap());
        assert!(requires_python_allows(">3.11", "3.11").unwrap());
        assert!(!requires_python_allows("<3.11", "3.11").unwrap());
        assert!(requires_python_allows("~=3", "3.11").is_err());
    }

    #[test]
    fn requires_python_rejects_text_that_is_not_a_specifier() {
        assert!(requires_python_allows("3.11", "3.11").is_err());
        assert!(requires_python_allows(">=three", "3.11").is_err());
    }

    #[test]
    fn check_python_names_the_package_and_both_versions() {
        let err = check_python("etl", Some(">=3.12"), None, "3.11.9", "this agent").unwrap_err();
        assert!(err.contains("`etl`"), "{err}");
        assert!(err.contains(">=3.12"), "{err}");
        assert!(err.contains("3.11.9"), "{err}");
        assert!(err.contains("this agent"), "{err}");
        check_python("etl", Some(">=3.10"), None, "3.11.9", "this agent").unwrap();
        check_python("etl", None, None, "3.11.9", "this agent").unwrap();
    }

    #[test]
    fn check_python_refuses_another_vendored_version() {
        let v = record(&["x86_64-linux"], true);
        check_python("etl", Some(">=3.10"), Some(&v), "3.11.2", "this server").unwrap();
        let err = check_python("etl", None, Some(&v), "3.12.1", "this server").unwrap_err();
        assert!(err.contains("vendored for Python 3.11"), "{err}");
        assert!(err.contains("3.12.1"), "{err}");
        assert!(err.contains("--python-version 3.12"), "{err}");
    }

    #[test]
    fn check_python_refuses_a_record_that_requires_python_excludes() {
        let v = record(&["x86_64-linux"], false);
        let err =
            check_python("etl", Some(">=3.12"), Some(&v), "3.11", "the build target").unwrap_err();
        assert!(err.contains("requires Python >=3.12"), "{err}");
    }

    #[test]
    fn wheel_tags_match_platforms() {
        assert!(wheel_tag_runs_on("any", "aarch64-linux"));
        assert!(wheel_tag_runs_on("manylinux_2_28_x86_64", "x86_64-linux"));
        assert!(!wheel_tag_runs_on("manylinux_2_28_x86_64", "aarch64-linux"));
        assert!(wheel_tag_runs_on("manylinux2014_aarch64", "aarch64-linux"));
        assert!(wheel_tag_runs_on("musllinux_1_1_x86_64", "x86_64-linux"));
        assert!(wheel_tag_runs_on("macosx_11_0_arm64", "aarch64-macos"));
        assert!(wheel_tag_runs_on("macosx_10_9_universal2", "x86_64-macos"));
        assert!(!wheel_tag_runs_on("macosx_11_0_arm64", "aarch64-linux"));
        assert!(wheel_tag_runs_on("win_amd64", "x86_64-windows"));
    }

    #[test]
    fn uv_platform_covers_the_defaults() {
        for p in DEFAULT_PLATFORMS {
            assert!(uv_platform(p).is_some());
        }
        for p in SUPPORTED_PLATFORMS {
            assert!(uv_platform(p).is_some());
        }
        assert!(uv_platform("riscv64-linux").is_none());
    }

    #[test]
    fn scan_reads_name_version_tags_and_modules() {
        let tmp = tempfile::tempdir().unwrap();
        write_dist(
            tmp.path(),
            "PyYAML",
            "6.0.2",
            "cp311-cp311-manylinux_2_17_x86_64.manylinux2014_x86_64",
            &[
                "yaml/__init__.py",
                "_yaml/__init__.py",
                "yaml/_yaml.cpython-311-x86_64-linux-gnu.so",
            ],
        );
        write_dist(tmp.path(), "six", "1.17.0", "py2.py3-none-any", &["six.py"]);
        // A namespace package: no __init__.py in `google/`.
        write_dist(
            tmp.path(),
            "google-auth",
            "2.0.0",
            "py3-none-any",
            &["google/auth/__init__.py"],
        );

        let dists = scan_dists(tmp.path());
        assert_eq!(dists.len(), 3);
        let yaml = dists.iter().find(|d| d.name == "PyYAML").unwrap();
        assert_eq!(yaml.version, "6.0.2");
        assert!(yaml.is_native());
        assert_eq!(
            yaml.platform_tags,
            vec!["manylinux2014_x86_64", "manylinux_2_17_x86_64"]
        );
        assert_eq!(yaml.top_level, vec!["_yaml", "yaml"]);
        let six = dists.iter().find(|d| d.name == "six").unwrap();
        assert!(!six.is_native());
        assert_eq!(six.top_level, vec!["six"]);
        let google = dists.iter().find(|d| d.name == "google-auth").unwrap();
        assert!(google.top_level.is_empty());

        let modules = vendored_modules(tmp.path());
        assert_eq!(
            modules.get("yaml"),
            Some(&("PyYAML".to_string(), "6.0.2".to_string()))
        );
        assert!(!modules.contains_key("google"));
        assert_eq!(native_files(tmp.path(), 10).len(), 1);
    }

    #[test]
    fn no_vendor_folder_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            select_vendor_dir("p", tmp.path(), None, "x86_64-linux").unwrap(),
            None
        );
    }

    #[test]
    fn own_platform_folder_is_chosen() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("vendor/x86_64-linux")).unwrap();
        fs::create_dir_all(tmp.path().join("vendor/aarch64-linux")).unwrap();
        let v = record(&["aarch64-linux", "x86_64-linux"], true);
        assert_eq!(
            select_vendor_dir("p", tmp.path(), Some(&v), "aarch64-linux").unwrap(),
            Some(tmp.path().join("vendor/aarch64-linux"))
        );
    }

    #[test]
    fn native_package_with_no_folder_for_this_platform_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("vendor/x86_64-linux")).unwrap();
        let v = record(&["x86_64-linux"], true);
        let err = select_vendor_dir("etl", tmp.path(), Some(&v), "aarch64-macos").unwrap_err();
        assert!(err.contains("`etl`"), "{err}");
        assert!(err.contains("x86_64-linux"), "{err}");
        assert!(err.contains("aarch64-macos"), "{err}");
        assert!(err.contains("--platform aarch64-macos"), "{err}");
    }

    #[test]
    fn a_record_that_says_pure_is_checked_against_the_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("vendor/x86_64-linux");
        write_dist(
            &dir,
            "fast",
            "1.0",
            "cp311-cp311-manylinux_2_28_x86_64",
            &[
                "fast/__init__.py",
                "fast/_c.cpython-311-x86_64-linux-gnu.so",
            ],
        );
        let v = record(&["x86_64-linux"], false);
        assert!(select_vendor_dir("etl", tmp.path(), Some(&v), "aarch64-linux").is_err());
    }

    #[test]
    fn pure_package_runs_from_another_platform_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("vendor/x86_64-linux");
        write_dist(&dir, "six", "1.17.0", "py3-none-any", &["six.py"]);
        let v = record(&["x86_64-linux"], false);
        assert_eq!(
            select_vendor_dir("etl", tmp.path(), Some(&v), "aarch64-macos").unwrap(),
            Some(dir)
        );
    }

    #[test]
    fn record_with_no_folder_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let v = record(&["x86_64-linux"], false);
        assert!(select_vendor_dir("etl", tmp.path(), Some(&v), "x86_64-linux").is_err());
        fs::create_dir_all(tmp.path().join("vendor")).unwrap();
        assert!(select_vendor_dir("etl", tmp.path(), Some(&v), "x86_64-linux").is_err());
    }

    #[test]
    fn hand_filled_vendor_is_used_as_it_is() {
        let tmp = tempfile::tempdir().unwrap();
        let vendor = tmp.path().join("vendor");
        write_dist(&vendor, "six", "1.17.0", "py3-none-any", &["six.py"]);
        assert_eq!(
            select_vendor_dir("etl", tmp.path(), None, "aarch64-linux").unwrap(),
            Some(vendor)
        );
    }

    #[test]
    fn hand_filled_vendor_with_a_wheel_for_another_platform_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let vendor = tmp.path().join("vendor");
        write_dist(
            &vendor,
            "pyarrow",
            "18.1.0",
            "cp311-cp311-macosx_12_0_arm64",
            &["pyarrow/__init__.py"],
        );
        let err = select_vendor_dir("etl", tmp.path(), None, "x86_64-linux").unwrap_err();
        assert!(err.contains("`pyarrow`"), "{err}");
        assert!(err.contains("macosx_12_0_arm64"), "{err}");
        assert!(err.contains("x86_64-linux"), "{err}");
        // The same folder loads on the platform the wheel was built for.
        select_vendor_dir("etl", tmp.path(), None, "aarch64-macos").unwrap();
    }

    #[test]
    fn hand_filled_vendor_with_a_compiled_file_for_another_platform_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let vendor = tmp.path().join("vendor/fast");
        fs::create_dir_all(&vendor).unwrap();
        fs::write(vendor.join("_c.cpython-311-x86_64-linux-gnu.so"), b"").unwrap();
        let err = select_vendor_dir("etl", tmp.path(), None, "aarch64-linux").unwrap_err();
        assert!(err.contains("_c.cpython-311-x86_64-linux-gnu.so"), "{err}");
        select_vendor_dir("etl", tmp.path(), None, "x86_64-linux").unwrap();
    }

    #[test]
    fn runnable_platforms_is_any_without_native_code() {
        assert_eq!(runnable_platforms(None), None);
        assert_eq!(
            runnable_platforms(Some(&record(&["x86_64-linux"], false))),
            None
        );
        assert_eq!(
            runnable_platforms(Some(&record(&["aarch64-linux", "x86_64-linux"], true))),
            Some(vec![
                "aarch64-linux".to_string(),
                "x86_64-linux".to_string()
            ])
        );
    }

    #[test]
    fn record_round_trips_through_toml() {
        let v = record(&["aarch64-linux", "x86_64-linux"], true);
        let text = toml::to_string(&v).unwrap();
        let back: Vendored = toml::from_str(&text).unwrap();
        assert_eq!(back, v);
        assert!(
            toml::from_str::<Vendored>("python_version = \"3.11\"\nplatforms = []\nx = 1").is_err()
        );
    }
}
