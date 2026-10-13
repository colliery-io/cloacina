---
title: "cloacina-python::import_guard"
description: "Supervision for packaged-Python module imports (CLOACI-T-0919)."
editUrl: false
sidebar:
  order: 16
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Supervision for packaged-Python module imports (CLOACI-T-0919).

User code runs arbitrary statements at module scope during a package
import. If it never returns, the import thread never returns either — and
because cloacina embeds ONE CPython interpreter per process, an abandoned
import thread that keeps the GIL takes the whole Python subsystem with it.
Before this module the only mitigation was a poll-join timeout that
*returned* to the caller while leaving the thread running: the package load
reported failure and the process quietly lost the ability to load any
Python package until restart.

**The ladder**

1. **Interrupt.** A *pure-Python* infinite loop does NOT hold the GIL
   exclusively — CPython's eval breaker switches threads every few
   milliseconds — so another thread can acquire the GIL and inject an
   asynchronous exception into the hung thread with
   `PyThreadState_SetAsyncExc`. The exception is raised at the hung
   thread's next bytecode boundary, the import unwinds, the thread joins,
   and the interpreter is left healthy. This recovers the overwhelmingly
   common case (`while True: pass`, a runaway comprehension, a spin-wait).

2. **Surface.** Interruption cannot beat everything: a hang inside a C
   extension that never returns to the eval loop never reaches a bytecode
   boundary, and code that catches `BaseException` inside its own loop can
   swallow the injected exception. Those are indistinguishable from here.
   When the grace join fails we latch the process-global wedged flag
   (`cloacina::python_runtime::mark_python_runtime_wedged`), which drives
   the `cloacina_python_runtime_wedged` gauge, a loud `error!`, and a
   failing `/ready` — the operator sees a broken replica instead of
   silently vanishing Python support.

Note on `time.sleep`: it releases the GIL, so a sleeping thread does not
block the interrupter, but the async exception only lands when the sleep
returns. A long sleep therefore looks like a wedge until it finishes.

## Structs

### <span id="struct.ImportThreadIdent"></span><code style="font-size: 0.85em">struct ImportThreadIdent</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Default`

Shared slot holding the Python-level thread id (`threading.get_ident()`) of an import thread, so the supervisor can target it with `PyThreadState_SetAsyncExc`. `-1` means "not recorded yet".

#### Methods

##### <span id="ImportThreadIdent.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new() -> Arc<Self>
```

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Arc<Self> {
        Arc::new(Self(AtomicI64::new(-1)))
    }
```

</details>

##### <span id="ImportThreadIdent.record"></span><code>record</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn record(&self, py: Python<'_>)
```

Record the CURRENT thread's Python ident. Called from inside the import thread once it holds the GIL, before any user code runs.

<details>
<summary>Source</summary>

```rust
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
```

</details>

## Functions

### <span id="fn.import_timeout"></span><code style="font-size: 0.85em">fn import_​timeout</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn import_timeout() -> Duration
```

Effective import timeout: [`IMPORT_TIMEOUT_ENV`](../../../rust/cloacina-python/import_guard/#const.IMPORT_TIMEOUT_ENV) if it parses as a positive integer, else [`DEFAULT_IMPORT_TIMEOUT_SECS`](../../../rust/cloacina-python/import_guard/#const.DEFAULT_IMPORT_TIMEOUT_SECS).

<details>
<summary>Source</summary>

```rust
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
```

</details>

### <span id="fn.ensure_python_runtime_usable"></span><code style="font-size: 0.85em">fn ensure_​python_​runtime_​usable</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn ensure_python_runtime_usable(
    label: &str) -> Result<(,
) -> Result<(), PythonLoaderError>
```

Refuse to start an import when the interpreter is already known-wedged.

Without this, every later load spawns a thread that blocks forever trying
to acquire a GIL nobody will release — leaking a thread per attempt and
turning a clear failure into a hang. Fail fast with the original cause
instead; only a restart fixes the wedge.

<details>
<summary>Source</summary>

```rust
pub fn ensure_python_runtime_usable(label: &str) -> Result<(), PythonLoaderError> {
    match cloacina::python_runtime::python_runtime_wedged_reason() {
        Some(reason) => Err(PythonLoaderError::RuntimeError(format!(
            "refusing to import '{label}': {reason}. The embedded Python interpreter \
             is unrecoverable in this process — restart it."
        ))),
        None => Ok(()),
    }
}
```

</details>

### <span id="fn.supervise_import"></span><code style="font-size: 0.85em">fn supervise_​import</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn supervise_import<T>(
    label: &str,
    handle: std::thread::JoinHandle<Result<T, PythonLoaderError>>,
    ident: Arc<ImportThreadIdent>,
    timeout: Duration,
) -> Result<T, PythonLoaderError>
```

Wait for a packaged-Python import thread, escalating through the interrupt → surface ladder if it blows the deadline.

`label` names the package (or graph) in logs and in the wedged reason.
`ident` must be the slot the import thread populated via
[`ImportThreadIdent::record`](../../../rust/cloacina-python/import_guard/#ImportThreadIdent.record).

<details>
<summary>Source</summary>

```rust
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
```

</details>

## Constants

| Name | Type | Value | Description |
|------|------|-------|-------------|
| <span id="const.DEFAULT_IMPORT_TIMEOUT_SECS"></span>`DEFAULT_IMPORT_TIMEOUT_SECS` | `u64` | `60` | Default timeout for Python module import (seconds). |
| <span id="const.IMPORT_TIMEOUT_ENV"></span>`IMPORT_TIMEOUT_ENV` | `&str` | `"CLOACINA_PYTHON_IMPORT_TIMEOUT_SECS"` | Environment override for the import timeout. Exists so tests (and operators with pathologically slow packages) don't have to live with the 60s default; a hostile-fixture test cannot wait a minute. |
