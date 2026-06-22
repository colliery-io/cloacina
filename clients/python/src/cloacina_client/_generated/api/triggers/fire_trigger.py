from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_body import ErrorBody
from ...models.fire_trigger_request import FireTriggerRequest
from ...models.fire_trigger_response import FireTriggerResponse
from ...types import Response


def _get_kwargs(
    tenant_id: str,
    name: str,
    *,
    body: FireTriggerRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/v1/tenants/{tenant_id}/triggers/{name}/fire".format(
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
) -> ErrorBody | FireTriggerResponse | None:
    if response.status_code == 200:
        response_200 = FireTriggerResponse.from_dict(response.json())

        return response_200

    if response.status_code == 404:
        response_404 = ErrorBody.from_dict(response.json())

        return response_404

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> Response[ErrorBody | FireTriggerResponse]:
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
    body: FireTriggerRequest,
) -> Response[ErrorBody | FireTriggerResponse]:
    """POST /tenants/:tenant_id/triggers/:name/fire — manually fire a trigger,
    fanning out to every subscribed workflow (CLOACI-T-0777). One operator action
    instead of running each workflow by hand. An optional `event` is merged into
    each fired workflow's context (alongside trigger metadata). The started
    executions are marked `manual` (CLOACI-T-0776).

    Args:
        tenant_id (str):
        name (str):
        body (FireTriggerRequest): `POST /tenants/{tenant_id}/triggers/{name}/fire` request
            (CLOACI-T-0777).
            Manually push an event to a trigger; it fans out to every subscribed workflow.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | FireTriggerResponse]
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
    body: FireTriggerRequest,
) -> ErrorBody | FireTriggerResponse | None:
    """POST /tenants/:tenant_id/triggers/:name/fire — manually fire a trigger,
    fanning out to every subscribed workflow (CLOACI-T-0777). One operator action
    instead of running each workflow by hand. An optional `event` is merged into
    each fired workflow's context (alongside trigger metadata). The started
    executions are marked `manual` (CLOACI-T-0776).

    Args:
        tenant_id (str):
        name (str):
        body (FireTriggerRequest): `POST /tenants/{tenant_id}/triggers/{name}/fire` request
            (CLOACI-T-0777).
            Manually push an event to a trigger; it fans out to every subscribed workflow.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | FireTriggerResponse
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
    body: FireTriggerRequest,
) -> Response[ErrorBody | FireTriggerResponse]:
    """POST /tenants/:tenant_id/triggers/:name/fire — manually fire a trigger,
    fanning out to every subscribed workflow (CLOACI-T-0777). One operator action
    instead of running each workflow by hand. An optional `event` is merged into
    each fired workflow's context (alongside trigger metadata). The started
    executions are marked `manual` (CLOACI-T-0776).

    Args:
        tenant_id (str):
        name (str):
        body (FireTriggerRequest): `POST /tenants/{tenant_id}/triggers/{name}/fire` request
            (CLOACI-T-0777).
            Manually push an event to a trigger; it fans out to every subscribed workflow.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | FireTriggerResponse]
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
    body: FireTriggerRequest,
) -> ErrorBody | FireTriggerResponse | None:
    """POST /tenants/:tenant_id/triggers/:name/fire — manually fire a trigger,
    fanning out to every subscribed workflow (CLOACI-T-0777). One operator action
    instead of running each workflow by hand. An optional `event` is merged into
    each fired workflow's context (alongside trigger metadata). The started
    executions are marked `manual` (CLOACI-T-0776).

    Args:
        tenant_id (str):
        name (str):
        body (FireTriggerRequest): `POST /tenants/{tenant_id}/triggers/{name}/fire` request
            (CLOACI-T-0777).
            Manually push an event to a trigger; it fans out to every subscribed workflow.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | FireTriggerResponse
    """

    return (
        await asyncio_detailed(
            tenant_id=tenant_id,
            name=name,
            client=client,
            body=body,
        )
    ).parsed
