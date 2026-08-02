---
title: "Exceptions"
description: "How cloaca reports errors: built-in Python exceptions only, no custom hierarchy"
weight: 90
aliases:
  - "/python/api-reference/exceptions/"

---

# Exceptions

The `cloaca` package defines **no custom exception classes**. There is no
`CloacaException`, no `WorkflowError`, no `TaskError`, no `DatabaseError` — the
bindings raise standard Python built-in exceptions, chosen by error category.
Catch `ValueError`, `KeyError`, `RuntimeError`, and friends directly.

The `CloacinaApiError` class you may see in other Cloacina code belongs to the
separate **`cloacina-client`** service SDK (`pip install cloacina-client`), not
to `cloaca`. See the [Python SDK]({{< ref "/reference/sdks/python/" >}})
reference.

## Which built-in is raised when

| Exception | Raised by |
|-----------|-----------|
| `ValueError` | Most validation and operational failures: invalid decorator arguments (`@cloaca.task`, `@cloaca.trigger`, `@cloaca.constructor`), `WorkflowBuilder.add_task` / `build()` / `__exit__` failures (unknown task, cycle, validation), invalid `DefaultRunnerConfig` values, `DefaultRunner.with_schema` given a non-PostgreSQL URL or a bad schema name, and every `DefaultRunner` operation that the runtime rejects — `execute()` on an unknown workflow, an invalid cron expression in `register_cron_workflow`, unknown schedule/trigger ids, and so on |
| `KeyError` | `cloaca.var(name)` when `CLOACINA_VAR_<name>` is unset; `Context.update()`, `context[key]`, and `del context[key]` on a missing key; `Context.secret()` / `secret_field()` when the secret or field does not exist |
| `RuntimeError` | `DefaultRunner` / `DatabaseAdmin` construction failures (bad URL, connection or migration failure, runtime thread died); `DatabaseAdmin.create_tenant` / `remove_tenant` failures; `Context.secret()` when no secret resolver is configured (`CLOACINA_SECRET_KEK` unset) or the secrets backend fails |
| `PermissionError` | `Context.secret()` / `secret_field()` when the secret exists but is not granted to the workflow |
| `TypeError` | Wrong argument types passed to binding functions (for example a non-dict where a dict is required) |
| `AttributeError` | `ComputationGraphBuilder` exit when the topology references a node name with no matching `@cloaca.node` function |

Source: `crates/cloacina-python/src` — the crate contains no
`create_exception!` invocations; every error path maps onto one of the
built-ins above (see `context.rs`, `bindings/runner.rs`, `bindings/admin.rs`,
`computation_graph.rs`, `loader.rs`).

## Task failures are results, not exceptions

An exception raised *inside* a task body fails that task; the workflow engine
records the failure (and applies the task's retry policy) rather than
propagating the exception to the caller of `runner.execute()`. Inspect the
returned [`WorkflowResult`]({{< ref "/reference/python-api/pipeline-result/" >}})
instead:

```python
import cloaca

runner = cloaca.DefaultRunner("sqlite:///app.db")
result = runner.execute("my_workflow", cloaca.Context())

if result.status != "Completed":
    print(f"Workflow failed: {result.error_message}")
```

## Handling errors

```python
import cloaca

# Construction errors: RuntimeError
try:
    runner = cloaca.DefaultRunner("postgresql://user:pass@nonexistent:5432/db")
except RuntimeError as e:
    print(f"Could not start runner: {e}")

# Operational errors: ValueError
try:
    result = runner.execute("nonexistent_workflow", cloaca.Context())
except ValueError as e:
    print(f"Execution rejected: {e}")

try:
    runner.register_cron_workflow("daily_report", "not a cron expr", "UTC")
except ValueError as e:
    print(f"Invalid cron expression: {e}")

# Variable registry: KeyError
try:
    url = cloaca.var("DATABASE_URL")   # reads CLOACINA_VAR_DATABASE_URL
except KeyError:
    url = "sqlite:///default.db"
```

Because the categories are built-ins, standard Python patterns apply — there is
no package-specific base class to catch. If you need a catch-all around a
`cloaca` call, catch `Exception` and inspect the message.

## Service-client errors (`cloacina-client`, not `cloaca`)

Code that talks to a running `cloacina-server` over HTTP uses the separate
`cloacina-client` SDK, whose typed error **does** exist:

```python
from cloacina_client import Client, CloacinaApiError

client = Client("http://localhost:8080", api_key="...")
try:
    client.get_workflow("missing")
except CloacinaApiError as e:
    print(e.status, e.code, e.message)   # 404 workflow_not_found ...
```

`CloacinaApiError` carries the canonical `{error, code}` envelope
(`clients/python/src/cloacina_client/_client.py`). It is never raised by
`cloaca`.

## See Also

- **[DefaultRunner]({{< ref "/reference/python-api/runner/" >}})** — runner construction and operation errors
- **[Context]({{< ref "/reference/python-api/context/" >}})** — key and secret access errors
- **[WorkflowResult]({{< ref "/reference/python-api/pipeline-result/" >}})** — inspecting failed executions
- **[Python SDK]({{< ref "/reference/sdks/python/" >}})** — `CloacinaApiError` and the service client
