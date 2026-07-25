from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.create_secret_request import CreateSecretRequest
from ...models.error_body import ErrorBody
from ...models.secret_metadata_response import SecretMetadataResponse
from ...types import Response


def _get_kwargs(
    tenant_id: str,
    *,
    body: CreateSecretRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/v1/tenants/{tenant_id}/secrets".format(
            tenant_id=quote(str(tenant_id), safe=""),
        ),
    }

    _kwargs["json"] = body.to_dict()

    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> ErrorBody | SecretMetadataResponse | None:
    if response.status_code == 201:
        response_201 = SecretMetadataResponse.from_dict(response.json())

        return response_201

    if response.status_code == 401:
        response_401 = ErrorBody.from_dict(response.json())

        return response_401

    if response.status_code == 403:
        response_403 = ErrorBody.from_dict(response.json())

        return response_403

    if response.status_code == 409:
        response_409 = ErrorBody.from_dict(response.json())

        return response_409

    if response.status_code == 503:
        response_503 = ErrorBody.from_dict(response.json())

        return response_503

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> Response[ErrorBody | SecretMetadataResponse]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    tenant_id: str,
    *,
    client: AuthenticatedClient,
    body: CreateSecretRequest,
) -> Response[ErrorBody | SecretMetadataResponse]:
    """`POST /v1/tenants/{tenant_id}/secrets` — create a secret from a field map.
    Returns metadata only.

    Args:
        tenant_id (str):
        body (CreateSecretRequest): Request body for `POST /v1/tenants/{tenant_id}/secrets` —
            create a secret.

            `fields` is the `{field_name: value}` map. The values are write-only: they
            are encrypted at rest and never returned by any read endpoint.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | SecretMetadataResponse]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    tenant_id: str,
    *,
    client: AuthenticatedClient,
    body: CreateSecretRequest,
) -> ErrorBody | SecretMetadataResponse | None:
    """`POST /v1/tenants/{tenant_id}/secrets` — create a secret from a field map.
    Returns metadata only.

    Args:
        tenant_id (str):
        body (CreateSecretRequest): Request body for `POST /v1/tenants/{tenant_id}/secrets` —
            create a secret.

            `fields` is the `{field_name: value}` map. The values are write-only: they
            are encrypted at rest and never returned by any read endpoint.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | SecretMetadataResponse
    """

    return sync_detailed(
        tenant_id=tenant_id,
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    tenant_id: str,
    *,
    client: AuthenticatedClient,
    body: CreateSecretRequest,
) -> Response[ErrorBody | SecretMetadataResponse]:
    """`POST /v1/tenants/{tenant_id}/secrets` — create a secret from a field map.
    Returns metadata only.

    Args:
        tenant_id (str):
        body (CreateSecretRequest): Request body for `POST /v1/tenants/{tenant_id}/secrets` —
            create a secret.

            `fields` is the `{field_name: value}` map. The values are write-only: they
            are encrypted at rest and never returned by any read endpoint.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | SecretMetadataResponse]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    tenant_id: str,
    *,
    client: AuthenticatedClient,
    body: CreateSecretRequest,
) -> ErrorBody | SecretMetadataResponse | None:
    """`POST /v1/tenants/{tenant_id}/secrets` — create a secret from a field map.
    Returns metadata only.

    Args:
        tenant_id (str):
        body (CreateSecretRequest): Request body for `POST /v1/tenants/{tenant_id}/secrets` —
            create a secret.

            `fields` is the `{field_name: value}` map. The values are write-only: they
            are encrypted at rest and never returned by any read endpoint.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | SecretMetadataResponse
    """

    return (
        await asyncio_detailed(
            tenant_id=tenant_id,
            client=client,
            body=body,
        )
    ).parsed
