from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_body import ErrorBody
from ...models.tenant_list_response_sync_dead_letter import (
    TenantListResponseSyncDeadLetter,
)
from ...types import UNSET, Response, Unset


def _get_kwargs(
    tenant_id: str,
    name: str,
    *,
    limit: int | Unset = UNSET,
    offset: int | Unset = UNSET,
) -> dict[str, Any]:

    params: dict[str, Any] = {}

    params["limit"] = limit

    params["offset"] = offset

    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}

    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": "/v1/tenants/{tenant_id}/syncs/{name}/dead-letters".format(
            tenant_id=quote(str(tenant_id), safe=""),
            name=quote(str(name), safe=""),
        ),
        "params": params,
    }

    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> ErrorBody | TenantListResponseSyncDeadLetter | None:
    if response.status_code == 200:
        response_200 = TenantListResponseSyncDeadLetter.from_dict(response.json())

        return response_200

    if response.status_code == 400:
        response_400 = ErrorBody.from_dict(response.json())

        return response_400

    if response.status_code == 404:
        response_404 = ErrorBody.from_dict(response.json())

        return response_404

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> Response[ErrorBody | TenantListResponseSyncDeadLetter]:
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
    limit: int | Unset = UNSET,
    offset: int | Unset = UNSET,
) -> Response[ErrorBody | TenantListResponseSyncDeadLetter]:
    """`GET /v1/tenants/{tenant_id}/syncs/{name}/dead-letters` — a page of the
    records the sync set aside, newest first; `total` is how many it holds.

    Args:
        tenant_id (str):
        name (str):
        limit (int | Unset):
        offset (int | Unset):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | TenantListResponseSyncDeadLetter]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        name=name,
        limit=limit,
        offset=offset,
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
    limit: int | Unset = UNSET,
    offset: int | Unset = UNSET,
) -> ErrorBody | TenantListResponseSyncDeadLetter | None:
    """`GET /v1/tenants/{tenant_id}/syncs/{name}/dead-letters` — a page of the
    records the sync set aside, newest first; `total` is how many it holds.

    Args:
        tenant_id (str):
        name (str):
        limit (int | Unset):
        offset (int | Unset):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | TenantListResponseSyncDeadLetter
    """

    return sync_detailed(
        tenant_id=tenant_id,
        name=name,
        client=client,
        limit=limit,
        offset=offset,
    ).parsed


async def asyncio_detailed(
    tenant_id: str,
    name: str,
    *,
    client: AuthenticatedClient,
    limit: int | Unset = UNSET,
    offset: int | Unset = UNSET,
) -> Response[ErrorBody | TenantListResponseSyncDeadLetter]:
    """`GET /v1/tenants/{tenant_id}/syncs/{name}/dead-letters` — a page of the
    records the sync set aside, newest first; `total` is how many it holds.

    Args:
        tenant_id (str):
        name (str):
        limit (int | Unset):
        offset (int | Unset):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | TenantListResponseSyncDeadLetter]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        name=name,
        limit=limit,
        offset=offset,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    tenant_id: str,
    name: str,
    *,
    client: AuthenticatedClient,
    limit: int | Unset = UNSET,
    offset: int | Unset = UNSET,
) -> ErrorBody | TenantListResponseSyncDeadLetter | None:
    """`GET /v1/tenants/{tenant_id}/syncs/{name}/dead-letters` — a page of the
    records the sync set aside, newest first; `total` is how many it holds.

    Args:
        tenant_id (str):
        name (str):
        limit (int | Unset):
        offset (int | Unset):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | TenantListResponseSyncDeadLetter
    """

    return (
        await asyncio_detailed(
            tenant_id=tenant_id,
            name=name,
            client=client,
            limit=limit,
            offset=offset,
        )
    ).parsed
