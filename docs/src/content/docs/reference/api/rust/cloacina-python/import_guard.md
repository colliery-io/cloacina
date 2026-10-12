---
title: "cloacina-python::import_guard"
description: "Supervision for packaged-Python module imports (CLOACI-T-0919)."
sidebar:
  order: 32
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

## Structs

### <span id="struct.ImportThreadIdent"></span>`struct ImportThreadIdent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Default`

Shared slot holding the Python-level thread id (`threading.get_ident()`) of an import thread, so the supervisor can target it with `PyThreadState_SetAsyncExc`. `-1` means "not recorded yet".

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `0` | `AtomicI64` |  |

#### Methods

##### <span id="ImportThreadIdent.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn new () -> Arc < Self >
```

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Arc<Self> {
        Arc::new(Self(AtomicI64::new(-1)))
    }
```

</details>



##### <span id="ImportThreadIdent.record"></span>`record`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn record (& self , py : Python < '_ >)
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



##### <span id="ImportThreadIdent.get"></span>`get`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn get (& self) -> Option < i64 >
```

<details>
<summary>Source</summary>

```rust
    fn get(&self) -> Option<i64> {
        match self.0.load(Ordering::SeqCst) {
            -1 => None,
            id => Some(id),
        }
    }
```

</details>





## Enums

### <span id="enum.InterruptOutcome"></span>`enum InterruptOutcome`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


Outcome of an interruption attempt against a hung import thread.

#### Variants

- **`Injected`** - The async exception was installed on the target thread state.
- **`GilUnavailable`** - The GIL could not be acquired inside [`GIL_ACQUIRE_TIMEOUT`] — strong
evidence of a C-level hold that never yields.
- **`NoSuchThread`** - `PyThreadState_SetAsyncExc` found no thread with that id (it already
exited), or the ident was never recorded.



## Functions

### <span id="fn.import_timeout"></span>`fn import_timeout`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn import_timeout () -> Duration
```

Effective import timeout: [`IMPORT_TIMEOUT_ENV`] if it parses as a positive integer, else [`DEFAULT_IMPORT_TIMEOUT_SECS`].

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



### <span id="fn.ensure_python_runtime_usable"></span>`fn ensure_python_runtime_usable`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn ensure_python_runtime_usable (label : & str) -> Result < () , PythonLoaderError >
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



### <span id="fn.inject_async_exception"></span>`fn inject_async_exception`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn inject_async_exception (ident : i64) -> InterruptOutcome
```

Inject [`ImportTimeout`] into the Python thread `ident`, from a scratch thread so a GIL that never becomes available cannot hang the supervisor.

<details>
<summary>Source</summary>

```rust
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
```

</details>



### <span id="fn.join_within"></span>`fn join_within`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn join_within < T > (handle : & std :: thread :: JoinHandle < T > , timeout : Duration) -> bool
```

Poll-join `handle` until it finishes or `deadline` elapses.

<details>
<summary>Source</summary>

```rust
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
```

</details>



### <span id="fn.supervise_import"></span>`fn supervise_import`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn supervise_import < T > (label : & str , handle : std :: thread :: JoinHandle < Result < T , PythonLoaderError > > , ident : Arc < ImportThreadIdent > , timeout : Duration ,) -> Result < T , PythonLoaderError >
```

Wait for a packaged-Python import thread, escalating through the interrupt → surface ladder if it blows the deadline.

`label` names the package (or graph) in logs and in the wedged reason.
`ident` must be the slot the import thread populated via
[`ImportThreadIdent::record`].

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



### <span id="fn.wedge"></span>`fn wedge`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn wedge < T > (label : & str , detail : & str) -> Result < T , PythonLoaderError >
```

Latch the wedged flag and return the corresponding load error.

<details>
<summary>Source</summary>

```rust
fn wedge<T>(label: &str, detail: &str) -> Result<T, PythonLoaderError> {
    let reason = format!("python runtime wedged by package {label} import hang");
    cloacina::python_runtime::mark_python_runtime_wedged(reason.clone());
    Err(PythonLoaderError::RuntimeError(format!(
        "{reason}: {detail}. The embedded Python interpreter cannot be recovered \
         in-process — restart this process and remove the offending package."
    )))
}
```

</details>



