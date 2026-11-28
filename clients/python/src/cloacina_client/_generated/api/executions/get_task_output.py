from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_body import ErrorBody
from ...models.task_output_response import TaskOutputResponse
from ...types import UNSET, Response, Unset


def _get_kwargs(
    tenant_id: str,
    exec_id: str,
    task_id: str,
    *,
    attempt: int | Unset = UNSET,
) -> dict[str, Any]:

    params: dict[str, Any] = {}

    params["attempt"] = attempt

    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}

    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": "/v1/tenants/{tenant_id}/executions/{exec_id}/tasks/{task_id}/output".format(
            tenant_id=quote(str(tenant_id), safe=""),
            exec_id=quote(str(exec_id), safe=""),
            task_id=quote(str(task_id), safe=""),
        ),
        "params": params,
    }

    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> ErrorBody | TaskOutputResponse | None:
    if response.status_code == 200:
        response_200 = TaskOutputResponse.from_dict(response.json())

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

    if response.status_code == 500:
        response_500 = ErrorBody.from_dict(response.json())

        return response_500

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> Response[ErrorBody | TaskOutputResponse]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    tenant_id: str,
    exec_id: str,
    task_id: str,
    *,
    client: AuthenticatedClient,
    attempt: int | Unset = UNSET,
) -> Response[ErrorBody | TaskOutputResponse]:
    """GET /tenants/:tenant_id/executions/:id/tasks/:task_id/output — the end of
    the output of each attempt of a task (CLOACINA-T-1091).

    Args:
        tenant_id (str):
        exec_id (str):
        task_id (str):
        attempt (int | Unset):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | TaskOutputResponse]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        exec_id=exec_id,
        task_id=task_id,
        attempt=attempt,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    tenant_id: str,
    exec_id: str,
    task_id: str,
    *,
    client: AuthenticatedClient,
    attempt: int | Unset = UNSET,
) -> ErrorBody | TaskOutputResponse | None:
    """GET /tenants/:tenant_id/executions/:id/tasks/:task_id/output — the end of
    the output of each attempt of a task (CLOACINA-T-1091).

    Args:
        tenant_id (str):
        exec_id (str):
        task_id (str):
        attempt (int | Unset):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | TaskOutputResponse
    """

    return sync_detailed(
        tenant_id=tenant_id,
        exec_id=exec_id,
        task_id=task_id,
        client=client,
        attempt=attempt,
    ).parsed


async def asyncio_detailed(
    tenant_id: str,
    exec_id: str,
    task_id: str,
    *,
    client: AuthenticatedClient,
    attempt: int | Unset = UNSET,
) -> Response[ErrorBody | TaskOutputResponse]:
    """GET /tenants/:tenant_id/executions/:id/tasks/:task_id/output — the end of
    the output of each attempt of a task (CLOACINA-T-1091).

    Args:
        tenant_id (str):
        exec_id (str):
        task_id (str):
        attempt (int | Unset):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | TaskOutputResponse]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        exec_id=exec_id,
        task_id=task_id,
        attempt=attempt,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    tenant_id: str,
    exec_id: str,
    task_id: str,
    *,
    client: AuthenticatedClient,
    attempt: int | Unset = UNSET,
) -> ErrorBody | TaskOutputResponse | None:
    """GET /tenants/:tenant_id/executions/:id/tasks/:task_id/output — the end of
    the output of each attempt of a task (CLOACINA-T-1091).

    Args:
        tenant_id (str):
        exec_id (str):
        task_id (str):
        attempt (int | Unset):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | TaskOutputResponse
    """

    return (
        await asyncio_detailed(
            tenant_id=tenant_id,
            exec_id=exec_id,
            task_id=task_id,
            client=client,
            attempt=attempt,
        )
    ).parsed
