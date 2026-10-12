---
title: "cloacina-python::computation_graph"
description: "Python computation graph bindings."
sidebar:
  order: 27
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Python computation graph bindings.

Mirrors the WorkflowBuilder + @task pattern. Two flavors:
1. **Reactor-triggered** — the graph is bound to a `@cloaca.reactor` class.
Entry nodes consume the reactor's accumulator payloads.
```python
@cloaca.reactor(name="market_maker",
accumulators=["alpha", "beta"],
mode="when_any")
class MarketMaker: pass
with cloaca.ComputationGraphBuilder("market_maker",
reactor=MarketMaker,
graph={...}) as builder: ...
```
2. **Trigger-less** — the graph is invoked by a workflow task via
`@cloaca.task(invokes=...)`. Entry nodes take only the task context;
no accumulator/cache inputs are permitted.
```python
with cloaca.ComputationGraphBuilder("score_transactions",
graph={"score": {}}) as score_graph: ...
```
Pre-split (bundled) form:
```python
with cloaca.ComputationGraphBuilder("market_maker",
react={"mode": "when_any", "accumulators": ["alpha", "beta"]},
graph={
"decision": {"inputs": ["alpha", "beta"], "routes": {
"Signal": "signal_handler",
"NoAction": "audit_logger",
}},
"signal_handler": {},
"audit_logger": {},
}
) as builder:
@cloaca.node
def decision(alpha, beta):
if alpha["value"] + beta["estimate"] > 10:
return ("Signal", {"output": alpha["value"] + beta["estimate"]})
return ("NoAction", {"reason": "below threshold"})
@cloaca.node
def signal_handler(signal):
return {"published": True}
@cloaca.node
def audit_logger(reason):
return {"logged": True}
```

## Structs

### <span id="struct.PyAccumulatorRegistration"></span>`struct PyAccumulatorRegistration`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`

Metadata for a registered Python accumulator.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `accumulator_type` | `String` |  |
| `config` | `HashMap < String , String >` |  |
| `capacity` | `i32` | Capacity for `state` accumulators (bounded buffer). `> 0` bounded,
`< 0` unbounded, `0` write-only sink. Defaults to `0` for the other
accumulator types, which ignore it. |



### <span id="struct.GraphKey"></span>`struct GraphKey`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`

CLOACI-T-0921: process-global Python registries are keyed by `(tenant, package, name)`, not by bare name.

The `@graph` / `@*_accumulator` decorators run at import time; the loader
installs a [`RegistrationScope`] around each package import
([`crate::registration_scope`]) so a registration is stamped with whose it
is. Two tenants shipping a graph called `pipeline` used to share one
executor slot — last import wins, cross-tenant.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `Option < String >` |  |
| `package` | `Option < String >` |  |
| `name` | `String` |  |

#### Methods

##### <span id="GraphKey.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn new (scope : & RegistrationScope , name : & str) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(scope: &RegistrationScope, name: &str) -> Self {
        Self {
            tenant_id: scope.tenant_id.clone(),
            package: scope.package.clone(),
            name: name.to_string(),
        }
    }
```

</details>



##### <span id="GraphKey.current"></span>`current`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn current (name : & str) -> Self
```

The key this name would take under the thread's current scope.

<details>
<summary>Source</summary>

```rust
    pub fn current(name: &str) -> Self {
        Self::new(&current_registration_scope(), name)
    }
```

</details>



##### <span id="GraphKey.matches_scope"></span>`matches_scope`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn matches_scope (& self , scope : & RegistrationScope) -> bool
```

<details>
<summary>Source</summary>

```rust
    fn matches_scope(&self, scope: &RegistrationScope) -> bool {
        self.tenant_id == scope.tenant_id && self.package == scope.package
    }
```

</details>



##### <span id="GraphKey.same_tenant"></span>`same_tenant`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn same_tenant (& self , scope : & RegistrationScope) -> bool
```

<details>
<summary>Source</summary>

```rust
    fn same_tenant(&self, scope: &RegistrationScope) -> bool {
        self.tenant_id == scope.tenant_id
    }
```

</details>



##### <span id="GraphKey.describe"></span>`describe`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn describe (& self) -> String
```

<details>
<summary>Source</summary>

```rust
    pub fn describe(&self) -> String {
        format!(
            "{}::{}::{}",
            self.tenant_id.as_deref().unwrap_or("<untenanted>"),
            self.package.as_deref().unwrap_or("<unpackaged>"),
            self.name
        )
    }
```

</details>





### <span id="struct.PyNodeDecl"></span>`struct PyNodeDecl`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


**Derives:** `Debug`, `Clone`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `cache_inputs` | `Vec < String >` |  |
| `edge` | `PyEdgeDecl` |  |



### <span id="struct.PyComputationGraphBuilder"></span>`class ComputationGraphBuilder`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.computation_graph.ComputationGraphBuilder](../../../cloacina-python/computation_graph/#class.ComputationGraphBuilder)

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `reactor_binding` | `Option < ReactorBinding >` | Reactor binding for the split form; `None` means trigger-less. |
| `nodes_decl` | `Vec < PyNodeDecl >` |  |

#### Methods

##### <span id="PyComputationGraphBuilder.new"></span>`new`

```rust
fn new (_py : Python < '_ > , name : & str , graph : & Bound < '_ , PyDict > , reactor : Option < & Bound < '_ , PyAny > > , react : Option < & Bound < '_ , PyAny > > ,) -> PyResult < Self >
```

> **Python API**: [cloacina-python.computation_graph.ComputationGraphBuilder.new](../../../cloacina-python/computation_graph/#ComputationGraphBuilder.new)

<details>
<summary>Source</summary>

```rust
    pub fn new(
        _py: Python<'_>,
        name: &str,
        graph: &Bound<'_, PyDict>,
        reactor: Option<&Bound<'_, PyAny>>,
        react: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        // Hard-break migration: bundled `react={...}` form is gone — point users
        // at I-0101 with a clear error.
        if react.is_some() {
            return Err(PyValueError::new_err(
                "ComputationGraphBuilder no longer accepts the bundled `react={...}` kwarg. \
                 Declare a `@cloaca.reactor` class and pass `reactor=YourReactor`, \
                 or omit `reactor` entirely for a trigger-less graph invoked by \
                 `@cloaca.task(invokes=...)`. See initiative CLOACI-I-0101.",
            ));
        }

        let reactor_binding = match reactor {
            None => None,
            Some(r) => Some(extract_reactor_binding(r)?),
        };

        let nodes_decl = parse_graph_dict(graph)?;

        Ok(PyComputationGraphBuilder {
            name: name.to_string(),
            reactor_binding,
            nodes_decl,
        })
    }
```

</details>



##### <span id="PyComputationGraphBuilder.NAME"></span>`NAME`

```rust
fn NAME (& self) -> String
```

> **Python API**: [cloacina-python.computation_graph.ComputationGraphBuilder.NAME](../../../cloacina-python/computation_graph/#ComputationGraphBuilder.NAME)

The graph's declared name. Mirrors Rust's `Graph::NAME` so this instance can serve as the handle referenced by `@cloaca.task(invokes=...)`.

<details>
<summary>Source</summary>

```rust
    pub fn NAME(&self) -> String {
        self.name.clone()
    }
```

</details>



##### <span id="PyComputationGraphBuilder.__enter__"></span>`__enter__`

```rust
fn __enter__ (slf : PyRef < Self >) -> PyRef < Self >
```

> **Python API**: [cloacina-python.computation_graph.ComputationGraphBuilder.__enter__](../../../cloacina-python/computation_graph/#ComputationGraphBuilder.__enter__)

Context manager entry — establish graph context for @node decorators

<details>
<summary>Source</summary>

```rust
    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        push_graph_context(slf.name.clone());
        slf
    }
