from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_body import ErrorBody
from ...models.workflow_deleted_response import WorkflowDeletedResponse
from ...types import Response


def _get_kwargs(
    tenant_id: str,
    name: str,
    version: str,
) -> dict[str, Any]:

    _kwargs: dict[str, Any] = {
        "method": "delete",
        "url": "/v1/tenants/{tenant_id}/workflows/{name}/{version}".format(
            tenant_id=quote(str(tenant_id), safe=""),
            name=quote(str(name), safe=""),
            version=quote(str(version), safe=""),
        ),
    }

    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> ErrorBody | WorkflowDeletedResponse | None:
    if response.status_code == 200:
        response_200 = WorkflowDeletedResponse.from_dict(response.json())

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
) -> Response[ErrorBody | WorkflowDeletedResponse]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    tenant_id: str,
    name: str,
    version: str,
    *,
    client: AuthenticatedClient,
) -> Response[ErrorBody | WorkflowDeletedResponse]:
    """DELETE /tenants/:tenant_id/workflows/:name/:version — unregister workflow.

    Args:
        tenant_id (str):
        name (str):
        version (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | WorkflowDeletedResponse]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        name=name,
        version=version,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    tenant_id: str,
    name: str,
    version: str,
    *,
    client: AuthenticatedClient,
) -> ErrorBody | WorkflowDeletedResponse | None:
    """DELETE /tenants/:tenant_id/workflows/:name/:version — unregister workflow.

    Args:
        tenant_id (str):
        name (str):
        version (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | WorkflowDeletedResponse
    """

    return sync_detailed(
        tenant_id=tenant_id,
        name=name,
        version=version,
        client=client,
    ).parsed


async def asyncio_detailed(
    tenant_id: str,
    name: str,
    version: str,
    *,
    client: AuthenticatedClient,
) -> Response[ErrorBody | WorkflowDeletedResponse]:
    """DELETE /tenants/:tenant_id/workflows/:name/:version — unregister workflow.

    Args:
        tenant_id (str):
        name (str):
        version (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | WorkflowDeletedResponse]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        name=name,
        version=version,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    tenant_id: str,
    name: str,
    version: str,
    *,
    client: AuthenticatedClient,
) -> ErrorBody | WorkflowDeletedResponse | None:
    """DELETE /tenants/:tenant_id/workflows/:name/:version — unregister workflow.

    Args:
        tenant_id (str):
        name (str):
        version (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | WorkflowDeletedResponse
    """

    return (
        await asyncio_detailed(
            tenant_id=tenant_id,
            name=name,
            version=version,
            client=client,
        )
    ).parsed
