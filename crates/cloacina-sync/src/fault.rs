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

//! Test-only faults (COLLIERY-I-0632 / CLOACINA-T-0980), behind the
//! `fault-injection` feature. A sync definition asks for one through its
//! `fault` param; without the feature the param is read and ignored, so a
//! shipped binary cannot be made to fail this way.
//!
//! The faults sit between the destination's durable write and the completion
//! writes: the window in which a crash must not lose a record. A test that
//! kills a process there, or fails the task there, must see the segment
//! written again, absorbed by upsert, and the cursor advanced once.

use cloacina::Context;
use serde_json::Value;

use crate::params::keys;

/// The `fault` param of an execution, if any.
pub fn requested(context: &Context<Value>) -> Option<String> {
    match context.get(keys::FAULT) {
        Some(Value::String(s)) if !s.trim().is_empty() => Some(s.trim().to_string()),
        _ => None,
    }
}

/// Apply the requested fault after the write. Without the feature: nothing.
#[cfg(not(feature = "fault-injection"))]
pub async fn after_write(context: &Context<Value>) -> Result<(), String> {
    if let Some(f) = requested(context) {
        tracing::warn!(fault = %f, "sync: a fault was requested but fault-injection is not compiled in; ignored");
    }
    Ok(())
}

/// Apply the requested fault after the write.
#[cfg(feature = "fault-injection")]
pub async fn after_write(context: &Context<Value>) -> Result<(), String> {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};
    // Once per sync per process, so each scenario of a suite sees its fault.
    static FAILED_ONCE: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

    let Some(fault) = requested(context) else {
        return Ok(());
    };
    if fault == "fail_after_write_once" {
        let sync = context
            .get(keys::SYNC)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let first = FAILED_ONCE
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(sync);
        if first {
            tracing::warn!(
                "sync: injected fault, failing the write task once after the destination accepted the segment"
            );
            return Err("injected fault: fail_after_write_once".to_string());
        }
        return Ok(());
    }
    if let Some(ms) = fault.strip_prefix("slow_write_ms:") {
        let ms: u64 = ms
            .trim()
            .parse()
            .map_err(|_| format!("bad fault `{fault}`"))?;
        tracing::warn!(
            ms,
            "sync: injected fault, holding the write task after the destination accepted the segment"
        );
        tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        return Ok(());
    }
    Err(format!("unknown fault `{fault}`"))
}
