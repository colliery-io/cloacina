from http import HTTPStatus
from typing import Any

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_body import ErrorBody
from ...models.local_login_request import LocalLoginRequest
from ...models.local_login_response import LocalLoginResponse
from ...types import Response


def _get_kwargs(
    *,
    body: LocalLoginRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/v1/auth/local/login",
    }

    _kwargs["json"] = body.to_dict()

    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> ErrorBody | LocalLoginResponse | None:
    if response.status_code == 200:
        response_200 = LocalLoginResponse.from_dict(response.json())

        return response_200

    if response.status_code == 401:
        response_401 = ErrorBody.from_dict(response.json())

        return response_401

    if response.status_code == 500:
        response_500 = ErrorBody.from_dict(response.json())

        return response_500

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> Response[ErrorBody | LocalLoginResponse]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    *,
    client: AuthenticatedClient | Client,
    body: LocalLoginRequest,
) -> Response[ErrorBody | LocalLoginResponse]:
    """`POST /v1/auth/local/login` — verify a password, mint a short-TTL key.

    Args:
        body (LocalLoginRequest): A local-login attempt. `tenant` selects which tenant's account
            namespace to
            authenticate against (`None` = a global account).

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | LocalLoginResponse]
    """

    kwargs = _get_kwargs(
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    *,
    client: AuthenticatedClient | Client,
    body: LocalLoginRequest,
) -> ErrorBody | LocalLoginResponse | None:
    """`POST /v1/auth/local/login` — verify a password, mint a short-TTL key.

    Args:
        body (LocalLoginRequest): A local-login attempt. `tenant` selects which tenant's account
            namespace to
            authenticate against (`None` = a global account).

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | LocalLoginResponse
    """

    return sync_detailed(
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    *,
    client: AuthenticatedClient | Client,
    body: LocalLoginRequest,
) -> Response[ErrorBody | LocalLoginResponse]:
    """`POST /v1/auth/local/login` — verify a password, mint a short-TTL key.

    Args:
        body (LocalLoginRequest): A local-login attempt. `tenant` selects which tenant's account
            namespace to
            authenticate against (`None` = a global account).

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | LocalLoginResponse]
    """

    kwargs = _get_kwargs(
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    *,
    client: AuthenticatedClient | Client,
    body: LocalLoginRequest,
) -> ErrorBody | LocalLoginResponse | None:
    """`POST /v1/auth/local/login` — verify a password, mint a short-TTL key.

    Args:
        body (LocalLoginRequest): A local-login attempt. `tenant` selects which tenant's account
            namespace to
            authenticate against (`None` = a global account).

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | LocalLoginResponse
    """

    return (
        await asyncio_detailed(
            client=client,
            body=body,
        )
    ).parsed
