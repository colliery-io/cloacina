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

//! # Cloacina Macros
//!
//! This crate provides procedural macros for defining tasks and workflows in the Cloacina framework.
//! It enables compile-time validation of task dependencies and workflow structure.
//!
//! ## Key Features
//!
//! - `#[task]` attribute macro for defining tasks with retry policies and trigger rules
//! - `workflow!` macro for declarative workflow definition
//! - `#[packaged_workflow]` attribute macro for creating distributable workflow packages
//! - Compile-time validation of task dependencies and workflow structure
//! - Automatic task and workflow registration
//! - Code fingerprinting for task versioning
//!
//! ## Example
//!
//! ```rust
//! use cloacina_macros::task;
//!
//! #[task(
//!     id = "my_task",
//!     dependencies = ["other_task"],
//!     retry_attempts = 3,
//!     retry_backoff = "exponential"
//! )]
//! async fn my_task(context: &mut Context<Value>) -> Result<(), TaskError> {
//!     // Task implementation
//!     Ok(())
//! }
//!
//! use cloacina_macros::workflow;
//!
//! let workflow = workflow! {
//!     name: "my_workflow",
//!     description: "A sample workflow",
//!     tasks: [my_task, other_task]
//! };
//! ```

mod packaged_workflow;
mod registry;
mod tasks;
mod workflow;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn task(args: TokenStream, input: TokenStream) -> TokenStream {
    tasks::task(args, input)
}

#[proc_macro]
pub fn workflow(input: TokenStream) -> TokenStream {
    workflow::workflow(input)
}

#[proc_macro_attribute]
pub fn packaged_workflow(args: TokenStream, input: TokenStream) -> TokenStream {
    packaged_workflow::packaged_workflow(args, input)
}
