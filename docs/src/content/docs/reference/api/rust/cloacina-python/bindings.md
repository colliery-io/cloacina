---
title: "cloacina-python::bindings"
description: "Python API wrapper types for the cloaca wheel."
editUrl: false
sidebar:
  order: 5
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Python API wrapper types for the cloaca wheel.

These types wrap cloacina's Rust API for Python consumers:
- `PyDefaultRunner` / `PyWorkflowResult` — workflow execution
- `PyDefaultRunnerConfig` — runner configuration
- `PyDatabaseAdmin` / `PyTenantConfig` / `PyTenantCredentials` — admin
- `PyTriggerResult` — trigger results
- `PyRetryPolicy` / `PyBackoffStrategy` / `PyRetryCondition` — retry config

## Re-exports

- [`PyDatabaseAdmin`](../../../rust/cloacina-python/bindings/admin/#struct.PyDatabaseAdmin)
- [`PyTenantConfig`](../../../rust/cloacina-python/bindings/admin/#struct.PyTenantConfig)
- [`PyTenantCredentials`](../../../rust/cloacina-python/bindings/admin/#struct.PyTenantCredentials)
- [`PyDefaultRunnerConfig`](../../../rust/cloacina-python/bindings/context/#struct.PyDefaultRunnerConfig)
- [`PyDefaultRunner`](../../../rust/cloacina-python/bindings/runner/#struct.PyDefaultRunner)
- [`PyWorkflowResult`](../../../rust/cloacina-python/bindings/runner/#struct.PyWorkflowResult)
- [`PyTriggerResult`](../../../rust/cloacina-python/bindings/trigger/#struct.PyTriggerResult)
- [`PyBackoffStrategy`](../../../rust/cloacina-python/bindings/value_objects/retry/#struct.PyBackoffStrategy)
- [`PyRetryCondition`](../../../rust/cloacina-python/bindings/value_objects/retry/#struct.PyRetryCondition)
- [`PyRetryPolicy`](../../../rust/cloacina-python/bindings/value_objects/retry/#struct.PyRetryPolicy)
- [`PyRetryPolicyBuilder`](../../../rust/cloacina-python/bindings/value_objects/retry/#struct.PyRetryPolicyBuilder)
