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

//! `cloaca.constructor(...)` — the Python consumer surface for packaged WASM
//! constructor providers (CLOACI-T-0831), mirroring Rust's `constructor!(...)`.
//!
//! Called inside a `WorkflowBuilder` context (exactly where `@cloaca.task`
//! functions are defined), it resolves the named member of a provider suite via
//! the SAME host loader the Rust surface uses (`load_constructor_node`:
//! provider-search-path resolution, name-keyed config binding, name-in-configure
//! member selection, default-closed capability grants) and registers the resolved
//! node into the current scoped [`Runtime`] under
//! `TaskNamespace(tenant, package, workflow, id)` — the workflow's DAG assembly
//! then picks it up exactly like a `@cloaca.task`.
//!
//! ```python
//! with cloaca.WorkflowBuilder("granted") as _:
//!     cloaca.constructor(
//!         id="reader",
//!         from_="cloacina-provider-fs@0.1.0",
//!         constructor="read_file",
//!         config={"path": "/data/secret.txt"},
//!         grants={"fs": ["ro:/data"]},
//!     )
//!
//!     @cloaca.task(id="echo", dependencies=["reader"])
//!     def echo(context): ...
//! ```
//!
//! Execution is language-agnostic — the node runs in the Rust runtime's WASM
//! sandbox; Python only authors the wiring. Providers resolve against the same
//! provider search path as Rust (`CLOACINA_PROVIDER_PATH` / `./providers` /
//! `set_provider_search_path`).

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::task::current_workflow_context;

/// Wire a packaged constructor-provider member into the current workflow as a
/// DAG node. See the module docs for the full model; `from_` is the provider
/// (Cargo package) name with an optional `@version` pin, `constructor` selects
/// the member, `config` binds by NAME against the member's declared `#[config]`
/// schema, and `grants` is the default-closed capability grant map
/// (`{"http": [...], "tcp": [...], "fs": [...], "env": [...]}`).
#[pyfunction]
#[pyo3(signature = (id, from_, constructor, config = None, grants = None, dependencies = None))]
pub fn constructor(
    py: Python,
    id: String,
    from_: String,
    constructor: String,
    config: Option<Bound<'_, PyDict>>,
    grants: Option<Bound<'_, PyDict>>,
    dependencies: Option<Vec<PyObject>>,
) -> PyResult<()> {
    let context = current_workflow_context()?;
    let (tenant_id, package_name, workflow_id) = context.as_components();
    let namespace = cloacina::TaskNamespace::new(tenant_id, package_name, workflow_id, &id);

    // config: dict → (name, value) pairs in WRITTEN order; the loader reorders
    // them into the member's declaration order and binds by name (kwargs).
    let mut config_pairs: Vec<(String, serde_json::Value)> = Vec::new();
    if let Some(cfg) = &config {
        for (k, v) in cfg.iter() {
            let key: String = k.extract().map_err(|_| {
                PyValueError::new_err("cloaca.constructor: config keys must be strings")
            })?;
            let value: serde_json::Value = pythonize::depythonize(&v).map_err(|e| {
                PyValueError::new_err(format!(
                    "cloaca.constructor: config value for '{key}' is not JSON-convertible: {e}"
                ))
            })?;
            config_pairs.push((key, value));
        }
    }

    // grants: {"kind": ["pattern", ...]} → GrantSpec (default-closed when absent).
    let mut grant_pairs: Vec<(String, Vec<String>)> = Vec::new();
    if let Some(g) = &grants {
        for (k, v) in g.iter() {
            let kind: String = k.extract().map_err(|_| {
                PyValueError::new_err(
                    "cloaca.constructor: grant keys must be strings (http/tcp/fs/env)",
                )
            })?;
            let patterns: Vec<String> = v.extract().map_err(|_| {
                PyValueError::new_err(format!(
                    "cloaca.constructor: grants['{kind}'] must be a list of strings"
                ))
            })?;
            grant_pairs.push((kind, patterns));
        }
    }
    let grant_spec = cloacina::registry::loader::grants::GrantSpec::from_pairs(grant_pairs);

    // dependencies: strings or function objects (same convention as @task deps).
    let mut dep_namespaces: Vec<cloacina::TaskNamespace> = Vec::new();
    for (i, dep) in dependencies.unwrap_or_default().iter().enumerate() {
        let dep_id = if let Ok(s) = dep.extract::<String>(py) {
            s
        } else if let Ok(name) = dep.getattr(py, "__name__") {
            name.extract::<String>(py).map_err(|e| {
                PyValueError::new_err(format!(
                    "cloaca.constructor: dependency {i} __name__ is not a string: {e}"
                ))
            })?
        } else {
            return Err(PyValueError::new_err(format!(
                "cloaca.constructor: dependency {i} must be a string or a task function"
            )));
        };
        dep_namespaces.push(cloacina::TaskNamespace::new(
            tenant_id,
            package_name,
            workflow_id,
            &dep_id,
        ));
    }

    // Resolve the member through the host loader (wasmtime compile is blocking
    // work — release the GIL for it). Fails closed on a missing provider/member,
    // a bad config kwarg, or a malformed grant.
    let node = py
        .allow_threads(|| {
            cloacina::registry::loader::load_constructor_node(
                &id,
                &from_,
                &constructor,
                config_pairs,
                dep_namespaces,
                grant_spec,
            )
        })
        .map_err(|e| {
            PyValueError::new_err(format!(
                "cloaca.constructor: resolve '{constructor}' from provider '{from_}': {e}"
            ))
        })?;

    // Register into the current scoped Runtime — the workflow DAG assembly picks
    // it up exactly like a @cloaca.task registration.
    let rt = crate::runtime_scope::current_runtime().ok_or_else(|| {
        PyValueError::new_err(
            "cloaca.constructor called outside a Runtime scope — install a ScopedRuntime \
             before importing Python workflow modules",
        )
    })?;
    py.allow_threads(|| {
        rt.register_task(namespace, move || node.clone());
    });

    Ok(())
}
