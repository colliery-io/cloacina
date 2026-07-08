---
title: "Use Secrets in a Workflow"
description: "Declare the secrets a workflow needs, read them from a task through the resolution accessor, and bind them per instance with a $secret reference."
weight: 20
---

# Use Secrets in a Workflow

A [secret]({{< ref "/service/explanation/secrets" >}}) is the encrypted sibling
of a parameter: a named bundle of named fields a workflow references **by name**
rather than carrying in the clear. This guide covers the three authoring steps —
declaring the secrets a workflow requires, reading their values inside a task,
and binding a concrete secret to an instance.

For creating and rotating the secrets themselves, see
[Manage Secrets]({{< ref "/service/how-to/manage-secrets" >}}).

## Declare required secrets

Declare the secrets a workflow needs next to its params. Declared secrets ride
in the packaged manifest as encrypted, required input slots, so the runtime
knows which secrets a workflow expects — but their values are delivered through
a separate channel, never as params.

**Rust** — add a `secrets(...)` list to `#[workflow(...)]`:

```rust,ignore
#[workflow(
    name = "reach_out",
    params(region: String = "us-east-1".to_string()),
    secrets(db_prod, stripe_key)
)]
pub mod reach_out {
    use super::*;

    #[task(id = "call_api", dependencies = [])]
    pub async fn call_api(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        // ... read secrets here (see below)
        Ok(())
    }
}
```

A declared secret name may not collide with a declared param name, and duplicate
secret names are rejected at compile time.

**Python** — apply `@cloaca.workflow_secrets(...)`:

```python
@cloaca.workflow_secrets("db_prod", "stripe_key")
@cloaca.workflow("reach_out")
def build():
    ...
```

The compiler parses the decorator from source at build time into the same
encrypted manifest slots as the Rust form; at runtime it is a no-op
pass-through, so importing a package that uses it never fails.

## Read a secret from a task

Task bodies read resolved secrets through the `Context` secret accessor. The
returned map is handed to the task and is **never** written into the context's
serialized data, so it cannot reach `schedules.params`, the fires log, or
execution history.

```rust,ignore
// Whole secret: a { field: value } map.
let db = context.secret("db_prod").await?;
let host = &db["host"];

// A single field directly.
let password = context.secret_field("db_prod", "password").await?;
```

`secret(...)` errors clearly when no resolver is configured on the deployment
(`CLOACINA_SECRET_KEK` unset), when the name is not found or not granted, and
`secret_field(...)` when the secret exists but lacks the requested field.

On the embedded / in-process path, the host wires the resolver into the runner —
for example from `CLOACINA_SECRET_KEK`:

```rust,ignore
use cloacina::security::{SecretStore, SecretStoreResolver};

let store = SecretStore::new(dal);
if let Some(resolver) = SecretStoreResolver::from_env(store, org_id)? {
    runner_builder = runner_builder.secret_resolver(resolver.into_arc());
}
```

On the service path, the server attaches the resolver for you (directly for
in-process execution, or via the per-execution envelope wrap for the
[fleet]({{< ref "/service/explanation/execution-agent-fleet" >}})).

> The reading accessor (`context.secret` / `context.secret_field`) is a Rust
> `Context` API. Python workflows can **declare** required secrets with
> `@cloaca.workflow_secrets(...)`, and instance binding works identically, but a
> Python task-body accessor for reading resolved values is not yet part of the
> `cloaca` `Context`.

## Bind a secret to an instance

A [workflow instance]({{< ref "/engine/scheduling/workflow-instances" >}}) binds
values to a workflow's declared surface. To bind a **secret** instead of a plain
value, use a `$secret` **reference** in place of the value:

```json
{
  "region": "eu-west-1",
  "dst_credentials": { "$secret": "s3_prod" }
}
```

`region` is an ordinary param; `dst_credentials` is a *reference* to the secret
named `s3_prod`. At fire time the reference is routed away from the plaintext
param map into a separate name-to-name alias map (which carries no values), so
the task can read it by its declared binding name:

```rust,ignore
// Resolves the mapped secret `s3_prod` behind the local binding `dst_credentials`.
let creds = context.secret("dst_credentials").await?;
```

A `$secret` value must be exactly `{"$secret": "<name>"}` (a single key mapping
to a non-empty string); anything else is rejected as malformed. This composes
with the rest of the instance model — rotating `s3_prod` takes effect on the
next fire with no change to the instance or the workflow, because the binding
names the secret rather than embedding its value.

## See also

- [Secrets]({{< ref "/service/explanation/secrets" >}}) — the security model:
  encryption at rest, the no-leak guarantee, and fleet delivery.
- [Manage Secrets]({{< ref "/service/how-to/manage-secrets" >}}) — create,
  rotate, list, and delete secrets.
- [Workflow Instances]({{< ref "/engine/scheduling/workflow-instances" >}}) — the
  instance model `$secret` references compose with.
