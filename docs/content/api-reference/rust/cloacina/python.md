# cloacina::python <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Python integration for Cloacina.

This module provides:
- Abstract [`PythonTaskExecutor`] trait for executing Python tasks from packages
- Concrete PyO3 bindings: [`PyContext`], [`PyWorkflowBuilder`], [`PyTaskHandle`],
[`TaskDecorator`] (`@task`), and [`PythonTaskWrapper`] (implements [`crate::Task`])
The `@task` decorator machinery and `WorkflowBuilder` context manager are compiled
into the cloacina binary. The `cloaca` Python wheel re-exports these types via its
`#[pymodule]` definition.