```

</details>



##### <span id="PyComputationGraphBuilder.__exit__"></span>`__exit__`

```rust
fn __exit__ (& self , py : Python , _exc_type : Option < & Bound < PyAny > > , _exc_value : Option < & Bound < PyAny > > , _traceback : Option < & Bound < PyAny > > ,) -> PyResult < bool >
```

> **Python API**: [cloacina-python.computation_graph.ComputationGraphBuilder.__exit__](../../../cloacina-python/computation_graph/#ComputationGraphBuilder.__exit__)

Context manager exit — validate nodes against topology, build executor

<details>
<summary>Source</summary>

```rust
    pub fn __exit__(
        &self,
        py: Python,
        _exc_type: Option<&Bound<PyAny>>,
        _exc_value: Option<&Bound<PyAny>>,
        _traceback: Option<&Bound<PyAny>>,
    ) -> PyResult<bool> {
        pop_graph_context();

        let registered_nodes = drain_nodes();

        // Validate: every node in topology must have a registered function
        for decl in &self.nodes_decl {
            if !registered_nodes.contains_key(&decl.name) {
                return Err(PyAttributeError::new_err(format!(
                    "computation graph '{}' topology references node '{}' but no @cloaca.node function with that name was defined",
                    self.name, decl.name
                )));
            }
        }

        // Validate: every registered function must appear in topology
        for fn_name in registered_nodes.keys() {
            if !self.nodes_decl.iter().any(|d| d.name == *fn_name) {
                return Err(PyValueError::new_err(format!(
                    "function '{}' was decorated with @cloaca.node but does not appear in the graph topology",
                    fn_name
                )));
            }
        }

        // Trigger-less entry contract: in trigger-less form, no node may
        // declare cache `inputs=[...]` — those inputs only exist when a
        // reactor is feeding the graph. Trigger-less graphs receive their
        // input via the task `Context` passed by `@cloaca.task(invokes=...)`.
        if self.reactor_binding.is_none() {
            for decl in &self.nodes_decl {
                if !decl.cache_inputs.is_empty() {
                    return Err(PyValueError::new_err(format!(
                        "trigger-less computation graph '{}': node '{}' declares cache \
                         inputs={:?}, but trigger-less graphs have no accumulator inputs. \
                         Either remove the inputs (graph receives data through the task \
                         context) or bind a reactor with `reactor=YourReactor`.",
                        self.name, decl.name, decl.cache_inputs
                    )));
                }
            }
        } else if let Some(ref binding) = self.reactor_binding {
            // Split-form sanity: every cache input declared by a node must be
            // one of the reactor's accumulators. Catches typos at registration
            // time rather than producing silent `None`s at execution time.
            for decl in &self.nodes_decl {
                for input in &decl.cache_inputs {
                    if !binding.accumulators.contains(input) {
                        return Err(PyValueError::new_err(format!(
                            "computation graph '{}': node '{}' references cache input \
                             '{}' which is not in reactor '{}' accumulators {:?}",
                            self.name, decl.name, input, binding.name, binding.accumulators
                        )));
                    }
                }
            }
        }

        // Build the executor
        let node_map: HashMap<String, PyNodeDecl> = self
            .nodes_decl
            .iter()
            .cloned()
            .map(|n| (n.name.clone(), n))
            .collect();
        let execution_order = compute_execution_order(&self.nodes_decl);

        let (react_mode, accumulators, reactor_name) = match &self.reactor_binding {
            Some(b) => (b.mode.clone(), b.accumulators.clone(), Some(b.name.clone())),
            None => (String::new(), Vec::new(), None),
        };

        let executor = PythonGraphExecutor {
            name: self.name.clone(),
            node_functions: registered_nodes,
            node_map,
            execution_order,
            react_mode,
            accumulators,
            has_reactor: self.reactor_binding.is_some(),
            reactor_name,
        };

        // Register the executor globally (similar to workflow registration)
        register_graph_executor(self.name.clone(), executor, py)?;

        Ok(false)
    }
```

</details>



##### <span id="PyComputationGraphBuilder.__repr__"></span>`__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloacina-python.computation_graph.ComputationGraphBuilder.__repr__](../../../cloacina-python/computation_graph/#ComputationGraphBuilder.__repr__)

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        format!(
            "ComputationGraphBuilder(name='{}', nodes={})",
            self.name,
            self.nodes_decl.len()
        )
    }
```

</details>



##### <span id="PyComputationGraphBuilder.execute"></span>`execute`

```rust
fn execute (& self , py : Python < '_ > , inputs : & Bound < '_ , PyDict >) -> PyResult < PyObject >
```

> **Python API**: [cloacina-python.computation_graph.ComputationGraphBuilder.execute](../../../cloacina-python/computation_graph/#ComputationGraphBuilder.execute)

Execute the computation graph with the given input cache.

`inputs` is a dict mapping source names to their data dicts.
Returns the terminal node's output dict.

<details>
<summary>Source</summary>

```rust
    pub fn execute(&self, py: Python<'_>, inputs: &Bound<'_, PyDict>) -> PyResult<PyObject> {
        let executor = get_graph_executor(&self.name).ok_or_else(|| {
            PyValueError::new_err(format!(
                "graph '{}' not built yet — call execute after the 'with' block exits",
                self.name
            ))
        })?;

        // Convert Python dict inputs → HashMap<String, PyObject>
        let mut cache: HashMap<String, PyObject> = HashMap::new();
        for (key, value) in inputs.iter() {
            let key_str: String = key.extract()?;
            cache.insert(key_str, value.unbind());
        }

        // Execute synchronously (this is a simplified path for tutorials)
        let result = executor.execute_sync(py, &cache)?;
        Ok(result)
    }
```

</details>





### <span id="struct.ReactorBinding"></span>`struct ReactorBinding`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


**Derives:** `Debug`, `Clone`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `accumulators` | `Vec < String >` |  |
| `mode` | `String` | `"when_any"` | `"when_all"`. |



### <span id="struct.PythonGraphExecutor"></span>`class PythonGraphExecutor`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.computation_graph.PythonGraphExecutor](../../../cloacina-python/computation_graph/#class.PythonGraphExecutor)

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `node_functions` | `HashMap < String , PyObject >` |  |
| `node_map` | `HashMap < String , PyNodeDecl >` |  |
| `execution_order` | `Vec < String >` |  |
| `react_mode` | `String` |  |
| `accumulators` | `Vec < String >` |  |
| `has_reactor` | `bool` | `true` when the graph is bound to a reactor. Trigger-less graphs
(`false`) cannot be turned into a `ComputationGraphDeclaration`. |
| `reactor_name` | `Option < String >` | Reactor name from the `@cloaca.reactor`-decorated class. Threaded
into the resulting `ComputationGraphDeclaration.reactor_name` so
two packages declaring the same reactor share a single runtime
instance (T-0544 M5 cross-package fan-out). |

#### Methods

##### <span id="PythonGraphExecutor.execute_sync"></span>`execute_sync`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn execute_sync (& self , py : Python < '_ > , inputs : & HashMap < String , PyObject > ,) -> PyResult < PyObject >
```

Execute the graph synchronously from Python with dict inputs.

Used by `ComputationGraphBuilder.execute()` for tutorials.

<details>
<summary>Source</summary>

```rust
    pub fn execute_sync(
        &self,
        py: Python<'_>,
        inputs: &HashMap<String, PyObject>,
    ) -> PyResult<PyObject> {
        // Convert PyObject inputs to serde_json::Value for the executor
        let mut cache_values: HashMap<String, serde_json::Value> = HashMap::new();
        for (name, obj) in inputs {
            let val = pythonize::depythonize::<serde_json::Value>(obj.bind(py))?;
            cache_values.insert(name.clone(), val);
        }

        let result = execute_graph_sync(
            py,
            &self.node_functions,
            &self.execution_order,
            &self.node_map,
            &cache_values,
        );

        match result {
            Ok(outputs) => {
                // Terminal results are Box<dyn Any + Send> containing serde_json::Value
                if let Some(last) = outputs.last() {
                    if let Some(json_val) = last.downcast_ref::<serde_json::Value>() {
                        let py_obj = pythonize::pythonize(py, json_val).map_err(|e| {
                            PyValueError::new_err(format!("result conversion failed: {}", e))
                        })?;
                        Ok(py_obj.unbind())
                    } else {
                        Ok(py.None())
                    }
                } else {
                    Ok(py.None())
                }
            }
            Err(e) => Err(PyValueError::new_err(format!(
                "graph execution failed: {}",
                e
            ))),
        }
    }
```

</details>



