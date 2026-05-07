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

//! Primitive-only fixture (T-0553 / I-0102 T-D): triggers only, no
//! reactor, no CG, no workflow. Exercises the unified shell macro's
//! `get_trigger_metadata` walking both cron-shaped and custom-poll
//! `TriggerEntry` inventories at FFI call time.

use cloacina_macros::trigger;
use cloacina_workflow::TriggerResult;

cloacina_workflow_plugin::package!();

/// Cron trigger — fires every 10 seconds.
#[trigger(on = "trigger_only_cron_workflow", cron = "*/10 * * * * *")]
pub async fn trigger_only_cron() {}

/// Custom-poll trigger — always skips. Used to verify the non-cron path
/// of `get_trigger_metadata` returns the entry with `cron_expression: None`.
#[trigger(on = "trigger_only_custom_workflow", poll_interval = "5s")]
pub async fn trigger_only_custom() -> Result<TriggerResult, cloacina_workflow::TriggerError> {
    Ok(TriggerResult::Skip)
}
