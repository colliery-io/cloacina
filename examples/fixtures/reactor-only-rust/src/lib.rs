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

use cloacina_macros::reactor;
use cloacina_workflow_plugin as cloacina;

#[reactor(
    name = "shared_rx",
    accumulators = [alpha, beta],
    criteria = when_any(alpha, beta),
)]
pub struct SharedRx;

cloacina::package!();
