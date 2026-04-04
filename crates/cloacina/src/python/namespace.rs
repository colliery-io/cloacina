/*
 *  Copyright 2025-2026 Colliery Software
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
    inner: crate::TaskNamespace,
}

#[pymethods]
impl PyTaskNamespace {
    /// Create a new TaskNamespace
    #[new]
    pub fn new(tenant_id: &str, package_name: &str, workflow_id: &str, task_id: &str) -> Self {
        Self {
            inner: crate::TaskNamespace::new(tenant_id, package_name, workflow_id, task_id),
        }
    }

    /// Parse TaskNamespace from string format "tenant::package::workflow::task"
    #[staticmethod]
    pub fn from_string(namespace_str: &str) -> PyResult<Self> {
        crate::TaskNamespace::from_string(namespace_str)
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
            inner: crate::TaskNamespace::new(
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
    pub fn from_rust(namespace: crate::TaskNamespace) -> Self {
        Self { inner: namespace }
    }

    /// Convert to Rust TaskNamespace (for internal use)
    pub fn to_rust(&self) -> crate::TaskNamespace {
        self.inner.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_getters() {
        pyo3::prepare_freethreaded_python();
        let ns = PyTaskNamespace::new("tenant1", "pkg1", "wf1", "task1");
        assert_eq!(ns.tenant_id(), "tenant1");
        assert_eq!(ns.package_name(), "pkg1");
        assert_eq!(ns.workflow_id(), "wf1");
        assert_eq!(ns.task_id(), "task1");
    }

    #[test]
    fn test_from_string_valid() {
        pyo3::prepare_freethreaded_python();
        let ns = PyTaskNamespace::from_string("tenant1::pkg1::wf1::task1").unwrap();
        assert_eq!(ns.tenant_id(), "tenant1");
        assert_eq!(ns.package_name(), "pkg1");
        assert_eq!(ns.workflow_id(), "wf1");
        assert_eq!(ns.task_id(), "task1");
    }

    #[test]
    fn test_from_string_invalid() {
        pyo3::prepare_freethreaded_python();
        assert!(PyTaskNamespace::from_string("invalid").is_err());
        assert!(PyTaskNamespace::from_string("a::b").is_err());
        assert!(PyTaskNamespace::from_string("a::b::c").is_err());
    }

    #[test]
    fn test_parent() {
        pyo3::prepare_freethreaded_python();
        let ns = PyTaskNamespace::new("t", "p", "w", "task1");
        let parent = ns.parent();
        assert_eq!(parent.tenant_id(), "t");
        assert_eq!(parent.package_name(), "p");
        assert_eq!(parent.workflow_id(), "w");
        assert_eq!(parent.task_id(), "");
    }

    #[test]
    fn test_is_child_of() {
        pyo3::prepare_freethreaded_python();
        let child = PyTaskNamespace::new("t", "p", "w", "task1");
        let parent = PyTaskNamespace::new("t", "p", "w", "");
        let non_parent = PyTaskNamespace::new("t", "p", "other_wf", "");

        assert!(child.is_child_of(&parent));
        assert!(!child.is_child_of(&non_parent));
        // A parent is not a child of itself
        assert!(!parent.is_child_of(&parent));
    }

    #[test]
    fn test_is_sibling_of() {
        pyo3::prepare_freethreaded_python();
        let task1 = PyTaskNamespace::new("t", "p", "w", "task1");
        let task2 = PyTaskNamespace::new("t", "p", "w", "task2");
        let other = PyTaskNamespace::new("t", "p", "other_wf", "task3");

        assert!(task1.is_sibling_of(&task2));
        assert!(!task1.is_sibling_of(&other));
        // A task is not a sibling of itself
        assert!(!task1.is_sibling_of(&task1));
    }

    #[test]
    fn test_str_and_repr() {
        pyo3::prepare_freethreaded_python();
        let ns = PyTaskNamespace::new("t", "p", "w", "task1");
        assert_eq!(ns.__str__(), "t::p::w::task1");
        assert_eq!(ns.__repr__(), "TaskNamespace('t', 'p', 'w', 'task1')");
    }

    #[test]
    fn test_eq() {
        pyo3::prepare_freethreaded_python();
        let a = PyTaskNamespace::new("t", "p", "w", "task1");
        let b = PyTaskNamespace::new("t", "p", "w", "task1");
        let c = PyTaskNamespace::new("t", "p", "w", "task2");
        assert!(a.__eq__(&b));
        assert!(!a.__eq__(&c));
    }

    #[test]
    fn test_hash_consistency() {
        pyo3::prepare_freethreaded_python();
        let a = PyTaskNamespace::new("t", "p", "w", "task1");
        let b = PyTaskNamespace::new("t", "p", "w", "task1");
        assert_eq!(a.__hash__(), b.__hash__());
    }

    #[test]
    fn test_from_rust_to_rust_roundtrip() {
        pyo3::prepare_freethreaded_python();
        let rust_ns = crate::TaskNamespace::new("t", "p", "w", "task1");
        let py_ns = PyTaskNamespace::from_rust(rust_ns.clone());
        let roundtripped = py_ns.to_rust();
        assert_eq!(rust_ns, roundtripped);
    }
}
