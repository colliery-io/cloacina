from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_body import ErrorBody
from ...models.execute_response import ExecuteResponse
from ...types import Response


def _get_kwargs(
    tenant_id: str,
    name: str,
) -> dict[str, Any]:

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/v1/tenants/{tenant_id}/syncs/{name}/run".format(
            tenant_id=quote(str(tenant_id), safe=""),
            name=quote(str(name), safe=""),
        ),
    }

    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> ErrorBody | ExecuteResponse | None:
    if response.status_code == 200:
        response_200 = ExecuteResponse.from_dict(response.json())

        return response_200

    if response.status_code == 401:
        response_401 = ErrorBody.from_dict(response.json())

        return response_401

    if response.status_code == 403:
        response_403 = ErrorBody.from_dict(response.json())

        return response_403

    if response.status_code == 404:
        response_404 = ErrorBody.from_dict(response.json())

        return response_404

    if response.status_code == 500:
        response_500 = ErrorBody.from_dict(response.json())

        return response_500

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> Response[ErrorBody | ExecuteResponse]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    tenant_id: str,
    name: str,
    *,
    client: AuthenticatedClient,
) -> Response[ErrorBody | ExecuteResponse]:
    """`POST /v1/tenants/{tenant_id}/syncs/{name}/run` — run a pass of the sync
    now, as its cron fire does (CLOACINA-T-1005). Returns its first segment's
    execution; the pass goes on.

    Args:
        tenant_id (str):
        name (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | ExecuteResponse]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        name=name,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    tenant_id: str,
    name: str,
    *,
    client: AuthenticatedClient,
) -> ErrorBody | ExecuteResponse | None:
    """`POST /v1/tenants/{tenant_id}/syncs/{name}/run` — run a pass of the sync
    now, as its cron fire does (CLOACINA-T-1005). Returns its first segment's
    execution; the pass goes on.

    Args:
        tenant_id (str):
        name (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | ExecuteResponse
    """

    return sync_detailed(
        tenant_id=tenant_id,
        name=name,
        client=client,
    ).parsed


async def asyncio_detailed(
    tenant_id: str,
    name: str,
    *,
    client: AuthenticatedClient,
) -> Response[ErrorBody | ExecuteResponse]:
    """`POST /v1/tenants/{tenant_id}/syncs/{name}/run` — run a pass of the sync
    now, as its cron fire does (CLOACINA-T-1005). Returns its first segment's
    execution; the pass goes on.

    Args:
        tenant_id (str):
        name (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | ExecuteResponse]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        name=name,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    tenant_id: str,
    name: str,
    *,
    client: AuthenticatedClient,
) -> ErrorBody | ExecuteResponse | None:
    """`POST /v1/tenants/{tenant_id}/syncs/{name}/run` — run a pass of the sync
    now, as its cron fire does (CLOACINA-T-1005). Returns its first segment's
    execution; the pass goes on.

    Args:
        tenant_id (str):
        name (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | ExecuteResponse
    """

    return (
        await asyncio_detailed(
            tenant_id=tenant_id,
            name=name,
            client=client,
        )
    ).parsed
