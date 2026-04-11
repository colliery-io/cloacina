# cloacina <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>




## Functions

### `cloacina::setup_test`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn setup_test ()
```

<details>
<summary>Source</summary>

```rust
pub fn setup_test() {
    init_test_logging();
}
```

</details>



### `cloacina::cloaca`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn cloaca (m : & Bound < '_ , PyModule >) -> PyResult < () >
```

<details>
<summary>Source</summary>

```rust
fn cloaca(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Context class
    m.add_class::<python::context::PyContext>()?;

    // Task decorator and handle
    m.add_function(wrap_pyfunction!(python::task::task, m)?)?;
    m.add_class::<python::task::PyTaskHandle>()?;

    // Trigger decorator and result
    m.add_function(wrap_pyfunction!(python::trigger::trigger, m)?)?;
    m.add_class::<python::bindings::trigger::PyTriggerResult>()?;

    // Workflow classes
    m.add_class::<python::workflow::PyWorkflowBuilder>()?;
    m.add_class::<python::workflow::PyWorkflow>()?;
    m.add_function(wrap_pyfunction!(
        python::workflow::register_workflow_constructor,
        m
    )?)?;

    // Runner classes
    m.add_class::<python::bindings::runner::PyDefaultRunner>()?;
    m.add_class::<python::bindings::runner::PyWorkflowResult>()?;
    m.add_class::<python::bindings::context::PyDefaultRunnerConfig>()?;

    // Value objects
    m.add_class::<python::namespace::PyTaskNamespace>()?;
    m.add_class::<python::workflow_context::PyWorkflowContext>()?;
    m.add_class::<python::bindings::value_objects::PyRetryPolicy>()?;
    m.add_class::<python::bindings::value_objects::PyRetryPolicyBuilder>()?;
    m.add_class::<python::bindings::value_objects::PyBackoffStrategy>()?;
    m.add_class::<python::bindings::value_objects::PyRetryCondition>()?;

    // Computation graph builder + node decorator + accumulator decorators
    m.add_class::<python::computation_graph::PyComputationGraphBuilder>()?;
    m.add_function(wrap_pyfunction!(python::computation_graph::node, m)?)?;
    m.add_function(wrap_pyfunction!(
        python::computation_graph::passthrough_accumulator_decorator,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        python::computation_graph::stream_accumulator_decorator,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        python::computation_graph::polling_accumulator_decorator,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        python::computation_graph::batch_accumulator_decorator,
        m
    )?)?;

    // Admin classes (postgres only)
    #[cfg(feature = "postgres")]
    {
        m.add_class::<python::bindings::admin::PyDatabaseAdmin>()?;
        m.add_class::<python::bindings::admin::PyTenantConfig>()?;
        m.add_class::<python::bindings::admin::PyTenantCredentials>()?;
    }

    Ok(())
}
```

</details>