##### <span id="PythonGraphExecutor.execute"></span>`execute`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn execute (& self , cache : & cloacina :: computation_graph :: types :: InputCache ,) -> GraphResult
```

Execute the graph with the given input cache.

<details>
<summary>Source</summary>

```rust
    pub async fn execute(
        &self,
        cache: &cloacina::computation_graph::types::InputCache,
    ) -> GraphResult {
        let executor = self.clone();

        // Deserialize cache inputs. Boundary frames are bincode(Vec<u8>) of
        // raw JSON (passthrough events AND — since CLOACI-T-0842 — state
        // windows). The old `cache.get::<serde_json::Value>` could NEVER
        // decode either shape (`Value` can't deserialize from bincode —
        // deserialize_any), so node inputs were silently empty; decode the
        // real wire shape.
        let mut cache_values: HashMap<String, serde_json::Value> = HashMap::new();
        for acc_name in &executor.accumulators {
            if let Some(Ok(json_bytes)) = cache.get::<Vec<u8>>(acc_name) {
                if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&json_bytes) {
                    cache_values.insert(acc_name.clone(), val);
                }
            }
        }

        let result = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                execute_graph_sync(
                    py,
                    &executor.node_functions,
                    &executor.execution_order,
                    &executor.node_map,
                    &cache_values,
                )
            })
        })
        .await;

        match result {
            Ok(Ok(outputs)) => GraphResult::completed(outputs),
            Ok(Err(e)) => GraphResult::error(e),
            Err(join_err) => GraphResult::error(GraphError::NodeExecution(format!(
                "graph execution panicked: {}",
                join_err
            ))),
        }
    }
```

</details>



##### <span id="PythonGraphExecutor.terminal_names"></span>`terminal_names`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn terminal_names (& self) -> Vec < String >
```

Terminal node names in `execution_order`, in the order their outputs will be appended to the result vector. Used by `@cloaca.task(invokes=...)` to route outputs back into the task context under their terminal name.

<details>
<summary>Source</summary>

```rust
    pub fn terminal_names(&self) -> Vec<String> {
        self.execution_order
            .iter()
            .filter(|n| {
                matches!(
                    self.node_map.get(*n).map(|d| &d.edge),
                    Some(PyEdgeDecl::Terminal)
                )
            })
            .cloned()
            .collect()
    }
```

</details>



##### <span id="PythonGraphExecutor.execute_trigger_less"></span>`execute_trigger_less`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn execute_trigger_less (& self , ctx : PyObject ,) -> Result < Vec < (String , serde_json :: Value) > , GraphError >
```

Execute a trigger-less graph with a Python `Context` object.

Each "root" node — one with no upstream edges and no cache inputs —
receives the context as its single argument. Downstream nodes get
their predecessor's output as before. Returns `(terminal_name, value)`
pairs in the order each terminal *actually fired* — so routing
branches that were skipped do not appear in the output. The task
wrapper routes each pair into the task context under its terminal
node name.

<details>
<summary>Source</summary>

```rust
    pub async fn execute_trigger_less(
        &self,
        ctx: PyObject,
    ) -> Result<Vec<(String, serde_json::Value)>, GraphError> {
        let executor = self.clone();
        match tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                execute_graph_sync_named_terminals(
                    py,
                    &executor.node_functions,
                    &executor.execution_order,
                    &executor.node_map,
                    &ctx,
                )
            })
        })
        .await
        {
            Ok(Ok(pairs)) => Ok(pairs),
            Ok(Err(e)) => Err(e),
            Err(join_err) => Err(GraphError::NodeExecution(format!(
                "trigger-less graph execution panicked: {}",
                join_err
            ))),
        }
    }
```

</details>





## Enums

### <span id="enum.PyEdgeDecl"></span>`enum PyEdgeDecl`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


#### Variants

- **`Linear`**
- **`Routing`**
- **`Terminal`**



## Functions

### <span id="fn.push_graph_context"></span>`fn push_graph_context`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn push_graph_context (name : String)
```

<details>
<summary>Source</summary>

```rust
fn push_graph_context(name: String) {
    let mut ctx = ACTIVE_GRAPH_CONTEXT.lock().unwrap();
    *ctx = Some(name);
}
```

</details>



### <span id="fn.pop_graph_context"></span>`fn pop_graph_context`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn pop_graph_context ()
```

<details>
<summary>Source</summary>

```rust
fn pop_graph_context() {
    let mut ctx = ACTIVE_GRAPH_CONTEXT.lock().unwrap();
    *ctx = None;
}
```

</details>



### <span id="fn.current_graph_context"></span>`fn current_graph_context`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn current_graph_context () -> Option < String >
```

<details>
<summary>Source</summary>

```rust
fn current_graph_context() -> Option<String> {
    ACTIVE_GRAPH_CONTEXT.lock().unwrap().clone()
}
```

</details>



### <span id="fn.register_node"></span>`fn register_node`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn register_node (name : String , func : PyObject)
```

<details>
<summary>Source</summary>

```rust
fn register_node(name: String, func: PyObject) {
    NODE_REGISTRY.lock().unwrap().insert(name, func);
}
```

</details>



### <span id="fn.drain_nodes"></span>`fn drain_nodes`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn drain_nodes () -> HashMap < String , PyObject >
```

<details>
<summary>Source</summary>

```rust
fn drain_nodes() -> HashMap<String, PyObject> {
    let mut registry = NODE_REGISTRY.lock().unwrap();
    std::mem::take(&mut *registry)
}
```

</details>



### <span id="fn.resolve_graph_key"></span>`fn resolve_graph_key`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn resolve_graph_key < V > (map : & HashMap < GraphKey , V > , name : & str) -> Option < GraphKey >
```

Resolve a bare `name` to a concrete key within the thread's current scope.

Order, mirroring `EndpointRegistry::resolve_key` on the Rust side:
1. exact `(tenant, package, name)` — the normal packaged path;
2. same tenant, any package — a tenant addressing its own graph from a
differently-scoped thread;
3. the unscoped entry — embedded / direct-import / test registrations;
4. a unique match by name anywhere (the fleet-agent path, which dispatches
by graph name with no scope installed). Ambiguity resolves to `None`
rather than a coin flip.

<details>
<summary>Source</summary>

```rust
fn resolve_graph_key<V>(map: &HashMap<GraphKey, V>, name: &str) -> Option<GraphKey> {
    let scope = current_registration_scope();

    let exact = GraphKey::new(&scope, name);
    if map.contains_key(&exact) {
        return Some(exact);
    }

    if scope.tenant_id.is_some() {
        let same_tenant: Vec<&GraphKey> = map
            .keys()
            .filter(|k| k.name == name && k.same_tenant(&scope))
            .collect();
        if same_tenant.len() == 1 {
            return Some(same_tenant[0].clone());
        }
        if same_tenant.len() > 1 {
            return None;
        }
    }

    let unscoped = GraphKey::new(&RegistrationScope::unscoped(), name);
    if map.contains_key(&unscoped) {
        return Some(unscoped);
    }

    // A scoped caller must never fall through to another tenant's entry.
    if scope.tenant_id.is_some() {
        return None;
    }

    let any: Vec<&GraphKey> = map.keys().filter(|k| k.name == name).collect();
    match any.len() {
        1 => Some(any[0].clone()),
        0 => None,
        _ => {
            tracing::warn!(
                graph = %name,
                candidates = ?any.iter().map(|k| k.describe()).collect::<Vec<_>>(),
                "ambiguous graph name across tenants/packages — refusing to guess"
            );
            None
        }
    }
}
```

</details>



### <span id="fn.register_accumulator"></span>`fn register_accumulator`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn register_accumulator (name : String , func : PyObject , reg : PyAccumulatorRegistration)
```

<details>
<summary>Source</summary>

```rust
fn register_accumulator(name: String, func: PyObject, reg: PyAccumulatorRegistration) {
    ACCUMULATOR_REGISTRY
        .lock()
        .unwrap()
        .insert(GraphKey::current(&name), (func, reg));
}
```

</details>



