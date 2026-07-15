/*
 *  Copyright 2026 Colliery Software
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

//! Primitive-only fixture: reactor-only Rust cdylib using the unified
//! `cloacina::package!()` plugin shell. Exercises T-A's invariant that a
//! cdylib declaring just a `#[reactor]` (no workflow, no CG, no trigger)
//! produces a valid `CloacinaPlugin` whose `get_reactor_metadata` returns
//! the reactor's accumulators.

use cloacina_macros::{reactor, state_accumulator};
use cloacina_workflow_plugin as cloacina;
use std::collections::VecDeque;

// CLOACI-T-0896 (Rust-cdylib side): declare `alpha` as a STATE accumulator via
// the macro. In packaged mode the macro emits only this function + an
// `AccumulatorEntry` inventory record (the host provides the runtime; the
// `cloacina::`-bound struct is gated out), so `get_reactor_metadata` reports
// `alpha` as `state` (capacity 5) instead of the old hardcoded passthrough.
// `beta` stays an implicit passthrough (no decorated fn) — proving the fallback.
#[state_accumulator(capacity = 5)]
pub fn alpha() -> VecDeque<u64> {
    VecDeque::new()
}

#[reactor(
    name = "shared_rx",
    accumulators = [alpha, beta],
    criteria = when_any(alpha, beta),
)]
pub struct SharedRx;

cloacina::package!();
