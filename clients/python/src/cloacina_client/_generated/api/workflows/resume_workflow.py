from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_body import ErrorBody
from ...models.workflow_pause_response import WorkflowPauseResponse
from ...types import Response


def _get_kwargs(
    tenant_id: str,
    name: str,
) -> dict[str, Any]:

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/v1/tenants/{tenant_id}/workflows/{name}/resume".format(
            tenant_id=quote(str(tenant_id), safe=""),
            name=quote(str(name), safe=""),
        ),
    }

    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> ErrorBody | WorkflowPauseResponse | None:
    if response.status_code == 200:
        response_200 = WorkflowPauseResponse.from_dict(response.json())

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
) -> Response[ErrorBody | WorkflowPauseResponse]:
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
) -> Response[ErrorBody | WorkflowPauseResponse]:
    """POST /tenants/:tenant_id/workflows/:name/resume — resume a paused workflow
    (CLOACI-T-0749). New executions are accepted again.

    Args:
        tenant_id (str):
        name (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | WorkflowPauseResponse]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        name=name,
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
) -> ErrorBody | WorkflowPauseResponse | None:
    """POST /tenants/:tenant_id/workflows/:name/resume — resume a paused workflow
    (CLOACI-T-0749). New executions are accepted again.

    Args:
        tenant_id (str):
        name (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | WorkflowPauseResponse
    """

    return sync_detailed(
        tenant_id=tenant_id,
        name=name,
        client=client,
    ).parsed


async def asyncio_detailed(
    tenant_id: str,
    name: str,
    *,
    client: AuthenticatedClient,
) -> Response[ErrorBody | WorkflowPauseResponse]:
    """POST /tenants/:tenant_id/workflows/:name/resume — resume a paused workflow
    (CLOACI-T-0749). New executions are accepted again.

    Args:
        tenant_id (str):
        name (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | WorkflowPauseResponse]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        name=name,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    tenant_id: str,
    name: str,
    *,
    client: AuthenticatedClient,
) -> ErrorBody | WorkflowPauseResponse | None:
    """POST /tenants/:tenant_id/workflows/:name/resume — resume a paused workflow
    (CLOACI-T-0749). New executions are accepted again.

    Args:
        tenant_id (str):
        name (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | WorkflowPauseResponse
    """

    return (
        await asyncio_detailed(
            tenant_id=tenant_id,
            name=name,
            client=client,
        )
    ).parsed
