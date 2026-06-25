from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_body import ErrorBody
from ...models.key_revoked_response import KeyRevokedResponse
from ...types import Response


def _get_kwargs(
    tenant_id: str,
    key_id: str,
) -> dict[str, Any]:

    _kwargs: dict[str, Any] = {
        "method": "delete",
        "url": "/v1/tenants/{tenant_id}/keys/{key_id}".format(
            tenant_id=quote(str(tenant_id), safe=""),
            key_id=quote(str(key_id), safe=""),
        ),
    }

    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> ErrorBody | KeyRevokedResponse | None:
    if response.status_code == 200:
        response_200 = KeyRevokedResponse.from_dict(response.json())

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

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> Response[ErrorBody | KeyRevokedResponse]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    tenant_id: str,
    key_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[ErrorBody | KeyRevokedResponse]:
    """DELETE /tenants/:tenant_id/keys/:key_id — revoke a key owned by one tenant
    (CLOACI-T-0784). The middleware confines the caller to `tenant_id`; this
    handler additionally verifies the target key belongs to that tenant before
    revoking. A cross-tenant or unknown id is reported as not-found (never
    revoked) so a tenant-admin can't probe or touch another tenant's keys.

    Args:
        tenant_id (str):
        key_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | KeyRevokedResponse]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        key_id=key_id,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    tenant_id: str,
    key_id: str,
    *,
    client: AuthenticatedClient,
) -> ErrorBody | KeyRevokedResponse | None:
    """DELETE /tenants/:tenant_id/keys/:key_id — revoke a key owned by one tenant
    (CLOACI-T-0784). The middleware confines the caller to `tenant_id`; this
    handler additionally verifies the target key belongs to that tenant before
    revoking. A cross-tenant or unknown id is reported as not-found (never
    revoked) so a tenant-admin can't probe or touch another tenant's keys.

    Args:
        tenant_id (str):
        key_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | KeyRevokedResponse
    """

    return sync_detailed(
        tenant_id=tenant_id,
        key_id=key_id,
        client=client,
    ).parsed


async def asyncio_detailed(
    tenant_id: str,
    key_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[ErrorBody | KeyRevokedResponse]:
    """DELETE /tenants/:tenant_id/keys/:key_id — revoke a key owned by one tenant
    (CLOACI-T-0784). The middleware confines the caller to `tenant_id`; this
    handler additionally verifies the target key belongs to that tenant before
    revoking. A cross-tenant or unknown id is reported as not-found (never
    revoked) so a tenant-admin can't probe or touch another tenant's keys.

    Args:
        tenant_id (str):
        key_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | KeyRevokedResponse]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        key_id=key_id,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    tenant_id: str,
    key_id: str,
    *,
    client: AuthenticatedClient,
) -> ErrorBody | KeyRevokedResponse | None:
    """DELETE /tenants/:tenant_id/keys/:key_id — revoke a key owned by one tenant
    (CLOACI-T-0784). The middleware confines the caller to `tenant_id`; this
    handler additionally verifies the target key belongs to that tenant before
    revoking. A cross-tenant or unknown id is reported as not-found (never
    revoked) so a tenant-admin can't probe or touch another tenant's keys.

    Args:
        tenant_id (str):
        key_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | KeyRevokedResponse
    """

    return (
        await asyncio_detailed(
            tenant_id=tenant_id,
            key_id=key_id,
            client=client,
        )
    ).parsed
