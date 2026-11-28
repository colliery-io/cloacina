from http import HTTPStatus
from typing import Any

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_body import ErrorBody
from ...models.list_response_package_needing_new_build import (
    ListResponsePackageNeedingNewBuild,
)
from ...types import Response


def _get_kwargs() -> dict[str, Any]:

    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": "/v1/packages/needs-new-build",
    }

    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> ErrorBody | ListResponsePackageNeedingNewBuild | None:
    if response.status_code == 200:
        response_200 = ListResponsePackageNeedingNewBuild.from_dict(response.json())

        return response_200

    if response.status_code == 401:
        response_401 = ErrorBody.from_dict(response.json())

        return response_401

    if response.status_code == 403:
        response_403 = ErrorBody.from_dict(response.json())

        return response_403

    if response.status_code == 500:
        response_500 = ErrorBody.from_dict(response.json())

        return response_500

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> Response[ErrorBody | ListResponsePackageNeedingNewBuild]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    *,
    client: AuthenticatedClient,
) -> Response[ErrorBody | ListResponsePackageNeedingNewBuild]:
    """GET /packages/needs-new-build — the packages of every tenant that need a
    new publish (CLOACINA-T-1085).

     After a minor release that changes the plugin interface, each Rust package
    built for the old interface gets one rebuild from its kept source. A
    package that pins the old minor rebuilds with the old interface again and
    is listed here, with its tenant, name, version, both interface versions
    and the step. A platform admin reads it during an upgrade.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | ListResponsePackageNeedingNewBuild]
    """

    kwargs = _get_kwargs()

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    *,
    client: AuthenticatedClient,
) -> ErrorBody | ListResponsePackageNeedingNewBuild | None:
    """GET /packages/needs-new-build — the packages of every tenant that need a
    new publish (CLOACINA-T-1085).

     After a minor release that changes the plugin interface, each Rust package
    built for the old interface gets one rebuild from its kept source. A
    package that pins the old minor rebuilds with the old interface again and
    is listed here, with its tenant, name, version, both interface versions
    and the step. A platform admin reads it during an upgrade.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | ListResponsePackageNeedingNewBuild
    """

    return sync_detailed(
        client=client,
    ).parsed


async def asyncio_detailed(
    *,
    client: AuthenticatedClient,
) -> Response[ErrorBody | ListResponsePackageNeedingNewBuild]:
    """GET /packages/needs-new-build — the packages of every tenant that need a
    new publish (CLOACINA-T-1085).

     After a minor release that changes the plugin interface, each Rust package
    built for the old interface gets one rebuild from its kept source. A
    package that pins the old minor rebuilds with the old interface again and
    is listed here, with its tenant, name, version, both interface versions
    and the step. A platform admin reads it during an upgrade.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | ListResponsePackageNeedingNewBuild]
    """

    kwargs = _get_kwargs()

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    *,
    client: AuthenticatedClient,
) -> ErrorBody | ListResponsePackageNeedingNewBuild | None:
    """GET /packages/needs-new-build — the packages of every tenant that need a
    new publish (CLOACINA-T-1085).

     After a minor release that changes the plugin interface, each Rust package
    built for the old interface gets one rebuild from its kept source. A
    package that pins the old minor rebuilds with the old interface again and
    is listed here, with its tenant, name, version, both interface versions
    and the step. A platform admin reads it during an upgrade.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | ListResponsePackageNeedingNewBuild
    """

    return (
        await asyncio_detailed(
            client=client,
        )
    ).parsed
