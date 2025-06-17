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

use super::namespace::PyTaskNamespace;
use pyo3::prelude::*;

/// WorkflowContext provides namespace management for Python workflows
#[pyclass(name = "WorkflowContext")]
#[derive(Clone, Debug)]
pub struct PyWorkflowContext {
    tenant_id: String,
    package_name: String,
    workflow_id: String,
}

#[pymethods]
impl PyWorkflowContext {
    /// Create a new WorkflowContext
    #[new]
    pub fn new(tenant_id: &str, package_name: &str, workflow_id: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            package_name: package_name.to_string(),
            workflow_id: workflow_id.to_string(),
        }
    }

    /// Get tenant ID
    #[getter]
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    /// Get package name
    #[getter]
    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    /// Get workflow ID
    #[getter]
    pub fn workflow_id(&self) -> &str {
        &self.workflow_id
    }

    /// Generate a TaskNamespace for a task within this workflow context
    pub fn task_namespace(&self, task_id: &str) -> PyTaskNamespace {
        PyTaskNamespace::from_rust(cloacina::TaskNamespace::new(
            &self.tenant_id,
            &self.package_name,
            &self.workflow_id,
            task_id,
        ))
    }

    /// Resolve a dependency task name to a full TaskNamespace within this context
    pub fn resolve_dependency(&self, task_name: &str) -> PyTaskNamespace {
        self.task_namespace(task_name)
    }

    /// Get the workflow namespace (without task_id)
    pub fn workflow_namespace(&self) -> PyTaskNamespace {
        PyTaskNamespace::from_rust(cloacina::TaskNamespace::new(
            &self.tenant_id,
            &self.package_name,
            &self.workflow_id,
            "",
        ))
    }

    /// Check if a namespace belongs to this workflow context
    pub fn contains_namespace(&self, namespace: &PyTaskNamespace) -> bool {
        namespace.tenant_id() == self.tenant_id
            && namespace.package_name() == self.package_name
            && namespace.workflow_id() == self.workflow_id
    }

    /// String representation
    pub fn __str__(&self) -> String {
        format!(
            "{}::{}::{}",
            self.tenant_id, self.package_name, self.workflow_id
        )
    }

    /// String representation
    pub fn __repr__(&self) -> String {
        format!(
            "WorkflowContext('{}', '{}', '{}')",
            self.tenant_id, self.package_name, self.workflow_id
        )
    }

    /// Equality comparison
    pub fn __eq__(&self, other: &PyWorkflowContext) -> bool {
        self.tenant_id == other.tenant_id
            && self.package_name == other.package_name
            && self.workflow_id == other.workflow_id
    }
}

impl PyWorkflowContext {
    /// Get the default workflow context (for backward compatibility)
    pub fn default() -> Self {
        Self {
            tenant_id: "public".to_string(),
            package_name: "embedded".to_string(),
            workflow_id: "default".to_string(),
        }
    }

    /// Convert to namespace components
    pub fn as_components(&self) -> (&str, &str, &str) {
        (&self.tenant_id, &self.package_name, &self.workflow_id)
    }
}
