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

use pyo3::prelude::*;

mod context;
mod task;
mod runner;

use context::{PyContext, PyDefaultRunnerConfig};
use task::task as task_decorator;
use runner::{PyDefaultRunner, PyPipelineResult};

/// A simple hello world class for testing
#[pyclass]
pub struct HelloClass {
    message: String,
}

#[pymethods]
impl HelloClass {
    #[new]
    pub fn new() -> Self {
        HelloClass {
            message: "Hello from HelloClass!".to_string(),
        }
    }
    
    pub fn get_message(&self) -> String {
        self.message.clone()
    }
    
    pub fn __repr__(&self) -> String {
        format!("HelloClass(message='{}')", self.message)
    }
}

/// A simple hello world function for testing
#[pyfunction]
fn hello_world() -> String {
    "Hello from Cloaca backend!".to_string()
}

/// Get the backend type based on compiled features
#[pyfunction]
fn get_backend() -> &'static str {
    #[cfg(feature = "postgres")]
    {
        return "postgres";
    }

    #[cfg(feature = "sqlite")]
    {
        return "sqlite";
    }

    #[cfg(not(any(feature = "postgres", feature = "sqlite")))]
    {
        "unknown"
    }
}

/// A Python module implemented in Rust.
#[pymodule]
#[cfg(feature = "postgres")]
fn cloaca_postgres(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Simple test functions
    m.add_function(wrap_pyfunction!(hello_world, m)?)?;
    m.add_function(wrap_pyfunction!(get_backend, m)?)?;

    // Test class
    m.add_class::<HelloClass>()?;

    // Context class
    m.add_class::<PyContext>()?;

    // Configuration class
    m.add_class::<PyDefaultRunnerConfig>()?;

    // Task decorator function
    m.add_function(wrap_pyfunction!(task_decorator, m)?)?;


    // Runner classes
    m.add_class::<PyDefaultRunner>()?;
    m.add_class::<PyPipelineResult>()?;

    // Module metadata (version automatically added by maturin from Cargo.toml)
    m.add("__backend__", "postgres")?;

    Ok(())
}

#[pymodule]
#[cfg(feature = "sqlite")]
fn cloaca_sqlite(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Simple test functions
    m.add_function(wrap_pyfunction!(hello_world, m)?)?;
    m.add_function(wrap_pyfunction!(get_backend, m)?)?;

    // Test class
    m.add_class::<HelloClass>()?;

    // Context class
    m.add_class::<PyContext>()?;

    // Configuration class
    m.add_class::<PyDefaultRunnerConfig>()?;

    // Task decorator function
    m.add_function(wrap_pyfunction!(task_decorator, m)?)?;


    // Runner classes
    m.add_class::<PyDefaultRunner>()?;
    m.add_class::<PyPipelineResult>()?;

    // Module metadata (version automatically added by maturin from Cargo.toml)
    m.add("__backend__", "sqlite")?;

    Ok(())
}
