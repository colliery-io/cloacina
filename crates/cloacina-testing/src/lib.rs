/*
 *  Copyright 2025-2026 Colliery Software
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

//! # cloacina-testing
//!
//! Test utilities for Cloacina workflows — no database required.
//!
//! This crate provides a lightweight, in-process test runner that executes
//! tasks in dependency order without any database, scheduler, or background
//! threads. It is designed for unit testing task logic.
//!
//! ## Quick Start
//!
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! [dev-dependencies]
//! cloacina-testing = { version = "0.3" }
//! ```
//!
//! Write a test:
//!
//! ```rust,ignore
//! use cloacina_testing::{TestRunner, TestResult};
//! use cloacina_workflow::Context;
//! use std::sync::Arc;
//!
//! #[tokio::test]
//! async fn test_my_pipeline() {
//!     let result = TestRunner::new()
//!         .register(Arc::new(NormalizeTask))
//!         .register(Arc::new(ValidateTask))
//!         .run(Context::new())
//!         .await
//!         .unwrap();
//!
//!     result.assert_all_completed();
//! }
//! ```
//!
//! ## Feature Flags
//!
//! - **`continuous`** — Enables `BoundaryEmitter` and `MockDataConnection` for
//!   testing continuous/reactive tasks. Requires the continuous scheduling
//!   types from `cloacina` (available once CLOACI-I-0023 lands).

pub mod assertions;
pub mod result;
pub mod runner;

#[cfg(feature = "db")]
pub mod test_db;

#[cfg(feature = "continuous")]
pub mod boundary;
#[cfg(feature = "continuous")]
pub mod mock;

// Re-exports
pub use result::{TaskOutcome, TestResult};
pub use runner::{TestRunner, TestRunnerError};

#[cfg(feature = "continuous")]
pub use boundary::BoundaryEmitter;
#[cfg(feature = "continuous")]
pub use mock::MockDataConnection;

#[cfg(feature = "db")]
pub use test_db::{test_dal, test_db};
