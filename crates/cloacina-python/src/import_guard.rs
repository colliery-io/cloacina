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

//! Supervision for packaged-Python module imports (CLOACI-T-0919).
//!
//! User code runs arbitrary statements at module scope during a package
//! import. If it never returns, the import thread never returns either — and
//! because cloacina embeds ONE CPython interpreter per process, an abandoned
//! import thread that keeps the GIL takes the whole Python subsystem with it.
//! Before this module the only mitigation was a poll-join timeout that
//! *returned* to the caller while leaving the thread running: the package load
//! reported failure and the process quietly lost the ability to load any
//! Python package until restart.
//!
//! # The ladder
//!
//! 1. **Interrupt.** A *pure-Python* infinite loop does NOT hold the GIL
//!    exclusively — CPython's eval breaker switches threads every few
//!    milliseconds — so another thread can acquire the GIL and inject an
//!    asynchronous exception into the hung thread with
//!    `PyThreadState_SetAsyncExc`. The exception is raised at the hung
//!    thread's next bytecode boundary, the import unwinds, the thread joins,
//!    and the interpreter is left healthy. This recovers the overwhelmingly
//!    common case (`while True: pass`, a runaway comprehension, a spin-wait).
//!
//! 2. **Surface.** Interruption cannot beat everything: a hang inside a C
//!    extension that never returns to the eval loop never reaches a bytecode
//!    boundary, and code that catches `BaseException` inside its own loop can
//!    swallow the injected exception. Those are indistinguishable from here.
//!    When the grace join fails we latch the process-global wedged flag
//!    (`cloacina::python_runtime::mark_python_runtime_wedged`), which drives
//!    the `cloacina_python_runtime_wedged` gauge, a loud `error!`, and a
//!    failing `/ready` — the operator sees a broken replica instead of
//!    silently vanishing Python support.
//!
//! Note on `time.sleep`: it releases the GIL, so a sleeping thread does not
//! block the interrupter, but the async exception only lands when the sleep
//! returns. A long sleep therefore looks like a wedge until it finishes.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use pyo3::prelude::*;
use pyo3::type_object::PyTypeInfo;

use crate::loader::PythonLoaderError;

/// Default timeout for Python module import (seconds).
pub const DEFAULT_IMPORT_TIMEOUT_SECS: u64 = 60;

/// Environment override for the import timeout. Exists so tests (and
/// operators with pathologically slow packages) don't have to live with the
/// 60s default; a hostile-fixture test cannot wait a minute.
pub const IMPORT_TIMEOUT_ENV: &str = "CLOACINA_PYTHON_IMPORT_TIMEOUT_SECS";

/// How long the interrupter thread gets to acquire the GIL. Failing to get it
/// in this window is itself evidence of a C-level hold that never returns to
/// the eval loop (a pure-Python loop yields the GIL every few ms).
const GIL_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);

/// How long the hung thread gets to unwind after the async exception lands.
const INTERRUPT_GRACE: Duration = Duration::from_secs(5);

/// Async-exception injection attempts before declaring the runtime wedged.
/// Two, not one: the exception is dropped if the target is inside a C call
/// when it is set, so a second attempt after the grace window catches threads
/// that returned to the eval loop in between.
const INTERRUPT_ATTEMPTS: u32 = 2;

/// Poll granularity for the join loops.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

pyo3::create_exception!(
    cloaca,
    ImportTimeout,
    pyo3::exceptions::PyBaseException,
    "Raised inside a packaged-workflow import thread that exceeded the \
     import deadline. Inherits BaseException so a module-scope `except \
     Exception` cannot swallow the deadline."
);

/// Effective import timeout: [`IMPORT_TIMEOUT_ENV`] if it parses as a positive
/// integer, else [`DEFAULT_IMPORT_TIMEOUT_SECS`].
pub fn import_timeout() -> Duration {
    match std::env::var(IMPORT_TIMEOUT_ENV) {
        Ok(raw) => match raw.trim().parse::<u64>() {
            Ok(secs) if secs > 0 => Duration::from_secs(secs),
            _ => {
                tracing::warn!(
                    value = %raw,
                    "{IMPORT_TIMEOUT_ENV} is not a positive integer — using the {DEFAULT_IMPORT_TIMEOUT_SECS}s default"
                );
                Duration::from_secs(DEFAULT_IMPORT_TIMEOUT_SECS)
            }
        },
        Err(_) => Duration::from_secs(DEFAULT_IMPORT_TIMEOUT_SECS),
    }
}

