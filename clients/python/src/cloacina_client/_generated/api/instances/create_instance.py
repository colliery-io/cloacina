from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.create_instance_request import CreateInstanceRequest
from ...models.error_body import ErrorBody
from ...models.workflow_instance_summary import WorkflowInstanceSummary
from ...types import Response


def _get_kwargs(
    tenant_id: str,
    name: str,
    *,
    body: CreateInstanceRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/v1/tenants/{tenant_id}/workflows/{name}/instances".format(
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

    if response.status_code == 409:
        response_409 = ErrorBody.from_dict(response.json())

        return response_409

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
    *,
    client: AuthenticatedClient,
    body: CreateInstanceRequest,
) -> Response[ErrorBody | WorkflowInstanceSummary]:
    """POST /tenants/:tenant_id/workflows/:name/instances — create a named instance.

     Params are validated against the workflow's declared `params(...)` slots
    using the same `validate_declared_params` the execute route uses, so a
    scheduled instance cannot be created with a binding that would fail at every
    fire — the failure surfaces at creation time instead of silently at 3am.

    `cron` is optional. Without it the instance is created **unscheduled**: a
    durable named param binding with `next_run_at = NULL`, which the scheduler's
    due-query can never select (`NULL <= now` is never true).

    Args:
        tenant_id (str):
        name (str):
        body (CreateInstanceRequest): Body for `POST
            /tenants/{tenant_id}/workflows/{name}/instances`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | WorkflowInstanceSummary]
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
    body: CreateInstanceRequest,
) -> ErrorBody | WorkflowInstanceSummary | None:
    """POST /tenants/:tenant_id/workflows/:name/instances — create a named instance.

     Params are validated against the workflow's declared `params(...)` slots
    using the same `validate_declared_params` the execute route uses, so a
    scheduled instance cannot be created with a binding that would fail at every
    fire — the failure surfaces at creation time instead of silently at 3am.

    `cron` is optional. Without it the instance is created **unscheduled**: a
    durable named param binding with `next_run_at = NULL`, which the scheduler's
    due-query can never select (`NULL <= now` is never true).

    Args:
        tenant_id (str):
        name (str):
        body (CreateInstanceRequest): Body for `POST
            /tenants/{tenant_id}/workflows/{name}/instances`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | WorkflowInstanceSummary
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
    body: CreateInstanceRequest,
) -> Response[ErrorBody | WorkflowInstanceSummary]:
    """POST /tenants/:tenant_id/workflows/:name/instances — create a named instance.

     Params are validated against the workflow's declared `params(...)` slots
    using the same `validate_declared_params` the execute route uses, so a
    scheduled instance cannot be created with a binding that would fail at every
    fire — the failure surfaces at creation time instead of silently at 3am.

    `cron` is optional. Without it the instance is created **unscheduled**: a
    durable named param binding with `next_run_at = NULL`, which the scheduler's
    due-query can never select (`NULL <= now` is never true).

    Args:
        tenant_id (str):
        name (str):
        body (CreateInstanceRequest): Body for `POST
            /tenants/{tenant_id}/workflows/{name}/instances`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | WorkflowInstanceSummary]
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
    body: CreateInstanceRequest,
) -> ErrorBody | WorkflowInstanceSummary | None:
    """POST /tenants/:tenant_id/workflows/:name/instances — create a named instance.

     Params are validated against the workflow's declared `params(...)` slots
    using the same `validate_declared_params` the execute route uses, so a
    scheduled instance cannot be created with a binding that would fail at every
    fire — the failure surfaces at creation time instead of silently at 3am.

    `cron` is optional. Without it the instance is created **unscheduled**: a
    durable named param binding with `next_run_at = NULL`, which the scheduler's
    due-query can never select (`NULL <= now` is never true).

    Args:
        tenant_id (str):
        name (str):
        body (CreateInstanceRequest): Body for `POST
            /tenants/{tenant_id}/workflows/{name}/instances`.

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
            client=client,
            body=body,
        )
    ).parsed
