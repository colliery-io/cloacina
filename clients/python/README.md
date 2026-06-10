# cloacina-client

Python SDK for [cloacina-server](https://github.com/colliery-io/cloacina) — a typed REST client plus WebSocket execution-event streaming, for Python 3.10+.

> **This is the service client.** It talks to a running cloacina-server over HTTP/WebSocket. The embedded workflow runtime is the separate [`cloaca`](https://pypi.org/project/cloaca/) package — the two are deliberately distinct.

**Version lockstep:** `cloacina-client X.Y.Z` is generated from, tested against, and only supported on `cloacina-server X.Y.Z`.

## Install

```bash
pip install cloacina-client
```

## Quickstart

```python
from cloacina_client import Client

client = Client("http://localhost:8080", api_key="clk_...", tenant="public")

accepted = client.execute_workflow("my_workflow", {"input": 42})
print(accepted.execution_id)

for execution in client.iterate_executions(status="Failed"):
    print(execution.id, execution.workflow_name)
```

Every helper raises `CloacinaApiError` (with `.status` and the server's machine-readable `.code`) on non-2xx responses. The generated [`openapi-python-client`](https://github.com/openapi-generators/openapi-python-client) client is available as `client.generated` for anything the shim doesn't cover.

## Async + live execution events

```python
import asyncio
from cloacina_client import AsyncClient

async def main():
    client = AsyncClient("http://localhost:8080", api_key="clk_...")
    accepted = await client.execute_workflow("my_workflow", {"input": 42})
    # Reconnect, dedup, and acks are handled for you.
    async for event in client.follow_execution_events(accepted.execution_id):
        print(event)

asyncio.run(main())
```

WebSocket connections never carry the long-lived key: the SDK mints a single-use, 60-second ticket (`POST /v1/auth/ws-ticket`) per connection. A `4426` close (protocol version mismatch) raises `ProtocolVersionError` — upgrade the SDK.

## Regenerating

`src/cloacina_client/_generated/` is produced by a pinned generator from the committed server contract:

```bash
uvx openapi-python-client@0.29.0 generate \
  --path ../../docs/static/openapi.json \
  --config generator-config.yaml --meta none \
  --output-path src/cloacina_client/_generated --overwrite
```

## Contract tests

`tests/test_contract.py` exercises every documented endpoint plus the WebSocket lifecycle against a live server:

```bash
CLOACINA_SERVER_URL=http://localhost:8080 \
CLOACINA_API_KEY=<bootstrap-key> \
uv run pytest
```

The full execute→event-stream flow (which needs a compiled `.cloacina` package) runs in the repo's `angreal test sdk-contract` harness.
