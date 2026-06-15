---
title: "Python SDK"
description: "cloacina-client — typed Python client for cloacina-server"
weight: 20
aliases:
  - "/sdks/python/"

---

# Python SDK (`cloacina-client`)

> This is the **service client** (`pip install cloacina-client`, `import cloacina_client`). The embedded workflow runtime is the separate [`cloaca`](https://pypi.org/project/cloaca/) package.

Generated from the server's OpenAPI contract by a pinned `openapi-python-client`, wrapped in a hand-written shim. Python 3.10+.

## Tutorial: execute a workflow and follow its events

```bash
pip install cloacina-client
```

```python
import asyncio
import os

from cloacina_client import AsyncClient, Client

client = Client("http://localhost:8080", api_key=os.environ["CLOACINA_API_KEY"], tenant="public")
accepted = client.execute_workflow("my_workflow", {"input": 42})
print("scheduled", accepted.execution_id)

async def follow() -> None:
    aclient = AsyncClient("http://localhost:8080", api_key=os.environ["CLOACINA_API_KEY"])
    async for event in aclient.follow_execution_events(accepted.execution_id):
        print(event)

asyncio.run(follow())
```

## How-to

**Paginate executions** (sync generator; `AsyncClient.iterate_executions` is the async twin):

```python
for execution in client.iterate_executions(status="Failed", page_size=100):
    print(execution.id, execution.workflow_name)
```

**Handle errors** — `CloacinaApiError` carries the canonical envelope:

```python
from cloacina_client import CloacinaApiError

try:
    client.get_workflow("missing")
except CloacinaApiError as e:
    print(e.status, e.code, e.message)  # 404 workflow_not_found ...
```

**Subscribe to raw delivery pushes:**

```python
async for push in aclient.subscribe_delivery("exec_events:<id>"):
    print(push.kind, push.payload_json())
```

Reconnection, dedup-on-row-id, and acks are handled inside the iterator; a `4426` close raises `ProtocolVersionError` (upgrade the SDK).

**Reach past the shim** — the generated `openapi-python-client` is exposed as `client.generated` for anything the helpers don't cover.

## Reference

- Wire contract: [OpenAPI document](/openapi.json), [WebSocket protocol](/reference/websocket-protocol/)
- Generated models live under `cloacina_client.models`
- Regeneration (pinned): see `clients/python/README.md` in the repository
