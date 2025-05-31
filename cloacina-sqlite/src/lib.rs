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
mod error;
mod executor;
mod task;
mod workflow;

use executor::PyUnifiedExecutor;
use task::{task_decorator, PyTaskDecorator};
use workflow::PyWorkflow;

#[pymodule]
fn _cloacina_sqlite(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // Core classes - thin wrappers around existing Rust types
    m.add_class::<PyWorkflow>()?;
    m.add_class::<PyUnifiedExecutor>()?;
    m.add_class::<PyTaskDecorator>()?;

    // Task decorator function
    m.add_function(wrap_pyfunction!(task_decorator, m)?)?;

    Ok(())
}
