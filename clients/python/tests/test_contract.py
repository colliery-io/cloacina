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
from cloacina_client import (
    DELIVERY_PROTOCOL_VERSION,
    AsyncClient,
    Client,
    CloacinaApiError,
)

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


def test_ready(client: Client) -> None:
    status, body = client.ready()
    assert status in (200, 503)
    assert "status" in body


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


# ---- audit record (CLOACINA-T-1024) ----


def test_tenant_audit_has_the_tenant_create(client: Client, tenant_name: str) -> None:
    page = client.list_tenant_audit(action="tenant.create")
    assert page.total == len(page.items)
    rows = [r for r in page.items if r.target == tenant_name]
    assert rows, "the tenant create is in the tenant's audit record"
    assert rows[0].tenant_id == tenant_name
    assert rows[0].outcome == "ok"
    assert rows[0].actor_key_id


def test_audit_filters_by_tenant_and_pages(client: Client, tenant_name: str) -> None:
    page = client.list_audit(tenant=tenant_name, limit=1)
    assert len(page.items) == 1
    assert page.items[0].tenant_id == tenant_name
    assert page.next_cursor == page.items[0].id

    older = client.list_audit(tenant=tenant_name, limit=1, cursor=page.next_cursor)
    assert all(r.id < page.items[0].id for r in older.items)

    with pytest.raises(CloacinaApiError) as exc:
        client.list_audit(limit=0)
    assert exc.value.status == 400


# ---- secrets (CLOACI-I-0133 / T-0862) ----


def test_secret_lifecycle(client: Client) -> None:
    """create → list → get → rotate → delete; every read is metadata-only."""
    name = f"py-contract-secret-{int(time.time() * 1000)}"

    created = client.create_secret(name, {"token": "s3cr3t"})
    assert created.name == name
    assert created.field_names == ["token"]
    # Metadata-only contract: no value ever comes back on a read.
    assert "s3cr3t" not in json.dumps(created.to_dict())

    page = client.list_secrets()
    assert name in [s.name for s in page.items]

    fetched = client.get_secret(name)
    assert fetched.id == created.id

    rotated = client.rotate_secret(name, {"token": "rotated", "extra": "field"})
    assert len(rotated.field_names) == 2

    with pytest.raises(CloacinaApiError) as exc:
        client.get_secret("does-not-exist")
    assert exc.value.status == 404

    deleted = client.delete_secret(name)
    assert deleted.status == "deleted"
    assert deleted.name == name


# ---- trusted keys of package signing (CLOACINA-T-1061) ----


def test_trusted_key_lifecycle(client: Client) -> None:
    """add → list → revoke; a revoked key stays in the list, marked."""
    public_key = os.urandom(32).hex()
    name = f"py-contract-key-{int(time.time() * 1000)}"

    added = client.add_trusted_key(public_key=public_key, name=name)
    assert added.name == name
    assert len(added.fingerprint) == 64
    assert added.revoked is False

    page = client.list_trusted_keys()
    assert added.id in [k.id for k in page.items]

    # The tenant holds a key once.
    with pytest.raises(CloacinaApiError) as exc:
        client.add_trusted_key(public_key=public_key)
    assert exc.value.status == 409

    revoked = client.revoke_trusted_key(added.id)
    assert revoked.status == "revoked"
    assert revoked.id == added.id

    page = client.list_trusted_keys()
    assert [k.revoked for k in page.items if k.id == added.id] == [True]

    with pytest.raises(CloacinaApiError) as exc:
        client.revoke_trusted_key("00000000-0000-4000-8000-000000000000")
    assert exc.value.status == 404


# ---- agent pools (CLOACINA-T-1021) ----


def test_agent_pool_lifecycle(client: Client) -> None:
    """list → put → get → replace → delete; a bad quantity is a 400."""
    pool = f"py-{int(time.time() * 1000)}"

    # Every tenant has the pool `default`.
    assert "default" in [p.pool for p in client.list_pools().items]

    created = client.put_pool(
        pool,
        {
            "memory_limit": "2Gi",
            "max_concurrency": 1,
            "pod_labels": {"team": "data"},
            "max_agents": 0,
        },
    )
    assert created.pool == pool
    assert created.settings.memory_limit == "2Gi"
    assert created.desired_agents == 0

    fetched = client.get_pool(pool)
    assert fetched.settings.max_concurrency == 1

    page = client.list_pools()
    assert pool in [p.pool for p in page.items]
    assert page.total == len(page.items)

    # PUT replaces the settings as a whole.
    replaced = client.put_pool(pool, {"max_agents": 0})
    assert replaced.settings.to_dict() == {"max_agents": 0}

    with pytest.raises(CloacinaApiError) as exc:
        client.put_pool(pool, {"memory_limit": "2GB", "max_agents": 0})
    assert exc.value.status == 400

    client.delete_pool(pool)
    with pytest.raises(CloacinaApiError) as exc:
        client.get_pool(pool)
    assert exc.value.status == 404


