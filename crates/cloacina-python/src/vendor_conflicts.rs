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

//! The rule for two packages that vendor different versions of one library
//! (CLOACINA-T-1048).
//!
//! One process has one Python interpreter, one `sys.modules` and one
//! `sys.path`. The first copy of a module that is imported is the copy every
//! package gets. So when a package loads, each top-level module of its vendor
//! folder is compared with the vendor folders of the packages loaded before
//! it. A module that another package provides at a different version, and
//! that is already imported from that package's folder or would be found
//! there first on `sys.path`, fails the load of the new package. The package
//! loaded first keeps running.
//!
//! The versions come from the `.dist-info` folders that an install writes.
//! What this does not see is in the documentation of [`find_conflict`].

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::loader::PythonLoaderError;

/// The vendor folder of a package that was loaded in this process.
#[derive(Debug, Clone)]
pub struct LoadedVendor {
    /// Who loaded it, as the error names it.
    pub owner: String,
    /// The folder that was put on `sys.path`.
    pub dir: PathBuf,
    /// Top-level module → (distribution, version).
    pub modules: BTreeMap<String, (String, String)>,
}

/// Two packages that vendor one module at different versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorConflict {
    pub module: String,
    pub package: String,
    pub distribution: String,
    pub version: String,
    pub other_package: String,
    pub other_distribution: String,
    pub other_version: String,
    /// True when the module is already imported from the other package's
    /// folder. False when it is not imported yet, and the other folder is
    /// first on `sys.path`.
    pub imported: bool,
}

impl std::fmt::Display for VendorConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} vendors the module `{}` at version {} ({} {}), and {}, loaded before it in \
             this process, vendors `{}` at version {} ({} {}){}. One Python interpreter holds \
             one version of a module, so {} is not loaded. Vendor the same version in both \
             packages, or run them on different agents.",
            self.package,
            self.module,
            self.version,
            self.distribution,
            self.version,
            self.other_package,
            self.module,
            self.other_version,
            self.other_distribution,
            self.other_version,
            if self.imported {
                ", which is already imported"
            } else {
                ", which is first on sys.path"
            },
            self.package,
        )
    }
}

/// Find the first module of a package's vendor folder that a package loaded
/// before provides at a different version.
///
/// - `owner`, `dir`, `modules`: the package that loads, its vendor folder and
///   the modules in it.
/// - `loaded`: the vendor folders of the packages loaded before.
/// - `sys_path`: the entries of `sys.path` now.
/// - `imported`: for each module of `modules` that is in `sys.modules`, the
///   file it was imported from.
///
/// Not detected:
/// - A namespace package (a top-level folder with no `__init__.py`, such as
///   `google`): more than one distribution puts files in it, so it has no one
///   version. [`cloacina_workflow_plugin::python_env::scan_dists`] leaves it
///   out.
/// - A vendor folder with no `.dist-info` folders (files copied by hand): it
///   has no versions to compare.
/// - Two versions of one shared library that two different modules carry
///   (for example two wheels that each bundle their own `libssl`): the
///   comparison is by Python module.
/// - A new version of the same package that changes a vendored version: the
///   comparison skips folders of the same owner, and the copy that the old
///   version imported stays in `sys.modules`.
/// - A module that a package's own `workflow/` code or the host's
///   `site-packages` provides.
pub fn find_conflict(
    owner: &str,
    dir: &Path,
    modules: &BTreeMap<String, (String, String)>,
    loaded: &[LoadedVendor],
    sys_path: &[PathBuf],
    imported: &HashMap<String, PathBuf>,
) -> Option<VendorConflict> {
    for (module, (dist, version)) in modules {
        for other in loaded {
            if other.owner == owner || other.dir == dir {
                continue;
            }
            let Some((other_dist, other_version)) = other.modules.get(module) else {
                continue;
            };
            if other_version == version {
                continue;
            }
            let is_imported = imported
                .get(module)
                .is_some_and(|file| file.starts_with(&other.dir));
            let on_path = sys_path.iter().any(|p| p == &other.dir);
            if is_imported || on_path {
                return Some(VendorConflict {
                    module: module.clone(),
                    package: owner.to_string(),
                    distribution: dist.clone(),
                    version: version.clone(),
                    other_package: other.owner.clone(),
                    other_distribution: other_dist.clone(),
                    other_version: other_version.clone(),
                    imported: is_imported,
                });
            }
        }
    }
    None
}

