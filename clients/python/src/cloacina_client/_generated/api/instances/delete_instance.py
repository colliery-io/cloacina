from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.delete_instance_response import DeleteInstanceResponse
from ...models.error_body import ErrorBody
from ...types import Response


def _get_kwargs(
    tenant_id: str,
    name: str,
    instance: str,
) -> dict[str, Any]:

    _kwargs: dict[str, Any] = {
        "method": "delete",
        "url": "/v1/tenants/{tenant_id}/workflows/{name}/instances/{instance}".format(
            tenant_id=quote(str(tenant_id), safe=""),
            name=quote(str(name), safe=""),
            instance=quote(str(instance), safe=""),
        ),
    }

    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> DeleteInstanceResponse | ErrorBody | None:
    if response.status_code == 200:
        response_200 = DeleteInstanceResponse.from_dict(response.json())

        return response_200

    if response.status_code == 401:
        response_401 = ErrorBody.from_dict(response.json())

        return response_401

    if response.status_code == 403:
        response_403 = ErrorBody.from_dict(response.json())

        return response_403

    if response.status_code == 404:
        response_404 = ErrorBody.from_dict(response.json())

        return response_404

    if response.status_code == 500:
        response_500 = ErrorBody.from_dict(response.json())

        return response_500

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> Response[DeleteInstanceResponse | ErrorBody]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    tenant_id: str,
    name: str,
    instance: str,
    *,
    client: AuthenticatedClient,
) -> Response[DeleteInstanceResponse | ErrorBody]:
    """DELETE /tenants/:tenant_id/workflows/:name/instances/:instance — remove a
    named instance.

     Deletes the binding and its schedule. In-flight executions already started
    by this instance are unaffected; only future fires are prevented.

    Args:
        tenant_id (str):
        name (str):
        instance (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[DeleteInstanceResponse | ErrorBody]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        name=name,
        instance=instance,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    tenant_id: str,
    name: str,
    instance: str,
    *,
    client: AuthenticatedClient,
) -> DeleteInstanceResponse | ErrorBody | None:
    """DELETE /tenants/:tenant_id/workflows/:name/instances/:instance — remove a
    named instance.

     Deletes the binding and its schedule. In-flight executions already started
    by this instance are unaffected; only future fires are prevented.

    Args:
        tenant_id (str):
        name (str):
        instance (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        DeleteInstanceResponse | ErrorBody
    """

    return sync_detailed(
        tenant_id=tenant_id,
        name=name,
        instance=instance,
        client=client,
    ).parsed


async def asyncio_detailed(
    tenant_id: str,
    name: str,
    instance: str,
    *,
    client: AuthenticatedClient,
) -> Response[DeleteInstanceResponse | ErrorBody]:
    """DELETE /tenants/:tenant_id/workflows/:name/instances/:instance — remove a
    named instance.

     Deletes the binding and its schedule. In-flight executions already started
    by this instance are unaffected; only future fires are prevented.

    Args:
        tenant_id (str):
        name (str):
        instance (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[DeleteInstanceResponse | ErrorBody]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        name=name,
        instance=instance,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    tenant_id: str,
    name: str,
    instance: str,
    *,
    client: AuthenticatedClient,
) -> DeleteInstanceResponse | ErrorBody | None:
    """DELETE /tenants/:tenant_id/workflows/:name/instances/:instance — remove a
    named instance.

     Deletes the binding and its schedule. In-flight executions already started
    by this instance are unaffected; only future fires are prevented.

    Args:
        tenant_id (str):
        name (str):
        instance (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        DeleteInstanceResponse | ErrorBody
    """

    return (
        await asyncio_detailed(
            tenant_id=tenant_id,
            name=name,
            instance=instance,
            client=client,
        )
    ).parsed
