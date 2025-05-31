"""
Cloacina: Python bindings for the Cloacina workflow orchestration framework.

This package provides Python bindings for Cloacina, allowing you to define
and execute resilient task pipelines directly from Python code.

Example:
    ```python
    from cloacina import task, Workflow, UnifiedExecutor

    @task(id="extract_data", dependencies=[])
    def extract_data(context):
        # Your extraction logic here
        context["raw_data"] = {"users": [1, 2, 3]}
        return context

    @task(id="transform_data", dependencies=["extract_data"])
    def transform_data(context):
        # Your transformation logic here
        raw = context.get("raw_data", {})
        context["transformed_data"] = {"processed": raw}
        return context

    # Create and run workflow
    workflow = Workflow("my_etl_pipeline")

    async def main():
        executor = UnifiedExecutor()
        await executor.initialize()
        await executor.execute(workflow)
        await executor.shutdown()

    import asyncio
    asyncio.run(main())
    ```
"""

from ._cloacina_postgres import (
    __version__,
    task,
    Workflow,
    UnifiedExecutor,
    TaskDecorator,
)

__all__ = [
    "__version__",
    "task",
    "Workflow",
    "UnifiedExecutor",
    "TaskDecorator",
]