### <span id="fn.get_registered_accumulators"></span>`fn get_registered_accumulators`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn get_registered_accumulators () -> Vec < PyAccumulatorRegistration >
```

Get the accumulators registered in the thread's current scope (plus the unscoped ones, for embedded/test use). CLOACI-T-0921: a tenant-scoped caller no longer sees another tenant's accumulator declarations.

<details>
<summary>Source</summary>

```rust
pub fn get_registered_accumulators() -> Vec<PyAccumulatorRegistration> {
    let scope = current_registration_scope();
    ACCUMULATOR_REGISTRY
        .lock()
        .unwrap()
        .iter()
        .filter(|(key, _)| {
            if scope.is_unscoped() {
                true
            } else {
                key.matches_scope(&scope) || (key.tenant_id.is_none() && key.package.is_none())
            }
        })
        .map(|(_, (_, reg))| reg.clone())
        .collect()
}
```

</details>



### <span id="fn.get_all_registered_accumulators"></span>`fn get_all_registered_accumulators`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn get_all_registered_accumulators () -> Vec < (GraphKey , PyAccumulatorRegistration) >
```

Every registered accumulator, across all scopes — diagnostics only.

<details>
<summary>Source</summary>

```rust
pub fn get_all_registered_accumulators() -> Vec<(GraphKey, PyAccumulatorRegistration)> {
    ACCUMULATOR_REGISTRY
        .lock()
        .unwrap()
        .iter()
        .map(|(key, (_, reg))| (key.clone(), reg.clone()))
        .collect()
}
```

</details>



### <span id="fn.drain_accumulators"></span>`fn drain_accumulators`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn drain_accumulators () -> HashMap < GraphKey , (PyObject , PyAccumulatorRegistration) >
```

Drain all registered accumulators (used by builder on __exit__ / test reset).

<details>
<summary>Source</summary>

```rust
pub fn drain_accumulators() -> HashMap<GraphKey, (PyObject, PyAccumulatorRegistration)> {
    let mut registry = ACCUMULATOR_REGISTRY.lock().unwrap();
    std::mem::take(&mut *registry)
}
```

</details>



### <span id="fn.resolve_poll_closure"></span>`fn resolve_poll_closure`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn resolve_poll_closure (name : & str ,) -> Option < cloacina :: computation_graph :: packaging_bridge :: PollClosure >
```

Resolve the poll closure for a packaged POLLING accumulator (CLOACI-T-0896). Looks up the registered Python poll fn by name and wraps it so the packaged polling runtime can invoke it on the interval. The poll fn lives in-process (the reconciler imports the module in the server), so no FFI is needed. The closure does the GIL work itself — cloacina runs it under `spawn_blocking`, mirroring how `PythonTriggerWrapper` drives Python poll triggers off the async executor.

<details>
<summary>Source</summary>

```rust
fn resolve_poll_closure(
    name: &str,
) -> Option<cloacina::computation_graph::packaging_bridge::PollClosure> {
    let function = {
        let registry = ACCUMULATOR_REGISTRY.lock().unwrap();
        // CLOACI-T-0921: resolve within the thread's registration scope.
        let key = resolve_graph_key(&registry, name)?;
        let (func, reg) = registry.get(&key)?;
        if reg.accumulator_type != "polling" {
            return None;
        }
        Python::with_gil(|py| func.clone_ref(py))
    };
    let name = name.to_string();
    Some(std::sync::Arc::new(move || -> Option<Vec<u8>> {
        Python::with_gil(|py| {
            let result = match function.call0(py) {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("polling accumulator '{}' poll raised: {}", name, e);
                    return None;
                }
            };
            // `None` from the poll fn means "no change this tick" — skip.
            if result.is_none(py) {
                return None;
            }
            match pythonize::depythonize::<serde_json::Value>(result.bind(py)) {
                Ok(val) => serde_json::to_vec(&val).ok(),
                Err(e) => {
                    tracing::warn!(
                        "polling accumulator '{}' returned a non-JSON value: {}",
                        name,
                        e
                    );
                    None
                }
            }
        })
    }))
}
```

</details>



### <span id="fn.install_polling_accumulator_builder"></span>`fn install_polling_accumulator_builder`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">pub(crate)</span>


```rust
fn install_polling_accumulator_builder ()
```

Install the polling-accumulator poll-closure resolver into the packaged CG bridge (CLOACI-T-0896). Idempotent (the bridge's OnceLock keeps the first); called once from `register_authoring` so every embedding (pip wheel + the server's synthetic `cloaca` module) wires packaged polling accumulators.

<details>
<summary>Source</summary>

```rust
pub(crate) fn install_polling_accumulator_builder() {
    cloacina::computation_graph::packaging_bridge::register_polling_accumulator_builder(Box::new(
        resolve_poll_closure,
    ));
}
```

</details>



### <span id="fn.passthrough_accumulator_decorator"></span>`fn passthrough_accumulator_decorator`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.computation_graph.passthrough_accumulator_decorator](../../../cloacina-python/computation_graph/#fn.passthrough_accumulator_decorator)

```rust
fn passthrough_accumulator_decorator (py : Python < '_ > , func : PyObject) -> PyResult < PyObject >
```

The `@cloaca.passthrough_accumulator` decorator. Registers a function as a passthrough accumulator (Event → Output, no buffering).

<details>
<summary>Source</summary>

```rust
pub fn passthrough_accumulator_decorator(py: Python<'_>, func: PyObject) -> PyResult<PyObject> {
    let func_name: String = func.getattr(py, "__name__")?.extract(py)?;
    let reg = PyAccumulatorRegistration {
        name: func_name.clone(),
        accumulator_type: "passthrough".to_string(),
        config: HashMap::new(),
        capacity: 0,
    };
    register_accumulator(func_name, func.clone_ref(py), reg);
    Ok(func)
}
```

</details>



### <span id="fn.stream_accumulator_decorator"></span>`fn stream_accumulator_decorator`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.computation_graph.stream_accumulator_decorator](../../../cloacina-python/computation_graph/#fn.stream_accumulator_decorator)

```rust
fn stream_accumulator_decorator (py : Python < '_ > , r#type : String , topic : String , group : Option < String > ,) -> PyResult < PyObject >
```

Factory for `@cloaca.stream_accumulator(type=..., topic=...)`. Returns a decorator function that registers the target with stream config.

<details>
<summary>Source</summary>

```rust
pub fn stream_accumulator_decorator(
    py: Python<'_>,
    r#type: String,
    topic: String,
    group: Option<String>,
) -> PyResult<PyObject> {
    let config_type = r#type;
    let config_topic = topic;
    let config_group = group;

    // Return a decorator function
    let decorator = PyCFunction::new_closure(
        py,
        None,
        None,
        move |args: &Bound<'_, PyTuple>,
              _kwargs: Option<&Bound<'_, PyDict>>|
              -> PyResult<PyObject> {
            let _py = args.py();
            let func = args.get_item(0)?;
            let func_name: String = func.getattr("__name__")?.extract()?;

            let mut config = HashMap::new();
            config.insert("type".to_string(), config_type.clone());
            config.insert("topic".to_string(), config_topic.clone());
            if let Some(ref g) = config_group {
                config.insert("group".to_string(), g.clone());
            }

            let reg = PyAccumulatorRegistration {
                name: func_name.clone(),
                accumulator_type: "stream".to_string(),
                config,
                capacity: 0,
            };
            register_accumulator(func_name, func.clone().unbind(), reg);
            Ok(func.clone().unbind())
        },
    )?;

    Ok(decorator.into())
}
```

</details>



### <span id="fn.polling_accumulator_decorator"></span>`fn polling_accumulator_decorator`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.computation_graph.polling_accumulator_decorator](../../../cloacina-python/computation_graph/#fn.polling_accumulator_decorator)

```rust
fn polling_accumulator_decorator (py : Python < '_ > , interval : String) -> PyResult < PyObject >
```

Factory for `@cloaca.polling_accumulator(interval=...)`.

<details>
<summary>Source</summary>

```rust
pub fn polling_accumulator_decorator(py: Python<'_>, interval: String) -> PyResult<PyObject> {
    let config_interval = interval;

    let decorator = PyCFunction::new_closure(
        py,
        None,
        None,
        move |args: &Bound<'_, PyTuple>,
              _kwargs: Option<&Bound<'_, PyDict>>|
              -> PyResult<PyObject> {
            let _py = args.py();
            let func = args.get_item(0)?;
            let func_name: String = func.getattr("__name__")?.extract()?;

            let mut config = HashMap::new();
            config.insert("interval".to_string(), config_interval.clone());

            let reg = PyAccumulatorRegistration {
                name: func_name.clone(),
                accumulator_type: "polling".to_string(),
                config,
                capacity: 0,
            };
            register_accumulator(func_name, func.clone().unbind(), reg);
            Ok(func.clone().unbind())
        },
    )?;

    Ok(decorator.into())
}
```

