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

//! HA window-survival fixture (CLOACI-T-0851 / ADR CLOACI-A-0012).
//!
//! One STATE accumulator (`window`, capacity 8) on one reactor (`ha_state_rx`).
//! The k8s-leader lane's assertion 6e partially fills the window on the owning
//! replica, kills that replica, and asserts the NEW owner restores the same
//! entries from the DAL — the property that distinguishes "leadership plus
//! durable accumulators" from leadership alone. Capacity is deliberately
//! larger than the number of injected events so the window is PARTIAL: a full
//! window surviving could be re-derived from capacity alone, a partial one
//! only from restored state.

use cloacina_macros::{reactor, state_accumulator};
use cloacina_workflow_plugin as cloacina;
use std::collections::VecDeque;

#[state_accumulator(capacity = 8)]
pub fn window() -> VecDeque<u64> {
    VecDeque::new()
}

#[reactor(
    name = "ha_state_rx",
    accumulators = [window],
    criteria = when_any(window),
)]
pub struct HaStateRx;

cloacina::package!();
