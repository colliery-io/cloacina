# Copyright 2025-2026 Colliery Software
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Live-server contract suite (CLOACI-I-0113 / REQ-007).

Exercises the GENERATED client against a real cloacina-server — the
drift detector between utoipa annotations and handler behavior. Every
documented endpoint is hit at least once; success paths that need a
compiled .cloacina package assert their documented error contract
instead (the full execute→push flow rides `angreal test sdk-contract`,
T-0648).

Skipped unless both env vars are set:
    CLOACINA_SERVER_URL  e.g. http://localhost:8080
    CLOACINA_API_KEY     a god-mode (bootstrap) key
"""

import json
import os
import time
import urllib.request

import pytest

from cloacina_client import AsyncClient, Client, CloacinaApiError, DELIVERY_PROTOCOL_VERSION

SERVER = os.environ.get("CLOACINA_SERVER_URL")
API_KEY = os.environ.get("CLOACINA_API_KEY")

pytestmark = pytest.mark.skipif(
    not (SERVER and API_KEY),
    reason="CLOACINA_SERVER_URL / CLOACINA_API_KEY not set",
)

RANDOM_UUID = "00000000-0000-4000-8000-000000000000"


@pytest.fixture(scope="module")
def tenant_name() -> str:
    return f"sdk_py_contract_{int(time.time() * 1000)}"


@pytest.fixture(scope="module")
def client(tenant_name: str):
    c = Client(SERVER, api_key=API_KEY, tenant=tenant_name)
    c.create_tenant(tenant_name, description="python sdk contract")
    yield c
    try:
        c.remove_tenant(tenant_name)
    except CloacinaApiError:
        pass


# ---- operational ----


def test_health(client: Client) -> None:
    body = client.health()
    assert body["status"] == "ok"


def test_openapi_served() -> None:
    with urllib.request.urlopen(f"{SERVER}/openapi.json", timeout=5) as resp:
        doc = json.load(resp)
    assert doc["openapi"].startswith("3.1")


# ---- keys ----


def test_key_lifecycle(client: Client) -> None:
    created = client.create_key(f"py-contract-{time.time_ns()}", role="read")
    assert created.key, "plaintext returned exactly once"
    assert created.permissions == "read"

    listed = client.list_keys()
    assert listed.total > 0
    mine = next(k for k in listed.items if k.id == created.id)
    assert not hasattr(mine, "key"), "list never exposes plaintext"

    revoked = client.revoke_key(created.id)
    assert revoked.status == "revoked"


def test_tenant_scoped_key(client: Client, tenant_name: str) -> None:
    created = client.create_tenant_key(f"py-contract-t-{time.time_ns()}", role="write")
    assert created.tenant_id == tenant_name
    client.revoke_key(created.id)


def test_ws_ticket(client: Client) -> None:
    ticket = client.create_ws_ticket()
    assert ticket.ticket
    assert ticket.expires_in_seconds > 0


# ---- tenants ----


def test_tenant_listed(client: Client, tenant_name: str) -> None:
    page = client.list_tenants()
    assert tenant_name in [t.name for t in page.items]


# ---- workflows ----


def test_upload_rejects_garbage(client: Client) -> None:
    with pytest.raises(CloacinaApiError) as exc:
        client.upload_workflow(b"not a real package")
    assert exc.value.status == 400


def test_workflow_list_envelope(client: Client, tenant_name: str) -> None:
    page = client.list_workflows()
    assert page.tenant_id == tenant_name
    assert page.total == len(page.items)


def test_workflow_missing_404(client: Client) -> None:
    with pytest.raises(CloacinaApiError) as exc:
        client.get_workflow("does-not-exist")
    assert exc.value.status == 404
    assert exc.value.code == "workflow_not_found"


def test_workflow_delete_idempotent(client: Client) -> None:
    # Documented contract decision (T-0645): unregister is idempotent.
    deleted = client.delete_workflow("does-not-exist", "0.0.0")
    assert deleted.status == "deleted"


# ---- triggers ----


def test_trigger_list_and_pagination(client: Client, tenant_name: str) -> None:
    page = client.list_triggers(limit=10, offset=0)
    assert page.tenant_id == tenant_name

    with pytest.raises(CloacinaApiError) as exc:
        client.list_triggers(limit=100_000)
    assert exc.value.status == 400
    assert exc.value.code == "invalid_pagination"


def test_trigger_missing_404(client: Client) -> None:
    with pytest.raises(CloacinaApiError) as exc:
        client.get_trigger("does-not-exist")
    assert exc.value.status == 404


# ---- executions ----


def test_execute_unknown_workflow(client: Client) -> None:
    with pytest.raises(CloacinaApiError) as exc:
        client.execute_workflow("does-not-exist", {"k": "v"})
    assert exc.value.status == 400
    assert exc.value.code == "execution_failed"


def test_execution_list_and_iteration(client: Client, tenant_name: str) -> None:
    page = client.list_executions(status="Completed", limit=5)
    assert page.tenant_id == tenant_name
    drained = list(client.iterate_executions(page_size=2))
    assert isinstance(drained, list)


def test_execution_invalid_and_missing_ids(client: Client) -> None:
    with pytest.raises(CloacinaApiError) as exc:
        client.get_execution("not-a-uuid")
    assert exc.value.status == 400

    with pytest.raises(CloacinaApiError) as exc:
        client.get_execution(RANDOM_UUID)
    assert exc.value.status == 404

    # Events endpoint returns an empty envelope (not 404) for a valid
    # unknown UUID — documented contract.
    events = client.get_execution_events(RANDOM_UUID)
    assert events.execution_id == RANDOM_UUID
    assert events.events == []


# ---- computation-graph health ----


def test_graph_health_endpoints(client: Client) -> None:
    accs = client.list_accumulators()
    assert accs.total == len(accs.items)
    graphs = client.list_graphs()
    assert graphs.total == len(graphs.items)
    with pytest.raises(CloacinaApiError) as exc:
        client.get_graph("does-not-exist")
    assert exc.value.status == 404


# ---- async + WS subscription lifecycle ----


async def test_async_client_basics() -> None:
    aclient = AsyncClient(SERVER, api_key=API_KEY)
    body = await aclient.health()
    assert body["status"] == "ok"
    page = await aclient.list_executions(limit=1)
    assert page.total == len(page.items)


async def test_ws_subscription_lifecycle() -> None:
    import asyncio

    import websockets

    aclient = AsyncClient(SERVER, api_key=API_KEY)

    # Quiet recipient: the stream connects (welcome + hello round-trip
    # internally) and sits idle — a timeout with no error proves the
    # lifecycle.
    stream = aclient.subscribe_delivery(
        f"exec_events:{RANDOM_UUID}", reconnect=False
    )
    try:
        await asyncio.wait_for(stream.__anext__(), timeout=2.0)
    except asyncio.TimeoutError:
        pass  # connected and idle — expected
    finally:
        await stream.aclose()

    # hello with an unsupported version must close 4426.
    ticket = (await aclient.create_ws_ticket()).ticket
    ws_base = SERVER.replace("http://", "ws://").replace("https://", "wss://")
    url = f"{ws_base}/v1/ws/delivery/exec_events%3A{RANDOM_UUID}?token={ticket}"
    async with websockets.connect(url) as socket:
        await socket.send(
            json.dumps({"type": "hello", "protocol_version": 99, "since_id": None})
        )
        with pytest.raises(websockets.ConnectionClosed):
            while True:
                await asyncio.wait_for(socket.recv(), timeout=5.0)
    # The close frame carried the documented code.
    assert socket.close_code == 4426
    assert DELIVERY_PROTOCOL_VERSION == 1
