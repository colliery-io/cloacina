---
title: "cloacina-python"
description: "Python integration for Cloacina."
sidebar:
  order: 19
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Python integration for Cloacina.

This module provides PyO3 bindings: [`PyContext`], [`PyWorkflowBuilder`],
[`PyTaskHandle`], [`TaskDecorator`] (`@task`), and [`PythonTaskWrapper`]
(implements [`cloacina::Task`]). The `@task` decorator machinery and
`WorkflowBuilder` context manager live in this crate; the `cloaca` Python
wheel re-exports them via its `#[pymodule]` definition.

## Functions

### <span id="fn.register_authoring"></span>`fn register_authoring`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">pub(crate)</span>


```rust
fn register_authoring (m : & Bound < '_ , PyModule >) -> PyResult < () >
```

Register the `cloaca` AUTHORSHIP contract onto a module (CLOACI-I-0137).

This is the SINGLE source of truth for every symbol the workflow-authoring
surface exposes — `@cloaca.task`, `@cloaca.workflow`, the trigger/reactor/
accumulator decorators, `RetryPolicy`, `cloaca.var`, and so on. BOTH entry
points call it: the maturin `#[pymodule]` below (the pip wheel) and the
synthetic `loader::ensure_cloaca_module` (the embedded server). Registering
here — and ONLY here — makes wheel/server symbol drift structurally
impossible (the class of bug behind the `state_accumulator` production
failure and the latent `py_var` wheel gap).
`cloaca` is PURELY an authorship surface. Host/embedding-only symbols
(`PyDefaultRunner`, the admin classes) are deliberately NOT part of this
contract and live only in the wheel `#[pymodule]`; the server never exposes
a runner because the server IS the runner.

<details>
<summary>Source</summary>

```rust
pub(crate) fn register_authoring(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // CLOACI-T-0896: wire packaged polling accumulators to their in-process
    // Python poll fn (no FFI). Idempotent across embeddings.
    computation_graph::install_polling_accumulator_builder();

    m.add_class::<context::PyContext>()?;

    m.add_function(wrap_pyfunction!(task::task, m)?)?;
    m.add_class::<task::PyTaskHandle>()?;
    m.add_class::<task::TaskDecorator>()?;

    // CLOACI-T-0831: packaged constructor-provider consumption (mirrors Rust's `constructor!`).
    m.add_function(wrap_pyfunction!(constructor::constructor, m)?)?;

    // CLOACI-T-0763: Python trigger-rule builders (parity with Rust's DSL).
    m.add_function(wrap_pyfunction!(trigger_rules::context_value, m)?)?;
    m.add_function(wrap_pyfunction!(trigger_rules::task_success, m)?)?;
    m.add_function(wrap_pyfunction!(trigger_rules::task_failed, m)?)?;
    m.add_function(wrap_pyfunction!(trigger_rules::task_skipped, m)?)?;
    m.add_function(wrap_pyfunction!(trigger_rules::all_of, m)?)?;
    m.add_function(wrap_pyfunction!(trigger_rules::any_of, m)?)?;
    m.add_function(wrap_pyfunction!(trigger_rules::none_of, m)?)?;
    m.add_function(wrap_pyfunction!(trigger_rules::always, m)?)?;

    m.add_function(wrap_pyfunction!(trigger::trigger, m)?)?;
    m.add_class::<bindings::trigger::PyTriggerResult>()?;
    m.add_class::<trigger::TriggerDecorator>()?;

    m.add_function(wrap_pyfunction!(reactor::reactor, m)?)?;

    m.add_class::<workflow::PyWorkflowBuilder>()?;
    m.add_class::<workflow::PyWorkflow>()?;
    m.add_function(wrap_pyfunction!(workflow::py_register_workflow, m)?)?;
    // CLOACI-T-0760: declared-params decorator (runtime no-op; parsed from source).
    m.add_class::<workflow::PyWorkflowParamsDecorator>()?;
    m.add_function(wrap_pyfunction!(workflow::workflow_params, m)?)?;
    // CLOACI-T-0859: declared-secrets decorator (runtime no-op; parsed from source).
    m.add_class::<workflow::PyWorkflowSecretsDecorator>()?;
    m.add_function(wrap_pyfunction!(workflow::workflow_secrets, m)?)?;
    m.add_function(wrap_pyfunction!(workflow::boundary_schema, m)?)?;

    m.add_class::<namespace::PyTaskNamespace>()?;
    m.add_class::<workflow_context::PyWorkflowContext>()?;

    m.add_class::<bindings::value_objects::PyRetryPolicy>()?;
    m.add_class::<bindings::value_objects::PyRetryPolicyBuilder>()?;
    m.add_class::<bindings::value_objects::PyBackoffStrategy>()?;
    m.add_class::<bindings::value_objects::PyRetryCondition>()?;

    m.add_class::<computation_graph::PyComputationGraphBuilder>()?;
    m.add_function(wrap_pyfunction!(computation_graph::node, m)?)?;
    m.add_function(wrap_pyfunction!(
        computation_graph::passthrough_accumulator_decorator,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        computation_graph::stream_accumulator_decorator,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        computation_graph::polling_accumulator_decorator,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        computation_graph::batch_accumulator_decorator,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        computation_graph::state_accumulator_decorator,
        m
    )?)?;

    // Variable registry — `cloaca.var` / `cloaca.var_or`.
    m.add_function(wrap_pyfunction!(loader::py_var, m)?)?;
    m.add_function(wrap_pyfunction!(loader::py_var_or, m)?)?;

    Ok(())
}
```

