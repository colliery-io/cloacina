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

use pyo3::create_exception;
use pyo3::exceptions::{PyConnectionError, PyRuntimeError, PyTimeoutError, PyValueError};
use pyo3::prelude::*;

// Create a custom exception type
create_exception!(cloaca_backend, TaskError, PyException);

/// Simple task error struct for basic error handling
#[pyclass]
pub struct PyTaskError {
    #[pyo3(get)]
    message: String,
}

#[pymethods]
impl PyTaskError {
    #[new]
    fn new(message: String) -> Self {
        PyTaskError { message }
    }

    fn __str__(&self) -> &str {
        &self.message
    }

    fn __repr__(&self) -> String {
        format!("PyTaskError('{}')", self.message)
    }
}

/// Register all exception types with the Python module
pub fn register_exception_types(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("TaskError", m.py().get_type::<TaskError>())?;
    Ok(())
}
