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

// This file serves as the entry point for integration tests in this directory.

use ctor::ctor;

// Initialize OpenSSL before any tests run to prevent SIGSEGV on Linux.
// This fixes a known issue with diesel + postgres + connection pooling where
// OpenSSL's atexit handler causes thread-safety issues during cleanup.
// See: https://github.com/diesel-rs/diesel/issues/3441
#[ctor]
fn init_openssl() {
    openssl::init();
}

pub mod context;
pub mod dal;
pub mod database;
pub mod error;
pub mod executor;
pub mod logging;
pub mod models;
pub mod packaging;
pub mod packaging_inspection;
pub mod registry_simple_functional_test;
pub mod registry_storage_tests;
pub mod registry_workflow_registry_tests;
pub mod runner_configurable_registry_tests;
pub mod scheduler;
pub mod task;
pub mod workflow;

#[path = "../fixtures.rs"]
mod fixtures;