</details>



### <span id="fn.cloaca"></span>`fn cloaca`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn cloaca (m : & Bound < '_ , PyModule >) -> PyResult < () >
```

<details>
<summary>Source</summary>

```rust
fn cloaca(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Install a process-default ScopedRuntime for standalone Python usage
    // (tutorials, scripts, tests that import cloaca directly). Without this,
    // module-level `@cloaca.task` and `WorkflowBuilder.__exit__` fail because
    // the decorator/lifecycle code looks up the current Runtime via the
    // thread-local slot in `runtime_scope`. The server loader path installs
    // its own ScopedRuntime on a dedicated loader thread; this default is
    // thread-local to the import thread and does not conflict with that.
    if runtime_scope::current_runtime().is_none() {
        let rt = std::sync::Arc::new(cloacina::Runtime::empty());
        let guard = runtime_scope::ScopedRuntime::new(rt)
            .expect("install process-default Runtime in cloaca pymodule init");
        std::mem::forget(guard);
    }

    // The shared authorship contract — the single source of truth (see
    // `register_authoring`; the server's `ensure_cloaca_module` calls the same fn).
    register_authoring(m)?;

    // Host/embedding-only surface — the pip wheel additionally embeds a runner
    // and admin API. These are NOT part of the authorship contract, so the
    // synthetic server module deliberately omits them.
    m.add_class::<bindings::runner::PyDefaultRunner>()?;
    m.add_class::<bindings::runner::PyWorkflowResult>()?;
    m.add_class::<bindings::context::PyDefaultRunnerConfig>()?;

    // CLOACI-I-0140 (segfault class): join ALL runtime threads via atexit,
    // BEFORE interpreter finalization begins. This is the structural guarantee
    // that no runtime thread can decref a PyObject into a finalizing
    // interpreter — a user forgetting `runner.shutdown()` is now safe.
    m.add_function(wrap_pyfunction!(
        bindings::runner::_shutdown_all_runners,
        m
    )?)?;
    let atexit = m.py().import("atexit")?;
    atexit.call_method1("register", (m.getattr("_shutdown_all_runners")?,))?;

    #[cfg(feature = "postgres")]
    {
        m.add_class::<bindings::admin::PyDatabaseAdmin>()?;
        m.add_class::<bindings::admin::PyTenantConfig>()?;
        m.add_class::<bindings::admin::PyTenantCredentials>()?;
    }

    Ok(())
}
```

</details>



