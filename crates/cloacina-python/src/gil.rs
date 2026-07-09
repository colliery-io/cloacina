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

//! The one GIL-safe async→sync bridge for the Python bindings (CLOACI-I-0136).
//!
//! Every sync PyO3 method that must drive an async Rust future to completion
//! calls [`py_block_on`] — the single auditable place the GIL-deadlock invariant
//! lives. Do NOT hand-roll `Runtime::new().block_on(...)` or `Handle::current()
//! .block_on(...)` at a call site: doing so risks holding the GIL across the
//! await, the exact footgun behind the scenario 30/32/33 PyO3↔tokio deadlocks.
//!
//! NOTE ON SCOPE: this is for a ONE-SHOT future resolved from a GIL-holding sync
//! method. `bindings/runner.rs` is deliberately NOT routed through here — its
//! `run_event_loop` is a long-running actor loop on a dedicated OS thread that
//! does not hold the GIL, so the "release the GIL then block" contract is
//! meaningless there.

use pyo3::Python;
use std::future::Future;

/// Drive `fut` to completion from a synchronous PyO3 method, GIL-safely.
///
/// The GIL is RELEASED via [`Python::allow_threads`] BEFORE we block, so we
/// never hold it while awaiting on a tokio runtime — the invariant that keeps
/// PyO3↔tokio from deadlocking (see [scenario 30/32/33][hist]). The future is
/// driven on the ambient runtime via `Handle::block_on` when one is installed
/// (task bodies run on the executor's `spawn_blocking` pool, where a `Handle`
/// is available and blocking is legal); outside any runtime (e.g. Rust-binding
/// unit tests, the admin API called from a bare thread) we fall back to a
/// transient current-thread runtime.
///
/// [hist]: the `project_scenario32_cg_invocation_deadlock` history — never hold
/// the GIL across a `block_on`.
pub(crate) fn py_block_on<F, T>(py: Python<'_>, fut: F) -> T
where
    F: Future<Output = T> + Send,
    T: Send,
{
    py.allow_threads(|| match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.block_on(fut),
        Err(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build fallback current-thread runtime for py_block_on")
            .block_on(fut),
    })
}
