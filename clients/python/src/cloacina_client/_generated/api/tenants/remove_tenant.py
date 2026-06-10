from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_body import ErrorBody
from ...models.tenant_removed_response import TenantRemovedResponse
from ...types import Response


def _get_kwargs(
    schema_name: str,
) -> dict[str, Any]:

    _kwargs: dict[str, Any] = {
        "method": "delete",
        "url": "/v1/tenants/{schema_name}".format(
            schema_name=quote(str(schema_name), safe=""),
        ),
    }

    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> ErrorBody | TenantRemovedResponse | None:
    if response.status_code == 200:
        response_200 = TenantRemovedResponse.from_dict(response.json())

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
) -> Response[ErrorBody | TenantRemovedResponse]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    schema_name: str,
    *,
    client: AuthenticatedClient,
) -> Response[ErrorBody | TenantRemovedResponse]:
    """DELETE /tenants/:schema_name — remove a tenant via orchestrated teardown.

     CLOACI-T-0581: replaces the old single-call `admin.remove_tenant` with
    the four-step top-down order:
      1. Revoke every still-active API key for the tenant (close the auth
         surface so new requests fail).
      2. Evict the tenant's `DefaultRunner` from `TenantRunnerCache`,
         awaiting its graceful shutdown (drains in-flight executions,
         stops scheduler loop, closes per-tenant DB pool).
      3. Evict the tenant's `Database` from `TenantDatabaseCache`.
      4. Drop schema + user via `DatabaseAdmin::remove_tenant`.

    Each step emits a structured audit event with duration. Per-step
    failures bail out — but earlier steps stay committed, so a retry
    picks up where the failure occurred. Each step is idempotent.

    Closes SEC-14 (stale state after delete) and SEC-17 (unbounded
    caches surviving delete).

    Args:
        schema_name (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | TenantRemovedResponse]
    """

    kwargs = _get_kwargs(
        schema_name=schema_name,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    schema_name: str,
    *,
    client: AuthenticatedClient,
) -> ErrorBody | TenantRemovedResponse | None:
    """DELETE /tenants/:schema_name — remove a tenant via orchestrated teardown.

     CLOACI-T-0581: replaces the old single-call `admin.remove_tenant` with
    the four-step top-down order:
      1. Revoke every still-active API key for the tenant (close the auth
         surface so new requests fail).
      2. Evict the tenant's `DefaultRunner` from `TenantRunnerCache`,
         awaiting its graceful shutdown (drains in-flight executions,
         stops scheduler loop, closes per-tenant DB pool).
      3. Evict the tenant's `Database` from `TenantDatabaseCache`.
      4. Drop schema + user via `DatabaseAdmin::remove_tenant`.

    Each step emits a structured audit event with duration. Per-step
    failures bail out — but earlier steps stay committed, so a retry
    picks up where the failure occurred. Each step is idempotent.

    Closes SEC-14 (stale state after delete) and SEC-17 (unbounded
    caches surviving delete).

    Args:
        schema_name (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | TenantRemovedResponse
    """

    return sync_detailed(
        schema_name=schema_name,
        client=client,
    ).parsed


async def asyncio_detailed(
    schema_name: str,
    *,
    client: AuthenticatedClient,
) -> Response[ErrorBody | TenantRemovedResponse]:
    """DELETE /tenants/:schema_name — remove a tenant via orchestrated teardown.

     CLOACI-T-0581: replaces the old single-call `admin.remove_tenant` with
    the four-step top-down order:
      1. Revoke every still-active API key for the tenant (close the auth
         surface so new requests fail).
      2. Evict the tenant's `DefaultRunner` from `TenantRunnerCache`,
         awaiting its graceful shutdown (drains in-flight executions,
         stops scheduler loop, closes per-tenant DB pool).
      3. Evict the tenant's `Database` from `TenantDatabaseCache`.
      4. Drop schema + user via `DatabaseAdmin::remove_tenant`.

    Each step emits a structured audit event with duration. Per-step
    failures bail out — but earlier steps stay committed, so a retry
    picks up where the failure occurred. Each step is idempotent.

    Closes SEC-14 (stale state after delete) and SEC-17 (unbounded
    caches surviving delete).

    Args:
        schema_name (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | TenantRemovedResponse]
    """

    kwargs = _get_kwargs(
        schema_name=schema_name,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    schema_name: str,
    *,
    client: AuthenticatedClient,
) -> ErrorBody | TenantRemovedResponse | None:
    """DELETE /tenants/:schema_name — remove a tenant via orchestrated teardown.

     CLOACI-T-0581: replaces the old single-call `admin.remove_tenant` with
    the four-step top-down order:
      1. Revoke every still-active API key for the tenant (close the auth
         surface so new requests fail).
      2. Evict the tenant's `DefaultRunner` from `TenantRunnerCache`,
         awaiting its graceful shutdown (drains in-flight executions,
         stops scheduler loop, closes per-tenant DB pool).
      3. Evict the tenant's `Database` from `TenantDatabaseCache`.
      4. Drop schema + user via `DatabaseAdmin::remove_tenant`.

    Each step emits a structured audit event with duration. Per-step
    failures bail out — but earlier steps stay committed, so a retry
    picks up where the failure occurred. Each step is idempotent.

    Closes SEC-14 (stale state after delete) and SEC-17 (unbounded
    caches surviving delete).

    Args:
        schema_name (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | TenantRemovedResponse
    """

    return (
        await asyncio_detailed(
            schema_name=schema_name,
            client=client,
        )
    ).parsed
