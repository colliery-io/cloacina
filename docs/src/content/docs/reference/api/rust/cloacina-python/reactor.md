---
title: "cloacina-python::reactor"
description: "`@cloaca.reactor` class decorator — Python parity for Rust's `#[reactor]`."
editUrl: false
sidebar:
  order: 20
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

`@cloaca.reactor` class decorator — Python parity for Rust's `#[reactor]`.

Mirrors the Rust attribute macro: the decorated class becomes a handle that
carries `NAME`, `ACCUMULATORS`, and `REACTION_MODE` class attributes, and a
`ReactorRegistration` is registered into the active scoped Runtime.

```python
@cloaca.reactor(
    name="risk_signals",
    accumulators=["alpha", "beta"],
    mode="when_any",
)
class RiskSignals:
    pass
```

The class is then referenced by the (forthcoming) `reactor=ReactorClass`
kwarg on `ComputationGraphBuilder` and by `@cloaca.task(invokes=...)`.

## Re-exports

- [`dispatch_runtime_reactors_into_scheduler`](../../../rust/cloacina/computation_graph/packaging_bridge/#fn.dispatch_runtime_reactors_into_scheduler)

## Functions

### <span id="fn.reactor"></span><code style="font-size: 0.85em">fn reactor</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.reactor`](../../../cloaca/#fn.reactor)

```rust
fn reactor(
    py: Python<'_>,
    name: String,
    accumulators: Vec<String>,
    mode: String,
) -> PyResult<PyObject>
```

`@cloaca.reactor(name=..., accumulators=[...], mode="when_any"|"when_all")`.

Returns a decorator that, when applied to a class, validates the inputs,
sets `NAME`/`ACCUMULATORS`/`REACTION_MODE` on the class, and registers a
`ReactorRegistration` into the active scoped Runtime.

<details>
<summary>Source</summary>

```rust
pub fn reactor(
    py: Python<'_>,
    name: String,
    accumulators: Vec<String>,
    mode: String,
) -> PyResult<PyObject> {
    if name.is_empty() {
        return Err(PyValueError::new_err(
            "@cloaca.reactor: 'name' cannot be empty",
        ));
    }
    if accumulators.is_empty() {
        return Err(PyValueError::new_err(
            "@cloaca.reactor: 'accumulators' list cannot be empty",
        ));
    }
    let mut seen = std::collections::HashSet::new();
    for acc in &accumulators {
        if !seen.insert(acc.clone()) {
            return Err(PyValueError::new_err(format!(
                "@cloaca.reactor: accumulator '{}' listed more than once",
                acc
            )));
        }
    }
    let reaction_mode = match mode.as_str() {
        "when_any" => ReactionMode::WhenAny,
        "when_all" => ReactionMode::WhenAll,
        other => {
            return Err(PyValueError::new_err(format!(
                "@cloaca.reactor: unknown mode '{}', expected 'when_any' or 'when_all'",
                other
            )));
        }
    };

    // Capture state for the decorator closure.
    let name_clone = name.clone();
    let accumulators_clone = accumulators.clone();

    let decorator = pyo3::types::PyCFunction::new_closure(
        py,
        None,
        None,
        move |args: &Bound<'_, PyTuple>,
              _kwargs: Option<&Bound<'_, pyo3::types::PyDict>>|
              -> PyResult<PyObject> {
            let py = args.py();
            let cls_obj = args.get_item(0)?;

            // Require a class (type). Reject instances / functions to give a clear error.
            let cls = cls_obj.downcast::<PyType>().map_err(|_| {
                PyTypeError::new_err(
                    "@cloaca.reactor must be applied to a class (e.g. `class Rx: pass`)",
                )
            })?;

            // Set class attributes — these mirror Rust's `Reactor` trait associated consts.
            cls.setattr("NAME", PyString::new(py, &name_clone))?;
            cls.setattr("ACCUMULATORS", PyList::new(py, &accumulators_clone)?)?;
            cls.setattr("REACTION_MODE", PyString::new(py, reaction_mode.as_str()))?;

            // Register into the active scoped Runtime.
            let rt = current_runtime().ok_or_else(|| {
                PyValueError::new_err(
                    "@cloaca.reactor: no scoped Runtime installed — \
                     decorator must run inside a process or loader thread \
                     that has installed a ScopedRuntime",
                )
            })?;

            let reg_name = name_clone.clone();
            let reg_accs = accumulators_clone.clone();
            let reg_mode = reaction_mode;
            rt.register_reactor(reg_name.clone(), move || {
                // CLOACI-T-0839: carry the module's authored accumulator kinds
                // (`@cloaca.state_accumulator(capacity=N)` etc.) on the
                // registration, so runtime-walking load paths (packaged Python
                // via the reconciler) spawn the right accumulator runtime
                // instead of defaulting to passthrough. Resolved lazily — by
                // the time anyone calls `get_reactor`, all module decorators
                // have run regardless of declaration order.
                let accumulator_specs = crate::computation_graph::get_registered_accumulators()
                    .into_iter()
                    .filter(|areg| reg_accs.contains(&areg.name))
                    .map(|areg| {
                        let mut config = areg.config.clone();
                        // state capacity rides the config map (the same key
                        // `state_capacity_from_config` reads host-side).
                        if areg.accumulator_type == "state" {
                            config
                                .entry("capacity".to_string())
                                .or_insert_with(|| areg.capacity.to_string());
                        }
                        cloacina_computation_graph::AccumulatorSpec {
                            name: areg.name,
                            accumulator_type: areg.accumulator_type,
                            config,
                        }
                    })
                    .collect();
                ReactorRegistration {
                    name: reg_name.clone(),
                    accumulator_names: reg_accs.clone(),
                    accumulator_specs,
                    reaction_mode: reg_mode,
                    // CLOACI-T-0830: the Python reactor path doesn't (yet) author
                    // WASM reactor constructors — native dirty-flag firing only.
                    constructor: None,
                }
            });

            tracing::debug!(
                "Registered Python reactor: name={} accumulators={:?} mode={}",
                name_clone,
                accumulators_clone,
                reaction_mode.as_str()
            );

            Ok(cls.clone().unbind().into())
        },
    )?;

    Ok(decorator.into())
}
```

</details>