</details>



### <span id="fn.batch_accumulator_decorator"></span>`fn batch_accumulator_decorator`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.computation_graph.batch_accumulator_decorator](../../../cloacina-python/computation_graph/#fn.batch_accumulator_decorator)

```rust
fn batch_accumulator_decorator (py : Python < '_ > , flush_interval : String , max_buffer_size : Option < usize > ,) -> PyResult < PyObject >
```

Factory for `@cloaca.batch_accumulator(flush_interval=..., max_buffer_size=...)`.

<details>
<summary>Source</summary>

```rust
pub fn batch_accumulator_decorator(
    py: Python<'_>,
    flush_interval: String,
    max_buffer_size: Option<usize>,
) -> PyResult<PyObject> {
    let config_interval = flush_interval;
    let config_max = max_buffer_size;

    let decorator = PyCFunction::new_closure(
        py,
        None,
        None,
        move |args: &Bound<'_, PyTuple>,
              _kwargs: Option<&Bound<'_, PyDict>>|
              -> PyResult<PyObject> {
            let _py = args.py();
            let func = args.get_item(0)?;
            let func_name: String = func.getattr("__name__")?.extract()?;

            let mut config = HashMap::new();
            config.insert("flush_interval".to_string(), config_interval.clone());
            if let Some(max) = config_max {
                config.insert("max_buffer_size".to_string(), max.to_string());
            }

            let reg = PyAccumulatorRegistration {
                name: func_name.clone(),
                accumulator_type: "batch".to_string(),
                config,
                capacity: 0,
            };
            register_accumulator(func_name, func.clone().unbind(), reg);
            Ok(func.clone().unbind())
        },
    )?;

    Ok(decorator.into())
}
```

</details>



### <span id="fn.state_accumulator_decorator"></span>`fn state_accumulator_decorator`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.computation_graph.state_accumulator_decorator](../../../cloacina-python/computation_graph/#fn.state_accumulator_decorator)

```rust
fn state_accumulator_decorator (py : Python < '_ > , capacity : i32) -> PyResult < PyObject >
```

Factory for `@cloaca.state_accumulator(capacity=N)`.

Registers a function as a *state* accumulator: a bounded `VecDeque` that
receives values, persists to the DAL on every write, and emits the full
list back as the boundary (enabling cyclic feedback patterns). Mirrors
Rust's `#[state_accumulator(capacity=…)]`.
Capacity semantics (see `StateAccumulator` in cloacina core):
- `> 0`: bounded — evicts oldest when at capacity
- `< 0`: unbounded — grows without limit
- `0`:  write-only sink — no history emitted back

<details>
<summary>Source</summary>

```rust
pub fn state_accumulator_decorator(py: Python<'_>, capacity: i32) -> PyResult<PyObject> {
    let config_capacity = capacity;

    let decorator = PyCFunction::new_closure(
        py,
        None,
        None,
        move |args: &Bound<'_, PyTuple>,
              _kwargs: Option<&Bound<'_, PyDict>>|
              -> PyResult<PyObject> {
            let _py = args.py();
            let func = args.get_item(0)?;
            let func_name: String = func.getattr("__name__")?.extract()?;

            // Mirror capacity into config too, so it survives the
            // String-keyed config map used at the FFI/override boundary.
            let mut config = HashMap::new();
            config.insert("capacity".to_string(), config_capacity.to_string());

            let reg = PyAccumulatorRegistration {
                name: func_name.clone(),
                accumulator_type: "state".to_string(),
                config,
                capacity: config_capacity,
            };
            register_accumulator(func_name, func.clone().unbind(), reg);
            Ok(func.clone().unbind())
        },
    )?;

    Ok(decorator.into())
}
```

</details>



### <span id="fn.node"></span>`fn node`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.computation_graph.node](../../../cloacina-python/computation_graph/#fn.node)

```rust
fn node (py : Python < '_ > , func : PyObject) -> PyResult < PyObject >
```

The `@cloaca.node` decorator. Registers a function as a node in the current ComputationGraphBuilder context.

<details>
<summary>Source</summary>

```rust
pub fn node(py: Python<'_>, func: PyObject) -> PyResult<PyObject> {
    let _ctx = current_graph_context().ok_or_else(|| {
        PyValueError::new_err(
            "@cloaca.node must be used inside a ComputationGraphBuilder context manager",
        )
    })?;

    let func_name: String = func.getattr(py, "__name__")?.extract(py)?;
    register_node(func_name, func.clone_ref(py));

    // Return the function unchanged (transparent decorator)
    Ok(func)
}
```

</details>



### <span id="fn.register_graph_executor"></span>`fn register_graph_executor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn register_graph_executor (name : String , executor : PythonGraphExecutor , _py : Python < '_ > ,) -> PyResult < () >
```

<details>
<summary>Source</summary>

```rust
fn register_graph_executor(
    name: String,
    executor: PythonGraphExecutor,
    _py: Python<'_>,
) -> PyResult<()> {
    let key = GraphKey::current(&name);
    tracing::debug!(graph = %key.describe(), "registering Python graph executor");
    GRAPH_EXECUTORS.lock().unwrap().insert(key, executor);
    Ok(())
}
```

</details>



### <span id="fn.get_graph_executor"></span>`fn get_graph_executor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn get_graph_executor (name : & str) -> Option < PythonGraphExecutor >
```

Get a registered graph executor by name, resolved within the thread's current [`RegistrationScope`] (for testing / reactor / fleet-agent use).

<details>
<summary>Source</summary>

```rust
pub fn get_graph_executor(name: &str) -> Option<PythonGraphExecutor> {
    let registry = GRAPH_EXECUTORS.lock().unwrap();
    let key = resolve_graph_key(&registry, name)?;
    registry.get(&key).cloned()
}
```

</details>



### <span id="fn.get_graph_executor_by_key"></span>`fn get_graph_executor_by_key`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn get_graph_executor_by_key (key : & GraphKey) -> Option < PythonGraphExecutor >
```

Get a registered graph executor by its full `(tenant, package, name)` key, ignoring the thread's ambient scope.

<details>
<summary>Source</summary>

```rust
pub fn get_graph_executor_by_key(key: &GraphKey) -> Option<PythonGraphExecutor> {
    GRAPH_EXECUTORS.lock().unwrap().get(key).cloned()
}
```

</details>



### <span id="fn.registered_graph_keys"></span>`fn registered_graph_keys`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn registered_graph_keys () -> Vec < GraphKey >
```

Every registered graph executor key — diagnostics / tests.

<details>
<summary>Source</summary>

```rust
pub fn registered_graph_keys() -> Vec<GraphKey> {
    GRAPH_EXECUTORS.lock().unwrap().keys().cloned().collect()
}
```

</details>



### <span id="fn.get_graph_executors_for_reactor"></span>`fn get_graph_executors_for_reactor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn get_graph_executors_for_reactor (reactor : & str) -> Vec < PythonGraphExecutor >
```

All registered graph executors subscribed to `reactor` (CLOACI-T-0841). A reactor firing fans out to its subscriber graphs; the fleet agent receives the REACTOR name in the packet and uses this to find the graph(s) to execute — the agent-side analog of the scheduler's subscriber dispatcher.

<details>
<summary>Source</summary>

```rust
pub fn get_graph_executors_for_reactor(reactor: &str) -> Vec<PythonGraphExecutor> {
    // CLOACI-T-0921: when a scope is installed, only that tenant's subscriber
    // graphs (plus unscoped ones) fan out. The fleet agent dispatches with no
    // scope installed and still sees everything it hosts.
    let scope = current_registration_scope();
    GRAPH_EXECUTORS
        .lock()
        .unwrap()
        .iter()
        .filter(|(key, _)| {
            scope.tenant_id.is_none() || key.same_tenant(&scope) || key.tenant_id.is_none()
        })
        .map(|(_, ex)| ex)
        .filter(|ex| ex.has_reactor && ex.reactor_name.as_deref() == Some(reactor))
        .cloned()
        .collect()
}
```

</details>



### <span id="fn.build_python_graph_declaration"></span>`fn build_python_graph_declaration`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn build_python_graph_declaration (graph_name : & str , tenant_id : Option < String > , accumulator_overrides : & [cloacina_workflow_plugin :: types :: AccumulatorConfig] ,) -> Option < cloacina :: computation_graph :: scheduler :: ComputationGraphDeclaration >
```

