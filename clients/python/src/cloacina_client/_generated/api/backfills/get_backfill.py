from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.backfill_detail import BackfillDetail
from ...models.error_body import ErrorBody
from ...types import Response


def _get_kwargs(
    tenant_id: str,
    name: str,
    id: str,
) -> dict[str, Any]:

    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": "/v1/tenants/{tenant_id}/workflows/{name}/backfills/{id}".format(
            tenant_id=quote(str(tenant_id), safe=""),
            name=quote(str(name), safe=""),
            id=quote(str(id), safe=""),
        ),
    }

    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> BackfillDetail | ErrorBody | None:
    if response.status_code == 200:
        response_200 = BackfillDetail.from_dict(response.json())

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
) -> Response[BackfillDetail | ErrorBody]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    tenant_id: str,
    name: str,
    id: str,
    *,
    client: AuthenticatedClient,
) -> Response[BackfillDetail | ErrorBody]:
    """GET /tenants/:tenant_id/workflows/:name/backfills/:id — one backfill,
    with its progress and its failed intervals.

    Args:
        tenant_id (str):
        name (str):
        id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[BackfillDetail | ErrorBody]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        name=name,
        id=id,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    tenant_id: str,
    name: str,
    id: str,
    *,
    client: AuthenticatedClient,
) -> BackfillDetail | ErrorBody | None:
    """GET /tenants/:tenant_id/workflows/:name/backfills/:id — one backfill,
    with its progress and its failed intervals.

    Args:
        tenant_id (str):
        name (str):
        id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        BackfillDetail | ErrorBody
    """

    return sync_detailed(
        tenant_id=tenant_id,
        name=name,
        id=id,
        client=client,
    ).parsed


async def asyncio_detailed(
    tenant_id: str,
    name: str,
    id: str,
    *,
    client: AuthenticatedClient,
) -> Response[BackfillDetail | ErrorBody]:
    """GET /tenants/:tenant_id/workflows/:name/backfills/:id — one backfill,
    with its progress and its failed intervals.

    Args:
        tenant_id (str):
        name (str):
        id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[BackfillDetail | ErrorBody]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        name=name,
        id=id,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    tenant_id: str,
    name: str,
    id: str,
    *,
    client: AuthenticatedClient,
) -> BackfillDetail | ErrorBody | None:
    """GET /tenants/:tenant_id/workflows/:name/backfills/:id — one backfill,
    with its progress and its failed intervals.

    Args:
        tenant_id (str):
        name (str):
        id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        BackfillDetail | ErrorBody
    """

    return (
        await asyncio_detailed(
            tenant_id=tenant_id,
            name=name,
            id=id,
            client=client,
        )
    ).parsed