# ---- backfills (CLOACINA-T-1020) ----


def test_backfill_lifecycle(client: Client) -> None:
    """create → list → inspect → retry (409) → cancel; a bad range is a 400."""
    workflow = f"py_backfill_{int(time.time() * 1000)}"

    # A range in the future: the scheduler takes none of its intervals, so
    # the progress is the same on each read.
    created = client.create_backfill(
        workflow,
        "2099-01-01T00:00:00Z",
        "2099-01-01T04:00:00Z",
        cron="0 * * * *",
        max_active_runs=2,
        params={"region": "eu"},
    )
    assert created.workflow_name == workflow
    assert created.status == "running"
    assert created.max_active_runs == 2
    assert created.progress.total == 5
    assert created.progress.remaining == 5

    page = client.list_backfills(workflow, limit=10, offset=0)
    assert created.id in [b.id for b in page.items]

    detail = client.get_backfill(workflow, created.id)
    assert detail.backfill.id == created.id
    assert detail.failed_intervals == []
    assert detail.failed_intervals_truncated is False

    # A retry needs a failed interval.
    with pytest.raises(CloacinaApiError) as exc:
        client.retry_backfill(workflow, created.id)
    assert exc.value.status == 409

    cancelled = client.cancel_backfill(workflow, created.id)
    assert cancelled.status == "cancelled"

    with pytest.raises(CloacinaApiError) as exc:
        client.create_backfill(
            workflow,
            "2099-01-02T00:00:00Z",
            "2099-01-01T00:00:00Z",
            cron="0 * * * *",
        )
    assert exc.value.status == 400

    with pytest.raises(CloacinaApiError) as exc:
        client.get_backfill(workflow, "00000000-0000-4000-8000-000000000000")
    assert exc.value.status == 404


# ---- workflows ----


def test_upload_rejects_garbage(client: Client) -> None:
    with pytest.raises(CloacinaApiError) as exc:
        client.upload_workflow(b"not a real package")
    assert exc.value.status == 400


def test_upload_signature_is_sent_and_checked(client: Client) -> None:
    """The signature of a signed upload reaches the server (CLOACINA-T-1062).

    A text that is no signature is a 400 ``malformed_signature``. A signature
    made for other bytes is a 403 ``package_tampered``, before any key is
    looked up.
    """
    with pytest.raises(CloacinaApiError) as exc:
        client.upload_workflow(b"not a real package", signature="not a signature")
    assert exc.value.status == 400
    assert exc.value.code == "malformed_signature"

    for_other_bytes = json.dumps(
        {
            "version": 1,
            "algorithm": "ed25519",
            "package_hash": "0" * 64,
            "key_fingerprint": "0" * 64,
            "signature": "AAAA",
            "signed_at": "2026-01-01T00:00:00Z",
        }
    )
    with pytest.raises(CloacinaApiError) as exc:
        client.upload_workflow(b"not a real package", signature=for_other_bytes)
    assert exc.value.status == 403
    assert exc.value.code == "package_tampered"


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


def test_workflow_versions_missing_404(client: Client) -> None:
    # CLOACINA-T-1018: a package with no version is a 404.
    with pytest.raises(CloacinaApiError) as exc:
        client.list_workflow_versions("does-not-exist")
    assert exc.value.status == 404
    assert exc.value.code == "workflow_not_found"


def test_workflow_version_activate_missing_404(client: Client) -> None:
    with pytest.raises(CloacinaApiError) as exc:
        client.activate_workflow_version("does-not-exist", "0.0.0")
    assert exc.value.status == 404
    assert exc.value.code == "workflow_version_not_found"


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
    # I-0138 cross-tenant gate: a workflow not registered for THIS tenant
    # fails closed with 404 (the `public` tenant is exempt).
    with pytest.raises(CloacinaApiError) as exc:
        client.execute_workflow("does-not-exist", {"k": "v"})
    assert exc.value.status == 404
    assert exc.value.code == "workflow_not_found"


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
    except TimeoutError:
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
