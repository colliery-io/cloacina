from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_body import ErrorBody
from ...models.rotate_secret_request import RotateSecretRequest
from ...models.secret_metadata_response import SecretMetadataResponse
from ...types import Response


def _get_kwargs(
    tenant_id: str,
    name: str,
    *,
    body: RotateSecretRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "put",
        "url": "/v1/tenants/{tenant_id}/secrets/{name}".format(
            tenant_id=quote(str(tenant_id), safe=""),
            name=quote(str(name), safe=""),
        ),
    }

    _kwargs["json"] = body.to_dict()

    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> ErrorBody | SecretMetadataResponse | None:
    if response.status_code == 200:
        response_200 = SecretMetadataResponse.from_dict(response.json())

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
    name: str,
    *,
    client: AuthenticatedClient,
    body: RotateSecretRequest,
) -> Response[ErrorBody | SecretMetadataResponse]:
    """`PUT /v1/tenants/{tenant_id}/secrets/{name}` — rotate a secret's values in
    place (D-8/OQ-5). Returns metadata only; the next fire sees the new value.

    Args:
        tenant_id (str):
        name (str):
        body (RotateSecretRequest): Request body for `PUT|POST
            /v1/tenants/{tenant_id}/secrets/{name}` — rotate.

            Replaces the secret's field map in place (D-8/OQ-5: in-place, no versioning).

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | SecretMetadataResponse]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        name=name,
        body=body,
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
    body: RotateSecretRequest,
) -> ErrorBody | SecretMetadataResponse | None:
    """`PUT /v1/tenants/{tenant_id}/secrets/{name}` — rotate a secret's values in
    place (D-8/OQ-5). Returns metadata only; the next fire sees the new value.

    Args:
        tenant_id (str):
        name (str):
        body (RotateSecretRequest): Request body for `PUT|POST
            /v1/tenants/{tenant_id}/secrets/{name}` — rotate.

            Replaces the secret's field map in place (D-8/OQ-5: in-place, no versioning).

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | SecretMetadataResponse
    """

    return sync_detailed(
        tenant_id=tenant_id,
        name=name,
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    tenant_id: str,
    name: str,
    *,
    client: AuthenticatedClient,
    body: RotateSecretRequest,
) -> Response[ErrorBody | SecretMetadataResponse]:
    """`PUT /v1/tenants/{tenant_id}/secrets/{name}` — rotate a secret's values in
    place (D-8/OQ-5). Returns metadata only; the next fire sees the new value.

    Args:
        tenant_id (str):
        name (str):
        body (RotateSecretRequest): Request body for `PUT|POST
            /v1/tenants/{tenant_id}/secrets/{name}` — rotate.

            Replaces the secret's field map in place (D-8/OQ-5: in-place, no versioning).

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | SecretMetadataResponse]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        name=name,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    tenant_id: str,
    name: str,
    *,
    client: AuthenticatedClient,
    body: RotateSecretRequest,
) -> ErrorBody | SecretMetadataResponse | None:
    """`PUT /v1/tenants/{tenant_id}/secrets/{name}` — rotate a secret's values in
    place (D-8/OQ-5). Returns metadata only; the next fire sees the new value.

    Args:
        tenant_id (str):
        name (str):
        body (RotateSecretRequest): Request body for `PUT|POST
            /v1/tenants/{tenant_id}/secrets/{name}` — rotate.

            Replaces the secret's field map in place (D-8/OQ-5: in-place, no versioning).

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | SecretMetadataResponse
    """

    return (
        await asyncio_detailed(
            tenant_id=tenant_id,
            name=name,
            client=client,
            body=body,
        )
    ).parsed
