---
title: "cloacina-python"
description: "Python integration for Cloacina."
editUrl: false
sidebar:
  order: 4
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Python integration for Cloacina.

This module provides PyO3 bindings: [`PyContext`](../../rust/cloacina-python/context/#struct.PyContext), [`PyWorkflowBuilder`](../../rust/cloacina-python/workflow/#struct.PyWorkflowBuilder),
[`PyTaskHandle`](../../rust/cloacina-python/task/#struct.PyTaskHandle), [`TaskDecorator`](../../rust/cloacina-python/task/#struct.TaskDecorator) (`@task`), and [`PythonTaskWrapper`](../../rust/cloacina-python/task/#struct.PythonTaskWrapper)
(implements [`cloacina::Task`](../../rust/cloacina-workflow/task/#trait.Task)). The `@task` decorator machinery and
`WorkflowBuilder` context manager live in this crate; the `cloaca` Python
wheel re-exports them via its `#[pymodule]` definition.

## Re-exports

- [`PyContext`](../../rust/cloacina-python/context/#struct.PyContext)
- [`PyTaskNamespace`](../../rust/cloacina-python/namespace/#struct.PyTaskNamespace)
- [`PyTaskHandle`](../../rust/cloacina-python/task/#struct.PyTaskHandle)
- [`PythonTaskWrapper`](../../rust/cloacina-python/task/#struct.PythonTaskWrapper)
- [`TaskDecorator`](../../rust/cloacina-python/task/#struct.TaskDecorator)
- [`task_decorator`](../../rust/cloacina-python/task/#fn.task) (`task::task`)
- [`PyWorkflow`](../../rust/cloacina-python/workflow/#struct.PyWorkflow)
- [`PyWorkflowBuilder`](../../rust/cloacina-python/workflow/#struct.PyWorkflowBuilder)
- [`py_register_workflow`](../../rust/cloacina-python/workflow/#fn.py_register_workflow)
- [`PyWorkflowContext`](../../rust/cloacina-python/workflow_context/#struct.PyWorkflowContext)
- [`PyTriggerResult`](../../rust/cloacina-python/bindings/trigger/#struct.PyTriggerResult)
- [`PythonTriggerDef`](../../rust/cloacina-python/trigger/#struct.PythonTriggerDef)
- [`PythonTriggerWrapper`](../../rust/cloacina-python/trigger/#struct.PythonTriggerWrapper)
- [`TriggerDecorator`](../../rust/cloacina-python/trigger/#struct.TriggerDecorator)
- [`drain_python_triggers`](../../rust/cloacina-python/trigger/#fn.drain_python_triggers)
- [`trigger_decorator`](../../rust/cloacina-python/trigger/#fn.trigger) (`trigger::trigger`)
- [`PythonLoaderError`](../../rust/cloacina-python/loader/#enum.PythonLoaderError)
- [`ensure_cloaca_module`](../../rust/cloacina-python/loader/#fn.ensure_cloaca_module)
- [`import_and_register_python_workflow`](../../rust/cloacina-python/loader/#fn.import_and_register_python_workflow)
- [`import_python_computation_graph`](../../rust/cloacina-python/loader/#fn.import_python_computation_graph)
- [`CloacinaPythonRuntime`](../../rust/cloacina-python/runtime_impl/#struct.CloacinaPythonRuntime)
- [`install`](../../rust/cloacina-python/runtime_impl/#fn.install)
- [`ScopedRuntime`](../../rust/cloacina-python/runtime_scope/#struct.ScopedRuntime)
- [`current_runtime`](../../rust/cloacina-python/runtime_scope/#fn.current_runtime)
