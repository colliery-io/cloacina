from http import HTTPStatus
from typing import Any

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_body import ErrorBody
from ...models.ws_ticket_response import WsTicketResponse
from ...types import Response


def _get_kwargs() -> dict[str, Any]:

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/v1/auth/ws-ticket",
    }

    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> ErrorBody | WsTicketResponse | None:
    if response.status_code == 200:
        response_200 = WsTicketResponse.from_dict(response.json())

        return response_200

    if response.status_code == 401:
        response_401 = ErrorBody.from_dict(response.json())

        return response_401

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> Response[ErrorBody | WsTicketResponse]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    *,
    client: AuthenticatedClient,
) -> Response[ErrorBody | WsTicketResponse]:
    """POST /auth/ws-ticket — exchange a Bearer token for a single-use WebSocket ticket.

     Returns a short-lived ticket that can be used as a query parameter for
    WebSocket upgrade requests, avoiding long-lived API keys in URLs.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | WsTicketResponse]
    """

    kwargs = _get_kwargs()

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    *,
    client: AuthenticatedClient,
) -> ErrorBody | WsTicketResponse | None:
    """POST /auth/ws-ticket — exchange a Bearer token for a single-use WebSocket ticket.

     Returns a short-lived ticket that can be used as a query parameter for
    WebSocket upgrade requests, avoiding long-lived API keys in URLs.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | WsTicketResponse
    """

    return sync_detailed(
        client=client,
    ).parsed


async def asyncio_detailed(
    *,
    client: AuthenticatedClient,
) -> Response[ErrorBody | WsTicketResponse]:
    """POST /auth/ws-ticket — exchange a Bearer token for a single-use WebSocket ticket.

     Returns a short-lived ticket that can be used as a query parameter for
    WebSocket upgrade requests, avoiding long-lived API keys in URLs.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | WsTicketResponse]
    """

    kwargs = _get_kwargs()

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    *,
    client: AuthenticatedClient,
) -> ErrorBody | WsTicketResponse | None:
    """POST /auth/ws-ticket — exchange a Bearer token for a single-use WebSocket ticket.

     Returns a short-lived ticket that can be used as a query parameter for
    WebSocket upgrade requests, avoiding long-lived API keys in URLs.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | WsTicketResponse
    """

    return (
        await asyncio_detailed(
            client=client,
        )
    ).parsed