Build a [`ComputationGraphDeclaration`] from a registered Python graph executor.

This bridges the Python CG world (decorators + `ComputationGraphBuilder`) into
the Rust `ComputationGraphScheduler` by wrapping the Python executor in a `CompiledGraphFn`.

<details>
<summary>Source</summary>

```rust
pub fn build_python_graph_declaration(
    graph_name: &str,
    tenant_id: Option<String>,
    accumulator_overrides: &[cloacina_workflow_plugin::types::AccumulatorConfig],
) -> Option<cloacina::computation_graph::scheduler::ComputationGraphDeclaration> {
    use cloacina::computation_graph::packaging_bridge::{
        PassthroughAccumulatorFactory, ProviderStreamAccumulatorFactory, StateAccumulatorFactory,
    };
    use cloacina::computation_graph::reactor::{InputStrategy, ReactionCriteria};
    use cloacina::computation_graph::scheduler::{
        AccumulatorDeclaration, ComputationGraphDeclaration, ReactorDeclaration,
    };
    use cloacina_computation_graph::InputCache;
    use std::sync::Arc;

    let executor = get_graph_executor(graph_name)?;

    // Trigger-less graphs aren't reactor-driven and therefore have no
    // ComputationGraphDeclaration to publish. Callers that need the graph
    // (e.g. a `@cloaca.task(invokes=...)` body) look it up through
    // `get_graph_executor` directly.
    if !executor.has_reactor {
        return None;
    }

    let criteria = match executor.react_mode.as_str() {
        "when_all" => ReactionCriteria::WhenAll,
        _ => ReactionCriteria::WhenAny,
    };

    let accumulator_names = executor.accumulators.clone();
    let reactor_name = executor.reactor_name.clone();

    // Capture the node/edge topology before `executor` is moved into the
    // graph_fn closure, so the health API can render this Python CG's DAG
    // the same way Rust CGs do. (CLOACI-T-0673)
    let topology = Some(python_graph_topology_json(&executor));

    // Build CompiledGraphFn from the Python executor
    let graph_fn: cloacina_computation_graph::CompiledGraphFn =
        Arc::new(move |cache: InputCache| {
            let exec = executor.clone();
            Box::pin(async move { exec.execute(&cache).await })
        });

    // Build accumulator declarations — use package.toml overrides when available
    let accumulators = accumulator_names
        .iter()
        .map(|name| {
            let factory: Arc<dyn cloacina::computation_graph::scheduler::AccumulatorFactory> =
                if let Some(override_cfg) = accumulator_overrides.iter().find(|a| a.name == *name) {
                    match override_cfg.accumulator_type.as_str() {
                        // CLOACI-T-0898: stream sources come from a bundled
                        // constructor provider (`provider`/`constructor` config
                        // keys); a declaration without one fails loudly at spawn.
                        "stream" => Arc::new(ProviderStreamAccumulatorFactory::new(
                            override_cfg.config.clone(),
                        )),
                        "state" => Arc::new(StateAccumulatorFactory::new(
                            override_cfg
                                .config
                                .get("capacity")
                                .and_then(|c| c.parse::<i32>().ok())
                                .unwrap_or(0),
                        )),
                        _ => Arc::new(PassthroughAccumulatorFactory),
                    }
                } else {
                    Arc::new(PassthroughAccumulatorFactory)
                };
            AccumulatorDeclaration {
                name: name.clone(),
                factory,
            }
        })
        .collect();

    Some(ComputationGraphDeclaration {
        name: graph_name.to_string(),
        accumulators,
        reactor: ReactorDeclaration {
            criteria,
            strategy: InputStrategy::Latest,
            graph_fn,
            // CLOACI-T-0830: the Python CG path doesn't author WASM reactor
            // constructors — native dirty-flag firing only.
            constructor: None,
        },
        tenant_id,
        // T-0544 M5: thread the reactor name from the `@cloaca.reactor`
        // class through to the scheduler so Python packages naming the
        // same reactor as a Rust (or Python) package share a runtime
        // instance via M2's idempotent registration path.
        reactor_name,
        topology,
    })
}
```

</details>



### <span id="fn.python_graph_topology_json"></span>`fn python_graph_topology_json`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn python_graph_topology_json (executor : & PythonGraphExecutor) -> String
```

Serialize a Python graph executor's node/edge topology to the same compact JSON shape the Rust `#[computation_graph]` macro emits (`{"nodes":[{"id","inputs":[..]}],"edges":[{"from","to","label":null|"Variant"}]}`), so the API/UI render Python and Rust CG DAGs identically. (CLOACI-T-0673)

<details>
<summary>Source</summary>

```rust
fn python_graph_topology_json(executor: &PythonGraphExecutor) -> String {
    use serde_json::{json, Value};

    let nodes: Vec<Value> = executor
        .execution_order
        .iter()
        .filter_map(|n| executor.node_map.get(n))
        .map(|d| json!({ "id": d.name, "inputs": d.cache_inputs }))
        .collect();

    let mut edges: Vec<Value> = Vec::new();
    for name in &executor.execution_order {
        if let Some(decl) = executor.node_map.get(name) {
            match &decl.edge {
                PyEdgeDecl::Linear { target } => edges.push(json!({
                    "from": decl.name,
                    "to": target,
                    "label": Value::Null,
                })),
                PyEdgeDecl::Routing { variants } => {
                    for (variant_name, target) in variants {
                        edges.push(json!({
                            "from": decl.name,
                            "to": target,
                            "label": variant_name,
                        }));
                    }
                }
                // Terminal nodes have no outgoing edge.
                PyEdgeDecl::Terminal => {}
            }
        }
    }

    serde_json::to_string(&json!({ "nodes": nodes, "edges": edges })).unwrap_or_default()
}
```

</details>



### <span id="fn.execute_graph_sync"></span>`fn execute_graph_sync`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn execute_graph_sync (py : Python < '_ > , node_functions : & HashMap < String , PyObject > , execution_order : & [String] , node_map : & HashMap < String , PyNodeDecl > , cache_values : & HashMap < String , serde_json :: Value > ,) -> Result < Vec < Box < dyn std :: any :: Any + Send > > , GraphError >
```

Sync helper used by both reactor-triggered and trigger-less paths. `trigger_less_ctx`, when `Some`, is passed as the sole argument to every "root" node (one with no incoming edges and no cache inputs). The reactor-triggered path leaves this `None`.

<details>
<summary>Source</summary>

```rust
fn execute_graph_sync(
    py: Python<'_>,
    node_functions: &HashMap<String, PyObject>,
    execution_order: &[String],
    node_map: &HashMap<String, PyNodeDecl>,
    cache_values: &HashMap<String, serde_json::Value>,
) -> Result<Vec<Box<dyn std::any::Any + Send>>, GraphError> {
    execute_graph_sync_inner(
        py,
        node_functions,
        execution_order,
        node_map,
        cache_values,
        None,
    )
}
```

</details>



### <span id="fn.execute_graph_sync_named_terminals"></span>`fn execute_graph_sync_named_terminals`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn execute_graph_sync_named_terminals (py : Python < '_ > , node_functions : & HashMap < String , PyObject > , execution_order : & [String] , node_map : & HashMap < String , PyNodeDecl > , ctx : & PyObject ,) -> Result < Vec < (String , serde_json :: Value) > , GraphError >
```

Trigger-less variant that returns `(terminal_name, value)` pairs in the order each terminal fired. Routing branches that aren't selected emit no pair, so consumers (e.g. `@cloaca.task(invokes=...)`) can route outputs to context keys without misalignment.

<details>
<summary>Source</summary>

