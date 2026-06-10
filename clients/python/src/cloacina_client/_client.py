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

"""Ergonomics shim over the generated client (sync + async variants)."""

from __future__ import annotations

import json
from collections.abc import AsyncIterator, Iterator
from typing import Any

from ._generated import AuthenticatedClient
from ._generated.api.executions import (
    execute_workflow,
    get_execution,
    get_execution_events,
    list_executions,
)
from ._generated.api.graph_health import get_graph, list_accumulators, list_graphs
from ._generated.api.keys import (
    create_key,
    create_tenant_key,
    create_ws_ticket,
    list_keys,
    revoke_key,
)
from ._generated.api.operational import health
from ._generated.api.tenants import create_tenant, list_tenants, remove_tenant
from ._generated.api.triggers import get_trigger, list_triggers
from ._generated.api.workflows import (
    delete_workflow,
    get_workflow,
    list_workflows,
    upload_workflow,
)
from ._generated.models import (
    CreateKeyRequest,
    CreateTenantRequest,
    ErrorBody,
    ExecuteRequest,
    KeyRole,
    PackageUploadForm,
)
from ._generated.types import UNSET, File, Response, Unset


class CloacinaApiError(Exception):
    """Non-2xx response from cloacina-server.

    Attributes:
        status: HTTP status code.
        code: machine-readable error code from the canonical body.
        message: human-readable message.
    """

    def __init__(self, status: int, body: Any) -> None:
        if isinstance(body, ErrorBody):
            self.code = body.code
            self.message = body.error
        else:
            self.code = "unknown"
            self.message = str(body)
        self.status = status
        super().__init__(f"HTTP {status} [{self.code}]: {self.message}")


def _unwrap(response: Response[Any]) -> Any:
    status = int(response.status_code)
    if 200 <= status < 300:
        return response.parsed
    body = response.parsed
    if body is None:
        try:
            body = json.loads(response.content)
        except Exception:  # noqa: BLE001 — raw bytes fallback
            body = response.content
        if isinstance(body, dict) and "error" in body and "code" in body:
            body = ErrorBody(error=body["error"], code=body["code"])
    raise CloacinaApiError(status, body)


def _context(context: dict[str, Any] | None) -> Any | Unset:
    return UNSET if context is None else context


class _Base:
    """Shared construction for the sync/async shims."""

    def __init__(
        self,
        server: str,
        *,
        api_key: str,
        tenant: str | None = None,
        timeout: float = 30.0,
    ) -> None:
        self.server = server.rstrip("/")
        self.tenant = tenant
        self._gen = AuthenticatedClient(
            base_url=self.server,
            token=api_key,
            timeout=__import__("httpx").Timeout(timeout),
        )

    @property
    def generated(self) -> AuthenticatedClient:
        """The underlying generated client, for anything the shim lacks."""
        return self._gen

    def tenant_segment(self, override: str | None = None) -> str:
        """Tenant for tenant-scoped routes — explicit, default, or `public`."""
        return override or self.tenant or "public"


def _raw_json(response: Response[Any]) -> Any:
    """2xx body as JSON for endpoints the spec documents without a schema
    (`/health`)."""
    _unwrap(response)  # raises on non-2xx
    return json.loads(response.content)


