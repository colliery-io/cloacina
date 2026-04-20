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

//! # Trigger Registry Types
//!
//! The process-global trigger registry was removed in CLOACI-T-0509. Trigger
//! constructors are owned by [`crate::Runtime`], which is seeded from
//! `inventory` entries emitted by the `#[trigger]` macro. Dynamic registration
//! at runtime (e.g. from packages loaded via the reconciler) goes through
//! [`crate::Runtime::register_trigger`] directly.

use std::sync::Arc;

use super::Trigger;

/// Type alias for a trigger constructor function stored in the runtime registry.
pub type TriggerConstructor = Box<dyn Fn() -> Arc<dyn Trigger> + Send + Sync>;