/// Refuse to start an import when the interpreter is already known-wedged.
///
/// Without this, every later load spawns a thread that blocks forever trying
/// to acquire a GIL nobody will release — leaking a thread per attempt and
/// turning a clear failure into a hang. Fail fast with the original cause
/// instead; only a restart fixes the wedge.
pub fn ensure_python_runtime_usable(label: &str) -> Result<(), PythonLoaderError> {
    match cloacina::python_runtime::python_runtime_wedged_reason() {
        Some(reason) => Err(PythonLoaderError::RuntimeError(format!(
            "refusing to import '{label}': {reason}. The embedded Python interpreter \
             is unrecoverable in this process — restart it."
        ))),
        None => Ok(()),
    }
}

/// Shared slot holding the Python-level thread id (`threading.get_ident()`) of
/// an import thread, so the supervisor can target it with
/// `PyThreadState_SetAsyncExc`. `-1` means "not recorded yet".
#[derive(Debug, Default)]
pub struct ImportThreadIdent(AtomicI64);

impl ImportThreadIdent {
    pub fn new() -> Arc<Self> {
        Arc::new(Self(AtomicI64::new(-1)))
    }

    /// Record the CURRENT thread's Python ident. Called from inside the import
    /// thread once it holds the GIL, before any user code runs.
    pub fn record(&self, py: Python<'_>) {
        match py
            .import("threading")
            .and_then(|t| t.call_method0("get_ident"))
            .and_then(|id| id.extract::<i64>())
        {
            Ok(ident) => self.0.store(ident, Ordering::SeqCst),
            Err(e) => tracing::warn!(
                error = %e,
                "could not record Python thread ident — an import hang on this \
                 thread will not be interruptible"
            ),
        }
    }

    fn get(&self) -> Option<i64> {
        match self.0.load(Ordering::SeqCst) {
            -1 => None,
            id => Some(id),
        }
    }
}

/// Outcome of an interruption attempt against a hung import thread.
#[derive(Debug, PartialEq, Eq)]
enum InterruptOutcome {
    /// The async exception was installed on the target thread state.
    Injected,
    /// The GIL could not be acquired inside [`GIL_ACQUIRE_TIMEOUT`] — strong
    /// evidence of a C-level hold that never yields.
    GilUnavailable,
    /// `PyThreadState_SetAsyncExc` found no thread with that id (it already
    /// exited), or the ident was never recorded.
    NoSuchThread,
}

/// Inject [`ImportTimeout`] into the Python thread `ident`, from a scratch
/// thread so a GIL that never becomes available cannot hang the supervisor.
fn inject_async_exception(ident: i64) -> InterruptOutcome {
    let worker = std::thread::spawn(move || -> i32 {
        Python::with_gil(|py| {
            // SAFETY: the GIL is held for the duration of this call (required
            // by PyThreadState_SetAsyncExc, which walks the interpreter's
            // thread-state list). `exc` is a borrowed reference to a type
            // object that outlives the call — CPython INCREFs it itself.
            let exc = ImportTimeout::type_object(py);
            let modified =
                unsafe { pyo3::ffi::PyThreadState_SetAsyncExc(ident as _, exc.as_ptr()) };

            // Documented revert protocol: a return value > 1 means the call
            // did something unexpected (more than one thread state matched);
            // the caller must clear the pending exception by calling again
            // with NULL, or those threads carry an exception nobody asked for.
            if modified > 1 {
                unsafe {
                    pyo3::ffi::PyThreadState_SetAsyncExc(ident as _, std::ptr::null_mut());
                }
            }
            modified
        })
    });

    let deadline = Instant::now() + GIL_ACQUIRE_TIMEOUT;
    while Instant::now() < deadline {
        if worker.is_finished() {
            return match worker.join() {
                Ok(1) => InterruptOutcome::Injected,
                Ok(0) => InterruptOutcome::NoSuchThread,
                // >1 was reverted above; treat as "not usefully injected".
                Ok(_) | Err(_) => InterruptOutcome::NoSuchThread,
            };
        }
        std::thread::sleep(POLL_INTERVAL);
    }
    // The scratch thread is abandoned; it is parked in PyGILState_Ensure and
    // will finish harmlessly if the GIL is ever released.
    InterruptOutcome::GilUnavailable
}

