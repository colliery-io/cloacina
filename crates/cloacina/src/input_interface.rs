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

//! Injectable input interface (CLOACI-I-0128) — re-export of the canonical
//! helpers.
//!
//! The implementation lives in `cloacina-workflow` (not here) because the
//! `#[workflow(params(...))]` macro emits calls to `schema_for` into **packaged
//! cdylibs**, which depend on `cloacina-workflow`, not core `cloacina`. Core +
//! server code reach the same helpers through this re-export.
//!
//! Spec: [CLOACI-S-0013]; descriptor decision: [CLOACI-A-0007].

pub use cloacina_workflow::input_interface::{
    default_json, schema_for, slots_to_json, InputSlot, ProbeFallback, ProbeTyped, SchemaProbe,
};
