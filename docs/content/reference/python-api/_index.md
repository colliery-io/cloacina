---
title: "API Reference"
description: "Complete Python API reference for the cloaca package"
weight: 40
aliases:
  - "/python/api-reference/"

---

# Python API Reference

Complete reference for all classes, functions, and decorators in the `cloaca` package.

## Where each name is available

The `cloaca` module exists in two embeddings that share one registration
function (`register_authoring`, `crates/cloacina-python/src/lib.rs`): the pip
wheel (`pip install cloaca`) and the synthetic module the server/agent injects
when loading packaged Python workflows. Every **authoring** symbol is available
in both — the wheel additionally ships a **host surface** for running workflows
in your own process.

**Authoring surface (available everywhere):** `Context`, `@cloaca.task`,
`TaskHandle`, `@cloaca.constructor`, the trigger-rule builders
(`context_value`, `task_success`, `task_failed`, `task_skipped`, `all_of`,
`any_of`, `none_of`, `always`), `@cloaca.trigger`, `TriggerResult`,
`@cloaca.reactor`, `WorkflowBuilder`, `Workflow`,
`register_workflow_constructor`, `@cloaca.workflow_params`,
`@cloaca.workflow_secrets`, `@cloaca.boundary_schema`, `TaskNamespace`,
`WorkflowContext`, `RetryPolicy` / `RetryPolicyBuilder` / `BackoffStrategy` /
`RetryCondition`, `ComputationGraphBuilder`, `@cloaca.node`, the accumulator
decorators (`passthrough_accumulator`, `stream_accumulator`,
`polling_accumulator`, `batch_accumulator`, `state_accumulator`), and
`cloaca.var` / `cloaca.var_or`.

**Wheel-only host surface (pip wheel, never in packaged workflows):**

| Symbol | Notes |
|--------|-------|
| `DefaultRunner` | Embedded workflow runner |
| `WorkflowResult` | Returned by `DefaultRunner.execute()` |
| `DefaultRunnerConfig` | Runner configuration |
| `_shutdown_all_runners` | Internal; auto-registered with `atexit` so runner threads are joined before interpreter finalization |
| `DatabaseAdmin`, `TenantConfig`, `TenantCredentials` | Multi-tenant admin; compiled in only when the wheel's `postgres` feature is enabled (the published PyPI wheel enables it) |

Packaged workflows run inside a server or agent — the server **is** the
runner, so the host surface is deliberately absent there.

`@cloaca.boundary_schema` is a Python-only decorator: it declares the typed
shape of a Python accumulator's boundary, the parity of deriving
`schemars::JsonSchema` on a Rust boundary type. There is no Rust
`boundary_schema` macro.

{{< toc-tree >}}
