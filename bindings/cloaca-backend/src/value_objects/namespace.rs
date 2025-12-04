/*
 *  Copyright 2025 Colliery Software
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

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Python wrapper for TaskNamespace
#[pyclass(name = "TaskNamespace")]
#[derive(Clone, Debug)]
pub struct PyTaskNamespace {
    inner: cloacina::TaskNamespace,
}

#[pymethods]
impl PyTaskNamespace {
    /// Create a new TaskNamespace
    #[new]
    pub fn new(tenant_id: &str, package_name: &str, workflow_id: &str, task_id: &str) -> Self {
        Self {
            inner: cloacina::TaskNamespace::new(tenant_id, package_name, workflow_id, task_id),
        }
    }

    /// Parse TaskNamespace from string format "tenant::package::workflow::task"
    #[staticmethod]
    pub fn from_string(namespace_str: &str) -> PyResult<Self> {
        cloacina::TaskNamespace::from_string(namespace_str)
            .map(|inner| Self { inner })
            .map_err(|e| PyValueError::new_err(format!("Invalid namespace format: {}", e)))
    }

    /// Get tenant ID
    #[getter]
    pub fn tenant_id(&self) -> &str {
        &self.inner.tenant_id
    }

    /// Get package name
    #[getter]
    pub fn package_name(&self) -> &str {
        &self.inner.package_name
    }

    /// Get workflow ID
    #[getter]
    pub fn workflow_id(&self) -> &str {
        &self.inner.workflow_id
    }

    /// Get task ID
    #[getter]
    pub fn task_id(&self) -> &str {
        &self.inner.task_id
    }

    /// Get parent namespace (without task_id)
    pub fn parent(&self) -> Self {
        Self {
            inner: cloacina::TaskNamespace::new(
                &self.inner.tenant_id,
                &self.inner.package_name,
                &self.inner.workflow_id,
                "",
            ),
        }
    }

    /// Check if this namespace is a child of another
    pub fn is_child_of(&self, parent: &PyTaskNamespace) -> bool {
        self.inner.tenant_id == parent.inner.tenant_id
            && self.inner.package_name == parent.inner.package_name
            && self.inner.workflow_id == parent.inner.workflow_id
            && !self.inner.task_id.is_empty()
            && parent.inner.task_id.is_empty()
    }

    /// Check if this namespace is a sibling of another (same parent)
    pub fn is_sibling_of(&self, other: &PyTaskNamespace) -> bool {
        self.inner.tenant_id == other.inner.tenant_id
            && self.inner.package_name == other.inner.package_name
            && self.inner.workflow_id == other.inner.workflow_id
            && !self.inner.task_id.is_empty()
            && !other.inner.task_id.is_empty()
            && self.inner.task_id != other.inner.task_id
    }

    /// String representation
    pub fn __str__(&self) -> String {
        self.inner.to_string()
    }

    /// String representation
    pub fn __repr__(&self) -> String {
        format!(
            "TaskNamespace('{}', '{}', '{}', '{}')",
            self.inner.tenant_id,
            self.inner.package_name,
            self.inner.workflow_id,
            self.inner.task_id
        )
    }

    /// Equality comparison
    pub fn __eq__(&self, other: &PyTaskNamespace) -> bool {
        self.inner == other.inner
    }

    /// Hash for use in sets/dicts
    pub fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish()
    }
}

impl PyTaskNamespace {
    /// Convert from Rust TaskNamespace (for internal use)
    pub fn from_rust(namespace: cloacina::TaskNamespace) -> Self {
        Self { inner: namespace }
    }

    /// Convert to Rust TaskNamespace (for internal use)
    pub fn to_rust(&self) -> cloacina::TaskNamespace {
        self.inner.clone()
    }
}
