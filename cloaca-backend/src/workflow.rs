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

use crate::task::PyTask;
use pyo3::prelude::*;

/// Python workflow wrapper
#[pyclass]
pub struct PyWorkflow {
    name: String,
    description: Option<String>,
    tasks: Vec<PyTask>,
}

#[pymethods]
impl PyWorkflow {
    #[new]
    fn new(name: String, description: Option<String>) -> Self {
        PyWorkflow {
            name,
            description,
            tasks: Vec::new(),
        }
    }

    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    #[getter]
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    #[getter]
    fn tasks(&self) -> Vec<PyTask> {
        // Note: This clones the tasks, which might not be ideal for large workflows
        // but is simpler for the initial implementation
        self.tasks.clone()
    }

    fn add_task(&mut self, task: PyTask) -> PyResult<()> {
        self.tasks.push(task);
        Ok(())
    }

    fn add_tasks(&mut self, tasks: Vec<PyTask>) -> PyResult<()> {
        self.tasks.extend(tasks);
        Ok(())
    }

    fn build(&self) -> PyResult<PyWorkflow> {
        // For now, just return a clone
        // Later this will validate the workflow and create optimized internal representation
        Ok(PyWorkflow {
            name: self.name.clone(),
            description: self.description.clone(),
            tasks: self.tasks.clone(),
        })
    }

    fn __len__(&self) -> usize {
        self.tasks.len()
    }

    fn __repr__(&self) -> String {
        match &self.description {
            Some(desc) => format!(
                "PyWorkflow(name='{}', description='{}', tasks={})",
                self.name,
                desc,
                self.tasks.len()
            ),
            None => format!(
                "PyWorkflow(name='{}', tasks={})",
                self.name,
                self.tasks.len()
            ),
        }
    }
}
