from http import HTTPStatus
from typing import Any

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_body import ErrorBody
from ...models.list_response_agent_info import ListResponseAgentInfo
from ...types import Response


def _get_kwargs() -> dict[str, Any]:

    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": "/v1/agents",
    }

    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> ErrorBody | ListResponseAgentInfo | None:
    if response.status_code == 200:
        response_200 = ListResponseAgentInfo.from_dict(response.json())

        return response_200

    if response.status_code == 401:
        response_401 = ErrorBody.from_dict(response.json())

        return response_401

    if response.status_code == 403:
        response_403 = ErrorBody.from_dict(response.json())

        return response_403

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> Response[ErrorBody | ListResponseAgentInfo]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    *,
    client: AuthenticatedClient,
) -> Response[ErrorBody | ListResponseAgentInfo]:
    """`GET /v1/agents` — operator-facing snapshot of the execution-agent fleet
    roster (admin only). CLOACI-I-0124 / WS-0b. Per-replica: reflects the agents
    registered against *this* server instance.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | ListResponseAgentInfo]
    """

    kwargs = _get_kwargs()

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    *,
    client: AuthenticatedClient,
) -> ErrorBody | ListResponseAgentInfo | None:
    """`GET /v1/agents` — operator-facing snapshot of the execution-agent fleet
    roster (admin only). CLOACI-I-0124 / WS-0b. Per-replica: reflects the agents
    registered against *this* server instance.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | ListResponseAgentInfo
    """

    return sync_detailed(
        client=client,
    ).parsed


async def asyncio_detailed(
    *,
    client: AuthenticatedClient,
) -> Response[ErrorBody | ListResponseAgentInfo]:
    """`GET /v1/agents` — operator-facing snapshot of the execution-agent fleet
    roster (admin only). CLOACI-I-0124 / WS-0b. Per-replica: reflects the agents
    registered against *this* server instance.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | ListResponseAgentInfo]
    """

    kwargs = _get_kwargs()

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    *,
    client: AuthenticatedClient,
) -> ErrorBody | ListResponseAgentInfo | None:
    """`GET /v1/agents` — operator-facing snapshot of the execution-agent fleet
    roster (admin only). CLOACI-I-0124 / WS-0b. Per-replica: reflects the agents
    registered against *this* server instance.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | ListResponseAgentInfo
    """

    return (
        await asyncio_detailed(
            client=client,
        )
    ).parsed
