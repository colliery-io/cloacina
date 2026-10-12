---
title: "cloacina-python::bindings"
description: "Python API wrapper types for the cloaca wheel."
sidebar:
  order: 20
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