class Client(_Base):
    """Synchronous cloacina-server client."""

    # ---- operational ----

    def health(self) -> Any:
        return _raw_json(health.sync_detailed(client=self._gen))

    # ---- keys ----

    def create_key(self, name: str, role: str = "read"):
        body = CreateKeyRequest(name=name, role=KeyRole(role))
        return _unwrap(create_key.sync_detailed(client=self._gen, body=body))

    def list_keys(self):
        return _unwrap(list_keys.sync_detailed(client=self._gen))

    def revoke_key(self, key_id: str):
        return _unwrap(revoke_key.sync_detailed(key_id, client=self._gen))

    def create_tenant_key(self, name: str, role: str = "read", tenant: str | None = None):
        body = CreateKeyRequest(name=name, role=KeyRole(role))
        return _unwrap(
            create_tenant_key.sync_detailed(
                self.tenant_segment(tenant), client=self._gen, body=body
            )
        )

    def create_ws_ticket(self):
        return _unwrap(create_ws_ticket.sync_detailed(client=self._gen))

    # ---- tenants ----

    def create_tenant(
        self,
        name: str,
        *,
        description: str | None = None,
        password: str | None = None,
    ):
        body = CreateTenantRequest(
            name=name,
            description=description if description is not None else UNSET,
            password=password if password is not None else UNSET,
        )
        return _unwrap(create_tenant.sync_detailed(client=self._gen, body=body))

    def list_tenants(self):
        return _unwrap(list_tenants.sync_detailed(client=self._gen))

    def remove_tenant(self, schema_name: str):
        return _unwrap(remove_tenant.sync_detailed(schema_name, client=self._gen))

    # ---- workflows ----

    def upload_workflow(self, package: bytes, tenant: str | None = None):
        import io

        body = PackageUploadForm(
            file=File(payload=io.BytesIO(package), file_name="package.cloacina")
        )
        return _unwrap(
            upload_workflow.sync_detailed(
                self.tenant_segment(tenant), client=self._gen, body=body
            )
        )

    def list_workflows(self, tenant: str | None = None):
        return _unwrap(
            list_workflows.sync_detailed(self.tenant_segment(tenant), client=self._gen)
        )

    def get_workflow(self, name: str, tenant: str | None = None):
        return _unwrap(
            get_workflow.sync_detailed(
                self.tenant_segment(tenant), name, client=self._gen
            )
        )

    def delete_workflow(self, name: str, version: str, tenant: str | None = None):
        return _unwrap(
            delete_workflow.sync_detailed(
                self.tenant_segment(tenant), name, version, client=self._gen
            )
        )

    # ---- triggers ----

    def list_triggers(
        self,
        *,
        limit: int | None = None,
        offset: int | None = None,
        tenant: str | None = None,
    ):
        return _unwrap(
            list_triggers.sync_detailed(
                self.tenant_segment(tenant),
                client=self._gen,
                limit=limit if limit is not None else UNSET,
                offset=offset if offset is not None else UNSET,
            )
        )

    def get_trigger(self, name: str, tenant: str | None = None):
        return _unwrap(
            get_trigger.sync_detailed(
                self.tenant_segment(tenant), name, client=self._gen
            )
        )

    # ---- executions ----

    def execute_workflow(
        self,
        name: str,
        context: dict[str, Any] | None = None,
        tenant: str | None = None,
    ):
        body = ExecuteRequest(context=_context(context))
        return _unwrap(
            execute_workflow.sync_detailed(
                self.tenant_segment(tenant), name, client=self._gen, body=body
            )
        )

    def list_executions(
        self,
        *,
        status: str | None = None,
        workflow: str | None = None,
        limit: int | None = None,
        offset: int | None = None,
        tenant: str | None = None,
    ):
        return _unwrap(
            list_executions.sync_detailed(
                self.tenant_segment(tenant),
                client=self._gen,
                status=status if status is not None else UNSET,
                workflow=workflow if workflow is not None else UNSET,
                limit=limit if limit is not None else UNSET,
                offset=offset if offset is not None else UNSET,
            )
        )

    def iterate_executions(
        self,
        *,
        status: str | None = None,
        workflow: str | None = None,
        page_size: int = 100,
        tenant: str | None = None,
    ) -> Iterator[Any]:
        """Yield executions page by page until a short page arrives."""
        offset = 0
        while True:
            page = self.list_executions(
                status=status,
                workflow=workflow,
                limit=page_size,
                offset=offset,
                tenant=tenant,
            )
            yield from page.items
            if len(page.items) < page_size:
                return
            offset += page_size

    def get_execution(self, exec_id: str, tenant: str | None = None):
        return _unwrap(
            get_execution.sync_detailed(
                self.tenant_segment(tenant), exec_id, client=self._gen
            )
        )

    def get_execution_events(self, exec_id: str, tenant: str | None = None):
        return _unwrap(
            get_execution_events.sync_detailed(
                self.tenant_segment(tenant), exec_id, client=self._gen
            )
        )

    # ---- computation-graph health ----

    def list_accumulators(self):
        return _unwrap(list_accumulators.sync_detailed(client=self._gen))

    def list_graphs(self):
        return _unwrap(list_graphs.sync_detailed(client=self._gen))

    def get_graph(self, name: str):
        return _unwrap(get_graph.sync_detailed(name, client=self._gen))


