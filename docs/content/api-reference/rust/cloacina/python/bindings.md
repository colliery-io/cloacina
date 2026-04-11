# cloacina::python::bindings <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Python API wrapper types for the cloaca wheel.

These types wrap cloacina's Rust API for Python consumers:
- `PyDefaultRunner` / `PyWorkflowResult` — workflow execution
- `PyDefaultRunnerConfig` — runner configuration
- `PyDatabaseAdmin` / `PyTenantConfig` / `PyTenantCredentials` — admin
- `PyTriggerResult` — trigger results
- `PyRetryPolicy` / `PyBackoffStrategy` / `PyRetryCondition` — retry config