```rust
fn execute_graph_sync_named_terminals(
    py: Python<'_>,
    node_functions: &HashMap<String, PyObject>,
    execution_order: &[String],
    node_map: &HashMap<String, PyNodeDecl>,
    ctx: &PyObject,
) -> Result<Vec<(String, serde_json::Value)>, GraphError> {
    let cache_values: HashMap<String, serde_json::Value> = HashMap::new();
    let trigger_less_ctx = Some(ctx);

    let mut pairs: Vec<(String, serde_json::Value)> = Vec::new();
    let mut node_results: HashMap<String, PyObject> = HashMap::new();

    let mut incoming: HashMap<String, Vec<(String, Option<String>)>> = HashMap::new();
    for node in node_map.values() {
        match &node.edge {
            PyEdgeDecl::Linear { target } => {
                incoming
                    .entry(target.clone())
                    .or_default()
                    .push((node.name.clone(), None));
            }
            PyEdgeDecl::Routing { variants } => {
                for (variant_name, target) in variants {
                    incoming
                        .entry(target.clone())
                        .or_default()
                        .push((node.name.clone(), Some(variant_name.clone())));
                }
            }
            PyEdgeDecl::Terminal => {}
        }
    }

    let mut skipped_nodes: std::collections::HashSet<String> = std::collections::HashSet::new();

    for node_name in execution_order {
        if skipped_nodes.contains(node_name) {
            continue;
        }

        let node_decl = node_map.get(node_name).ok_or_else(|| {
            GraphError::Execution(format!("node '{}' not found in topology", node_name))
        })?;

        if let Some(sources) = incoming.get(node_name) {
            let mut should_skip = false;
            for (from_node, variant) in sources {
                if let Some(v) = variant {
                    let selected_key = format!("{}:__selected_variant", from_node);
                    if let Some(selected) = node_results.get(&selected_key) {
                        let selected_str: String = selected.extract(py).unwrap_or_default();
                        if selected_str != *v {
                            should_skip = true;
                        }
                    }
                }
            }
            if should_skip {
                skipped_nodes.insert(node_name.clone());
                continue;
            }
        }

        let args = build_node_args(
            py,
            node_name,
            node_decl,
            &cache_values,
            &node_results,
            &incoming,
            trigger_less_ctx,
        )?;

        let func = node_functions.get(node_name).ok_or_else(|| {
            GraphError::NodeExecution(format!("function '{}' not registered", node_name))
        })?;

        let result = func.call1(py, args).map_err(|e| {
            GraphError::NodeExecution(format!("node '{}' failed: {}", node_name, e))
        })?;

        match &node_decl.edge {
            PyEdgeDecl::Terminal => {
                let json_val: serde_json::Value =
                    pythonize::depythonize(result.bind(py)).map_err(|e| {
                        GraphError::Serialization(format!(
                            "terminal '{}' result conversion failed: {}",
                            node_name, e
                        ))
                    })?;
                pairs.push((node_name.clone(), json_val));
            }
            PyEdgeDecl::Linear { .. } => {
                node_results.insert(node_name.clone(), result);
            }
            PyEdgeDecl::Routing { .. } => {
                let tuple = result.downcast_bound::<PyTuple>(py).map_err(|_| {
                    GraphError::NodeExecution(format!(
                        "routing node '{}' must return a (variant_name, value) tuple",
                        node_name
                    ))
                })?;

                if tuple.len() != 2 {
                    return Err(GraphError::NodeExecution(format!(
                        "routing node '{}' returned tuple of length {}, expected 2",
                        node_name,
                        tuple.len()
                    )));
                }

                let variant_name = tuple
                    .get_item(0)
                    .map_err(|e| GraphError::NodeExecution(format!("tuple index error: {}", e)))?
                    .downcast::<PyString>()
                    .map_err(|_| {
                        GraphError::NodeExecution(format!(
                            "routing node '{}': first element must be a string",
                            node_name
                        ))
                    })?
                    .to_string();

                let variant_value = tuple
                    .get_item(1)
                    .map_err(|e| GraphError::NodeExecution(format!("tuple index error: {}", e)))?
                    .unbind();

                node_results.insert(format!("{}:{}", node_name, variant_name), variant_value);

                let variant_py = PyString::new(py, &variant_name);
                node_results.insert(
                    format!("{}:__selected_variant", node_name),
                    variant_py.unbind().into(),
                );
            }
        }
    }

    Ok(pairs)
}
```

</details>



### <span id="fn.execute_graph_sync_inner"></span>`fn execute_graph_sync_inner`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn execute_graph_sync_inner (py : Python < '_ > , node_functions : & HashMap < String , PyObject > , execution_order : & [String] , node_map : & HashMap < String , PyNodeDecl > , cache_values : & HashMap < String , serde_json :: Value > , trigger_less_ctx : Option < & PyObject > ,) -> Result < Vec < Box < dyn std :: any :: Any + Send > > , GraphError >
```

<details>
<summary>Source</summary>

```rust
fn execute_graph_sync_inner(
    py: Python<'_>,
    node_functions: &HashMap<String, PyObject>,
    execution_order: &[String],
    node_map: &HashMap<String, PyNodeDecl>,
    cache_values: &HashMap<String, serde_json::Value>,
    trigger_less_ctx: Option<&PyObject>,
) -> Result<Vec<Box<dyn std::any::Any + Send>>, GraphError> {
    let mut terminal_results: Vec<Box<dyn std::any::Any + Send>> = Vec::new();
    let mut node_results: HashMap<String, PyObject> = HashMap::new();

    // Build incoming edge map
    let mut incoming: HashMap<String, Vec<(String, Option<String>)>> = HashMap::new();
    for node in node_map.values() {
        match &node.edge {
            PyEdgeDecl::Linear { target } => {
                incoming
                    .entry(target.clone())
                    .or_default()
                    .push((node.name.clone(), None));
            }
            PyEdgeDecl::Routing { variants } => {
                for (variant_name, target) in variants {
                    incoming
                        .entry(target.clone())
                        .or_default()
                        .push((node.name.clone(), Some(variant_name.clone())));
                }
            }
            PyEdgeDecl::Terminal => {}
        }
    }

    let mut skipped_nodes: std::collections::HashSet<String> = std::collections::HashSet::new();

    for node_name in execution_order {
        if skipped_nodes.contains(node_name) {
            continue;
        }

        let node_decl = node_map.get(node_name).ok_or_else(|| {
            GraphError::Execution(format!("node '{}' not found in topology", node_name))
        })?;

        // Check if this node depends on a routing variant that wasn't taken
        if let Some(sources) = incoming.get(node_name) {
            let mut should_skip = false;
            for (from_node, variant) in sources {
                if let Some(v) = variant {
                    let selected_key = format!("{}:__selected_variant", from_node);
                    if let Some(selected) = node_results.get(&selected_key) {
                        let selected_str: String = selected.extract(py).unwrap_or_default();
                        if selected_str != *v {
                            should_skip = true;
                        }
                    }
                }
            }
            if should_skip {
                skipped_nodes.insert(node_name.clone());
                continue;
            }
        }

        // Build arguments. In trigger-less mode, root nodes (no incoming
        // edges, no cache inputs) take the task `Context` as their sole arg.
        let args = build_node_args(
            py,
            node_name,
            node_decl,
            cache_values,
            &node_results,
            &incoming,
            trigger_less_ctx,
        )?;

        // Call the function
        let func = node_functions.get(node_name).ok_or_else(|| {
            GraphError::NodeExecution(format!("function '{}' not registered", node_name))
        })?;

        let result = func.call1(py, args).map_err(|e| {
            GraphError::NodeExecution(format!("node '{}' failed: {}", node_name, e))
        })?;

        // Handle edge type
        match &node_decl.edge {
            PyEdgeDecl::Terminal => {
                let json_val: serde_json::Value =
                    pythonize::depythonize(result.bind(py)).map_err(|e| {
                        GraphError::Serialization(format!(
                            "terminal '{}' result conversion failed: {}",
                            node_name, e
                        ))
                    })?;
                terminal_results.push(Box::new(json_val) as Box<dyn std::any::Any + Send>);
            }
            PyEdgeDecl::Linear { .. } => {
                node_results.insert(node_name.clone(), result);
            }
            PyEdgeDecl::Routing { .. } => {
                let tuple = result.downcast_bound::<PyTuple>(py).map_err(|_| {
                    GraphError::NodeExecution(format!(
                        "routing node '{}' must return a (variant_name, value) tuple",
                        node_name
                    ))
                })?;

                if tuple.len() != 2 {
                    return Err(GraphError::NodeExecution(format!(
                        "routing node '{}' returned tuple of length {}, expected 2",
                        node_name,
                        tuple.len()
                    )));
                }

                let variant_name = tuple
                    .get_item(0)
                    .map_err(|e| GraphError::NodeExecution(format!("tuple index error: {}", e)))?
                    .downcast::<PyString>()
                    .map_err(|_| {
                        GraphError::NodeExecution(format!(
                            "routing node '{}': first element must be a string",
                            node_name
                        ))
                    })?
                    .to_string();

                let variant_value = tuple
                    .get_item(1)
                    .map_err(|e| GraphError::NodeExecution(format!("tuple index error: {}", e)))?
                    .unbind();

                node_results.insert(format!("{}:{}", node_name, variant_name), variant_value);

                let variant_py = PyString::new(py, &variant_name);
                node_results.insert(
                    format!("{}:__selected_variant", node_name),
                    variant_py.unbind().into(),
                );
            }
        }
    }

    Ok(terminal_results)
}
```

</details>



### <span id="fn.build_node_args"></span>`fn build_node_args`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn build_node_args < 'py > (py : Python < 'py > , node_name : & str , node_decl : & PyNodeDecl , cache_values : & HashMap < String , serde_json :: Value > , node_results : & HashMap < String , PyObject > , incoming : & HashMap < String , Vec < (String , Option < String >) > > , trigger_less_ctx : Option < & PyObject > ,) -> Result < Bound < 'py , PyTuple > , GraphError >
```