class AsyncClient(_Base):
    """Asynchronous cloacina-server client (httpx + websockets)."""

    # ---- operational ----

    async def health(self) -> Any:
        return _raw_json(await health.asyncio_detailed(client=self._gen))

    # ---- keys ----

    async def create_key(self, name: str, role: str = "read"):
        body = CreateKeyRequest(name=name, role=KeyRole(role))
        return _unwrap(await create_key.asyncio_detailed(client=self._gen, body=body))

    async def list_keys(self):
        return _unwrap(await list_keys.asyncio_detailed(client=self._gen))

    async def revoke_key(self, key_id: str):
        return _unwrap(await revoke_key.asyncio_detailed(key_id, client=self._gen))

    async def create_ws_ticket(self):
        return _unwrap(await create_ws_ticket.asyncio_detailed(client=self._gen))

    # ---- tenants ----

    async def list_tenants(self):
        return _unwrap(await list_tenants.asyncio_detailed(client=self._gen))

    # ---- executions ----

    async def execute_workflow(
        self,
        name: str,
        context: dict[str, Any] | None = None,
        tenant: str | None = None,
    ):
        body = ExecuteRequest(context=_context(context))
        return _unwrap(
            await execute_workflow.asyncio_detailed(
                self.tenant_segment(tenant), name, client=self._gen, body=body
            )
        )

    async def list_executions(
        self,
        *,
        status: str | None = None,
        workflow: str | None = None,
        limit: int | None = None,
        offset: int | None = None,
        tenant: str | None = None,
    ):
        return _unwrap(
            await list_executions.asyncio_detailed(
                self.tenant_segment(tenant),
                client=self._gen,
                status=status if status is not None else UNSET,
                workflow=workflow if workflow is not None else UNSET,
                limit=limit if limit is not None else UNSET,
                offset=offset if offset is not None else UNSET,
            )
        )

    async def iterate_executions(
        self,
        *,
        status: str | None = None,
        workflow: str | None = None,
        page_size: int = 100,
        tenant: str | None = None,
    ) -> AsyncIterator[Any]:
        """Async pagination — yields executions until a short page arrives."""
        offset = 0
        while True:
            page = await self.list_executions(
                status=status,
                workflow=workflow,
                limit=page_size,
                offset=offset,
                tenant=tenant,
            )
            for item in page.items:
                yield item
            if len(page.items) < page_size:
                return
            offset += page_size

    async def get_execution(self, exec_id: str, tenant: str | None = None):
        return _unwrap(
            await get_execution.asyncio_detailed(
                self.tenant_segment(tenant), exec_id, client=self._gen
            )
        )

    async def get_execution_events(self, exec_id: str, tenant: str | None = None):
        return _unwrap(
            await get_execution_events.asyncio_detailed(
                self.tenant_segment(tenant), exec_id, client=self._gen
            )
        )

    # ---- WebSocket (substrate delivery) ----

    def subscribe_delivery(self, recipient: str, **options: Any):
        """At-least-once delivery stream — see :mod:`cloacina_client._ws`."""
        from . import _ws

        return _ws.subscribe_delivery(self, recipient, **options)

    def follow_execution_events(self, execution_id: str, **options: Any):
        """Stream one execution's JSON events (recipient
        ``exec_events:<execution_id>`` — the same stream
        ``cloacinactl execution follow`` renders)."""
        from . import _ws

        return _ws.follow_execution_events(self, execution_id, **options)
