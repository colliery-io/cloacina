from http import HTTPStatus
from typing import Any, cast
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_body import ErrorBody
from ...types import Response


def _get_kwargs(
    tenant_id: str,
    pool: str,
) -> dict[str, Any]:

    _kwargs: dict[str, Any] = {
        "method": "delete",
        "url": "/v1/tenants/{tenant_id}/pools/{pool}".format(
            tenant_id=quote(str(tenant_id), safe=""),
            pool=quote(str(pool), safe=""),
        ),
    }

    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> Any | ErrorBody | None:
    if response.status_code == 204:
        response_204 = cast(Any, None)
        return response_204

    if response.status_code == 401:
        response_401 = ErrorBody.from_dict(response.json())

        return response_401

    if response.status_code == 403:
        response_403 = ErrorBody.from_dict(response.json())

        return response_403

    if response.status_code == 404:
        response_404 = ErrorBody.from_dict(response.json())

        return response_404

    if response.status_code == 409:
        response_409 = ErrorBody.from_dict(response.json())

        return response_409

    if response.status_code == 500:
        response_500 = ErrorBody.from_dict(response.json())

        return response_500

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> Response[Any | ErrorBody]:
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
) -> Response[Any | ErrorBody]:
    """`DELETE /v1/tenants/{tenant_id}/pools/{pool}` — remove a pool. God-only.

     A pool is removed only when it has no agents: its `max_agents` is 0 and no
    agent of it is alive (`409` otherwise). Then its Deployment and its
    NetworkPolicy are deleted and its row is removed. For `default` the call
    clears the pool's settings (back to the server-wide defaults); the pool
    itself stays.

    Args:
        tenant_id (str):
        pool (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any | ErrorBody]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        pool=pool,
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
) -> Any | ErrorBody | None:
    """`DELETE /v1/tenants/{tenant_id}/pools/{pool}` — remove a pool. God-only.

     A pool is removed only when it has no agents: its `max_agents` is 0 and no
    agent of it is alive (`409` otherwise). Then its Deployment and its
    NetworkPolicy are deleted and its row is removed. For `default` the call
    clears the pool's settings (back to the server-wide defaults); the pool
    itself stays.

    Args:
        tenant_id (str):
        pool (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Any | ErrorBody
    """

    return sync_detailed(
        tenant_id=tenant_id,
        pool=pool,
        client=client,
    ).parsed


async def asyncio_detailed(
    tenant_id: str,
    pool: str,
    *,
    client: AuthenticatedClient,
) -> Response[Any | ErrorBody]:
    """`DELETE /v1/tenants/{tenant_id}/pools/{pool}` — remove a pool. God-only.

     A pool is removed only when it has no agents: its `max_agents` is 0 and no
    agent of it is alive (`409` otherwise). Then its Deployment and its
    NetworkPolicy are deleted and its row is removed. For `default` the call
    clears the pool's settings (back to the server-wide defaults); the pool
    itself stays.

    Args:
        tenant_id (str):
        pool (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any | ErrorBody]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        pool=pool,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    tenant_id: str,
    pool: str,
    *,
    client: AuthenticatedClient,
) -> Any | ErrorBody | None:
    """`DELETE /v1/tenants/{tenant_id}/pools/{pool}` — remove a pool. God-only.

     A pool is removed only when it has no agents: its `max_agents` is 0 and no
    agent of it is alive (`409` otherwise). Then its Deployment and its
    NetworkPolicy are deleted and its row is removed. For `default` the call
    clears the pool's settings (back to the server-wide defaults); the pool
    itself stays.

    Args:
        tenant_id (str):
        pool (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Any | ErrorBody
    """

    return (
        await asyncio_detailed(
            tenant_id=tenant_id,
            pool=pool,
            client=client,
        )
    ).parsed