<details>
<summary>Source</summary>

```rust
fn build_node_args<'py>(
    py: Python<'py>,
    node_name: &str,
    node_decl: &PyNodeDecl,
    cache_values: &HashMap<String, serde_json::Value>,
    node_results: &HashMap<String, PyObject>,
    incoming: &HashMap<String, Vec<(String, Option<String>)>>,
    trigger_less_ctx: Option<&PyObject>,
) -> Result<Bound<'py, PyTuple>, GraphError> {
    let mut args: Vec<PyObject> = Vec::new();

    // Trigger-less root: a node with no incoming edges and no cache inputs
    // gets the task Context as its sole arg.
    let has_incoming = incoming
        .get(node_name)
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    if let Some(ctx) = trigger_less_ctx {
        if !has_incoming && node_decl.cache_inputs.is_empty() {
            args.push(ctx.clone_ref(py));
            return PyTuple::new(py, &args).map_err(|e| {
                GraphError::NodeExecution(format!("args tuple creation failed: {}", e))
            });
        }
    }

    // Cache inputs
    for input_name in &node_decl.cache_inputs {
        if let Some(val) = cache_values.get(input_name) {
            let py_val = pythonize::pythonize(py, val).map_err(|e| {
                GraphError::Deserialization(format!(
                    "cache input '{}' conversion failed: {}",
                    input_name, e
                ))
            })?;
            args.push(py_val.unbind());
        } else {
            args.push(py.None());
        }
    }

    // Upstream node outputs
    if let Some(sources) = incoming.get(node_name) {
        for (from_node, variant) in sources {
            let key = if let Some(v) = variant {
                format!("{}:{}", from_node, v)
            } else {
                from_node.clone()
            };
            if let Some(result) = node_results.get(&key) {
                args.push(result.clone_ref(py));
            }
        }
    }

    PyTuple::new(py, &args)
        .map_err(|e| GraphError::NodeExecution(format!("args tuple creation failed: {}", e)))
}
```

</details>



### <span id="fn.extract_reactor_binding"></span>`fn extract_reactor_binding`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn extract_reactor_binding (obj : & Bound < '_ , PyAny >) -> PyResult < ReactorBinding >
```

Pull `NAME`, `ACCUMULATORS`, and `REACTION_MODE` off a `@cloaca.reactor`- decorated class. Anything else (string, dict, instance) is rejected here so the user gets a precise error rather than a downstream `AttributeError`.

<details>
<summary>Source</summary>

```rust
fn extract_reactor_binding(obj: &Bound<'_, PyAny>) -> PyResult<ReactorBinding> {
    if !obj.hasattr("NAME")? || !obj.hasattr("ACCUMULATORS")? || !obj.hasattr("REACTION_MODE")? {
        return Err(PyTypeError::new_err(
            "ComputationGraphBuilder(reactor=...) expects a class decorated with \
             @cloaca.reactor (must expose NAME / ACCUMULATORS / REACTION_MODE)",
        ));
    }
    let name: String = obj.getattr("NAME")?.extract()?;
    let accumulators: Vec<String> = obj.getattr("ACCUMULATORS")?.extract()?;
    let mode: String = obj.getattr("REACTION_MODE")?.extract()?;
    Ok(ReactorBinding {
        name,
        accumulators,
        mode,
    })
}
```

</details>



### <span id="fn.parse_graph_dict"></span>`fn parse_graph_dict`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn parse_graph_dict (graph : & Bound < '_ , PyDict >) -> PyResult < Vec < PyNodeDecl > >
```

<details>
<summary>Source</summary>

```rust
fn parse_graph_dict(graph: &Bound<'_, PyDict>) -> PyResult<Vec<PyNodeDecl>> {
    let mut nodes = Vec::new();
    for (key, value) in graph.iter() {
        let name: String = key.extract()?;
        let node_dict = value
            .downcast::<PyDict>()
            .map_err(|_| PyTypeError::new_err(format!("graph['{}'] must be a dict", name)))?;

        let cache_inputs: Vec<String> = if let Some(inputs) = node_dict.get_item("inputs")? {
            inputs
                .downcast::<PyList>()
                .map_err(|_| {
                    PyTypeError::new_err(format!("graph['{}']['inputs'] must be a list", name))
                })?
                .iter()
                .map(|item| item.extract::<String>())
                .collect::<PyResult<_>>()?
        } else {
            Vec::new()
        };

        let edge = if let Some(routes) = node_dict.get_item("routes")? {
            let routes_dict = routes.downcast::<PyDict>().map_err(|_| {
                PyTypeError::new_err(format!("graph['{}']['routes'] must be a dict", name))
            })?;
            let mut variants = Vec::new();
            for (v_key, v_val) in routes_dict.iter() {
                variants.push((v_key.extract::<String>()?, v_val.extract::<String>()?));
            }
            PyEdgeDecl::Routing { variants }
        } else if let Some(next) = node_dict.get_item("next")? {
            PyEdgeDecl::Linear {
                target: next.extract::<String>()?,
            }
        } else {
            PyEdgeDecl::Terminal
        };

        nodes.push(PyNodeDecl {
            name,
            cache_inputs,
            edge,
        });
    }
    Ok(nodes)
}
```

</details>



### <span id="fn.compute_execution_order"></span>`fn compute_execution_order`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn compute_execution_order (nodes : & [PyNodeDecl]) -> Vec < String >
```

<details>
<summary>Source</summary>

```rust
fn compute_execution_order(nodes: &[PyNodeDecl]) -> Vec<String> {
    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    for node in nodes {
        incoming.entry(node.name.clone()).or_default();
        match &node.edge {
            PyEdgeDecl::Linear { target } => {
                incoming
                    .entry(target.clone())
                    .or_default()
                    .push(node.name.clone());
            }
            PyEdgeDecl::Routing { variants } => {
                for (_, target) in variants {
                    incoming
                        .entry(target.clone())
                        .or_default()
                        .push(node.name.clone());
                }
            }
            PyEdgeDecl::Terminal => {}
        }
    }

    let mut in_degree: HashMap<String, usize> = HashMap::new();
    for (name, sources) in &incoming {
        in_degree.insert(name.clone(), sources.len());
    }

    let mut queue: Vec<String> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(name, _)| name.clone())
        .collect();
    queue.sort();

    let mut sorted = Vec::new();
    while let Some(name) = queue.pop() {
        sorted.push(name.clone());
        if let Some(node) = nodes.iter().find(|n| n.name == name) {
            let targets: Vec<String> = match &node.edge {
                PyEdgeDecl::Linear { target } => vec![target.clone()],
                PyEdgeDecl::Routing { variants } => {
                    variants.iter().map(|(_, t)| t.clone()).collect()
                }
                PyEdgeDecl::Terminal => vec![],
            };
            for target in targets {
                if let Some(deg) = in_degree.get_mut(&target) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(target);
                        queue.sort();
                    }
                }
            }
        }
    }

    sorted
}
```

</details>



