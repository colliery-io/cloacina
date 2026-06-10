from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_body import ErrorBody
from ...models.execute_request import ExecuteRequest
from ...models.execute_response import ExecuteResponse
from ...types import Response


def _get_kwargs(
    tenant_id: str,
    name: str,
    *,
    body: ExecuteRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/v1/tenants/{tenant_id}/workflows/{name}/execute".format(
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
) -> ErrorBody | ExecuteResponse | None:
    if response.status_code == 202:
        response_202 = ExecuteResponse.from_dict(response.json())

        return response_202

    if response.status_code == 400:
        response_400 = ErrorBody.from_dict(response.json())

        return response_400

    if response.status_code == 401:
        response_401 = ErrorBody.from_dict(response.json())

        return response_401

    if response.status_code == 403:
        response_403 = ErrorBody.from_dict(response.json())

        return response_403

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
    body: ExecuteRequest,
) -> Response[ErrorBody | ExecuteResponse]:
    """POST /tenants/:tenant_id/workflows/:name/execute — execute a workflow.

     CLOACI-T-0580: execution is routed through `TenantRunnerCache`, which
    returns (or constructs) a `DefaultRunner` bound to the tenant's
    `Database`. The execution row + every event lands in the tenant's
    schema, never the admin schema.

    Args:
        tenant_id (str):
        name (str):
        body (ExecuteRequest): Request body for `POST
            /tenants/{tenant_id}/workflows/{name}/execute`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | ExecuteResponse]
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
    body: ExecuteRequest,
) -> ErrorBody | ExecuteResponse | None:
    """POST /tenants/:tenant_id/workflows/:name/execute — execute a workflow.

     CLOACI-T-0580: execution is routed through `TenantRunnerCache`, which
    returns (or constructs) a `DefaultRunner` bound to the tenant's
    `Database`. The execution row + every event lands in the tenant's
    schema, never the admin schema.

    Args:
        tenant_id (str):
        name (str):
        body (ExecuteRequest): Request body for `POST
            /tenants/{tenant_id}/workflows/{name}/execute`.

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
        body=body,
    ).parsed


async def asyncio_detailed(
    tenant_id: str,
    name: str,
    *,
    client: AuthenticatedClient,
    body: ExecuteRequest,
) -> Response[ErrorBody | ExecuteResponse]:
    """POST /tenants/:tenant_id/workflows/:name/execute — execute a workflow.

     CLOACI-T-0580: execution is routed through `TenantRunnerCache`, which
    returns (or constructs) a `DefaultRunner` bound to the tenant's
    `Database`. The execution row + every event lands in the tenant's
    schema, never the admin schema.

    Args:
        tenant_id (str):
        name (str):
        body (ExecuteRequest): Request body for `POST
            /tenants/{tenant_id}/workflows/{name}/execute`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | ExecuteResponse]
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
    body: ExecuteRequest,
) -> ErrorBody | ExecuteResponse | None:
    """POST /tenants/:tenant_id/workflows/:name/execute — execute a workflow.

     CLOACI-T-0580: execution is routed through `TenantRunnerCache`, which
    returns (or constructs) a `DefaultRunner` bound to the tenant's
    `Database`. The execution row + every event lands in the tenant's
    schema, never the admin schema.

    Args:
        tenant_id (str):
        name (str):
        body (ExecuteRequest): Request body for `POST
            /tenants/{tenant_id}/workflows/{name}/execute`.

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
            body=body,
        )
    ).parsed
