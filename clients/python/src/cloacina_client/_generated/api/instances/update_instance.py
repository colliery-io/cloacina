from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_body import ErrorBody
from ...models.update_instance_request import UpdateInstanceRequest
from ...models.workflow_instance_summary import WorkflowInstanceSummary
from ...types import Response


def _get_kwargs(
    tenant_id: str,
    name: str,
    instance: str,
    *,
    body: UpdateInstanceRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "put",
        "url": "/v1/tenants/{tenant_id}/workflows/{name}/instances/{instance}".format(
            tenant_id=quote(str(tenant_id), safe=""),
            name=quote(str(name), safe=""),
            instance=quote(str(instance), safe=""),
        ),
    }

    _kwargs["json"] = body.to_dict()

    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> ErrorBody | WorkflowInstanceSummary | None:
    if response.status_code == 200:
        response_200 = WorkflowInstanceSummary.from_dict(response.json())

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
) -> Response[ErrorBody | WorkflowInstanceSummary]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    tenant_id: str,
    name: str,
    instance: str,
    *,
    client: AuthenticatedClient,
    body: UpdateInstanceRequest,
) -> Response[ErrorBody | WorkflowInstanceSummary]:
    """PUT /tenants/:tenant_id/workflows/:name/instances/:instance — replace an
    instance's params, cron and timezone (CLOACINA-T-1001).

     The params are validated as on create. A sync definition's params are
    checked again, and its `sync` name may not change: its committed cursor
    and dead-letters are kept under that name, so an update keeps them. A new
    cron re-arms the scheduler; no cron makes the instance unscheduled.

    Args:
        tenant_id (str):
        name (str):
        instance (str):
        body (UpdateInstanceRequest): Body for `PUT
            /tenants/{tenant_id}/workflows/{name}/instances/{instance}`
            (CLOACINA-T-1001): the instance's new params, cron and timezone, which
            replace the old ones.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | WorkflowInstanceSummary]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        name=name,
        instance=instance,
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    tenant_id: str,
    name: str,
    instance: str,
    *,
    client: AuthenticatedClient,
    body: UpdateInstanceRequest,
) -> ErrorBody | WorkflowInstanceSummary | None:
    """PUT /tenants/:tenant_id/workflows/:name/instances/:instance — replace an
    instance's params, cron and timezone (CLOACINA-T-1001).

     The params are validated as on create. A sync definition's params are
    checked again, and its `sync` name may not change: its committed cursor
    and dead-letters are kept under that name, so an update keeps them. A new
    cron re-arms the scheduler; no cron makes the instance unscheduled.

    Args:
        tenant_id (str):
        name (str):
        instance (str):
        body (UpdateInstanceRequest): Body for `PUT
            /tenants/{tenant_id}/workflows/{name}/instances/{instance}`
            (CLOACINA-T-1001): the instance's new params, cron and timezone, which
            replace the old ones.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | WorkflowInstanceSummary
    """

    return sync_detailed(
        tenant_id=tenant_id,
        name=name,
        instance=instance,
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    tenant_id: str,
    name: str,
    instance: str,
    *,
    client: AuthenticatedClient,
    body: UpdateInstanceRequest,
) -> Response[ErrorBody | WorkflowInstanceSummary]:
    """PUT /tenants/:tenant_id/workflows/:name/instances/:instance — replace an
    instance's params, cron and timezone (CLOACINA-T-1001).

     The params are validated as on create. A sync definition's params are
    checked again, and its `sync` name may not change: its committed cursor
    and dead-letters are kept under that name, so an update keeps them. A new
    cron re-arms the scheduler; no cron makes the instance unscheduled.

    Args:
        tenant_id (str):
        name (str):
        instance (str):
        body (UpdateInstanceRequest): Body for `PUT
            /tenants/{tenant_id}/workflows/{name}/instances/{instance}`
            (CLOACINA-T-1001): the instance's new params, cron and timezone, which
            replace the old ones.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | WorkflowInstanceSummary]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        name=name,
        instance=instance,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    tenant_id: str,
    name: str,
    instance: str,
    *,
    client: AuthenticatedClient,
    body: UpdateInstanceRequest,
) -> ErrorBody | WorkflowInstanceSummary | None:
    """PUT /tenants/:tenant_id/workflows/:name/instances/:instance — replace an
    instance's params, cron and timezone (CLOACINA-T-1001).

     The params are validated as on create. A sync definition's params are
    checked again, and its `sync` name may not change: its committed cursor
    and dead-letters are kept under that name, so an update keeps them. A new
    cron re-arms the scheduler; no cron makes the instance unscheduled.

    Args:
        tenant_id (str):
        name (str):
        instance (str):
        body (UpdateInstanceRequest): Body for `PUT
            /tenants/{tenant_id}/workflows/{name}/instances/{instance}`
            (CLOACINA-T-1001): the instance's new params, cron and timezone, which
            replace the old ones.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | WorkflowInstanceSummary
    """

    return (
        await asyncio_detailed(
            tenant_id=tenant_id,
            name=name,
            instance=instance,
            client=client,
            body=body,
        )
    ).parsed
