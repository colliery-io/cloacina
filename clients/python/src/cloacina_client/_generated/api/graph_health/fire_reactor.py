from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_body import ErrorBody
from ...models.fire_reactor_request import FireReactorRequest
from ...models.fire_reactor_response import FireReactorResponse
from ...types import Response


def _get_kwargs(
    name: str,
    *,
    body: FireReactorRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/v1/health/reactors/{name}/fire".format(
            name=quote(str(name), safe=""),
        ),
    }

    _kwargs["json"] = body.to_dict()

    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> ErrorBody | FireReactorResponse | None:
    if response.status_code == 200:
        response_200 = FireReactorResponse.from_dict(response.json())

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
) -> Response[ErrorBody | FireReactorResponse]:
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
    body: FireReactorRequest,
) -> Response[ErrorBody | FireReactorResponse]:
    """POST /v1/health/reactors/{name}/fire — manually fire a running reactor
    (CLOACI-T-0751).

     Operator-facing REST surface over the existing reactor `ForceFire` /
    `FireWith` mechanics (previously WebSocket-only). Two modes:

    - `force_fire`: fire the graph with the reactor's current cache, untouched.
    - `fire_with`: replace the reactor's cache with the supplied typed `inputs`
      then fire. Full-replace only (mirrors the engine's `replace_all`); there
      is no partial/merge mode in v1.

    Typed JSON `inputs` are serialized to the boundary wire encoding
    server-side so operators never deal in raw `Vec<u8>`. Each source value is
    encoded exactly as the front-door accumulator path encodes it: the JSON is
    rendered to UTF-8 bytes, then those bytes are bincode-wrapped
    (`bincode(Vec<u8>)`) — the format the passthrough accumulator and the FFI
    bridge expect.

    Authorization reuses the existing per-op reactor policy
    (`ReactorOp::ForceFire` / `ReactorOp::FireWith`), the same gate the WS
    reactor endpoint enforces. Successful fires are audit-logged and marked
    `operator_injected` since they bypass the real event source.

    Args:
        name (str):
        body (FireReactorRequest): Request body for `POST /v1/health/reactors/{name}/fire`
            (CLOACI-T-0751).

            Operators supply typed JSON per source; the server serializes each value
            to the boundary wire encoding so callers never deal in raw `Vec<u8>`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | FireReactorResponse]
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
    body: FireReactorRequest,
) -> ErrorBody | FireReactorResponse | None:
    """POST /v1/health/reactors/{name}/fire — manually fire a running reactor
    (CLOACI-T-0751).

     Operator-facing REST surface over the existing reactor `ForceFire` /
    `FireWith` mechanics (previously WebSocket-only). Two modes:

    - `force_fire`: fire the graph with the reactor's current cache, untouched.
    - `fire_with`: replace the reactor's cache with the supplied typed `inputs`
      then fire. Full-replace only (mirrors the engine's `replace_all`); there
      is no partial/merge mode in v1.

    Typed JSON `inputs` are serialized to the boundary wire encoding
    server-side so operators never deal in raw `Vec<u8>`. Each source value is
    encoded exactly as the front-door accumulator path encodes it: the JSON is
    rendered to UTF-8 bytes, then those bytes are bincode-wrapped
    (`bincode(Vec<u8>)`) — the format the passthrough accumulator and the FFI
    bridge expect.

    Authorization reuses the existing per-op reactor policy
    (`ReactorOp::ForceFire` / `ReactorOp::FireWith`), the same gate the WS
    reactor endpoint enforces. Successful fires are audit-logged and marked
    `operator_injected` since they bypass the real event source.

    Args:
        name (str):
        body (FireReactorRequest): Request body for `POST /v1/health/reactors/{name}/fire`
            (CLOACI-T-0751).

            Operators supply typed JSON per source; the server serializes each value
            to the boundary wire encoding so callers never deal in raw `Vec<u8>`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | FireReactorResponse
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
    body: FireReactorRequest,
) -> Response[ErrorBody | FireReactorResponse]:
    """POST /v1/health/reactors/{name}/fire — manually fire a running reactor
    (CLOACI-T-0751).

     Operator-facing REST surface over the existing reactor `ForceFire` /
    `FireWith` mechanics (previously WebSocket-only). Two modes:

    - `force_fire`: fire the graph with the reactor's current cache, untouched.
    - `fire_with`: replace the reactor's cache with the supplied typed `inputs`
      then fire. Full-replace only (mirrors the engine's `replace_all`); there
      is no partial/merge mode in v1.

    Typed JSON `inputs` are serialized to the boundary wire encoding
    server-side so operators never deal in raw `Vec<u8>`. Each source value is
    encoded exactly as the front-door accumulator path encodes it: the JSON is
    rendered to UTF-8 bytes, then those bytes are bincode-wrapped
    (`bincode(Vec<u8>)`) — the format the passthrough accumulator and the FFI
    bridge expect.

    Authorization reuses the existing per-op reactor policy
    (`ReactorOp::ForceFire` / `ReactorOp::FireWith`), the same gate the WS
    reactor endpoint enforces. Successful fires are audit-logged and marked
    `operator_injected` since they bypass the real event source.

    Args:
        name (str):
        body (FireReactorRequest): Request body for `POST /v1/health/reactors/{name}/fire`
            (CLOACI-T-0751).

            Operators supply typed JSON per source; the server serializes each value
            to the boundary wire encoding so callers never deal in raw `Vec<u8>`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | FireReactorResponse]
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
    body: FireReactorRequest,
) -> ErrorBody | FireReactorResponse | None:
    """POST /v1/health/reactors/{name}/fire — manually fire a running reactor
    (CLOACI-T-0751).

     Operator-facing REST surface over the existing reactor `ForceFire` /
    `FireWith` mechanics (previously WebSocket-only). Two modes:

    - `force_fire`: fire the graph with the reactor's current cache, untouched.
    - `fire_with`: replace the reactor's cache with the supplied typed `inputs`
      then fire. Full-replace only (mirrors the engine's `replace_all`); there
      is no partial/merge mode in v1.

    Typed JSON `inputs` are serialized to the boundary wire encoding
    server-side so operators never deal in raw `Vec<u8>`. Each source value is
    encoded exactly as the front-door accumulator path encodes it: the JSON is
    rendered to UTF-8 bytes, then those bytes are bincode-wrapped
    (`bincode(Vec<u8>)`) — the format the passthrough accumulator and the FFI
    bridge expect.

    Authorization reuses the existing per-op reactor policy
    (`ReactorOp::ForceFire` / `ReactorOp::FireWith`), the same gate the WS
    reactor endpoint enforces. Successful fires are audit-logged and marked
    `operator_injected` since they bypass the real event source.

    Args:
        name (str):
        body (FireReactorRequest): Request body for `POST /v1/health/reactors/{name}/fire`
            (CLOACI-T-0751).

            Operators supply typed JSON per source; the server serializes each value
            to the boundary wire encoding so callers never deal in raw `Vec<u8>`.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | FireReactorResponse
    """

    return (
        await asyncio_detailed(
            name=name,
            client=client,
            body=body,
        )
    ).parsed
