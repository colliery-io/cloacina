from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.connector_probe_request import ConnectorProbeRequest
from ...models.connector_spec_response import ConnectorSpecResponse
from ...models.error_body import ErrorBody
from ...types import Response


def _get_kwargs(
    tenant_id: str,
    *,
    body: ConnectorProbeRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/v1/tenants/{tenant_id}/connectors/spec".format(
            tenant_id=quote(str(tenant_id), safe=""),
        ),
    }

    _kwargs["json"] = body.to_dict()

    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> ConnectorSpecResponse | ErrorBody | None:
    if response.status_code == 200:
        response_200 = ConnectorSpecResponse.from_dict(response.json())

        return response_200

    if response.status_code == 400:
        response_400 = ErrorBody.from_dict(response.json())

        return response_400

    if response.status_code == 422:
        response_422 = ErrorBody.from_dict(response.json())

        return response_422

    if response.status_code == 504:
        response_504 = ErrorBody.from_dict(response.json())

        return response_504

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> Response[ConnectorSpecResponse | ErrorBody]:
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
    body: ConnectorProbeRequest,
) -> Response[ConnectorSpecResponse | ErrorBody]:
    """`POST /v1/tenants/{tenant_id}/connectors/spec` — what a compiled connector
    says about itself: its config schema, roles and sync modes (CLOACINA-T-1006).
    It runs where connectors run, as check and discover do.

    Args:
        tenant_id (str):
        body (ConnectorProbeRequest): Body for `POST /tenants/{tenant_id}/connectors/check` and
            `POST /tenants/{tenant_id}/connectors/discover` (CLOACINA-T-1002): one side
            of a sync, as a sync definition gives it. A side names a compiled
            connector by `provider` and `member`, or a connector definition by
            `definition`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ConnectorSpecResponse | ErrorBody]
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
    body: ConnectorProbeRequest,
) -> ConnectorSpecResponse | ErrorBody | None:
    """`POST /v1/tenants/{tenant_id}/connectors/spec` — what a compiled connector
    says about itself: its config schema, roles and sync modes (CLOACINA-T-1006).
    It runs where connectors run, as check and discover do.

    Args:
        tenant_id (str):
        body (ConnectorProbeRequest): Body for `POST /tenants/{tenant_id}/connectors/check` and
            `POST /tenants/{tenant_id}/connectors/discover` (CLOACINA-T-1002): one side
            of a sync, as a sync definition gives it. A side names a compiled
            connector by `provider` and `member`, or a connector definition by
            `definition`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ConnectorSpecResponse | ErrorBody
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
    body: ConnectorProbeRequest,
) -> Response[ConnectorSpecResponse | ErrorBody]:
    """`POST /v1/tenants/{tenant_id}/connectors/spec` — what a compiled connector
    says about itself: its config schema, roles and sync modes (CLOACINA-T-1006).
    It runs where connectors run, as check and discover do.

    Args:
        tenant_id (str):
        body (ConnectorProbeRequest): Body for `POST /tenants/{tenant_id}/connectors/check` and
            `POST /tenants/{tenant_id}/connectors/discover` (CLOACINA-T-1002): one side
            of a sync, as a sync definition gives it. A side names a compiled
            connector by `provider` and `member`, or a connector definition by
            `definition`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ConnectorSpecResponse | ErrorBody]
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
    body: ConnectorProbeRequest,
) -> ConnectorSpecResponse | ErrorBody | None:
    """`POST /v1/tenants/{tenant_id}/connectors/spec` — what a compiled connector
    says about itself: its config schema, roles and sync modes (CLOACINA-T-1006).
    It runs where connectors run, as check and discover do.

    Args:
        tenant_id (str):
        body (ConnectorProbeRequest): Body for `POST /tenants/{tenant_id}/connectors/check` and
            `POST /tenants/{tenant_id}/connectors/discover` (CLOACINA-T-1002): one side
            of a sync, as a sync definition gives it. A side names a compiled
            connector by `provider` and `member`, or a connector definition by
            `definition`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ConnectorSpecResponse | ErrorBody
    """

    return (
        await asyncio_detailed(
            tenant_id=tenant_id,
            client=client,
            body=body,
        )
    ).parsed
