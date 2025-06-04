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

use crate::context::PyContext;
use pyo3::prelude::*;
use std::sync::Arc;

/// Python task wrapper
#[pyclass]
pub struct PyTask {
    task_id: String,
    task_fn: Arc<PyObject>,
    dependencies: Vec<String>,
}

#[pymethods]
impl PyTask {
    #[new]
    fn new(task_id: String, task_fn: PyObject, dependencies: Option<Vec<String>>) -> Self {
        PyTask {
            task_id,
            task_fn: Arc::new(task_fn),
            dependencies: dependencies.unwrap_or_default(),
        }
    }

    #[getter]
    fn task_id(&self) -> &str {
        &self.task_id
    }

    #[getter]
    fn dependencies(&self) -> Vec<String> {
        self.dependencies.clone()
    }

    /// Execute the Python task function
    fn execute(&self, context: &PyContext) -> PyResult<PyContext> {
        Python::with_gil(|py| {
            let task_fn = self.task_fn.as_ref(py);
            let result = task_fn.call1((context,))?;
            let result_context: PyContext = result.extract()?;
            Ok(result_context)
        })
    }
}

/// Python task decorator function
#[pyfunction(name = "task")]
#[pyo3(signature = (id, dependencies = None))]
pub fn task_decorator(
    id: String,
    dependencies: Option<Vec<String>>,
) -> impl Fn(PyObject) -> PyResult<PyTask> {
    move |func: PyObject| -> PyResult<PyTask> {
        Ok(PyTask::new(id.clone(), func, dependencies.clone()))
    }
}

impl PyTask {
    /// Get the underlying task function for execution
    pub fn get_function(&self) -> Arc<PyObject> {
        self.task_fn.clone()
    }
}
