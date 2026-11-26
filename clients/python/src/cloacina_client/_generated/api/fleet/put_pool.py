from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.agent_pool_info import AgentPoolInfo
from ...models.agent_pool_settings import AgentPoolSettings
from ...models.error_body import ErrorBody
from ...types import Response


def _get_kwargs(
    tenant_id: str,
    pool: str,
    *,
    body: AgentPoolSettings,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "put",
        "url": "/v1/tenants/{tenant_id}/pools/{pool}".format(
            tenant_id=quote(str(tenant_id), safe=""),
            pool=quote(str(pool), safe=""),
        ),
    }

    _kwargs["json"] = body.to_dict()

    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> AgentPoolInfo | ErrorBody | None:
    if response.status_code == 200:
        response_200 = AgentPoolInfo.from_dict(response.json())

        return response_200

    if response.status_code == 400:
        response_400 = ErrorBody.from_dict(response.json())

        return response_400

    if response.status_code == 401:
        response_401 = ErrorBody.from_dict(response.json())

        return response_401

    if response.status_code == 403:
        response_403 = ErrorBody.from_dict(response.json())

        return response_403

    if response.status_code == 500:
        response_500 = ErrorBody.from_dict(response.json())

        return response_500

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> Response[AgentPoolInfo | ErrorBody]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    tenant_id: str,
    pool: str,
    *,
    client: AuthenticatedClient,
    body: AgentPoolSettings,
) -> Response[AgentPoolInfo | ErrorBody]:
    """`PUT /v1/tenants/{tenant_id}/pools/{pool}` — create a pool, or replace its
    settings as a whole. God-only. The actuator applies the settings on its
    next reconcile; a changed setting rolls the pool's agents.

    Args:
        tenant_id (str):
        pool (str):
        body (AgentPoolSettings): Settings of one agent pool of a tenant (CLOACINA-T-1021).

            Each field is optional. A field that is absent means the server-wide
            default (`CLOACINA_AGENT_*` on the server). The settings apply to every
            managed agent of the pool: a pool has one size of agent.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[AgentPoolInfo | ErrorBody]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        pool=pool,
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    tenant_id: str,
    pool: str,
    *,
    client: AuthenticatedClient,
    body: AgentPoolSettings,
) -> AgentPoolInfo | ErrorBody | None:
    """`PUT /v1/tenants/{tenant_id}/pools/{pool}` — create a pool, or replace its
    settings as a whole. God-only. The actuator applies the settings on its
    next reconcile; a changed setting rolls the pool's agents.

    Args:
        tenant_id (str):
        pool (str):
        body (AgentPoolSettings): Settings of one agent pool of a tenant (CLOACINA-T-1021).

            Each field is optional. A field that is absent means the server-wide
            default (`CLOACINA_AGENT_*` on the server). The settings apply to every
            managed agent of the pool: a pool has one size of agent.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        AgentPoolInfo | ErrorBody
    """

    return sync_detailed(
        tenant_id=tenant_id,
        pool=pool,
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    tenant_id: str,
    pool: str,
    *,
    client: AuthenticatedClient,
    body: AgentPoolSettings,
) -> Response[AgentPoolInfo | ErrorBody]:
    """`PUT /v1/tenants/{tenant_id}/pools/{pool}` — create a pool, or replace its
    settings as a whole. God-only. The actuator applies the settings on its
    next reconcile; a changed setting rolls the pool's agents.

    Args:
        tenant_id (str):
        pool (str):
        body (AgentPoolSettings): Settings of one agent pool of a tenant (CLOACINA-T-1021).

            Each field is optional. A field that is absent means the server-wide
            default (`CLOACINA_AGENT_*` on the server). The settings apply to every
            managed agent of the pool: a pool has one size of agent.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[AgentPoolInfo | ErrorBody]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        pool=pool,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    tenant_id: str,
    pool: str,
    *,
    client: AuthenticatedClient,
    body: AgentPoolSettings,
) -> AgentPoolInfo | ErrorBody | None:
    """`PUT /v1/tenants/{tenant_id}/pools/{pool}` — create a pool, or replace its
    settings as a whole. God-only. The actuator applies the settings on its
    next reconcile; a changed setting rolls the pool's agents.

    Args:
        tenant_id (str):
        pool (str):
        body (AgentPoolSettings): Settings of one agent pool of a tenant (CLOACINA-T-1021).

            Each field is optional. A field that is absent means the server-wide
            default (`CLOACINA_AGENT_*` on the server). The settings apply to every
            managed agent of the pool: a pool has one size of agent.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        AgentPoolInfo | ErrorBody
    """

    return (
        await asyncio_detailed(
            tenant_id=tenant_id,
            pool=pool,
            client=client,
            body=body,
        )
    ).parsed
