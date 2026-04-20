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

//! Workflow registry types.
//!
//! The process-global workflow registry was removed in CLOACI-T-0509. Workflow
//! constructors are now owned by [`crate::Runtime`], which is seeded from the
//! `inventory` entries emitted by the `#[workflow]` macro and then mutated
//! dynamically by the reconciler when packages are loaded/unloaded.

use super::Workflow;

/// Type alias for the workflow constructor function.
pub type WorkflowConstructor = Box<dyn Fn() -> Workflow + Send + Sync>;
