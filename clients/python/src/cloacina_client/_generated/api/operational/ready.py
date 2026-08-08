from http import HTTPStatus
from typing import Any

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...types import Response


def _get_kwargs() -> dict[str, Any]:

    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": "/ready",
    }

    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> Any | None:
    if response.status_code == 200:
        return None

    if response.status_code == 503:
        return None

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> Response[Any]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    *,
    client: AuthenticatedClient | Client,
) -> Response[Any]:
    """GET /ready — readiness check, scoped to PLATFORM health only (DB
    reachable, embedded Python interpreter usable). CLOACI-T-0916:
    tenant-workload health is deliberately NOT part of readiness — one crashed
    computation graph used to flip `/ready` false on EVERY replica
    simultaneously, letting a single bad package eject the whole server fleet
    from the load-balancer pool. Crashed graphs remain visible via
    `GET /v1/health/graphs` (state `stopped`/`crashed`) and metrics.

     CLOACI-T-0919 adds one more PLATFORM predicate: a wedged Python runtime.
    There is exactly one embedded CPython per process, so an uninterruptible
    module-scope hang disables Python package loading process-wide until
    restart. That is this replica being broken — not a tenant workload
    misbehaving — so it belongs in readiness, and it is replica-local (the
    wedge is set by THIS process's own import thread), which is precisely the
    property #231 required.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
    """

    kwargs = _get_kwargs()

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


async def asyncio_detailed(
    *,
    client: AuthenticatedClient | Client,
) -> Response[Any]:
    """GET /ready — readiness check, scoped to PLATFORM health only (DB
    reachable, embedded Python interpreter usable). CLOACI-T-0916:
    tenant-workload health is deliberately NOT part of readiness — one crashed
    computation graph used to flip `/ready` false on EVERY replica
    simultaneously, letting a single bad package eject the whole server fleet
    from the load-balancer pool. Crashed graphs remain visible via
    `GET /v1/health/graphs` (state `stopped`/`crashed`) and metrics.

     CLOACI-T-0919 adds one more PLATFORM predicate: a wedged Python runtime.
    There is exactly one embedded CPython per process, so an uninterruptible
    module-scope hang disables Python package loading process-wide until
    restart. That is this replica being broken — not a tenant workload
    misbehaving — so it belongs in readiness, and it is replica-local (the
    wedge is set by THIS process's own import thread), which is precisely the
    property #231 required.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
    """

    kwargs = _get_kwargs()

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)
