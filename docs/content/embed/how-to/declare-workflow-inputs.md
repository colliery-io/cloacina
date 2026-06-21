---
title: "Declare and validate workflow inputs"
description: "Declare typed input params on a workflow with #[workflow(params(...))]; they surface as JSON-Schema-typed slots on the API and are validated at execute time (CLOACI-I-0128)."
weight: 19
aliases:
  - "/workflows/how-to-guides/declare-workflow-inputs/"

---

# Declare and validate workflow inputs

By default a workflow accepts a free-form execution context — any JSON object.
That's flexible, but it leaves callers (and the UI) guessing about *what* a
workflow expects. With **declared params** (CLOACI-I-0128) you state the inputs a
workflow accepts, their types, and their defaults right in the workflow
attribute. The declaration is:

- **Self-documenting** — the params surface on the workflow API as
  JSON-Schema-typed slots, so a UI can render a typed execute form.
- **Enforced** — the server validates the provided context against the
  declaration at execute time and rejects mismatches before the run starts.

Undeclared workflows are unchanged: with no `params(...)`, the context stays
free-form and nothing is validated.

## Declare params

Add a `params( … )` clause to the `#[workflow]` attribute. Each entry is
`name: Type`, optionally `= default`:

```rust
use cloacina_workflow::{task, workflow, Context, TaskError};

#[workflow(
    name = "analytics_workflow",
    description = "Analytics and data processing pipeline",
    params(
        source_id: String,        // required (no default)
        batch_size: u32 = 500,    // optional, defaults to 500
    )
)]
pub mod analytics_workflow {
    use super::*;

    #[task]
    pub async fn extract_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        // `source_id` and `batch_size` are available in the context.
        Ok(())
    }
}
```

- A param **without** a default is **required**.
- A param **with** a default is **optional** — omitting it at execute time is
  allowed.
- The type can be any `T: serde::Serialize + schemars::JsonSchema` (scalars,
  and structs/enums that derive `JsonSchema`). The JSON Schema is derived
  automatically via `schemars`.

### Python

Packaged Python workflows declare params with the `@cloaca.workflow_params(...)`
decorator on any task in the workflow module. The compiler parses it from source
at build time — at runtime the decorator is a no-op:

```python
import cloaca

@cloaca.workflow_params(
    source_id=str,            # required
    batch_size=(int, 500),    # optional, with default
)
@cloaca.task(dependencies=[])
def prepare(context):
    return context
```

Supported scalar types map to JSON Schema: `str`→string, `int`→integer,
`float`→number, `bool`→boolean, `list`→array, `dict`→object. Use `(type, default)`
for an optional param. The declared params surface and validate identically to
the Rust path.

## How params surface

Once the package is built and registered, each param is exposed as an
`InputSlot` on the workflow detail API
(`GET /v1/tenants/{tenant}/workflows/{name}`), under `declared_params`:

```json
{
  "workflow_name": "analytics_workflow",
  "declared_params": [
    { "name": "source_id", "schema": { "type": "string" }, "required": true },
    { "name": "batch_size", "schema": { "type": "integer", "format": "uint32" },
      "required": false, "default": 500 }
  ]
}
```

A UI reads `declared_params` to render a typed execute form instead of a raw
JSON textarea.

## Validation at execute time

When you execute a workflow that declares params
(`POST /v1/tenants/{tenant}/workflows/{name}/execute`), the server validates the
provided context against the declaration:

- A **missing required** param (with no default) is rejected.
- A **type mismatch** against the param's top-level JSON-Schema `type` is
  rejected.

On failure the server returns **`400`** with code `workflow_input_invalid` and a
per-field message, e.g.:

```json
{ "error": "workflow_input_invalid",
  "message": "invalid execution context: missing required param 'source_id'" }
```

Workflows that declare no params skip validation entirely (free-form context).

> **v1 scope.** Validation currently checks required-presence and the top-level
> scalar `type`. Full nested JSON-Schema validation (deeply-typed structs,
> enums, constraints) is a planned follow-up; compound schemas are accepted
> rather than rejected today.

## Related: operator injection for graph surfaces

The same "typed JSON in, encoded server-side" ergonomic is available for
operators driving running computation-graph surfaces for a manual check:

- **Reactor fire** — `POST /v1/health/reactors/{name}/fire`
  (`cloacinactl reactor fire <name> --input source=<json>`).
- **Accumulator inject** — `POST /v1/health/accumulators/{name}/inject`
  (`cloacinactl accumulator inject <name> --event <json>`).

Both serialize the supplied JSON to the boundary wire encoding for you and
audit-log the injection as operator-driven.