static LOADED: Mutex<Vec<LoadedVendor>> = Mutex::new(Vec::new());

/// How the errors name a package.
pub(crate) fn owner_label(tenant_id: Option<&str>, package_name: &str) -> String {
    match tenant_id {
        Some(t) if !t.is_empty() => format!("package `{package_name}` (tenant `{t}`)"),
        _ => format!("package `{package_name}`"),
    }
}

/// Check the vendor folder of a package that is about to load against the
/// packages loaded before, and record it. Call it before the folder goes on
/// `sys.path`. An error means: do not load the package.
pub(crate) fn check_and_register(
    py: Python<'_>,
    owner: &str,
    vendor_dir: &Path,
) -> Result<(), PythonLoaderError> {
    let modules = cloacina_workflow_plugin::python_env::vendored_modules(vendor_dir);
    if modules.is_empty() {
        return Ok(());
    }

    let sys = py.import("sys")?;
    let sys_path: Vec<PathBuf> = sys
        .getattr("path")?
        .cast_into::<PyList>()
        .map_err(PyErr::from)?
        .iter()
        .filter_map(|e| e.extract::<String>().ok())
        .map(PathBuf::from)
        .collect();
    let sys_modules = sys
        .getattr("modules")?
        .cast_into::<PyDict>()
        .map_err(PyErr::from)?;
    let mut imported: HashMap<String, PathBuf> = HashMap::new();
    for module in modules.keys() {
        if let Some(m) = sys_modules.get_item(module.as_str())?
            && let Ok(file) = m.getattr("__file__").and_then(|f| f.extract::<String>())
        {
            imported.insert(module.clone(), PathBuf::from(file));
        }
    }

    let mut loaded = LOADED.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(conflict) =
        find_conflict(owner, vendor_dir, &modules, &loaded, &sys_path, &imported)
    {
        return Err(PythonLoaderError::ImportError(conflict.to_string()));
    }
    loaded.retain(|l| l.dir != vendor_dir);
    loaded.push(LoadedVendor {
        owner: owner.to_string(),
        dir: vendor_dir.to_path_buf(),
        modules,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modules(list: &[(&str, &str, &str)]) -> BTreeMap<String, (String, String)> {
        list.iter()
            .map(|(m, d, v)| (m.to_string(), (d.to_string(), v.to_string())))
            .collect()
    }

    fn loaded(owner: &str, dir: &str, list: &[(&str, &str, &str)]) -> LoadedVendor {
        LoadedVendor {
            owner: owner.to_string(),
            dir: PathBuf::from(dir),
            modules: modules(list),
        }
    }

    #[test]
    fn different_version_already_imported_is_a_conflict_that_names_both_packages() {
        let before = [loaded(
            "package `billing`",
            "/stage/a/vendor/x86_64-linux",
            &[("yaml", "PyYAML", "5.4.1"), ("six", "six", "1.17.0")],
        )];
        let own = modules(&[("yaml", "PyYAML", "6.0.2"), ("six", "six", "1.17.0")]);
        let imported: HashMap<String, PathBuf> = [(
            "yaml".to_string(),
            PathBuf::from("/stage/a/vendor/x86_64-linux/yaml/__init__.py"),
        )]
        .into();

        let c = find_conflict(
            "package `reports`",
            Path::new("/stage/b/vendor/x86_64-linux"),
            &own,
            &before,
            &[],
            &imported,
        )
        .unwrap();
        assert_eq!(c.module, "yaml");
        assert!(c.imported);
        let text = c.to_string();
        assert!(text.contains("package `reports`"), "{text}");
        assert!(text.contains("package `billing`"), "{text}");
        assert!(text.contains("`yaml`"), "{text}");
        assert!(text.contains("6.0.2"), "{text}");
        assert!(text.contains("5.4.1"), "{text}");
        assert!(text.contains("already imported"), "{text}");
    }

    #[test]
    fn different_version_not_imported_yet_but_first_on_sys_path_is_a_conflict() {
        let before = [loaded(
            "package `billing`",
            "/stage/a/vendor",
            &[("yaml", "PyYAML", "5.4.1")],
        )];
        let own = modules(&[("yaml", "PyYAML", "6.0.2")]);
        let c = find_conflict(
            "package `reports`",
            Path::new("/stage/b/vendor"),
            &own,
            &before,
            &[PathBuf::from("/stage/a/vendor")],
            &HashMap::new(),
        )
        .unwrap();
        assert!(!c.imported);
        assert!(c.to_string().contains("first on sys.path"));
    }

    #[test]
    fn same_version_is_no_conflict() {
        let before = [loaded(
            "package `billing`",
            "/stage/a/vendor",
            &[("yaml", "PyYAML", "6.0.2")],
        )];
        let own = modules(&[("yaml", "PyYAML", "6.0.2")]);
        assert!(
            find_conflict(
                "package `reports`",
                Path::new("/stage/b/vendor"),
                &own,
                &before,
                &[PathBuf::from("/stage/a/vendor")],
                &HashMap::new(),
            )
            .is_none()
        );
    }

    #[test]
    fn a_new_version_of_the_same_package_is_not_compared_with_itself() {
        let before = [loaded(
            "package `reports`",
            "/stage/a/vendor",
            &[("yaml", "PyYAML", "5.4.1")],
        )];
        let own = modules(&[("yaml", "PyYAML", "6.0.2")]);
        assert!(
            find_conflict(
                "package `reports`",
                Path::new("/stage/b/vendor"),
                &own,
                &before,
                &[PathBuf::from("/stage/a/vendor")],
                &HashMap::new(),
            )
            .is_none()
        );
    }

    #[test]
    fn a_folder_that_left_sys_path_and_was_never_imported_is_no_conflict() {
        let before = [loaded(
            "package `billing`",
            "/stage/a/vendor",
            &[("yaml", "PyYAML", "5.4.1")],
        )];
        let own = modules(&[("yaml", "PyYAML", "6.0.2")]);
        assert!(
            find_conflict(
                "package `reports`",
                Path::new("/stage/b/vendor"),
                &own,
                &before,
                &[],
                &HashMap::new(),
            )
            .is_none()
        );
    }

    fn write_module(dir: &Path, module: &str, version: &str) {
        let info = dir.join(format!("{module}-{version}.dist-info"));
        std::fs::create_dir_all(&info).unwrap();
        std::fs::write(
            info.join("METADATA"),
            format!("Metadata-Version: 2.1\nName: {module}\nVersion: {version}\n"),
        )
        .unwrap();
        std::fs::write(info.join("WHEEL"), "Tag: py3-none-any\n").unwrap();
        std::fs::write(info.join("RECORD"), format!("{module}.py,sha256=x,1\n")).unwrap();
        std::fs::write(
            dir.join(format!("{module}.py")),
            format!("VERSION = \"{version}\"\n"),
        )
        .unwrap();
    }

    /// The whole rule against the interpreter: package A loads and imports a
    /// vendored module; package B, with another version of it, is refused;
    /// package C, with the same version as A, loads.
    #[test]
    fn second_package_with_another_version_of_an_imported_module_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let module = "t1048_conflict_probe";
        let (a, b, c) = (
            tmp.path().join("a/vendor"),
            tmp.path().join("b/vendor"),
            tmp.path().join("c/vendor"),
        );
        write_module(&a, module, "1.0.0");
        write_module(&b, module, "2.0.0");
        write_module(&c, module, "1.0.0");

        Python::initialize();
        Python::attach(|py| {
            check_and_register(py, "package `first`", &a).unwrap();
            let sys = py.import("sys").unwrap();
            sys.getattr("path")
                .unwrap()
                .call_method1("append", (a.to_str().unwrap(),))
                .unwrap();
            let imported = py.import(module).unwrap();
            let version: String = imported.getattr("VERSION").unwrap().extract().unwrap();
            assert_eq!(version, "1.0.0");

            let err = check_and_register(py, "package `second`", &b).unwrap_err();
            let text = err.to_string();
            assert!(text.contains("package `second`"), "{text}");
            assert!(text.contains("package `first`"), "{text}");
            assert!(text.contains(module), "{text}");
            assert!(text.contains("2.0.0") && text.contains("1.0.0"), "{text}");
            assert!(text.contains("already imported"), "{text}");

            // The same version in a third package is no conflict.
            check_and_register(py, "package `third`", &c).unwrap();
            // The first package still has its module.
            let again: String = py
                .import(module)
                .unwrap()
                .getattr("VERSION")
                .unwrap()
                .extract()
                .unwrap();
            assert_eq!(again, "1.0.0");
        });
    }

    #[test]
    fn owner_label_names_the_tenant_when_there_is_one() {
        assert_eq!(
            owner_label(Some("acme"), "etl"),
            "package `etl` (tenant `acme`)"
        );
        assert_eq!(owner_label(None, "etl"), "package `etl`");
    }
}
