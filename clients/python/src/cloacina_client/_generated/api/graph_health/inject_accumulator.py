from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_body import ErrorBody
from ...models.inject_accumulator_request import InjectAccumulatorRequest
from ...models.inject_accumulator_response import InjectAccumulatorResponse
from ...types import Response


def _get_kwargs(
    name: str,
    *,
    body: InjectAccumulatorRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/v1/health/accumulators/{name}/inject".format(
            name=quote(str(name), safe=""),
        ),
    }

    _kwargs["json"] = body.to_dict()

    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> ErrorBody | InjectAccumulatorResponse | None:
    if response.status_code == 200:
        response_200 = InjectAccumulatorResponse.from_dict(response.json())

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

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> Response[ErrorBody | InjectAccumulatorResponse]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    name: str,
    *,
    client: AuthenticatedClient,
    body: InjectAccumulatorRequest,
) -> Response[ErrorBody | InjectAccumulatorResponse]:
    """POST /v1/health/accumulators/{name}/inject — push a single typed event into
    a running accumulator (CLOACI-T-0753).

     Operator-facing REST analogue of the WS accumulator-push path. The typed JSON
    `event` is serialized to the boundary wire encoding server-side (same framing
    as the front-door accumulator socket), so operators never craft raw
    `Vec<u8>`. Authorization reuses the accumulator endpoint policy — the same
    gate the WS accumulator endpoint enforces. Successful injects are
    audit-logged and marked `operator_injected` since they bypass the real event
    source.

    Args:
        name (str):
        body (InjectAccumulatorRequest): Request body for `POST
            /v1/health/accumulators/{name}/inject` (CLOACI-T-0753)
            — push a single typed event into a running accumulator, the operator-facing
            REST analogue of the WS accumulator-push path. The JSON `event` is serialized
            to the boundary wire encoding server-side, so operators never craft raw
            `Vec<u8>`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | InjectAccumulatorResponse]
    """

    kwargs = _get_kwargs(
        name=name,
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    name: str,
    *,
    client: AuthenticatedClient,
    body: InjectAccumulatorRequest,
) -> ErrorBody | InjectAccumulatorResponse | None:
    """POST /v1/health/accumulators/{name}/inject — push a single typed event into
    a running accumulator (CLOACI-T-0753).

     Operator-facing REST analogue of the WS accumulator-push path. The typed JSON
    `event` is serialized to the boundary wire encoding server-side (same framing
    as the front-door accumulator socket), so operators never craft raw
    `Vec<u8>`. Authorization reuses the accumulator endpoint policy — the same
    gate the WS accumulator endpoint enforces. Successful injects are
    audit-logged and marked `operator_injected` since they bypass the real event
    source.

    Args:
        name (str):
        body (InjectAccumulatorRequest): Request body for `POST
            /v1/health/accumulators/{name}/inject` (CLOACI-T-0753)
            — push a single typed event into a running accumulator, the operator-facing
            REST analogue of the WS accumulator-push path. The JSON `event` is serialized
            to the boundary wire encoding server-side, so operators never craft raw
            `Vec<u8>`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | InjectAccumulatorResponse
    """

    return sync_detailed(
        name=name,
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    name: str,
    *,
    client: AuthenticatedClient,
    body: InjectAccumulatorRequest,
) -> Response[ErrorBody | InjectAccumulatorResponse]:
    """POST /v1/health/accumulators/{name}/inject — push a single typed event into
    a running accumulator (CLOACI-T-0753).

     Operator-facing REST analogue of the WS accumulator-push path. The typed JSON
    `event` is serialized to the boundary wire encoding server-side (same framing
    as the front-door accumulator socket), so operators never craft raw
    `Vec<u8>`. Authorization reuses the accumulator endpoint policy — the same
    gate the WS accumulator endpoint enforces. Successful injects are
    audit-logged and marked `operator_injected` since they bypass the real event
    source.

    Args:
        name (str):
        body (InjectAccumulatorRequest): Request body for `POST
            /v1/health/accumulators/{name}/inject` (CLOACI-T-0753)
            — push a single typed event into a running accumulator, the operator-facing
            REST analogue of the WS accumulator-push path. The JSON `event` is serialized
            to the boundary wire encoding server-side, so operators never craft raw
            `Vec<u8>`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | InjectAccumulatorResponse]
    """

    kwargs = _get_kwargs(
        name=name,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    name: str,
    *,
    client: AuthenticatedClient,
    body: InjectAccumulatorRequest,
) -> ErrorBody | InjectAccumulatorResponse | None:
    """POST /v1/health/accumulators/{name}/inject — push a single typed event into
    a running accumulator (CLOACI-T-0753).

     Operator-facing REST analogue of the WS accumulator-push path. The typed JSON
    `event` is serialized to the boundary wire encoding server-side (same framing
    as the front-door accumulator socket), so operators never craft raw
    `Vec<u8>`. Authorization reuses the accumulator endpoint policy — the same
    gate the WS accumulator endpoint enforces. Successful injects are
    audit-logged and marked `operator_injected` since they bypass the real event
    source.

    Args:
        name (str):
        body (InjectAccumulatorRequest): Request body for `POST
            /v1/health/accumulators/{name}/inject` (CLOACI-T-0753)
            — push a single typed event into a running accumulator, the operator-facing
            REST analogue of the WS accumulator-push path. The JSON `event` is serialized
            to the boundary wire encoding server-side, so operators never craft raw
            `Vec<u8>`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | InjectAccumulatorResponse
    """

    return (
        await asyncio_detailed(
            name=name,
            client=client,
            body=body,
        )
    ).parsed
