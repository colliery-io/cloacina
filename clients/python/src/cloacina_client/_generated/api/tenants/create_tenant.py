from http import HTTPStatus
from typing import Any

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.create_tenant_request import CreateTenantRequest
from ...models.error_body import ErrorBody
from ...models.tenant_created_response import TenantCreatedResponse
from ...types import Response


def _get_kwargs(
    *,
    body: CreateTenantRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/v1/tenants",
    }

    _kwargs["json"] = body.to_dict()

    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> ErrorBody | TenantCreatedResponse | None:
    if response.status_code == 201:
        response_201 = TenantCreatedResponse.from_dict(response.json())

        return response_201

    if response.status_code == 400:
        response_400 = ErrorBody.from_dict(response.json())

        return response_400

    if response.status_code == 401:
        response_401 = ErrorBody.from_dict(response.json())

        return response_401

    if response.status_code == 403:
        response_403 = ErrorBody.from_dict(response.json())

        return response_403

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> Response[ErrorBody | TenantCreatedResponse]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    *,
    client: AuthenticatedClient,
    body: CreateTenantRequest,
) -> Response[ErrorBody | TenantCreatedResponse]:
    """POST /tenants — create a new tenant (Postgres schema + user + migrations).
    Admin-only: only is_admin keys can create tenants.

    Args:
        body (CreateTenantRequest): Request body for `POST /tenants`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | TenantCreatedResponse]
    """

    kwargs = _get_kwargs(
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    *,
    client: AuthenticatedClient,
    body: CreateTenantRequest,
) -> ErrorBody | TenantCreatedResponse | None:
    """POST /tenants — create a new tenant (Postgres schema + user + migrations).
    Admin-only: only is_admin keys can create tenants.

    Args:
        body (CreateTenantRequest): Request body for `POST /tenants`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | TenantCreatedResponse
    """

    return sync_detailed(
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    *,
    client: AuthenticatedClient,
    body: CreateTenantRequest,
) -> Response[ErrorBody | TenantCreatedResponse]:
    """POST /tenants — create a new tenant (Postgres schema + user + migrations).
    Admin-only: only is_admin keys can create tenants.

    Args:
        body (CreateTenantRequest): Request body for `POST /tenants`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | TenantCreatedResponse]
    """

    kwargs = _get_kwargs(
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    *,
    client: AuthenticatedClient,
    body: CreateTenantRequest,
) -> ErrorBody | TenantCreatedResponse | None:
    """POST /tenants — create a new tenant (Postgres schema + user + migrations).
    Admin-only: only is_admin keys can create tenants.

    Args:
        body (CreateTenantRequest): Request body for `POST /tenants`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | TenantCreatedResponse
    """

    return (
        await asyncio_detailed(
            client=client,
            body=body,
        )
    ).parsed
