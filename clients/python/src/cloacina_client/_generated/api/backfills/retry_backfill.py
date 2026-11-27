from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.backfill_summary import BackfillSummary
from ...models.error_body import ErrorBody
from ...models.retry_backfill_request import RetryBackfillRequest
from ...types import Response


def _get_kwargs(
    tenant_id: str,
    name: str,
    id: str,
    *,
    body: RetryBackfillRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/v1/tenants/{tenant_id}/workflows/{name}/backfills/{id}/retry".format(
            tenant_id=quote(str(tenant_id), safe=""),
            name=quote(str(name), safe=""),
            id=quote(str(id), safe=""),
        ),
    }

    _kwargs["json"] = body.to_dict()

    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> BackfillSummary | ErrorBody | None:
    if response.status_code == 200:
        response_200 = BackfillSummary.from_dict(response.json())

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
) -> Response[BackfillSummary | ErrorBody]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    tenant_id: str,
    name: str,
    id: str,
    *,
    client: AuthenticatedClient,
    body: RetryBackfillRequest,
) -> Response[BackfillSummary | ErrorBody]:
    """POST /tenants/:tenant_id/workflows/:name/backfills/:id/retry — run the
    failed intervals of a backfill again.

     Creates a new backfill of the intervals that failed in backfill `id`, as
    they are at the time of the call, with the same cron expression, timezone
    and params. The backfill `id` is not changed.

    Args:
        tenant_id (str):
        name (str):
        id (str):
        body (RetryBackfillRequest): Body for `POST
            /tenants/{tenant_id}/workflows/{name}/backfills/{id}/retry`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[BackfillSummary | ErrorBody]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        name=name,
        id=id,
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    tenant_id: str,
    name: str,
    id: str,
    *,
    client: AuthenticatedClient,
    body: RetryBackfillRequest,
) -> BackfillSummary | ErrorBody | None:
    """POST /tenants/:tenant_id/workflows/:name/backfills/:id/retry — run the
    failed intervals of a backfill again.

     Creates a new backfill of the intervals that failed in backfill `id`, as
    they are at the time of the call, with the same cron expression, timezone
    and params. The backfill `id` is not changed.

    Args:
        tenant_id (str):
        name (str):
        id (str):
        body (RetryBackfillRequest): Body for `POST
            /tenants/{tenant_id}/workflows/{name}/backfills/{id}/retry`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        BackfillSummary | ErrorBody
    """

    return sync_detailed(
        tenant_id=tenant_id,
        name=name,
        id=id,
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    tenant_id: str,
    name: str,
    id: str,
    *,
    client: AuthenticatedClient,
    body: RetryBackfillRequest,
) -> Response[BackfillSummary | ErrorBody]:
    """POST /tenants/:tenant_id/workflows/:name/backfills/:id/retry — run the
    failed intervals of a backfill again.

     Creates a new backfill of the intervals that failed in backfill `id`, as
    they are at the time of the call, with the same cron expression, timezone
    and params. The backfill `id` is not changed.

    Args:
        tenant_id (str):
        name (str):
        id (str):
        body (RetryBackfillRequest): Body for `POST
            /tenants/{tenant_id}/workflows/{name}/backfills/{id}/retry`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[BackfillSummary | ErrorBody]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        name=name,
        id=id,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    tenant_id: str,
    name: str,
    id: str,
    *,
    client: AuthenticatedClient,
    body: RetryBackfillRequest,
) -> BackfillSummary | ErrorBody | None:
    """POST /tenants/:tenant_id/workflows/:name/backfills/:id/retry — run the
    failed intervals of a backfill again.

     Creates a new backfill of the intervals that failed in backfill `id`, as
    they are at the time of the call, with the same cron expression, timezone
    and params. The backfill `id` is not changed.

    Args:
        tenant_id (str):
        name (str):
        id (str):
        body (RetryBackfillRequest): Body for `POST
            /tenants/{tenant_id}/workflows/{name}/backfills/{id}/retry`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        BackfillSummary | ErrorBody
    """

    return (
        await asyncio_detailed(
            tenant_id=tenant_id,
            name=name,
            id=id,
            client=client,
            body=body,
        )
    ).parsed
