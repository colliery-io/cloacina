---
title: "WorkflowResult"
description: "The result object returned by DefaultRunner.execute()"
weight: 80
aliases:
  - "/python/api-reference/pipeline-result/"

---

# WorkflowResult

`DefaultRunner.execute()` returns a `WorkflowResult` — the outcome of one
workflow execution. There is no `PipelineResult` class in `cloaca`; the class
is named `WorkflowResult` (`crates/cloacina-python/src/bindings/runner.rs`,
`#[pyclass(name = "WorkflowResult")]`).

`WorkflowResult` is **wheel-only**: it exists in the pip-installed `cloaca`
package alongside `DefaultRunner`, and is not part of the authoring surface
available inside packaged workflows (where the server is the runner).

## Properties

All properties are read-only.

| Property | Type | Meaning |
|----------|------|---------|
| `status` | `str` | Final execution status (see values below) |
| `start_time` | `str` | Execution start, RFC 3339 timestamp string |
| `end_time` | `str` or `None` | Execution end, RFC 3339 timestamp string; `None` if not recorded |
| `final_context` | `Context` | The context after execution finished |
| `error_message` | `str` or `None` | Failure message when the workflow did not complete; `None` on success |

Timestamps are strings, not `datetime` objects — parse with
`datetime.fromisoformat()` if you need arithmetic. There is no `duration`,
`workflow_name`, or `execution_id` property.

## Status values

`status` is the string form of the engine's workflow status enum
(`crates/cloacina/src/executor/workflow_executor.rs`, `WorkflowStatus`):

- `"Completed"` — all tasks finished successfully
- `"Failed"` — the workflow failed
- `"Pending"`, `"Running"`, `"Cancelled"`, `"Paused"` — the remaining enum
  states; a synchronous `execute()` call normally returns only a terminal
  status

## Usage

```python
import cloaca
from datetime import datetime

runner = cloaca.DefaultRunner("sqlite:///:memory:")
result = runner.execute("my_workflow", cloaca.Context({"input": 42}))

print(result)  # WorkflowResult(status=Completed, error=None)

if result.status == "Completed":
    output = result.final_context.get("output_data")
    print(f"Output: {output}")
else:
    print(f"Workflow failed: {result.error_message}")

# Timing (timestamps are RFC 3339 strings)
started = datetime.fromisoformat(result.start_time)
if result.end_time is not None:
    finished = datetime.fromisoformat(result.end_time)
    print(f"Took {(finished - started).total_seconds():.2f}s")
```

## Accessing the final context

`final_context` is a regular [`Context`]({{< ref "/reference/python-api/context/" >}})
— use `get()`, `to_dict()`, and the dictionary-style operations:

```python
final = result.final_context

records = final.get("records_processed", 0)
everything = final.to_dict()
```

## Failure inspection

When `status` is not `"Completed"`, `error_message` carries the engine's
failure message. Task-level detail (which task failed and why) lives in the
execution records in the database, not on the result object; anything a task
wrote into the context before the failure is still visible in
`final_context`.

```python
if result.status == "Failed":
    print(f"Error: {result.error_message}")
    partial = result.final_context.to_dict()
```

## See Also

- **[DefaultRunner]({{< ref "/reference/python-api/runner/" >}})** — `execute()` returns this object
- **[Context]({{< ref "/reference/python-api/context/" >}})** — the type of `final_context`
- **[Exceptions]({{< ref "/reference/python-api/exceptions/" >}})** — task failures are results, binding errors are exceptions
