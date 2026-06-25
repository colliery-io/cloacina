from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.account_action_response import AccountActionResponse
from ...models.error_body import ErrorBody
from ...types import Response


def _get_kwargs(
    tenant_id: str,
    account_id: str,
) -> dict[str, Any]:

    _kwargs: dict[str, Any] = {
        "method": "delete",
        "url": "/v1/tenants/{tenant_id}/accounts/{account_id}".format(
            tenant_id=quote(str(tenant_id), safe=""),
            account_id=quote(str(account_id), safe=""),
        ),
    }

    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> AccountActionResponse | ErrorBody | None:
    if response.status_code == 200:
        response_200 = AccountActionResponse.from_dict(response.json())

        return response_200

    if response.status_code == 400:
        response_400 = ErrorBody.from_dict(response.json())

        return response_400

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
) -> Response[AccountActionResponse | ErrorBody]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    tenant_id: str,
    account_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[AccountActionResponse | ErrorBody]:
    """`DELETE /v1/tenants/{tenant_id}/accounts/{account_id}` — disable an account.
    Disable (not hard-delete) preserves history; already-minted keys lapse at
    their TTL (deprovisioning latency bounded by the short TTL).

    Args:
        tenant_id (str):
        account_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[AccountActionResponse | ErrorBody]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        account_id=account_id,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    tenant_id: str,
    account_id: str,
    *,
    client: AuthenticatedClient,
) -> AccountActionResponse | ErrorBody | None:
    """`DELETE /v1/tenants/{tenant_id}/accounts/{account_id}` — disable an account.
    Disable (not hard-delete) preserves history; already-minted keys lapse at
    their TTL (deprovisioning latency bounded by the short TTL).

    Args:
        tenant_id (str):
        account_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        AccountActionResponse | ErrorBody
    """

    return sync_detailed(
        tenant_id=tenant_id,
        account_id=account_id,
        client=client,
    ).parsed


async def asyncio_detailed(
    tenant_id: str,
    account_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[AccountActionResponse | ErrorBody]:
    """`DELETE /v1/tenants/{tenant_id}/accounts/{account_id}` — disable an account.
    Disable (not hard-delete) preserves history; already-minted keys lapse at
    their TTL (deprovisioning latency bounded by the short TTL).

    Args:
        tenant_id (str):
        account_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[AccountActionResponse | ErrorBody]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        account_id=account_id,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    tenant_id: str,
    account_id: str,
    *,
    client: AuthenticatedClient,
) -> AccountActionResponse | ErrorBody | None:
    """`DELETE /v1/tenants/{tenant_id}/accounts/{account_id}` — disable an account.
    Disable (not hard-delete) preserves history; already-minted keys lapse at
    their TTL (deprovisioning latency bounded by the short TTL).

    Args:
        tenant_id (str):
        account_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        AccountActionResponse | ErrorBody
    """

    return (
        await asyncio_detailed(
            tenant_id=tenant_id,
            account_id=account_id,
            client=client,
        )
    ).parsed
