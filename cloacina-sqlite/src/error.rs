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

use pyo3::exceptions::{PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use std::fmt;

#[derive(Debug)]
pub enum CloacinaError {
    RuntimeError(String),
    ValueError(String),
    TypeError(String),
    InternalError(String),
}

impl fmt::Display for CloacinaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CloacinaError::RuntimeError(msg) => write!(f, "RuntimeError: {}", msg),
            CloacinaError::ValueError(msg) => write!(f, "ValueError: {}", msg),
            CloacinaError::TypeError(msg) => write!(f, "TypeError: {}", msg),
            CloacinaError::InternalError(msg) => write!(f, "InternalError: {}", msg),
        }
    }
}

impl std::error::Error for CloacinaError {}

impl From<CloacinaError> for PyErr {
    fn from(err: CloacinaError) -> PyErr {
        match err {
            CloacinaError::RuntimeError(msg) => PyRuntimeError::new_err(msg),
            CloacinaError::ValueError(msg) => PyValueError::new_err(msg),
            CloacinaError::TypeError(msg) => PyTypeError::new_err(msg),
            CloacinaError::InternalError(msg) => {
                PyRuntimeError::new_err(format!("Internal error: {}", msg))
            }
        }
    }
}

// Convert from cloacina error types
impl From<cloacina::TaskError> for CloacinaError {
    fn from(err: cloacina::TaskError) -> Self {
        CloacinaError::RuntimeError(err.to_string())
    }
}

impl From<cloacina::ContextError> for CloacinaError {
    fn from(err: cloacina::ContextError) -> Self {
        CloacinaError::RuntimeError(err.to_string())
    }
}

impl From<cloacina::ValidationError> for CloacinaError {
    fn from(err: cloacina::ValidationError) -> Self {
        CloacinaError::RuntimeError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, CloacinaError>;
