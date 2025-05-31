# Cloacina

Python bindings for the Cloacina workflow orchestration framework.

## Installation

Choose your database backend:

```bash
# PostgreSQL backend (recommended)
pip install cloacina[postgres]

# SQLite backend
pip install cloacina[sqlite]
```

## Quick Start

```python
from cloacina import task, Workflow, UnifiedExecutor

@task(id="extract_data", dependencies=[])
def extract_data(context):
    context["raw_data"] = {"users": [1, 2, 3]}
    return context

@task(id="transform_data", dependencies=["extract_data"])
def transform_data(context):
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

## Documentation

For full documentation, visit: https://colliery-io.github.io/cloacina/