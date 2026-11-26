from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.audit_list_response import AuditListResponse
from ...models.error_body import ErrorBody
from ...types import UNSET, Response, Unset


def _get_kwargs(
    tenant_id: str,
    *,
    tenant: str | Unset = UNSET,
    action: str | Unset = UNSET,
    actor: str | Unset = UNSET,
    since: str | Unset = UNSET,
    until: str | Unset = UNSET,
    limit: int | Unset = UNSET,
    cursor: int | Unset = UNSET,
) -> dict[str, Any]:

    params: dict[str, Any] = {}

    params["tenant"] = tenant

    params["action"] = action

    params["actor"] = actor

    params["since"] = since

    params["until"] = until

    params["limit"] = limit

    params["cursor"] = cursor

    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}

    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": "/v1/tenants/{tenant_id}/audit".format(
            tenant_id=quote(str(tenant_id), safe=""),
        ),
        "params": params,
    }

    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> AuditListResponse | ErrorBody | None:
    if response.status_code == 200:
        response_200 = AuditListResponse.from_dict(response.json())

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
) -> Response[AuditListResponse | ErrorBody]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    tenant_id: str,
    *,
    client: AuthenticatedClient,
    tenant: str | Unset = UNSET,
    action: str | Unset = UNSET,
    actor: str | Unset = UNSET,
    since: str | Unset = UNSET,
    until: str | Unset = UNSET,
    limit: int | Unset = UNSET,
    cursor: int | Unset = UNSET,
) -> Response[AuditListResponse | ErrorBody]:
    """GET /tenants/:tenant_id/audit — the audit record of one tenant, newest
    first. Tenant admin.

    Args:
        tenant_id (str):
        tenant (str | Unset):
        action (str | Unset):
        actor (str | Unset):
        since (str | Unset):
        until (str | Unset):
        limit (int | Unset):
        cursor (int | Unset):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[AuditListResponse | ErrorBody]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        tenant=tenant,
        action=action,
        actor=actor,
        since=since,
        until=until,
        limit=limit,
        cursor=cursor,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    tenant_id: str,
    *,
    client: AuthenticatedClient,
    tenant: str | Unset = UNSET,
    action: str | Unset = UNSET,
    actor: str | Unset = UNSET,
    since: str | Unset = UNSET,
    until: str | Unset = UNSET,
    limit: int | Unset = UNSET,
    cursor: int | Unset = UNSET,
) -> AuditListResponse | ErrorBody | None:
    """GET /tenants/:tenant_id/audit — the audit record of one tenant, newest
    first. Tenant admin.

    Args:
        tenant_id (str):
        tenant (str | Unset):
        action (str | Unset):
        actor (str | Unset):
        since (str | Unset):
        until (str | Unset):
        limit (int | Unset):
        cursor (int | Unset):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        AuditListResponse | ErrorBody
    """

    return sync_detailed(
        tenant_id=tenant_id,
        client=client,
        tenant=tenant,
        action=action,
        actor=actor,
        since=since,
        until=until,
        limit=limit,
        cursor=cursor,
    ).parsed


async def asyncio_detailed(
    tenant_id: str,
    *,
    client: AuthenticatedClient,
    tenant: str | Unset = UNSET,
    action: str | Unset = UNSET,
    actor: str | Unset = UNSET,
    since: str | Unset = UNSET,
    until: str | Unset = UNSET,
    limit: int | Unset = UNSET,
    cursor: int | Unset = UNSET,
) -> Response[AuditListResponse | ErrorBody]:
    """GET /tenants/:tenant_id/audit — the audit record of one tenant, newest
    first. Tenant admin.

    Args:
        tenant_id (str):
        tenant (str | Unset):
        action (str | Unset):
        actor (str | Unset):
        since (str | Unset):
        until (str | Unset):
        limit (int | Unset):
        cursor (int | Unset):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[AuditListResponse | ErrorBody]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        tenant=tenant,
        action=action,
        actor=actor,
        since=since,
        until=until,
        limit=limit,
        cursor=cursor,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    tenant_id: str,
    *,
    client: AuthenticatedClient,
    tenant: str | Unset = UNSET,
    action: str | Unset = UNSET,
    actor: str | Unset = UNSET,
    since: str | Unset = UNSET,
    until: str | Unset = UNSET,
    limit: int | Unset = UNSET,
    cursor: int | Unset = UNSET,
) -> AuditListResponse | ErrorBody | None:
    """GET /tenants/:tenant_id/audit — the audit record of one tenant, newest
    first. Tenant admin.

    Args:
        tenant_id (str):
        tenant (str | Unset):
        action (str | Unset):
        actor (str | Unset):
        since (str | Unset):
        until (str | Unset):
        limit (int | Unset):
        cursor (int | Unset):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        AuditListResponse | ErrorBody
    """

    return (
        await asyncio_detailed(
            tenant_id=tenant_id,
            client=client,
            tenant=tenant,
            action=action,
            actor=actor,
            since=since,
            until=until,
            limit=limit,
            cursor=cursor,
        )
    ).parsed