/// Poll-join `handle` until it finishes or `deadline` elapses.
fn join_within<T>(handle: &std::thread::JoinHandle<T>, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if handle.is_finished() {
            return true;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
    handle.is_finished()
}

/// Wait for a packaged-Python import thread, escalating through the
/// interrupt → surface ladder if it blows the deadline.
///
/// `label` names the package (or graph) in logs and in the wedged reason.
/// `ident` must be the slot the import thread populated via
/// [`ImportThreadIdent::record`].
pub fn supervise_import<T>(
    label: &str,
    handle: std::thread::JoinHandle<Result<T, PythonLoaderError>>,
    ident: Arc<ImportThreadIdent>,
    timeout: Duration,
) -> Result<T, PythonLoaderError> {
    // Happy path: the import finishes inside the deadline.
    if join_within(&handle, timeout) {
        return handle.join().map_err(|_| {
            PythonLoaderError::RuntimeError("Python import thread panicked".to_string())
        })?;
    }

    tracing::error!(
        package = %label,
        timeout_secs = timeout.as_secs(),
        "Python import exceeded its deadline — attempting to interrupt the \
         import thread (CLOACI-T-0919)"
    );

    let Some(thread_ident) = ident.get() else {
        return wedge(
            label,
            "import thread ident was never recorded (hang before the GIL was acquired)",
        );
    };

    for attempt in 1..=INTERRUPT_ATTEMPTS {
        match inject_async_exception(thread_ident) {
            InterruptOutcome::Injected => {
                if join_within(&handle, INTERRUPT_GRACE) {
                    // Recovered: the thread unwound, the GIL is back, the
                    // interpreter is healthy. Only THIS package load fails.
                    cloacina::python_runtime::record_python_import_interrupted();
                    tracing::error!(
                        package = %label,
                        attempt,
                        "Python import hang INTERRUPTED — the import thread unwound and \
                         the interpreter stayed healthy. The package failed to load; its \
                         module-scope code must not block."
                    );
                    // Drain the thread's own result (an ImportTimeout PyErr)
                    // and replace it with a message the operator can act on.
                    let _ = handle.join();
                    return Err(PythonLoaderError::RuntimeError(format!(
                        "Python import for '{label}' timed out after {}s and was interrupted — \
                         module-scope code must not block (the import thread was recovered)",
                        timeout.as_secs()
                    )));
                }
                tracing::warn!(
                    package = %label,
                    attempt,
                    "async exception was injected but the import thread did not unwind \
                     within the grace window"
                );
            }
            InterruptOutcome::GilUnavailable => {
                return wedge(
                    label,
                    "the GIL could not be acquired to interrupt the import thread \
                     (C-level hold that never returns to the eval loop)",
                );
            }
            InterruptOutcome::NoSuchThread => {
                // The thread may have exited between the deadline check and
                // the injection; give the join one more chance before wedging.
                if join_within(&handle, INTERRUPT_GRACE) {
                    return handle.join().map_err(|_| {
                        PythonLoaderError::RuntimeError("Python import thread panicked".to_string())
                    })?;
                }
                return wedge(
                    label,
                    "the import thread's Python thread state could not be found",
                );
            }
        }
    }

    wedge(
        label,
        "the import thread ignored the injected timeout exception (C-level hang, \
         or module-scope code swallowing BaseException)",
    )
}

/// Latch the wedged flag and return the corresponding load error.
fn wedge<T>(label: &str, detail: &str) -> Result<T, PythonLoaderError> {
    let reason = format!("python runtime wedged by package {label} import hang");
    cloacina::python_runtime::mark_python_runtime_wedged(reason.clone());
    Err(PythonLoaderError::RuntimeError(format!(
        "{reason}: {detail}. The embedded Python interpreter cannot be recovered \
         in-process — restart this process and remove the offending package."
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_timeout_defaults_without_env() {
        // The env var is not set in the default test environment.
        if std::env::var(IMPORT_TIMEOUT_ENV).is_err() {
            assert_eq!(
                import_timeout(),
                Duration::from_secs(DEFAULT_IMPORT_TIMEOUT_SECS)
            );
        }
    }

    #[test]
    fn ident_slot_starts_empty() {
        let slot = ImportThreadIdent::new();
        assert_eq!(slot.get(), None);
    }
}
