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

use crate::task::get_all_python_tasks;
use cloacina::{Workflow, WorkflowBuilder};
use pyo3::prelude::*;

/// Simple wrapper around the existing Rust Workflow
#[pyclass(name = "Workflow")]
pub struct PyWorkflow {
    inner: Workflow,
}

#[pymethods]
impl PyWorkflow {
    #[new]
    pub fn new(name: String) -> PyResult<Self> {
        let mut builder = WorkflowBuilder::new(&name);

        // Add all registered Python tasks to the workflow
        for py_task in get_all_python_tasks() {
            // Clone the Arc<PythonTask> to get a PythonTask
            let task = (*py_task).clone();
            builder = builder.add_task(task).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to add task: {}",
                    e
                ))
            })?;
        }

        let workflow = builder.build().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to build workflow: {}",
                e
            ))
        })?;

        Ok(PyWorkflow { inner: workflow })
    }

    pub fn __repr__(&self) -> String {
        format!("Workflow(name='{}')", self.inner.name())
    }
}

impl PyWorkflow {
    /// Get a reference to the inner Rust workflow
    pub fn inner(&self) -> &Workflow {
        &self.inner
    }
}
