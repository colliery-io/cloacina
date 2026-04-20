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

//! Thread-local "current Runtime" slot used by Python decorators/loader.
//!
//! CLOACI-T-0509 step 2: instead of pushing into the process-global
//! task/workflow/trigger registries, Python registration paths resolve
//! a scoped [`cloacina::Runtime`] via [`current_runtime`]. The caller
//! (typically [`crate::runtime_impl::CloacinaPythonRuntime`]) installs a
//! [`ScopedRuntime`] around the Python import so decorators see it.

use std::cell::RefCell;
use std::sync::Arc;

use cloacina::runtime::Runtime;

thread_local! {
    static CURRENT_RUNTIME: RefCell<Option<Arc<Runtime>>> = const { RefCell::new(None) };
}

/// Fetch the Runtime currently installed on this thread, if any.
pub fn current_runtime() -> Option<Arc<Runtime>> {
    CURRENT_RUNTIME.with(|slot| slot.borrow().clone())
}

/// RAII guard that installs a Runtime into the thread-local slot for the
/// duration of its lifetime. Errors on `new` if a runtime is already
/// installed on this thread — nesting would silently corrupt the caller's
/// registration target.
pub struct ScopedRuntime {
    _private: (),
}

impl ScopedRuntime {
    /// Install `runtime` into the thread-local slot. Errors if another
    /// runtime is already installed on this thread.
    pub fn new(runtime: Arc<Runtime>) -> Result<Self, String> {
        CURRENT_RUNTIME.with(|slot| {
            let mut borrow = slot.borrow_mut();
            if borrow.is_some() {
                return Err(
                    "ScopedRuntime::new called while a Runtime is already installed on this thread"
                        .to_string(),
                );
            }
            *borrow = Some(runtime);
            Ok(())
        })?;
        Ok(Self { _private: () })
    }
}

impl Drop for ScopedRuntime {
    fn drop(&mut self) {
        CURRENT_RUNTIME.with(|slot| {
            *slot.borrow_mut() = None;
        });
    }
}
