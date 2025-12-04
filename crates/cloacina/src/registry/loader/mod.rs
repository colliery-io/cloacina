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

//! Package loader module for workflow registry.
//!
//! This module provides functionality to load workflow packages (.so files),
//! extract metadata, validate package integrity, and register tasks with the
//! global task registry.

pub mod package_loader;
pub mod task_registrar;
pub mod validator;

pub use package_loader::PackageLoader;
pub use task_registrar::TaskRegistrar;
pub use validator::PackageValidator;
