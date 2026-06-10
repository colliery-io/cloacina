from http import HTTPStatus
from typing import Any
from urllib.parse import quote

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_body import ErrorBody
from ...models.package_upload_form import PackageUploadForm
from ...models.workflow_uploaded_response import WorkflowUploadedResponse
from ...types import Response


def _get_kwargs(
    tenant_id: str,
    *,
    body: PackageUploadForm,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/v1/tenants/{tenant_id}/workflows".format(
            tenant_id=quote(str(tenant_id), safe=""),
        ),
    }

    _kwargs["files"] = body.to_multipart()

    headers["Content-Type"] = "multipart/form-data; boundary=+++"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: AuthenticatedClient | Client, response: httpx.Response
) -> ErrorBody | WorkflowUploadedResponse | None:
    if response.status_code == 201:
        response_201 = WorkflowUploadedResponse.from_dict(response.json())

        return response_201

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
) -> Response[ErrorBody | WorkflowUploadedResponse]:
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
    body: PackageUploadForm,
) -> Response[ErrorBody | WorkflowUploadedResponse]:
    """POST /tenants/:tenant_id/workflows — multipart upload of .cloacina source package.

    Args:
        tenant_id (str):
        body (PackageUploadForm): Multipart form for workflow package upload. Spec-only type: the
            handler
            accepts the first file field regardless of name; `file` is the
            conventional field name.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | WorkflowUploadedResponse]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    tenant_id: str,
    *,
    client: AuthenticatedClient,
    body: PackageUploadForm,
) -> ErrorBody | WorkflowUploadedResponse | None:
    """POST /tenants/:tenant_id/workflows — multipart upload of .cloacina source package.

    Args:
        tenant_id (str):
        body (PackageUploadForm): Multipart form for workflow package upload. Spec-only type: the
            handler
            accepts the first file field regardless of name; `file` is the
            conventional field name.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | WorkflowUploadedResponse
    """

    return sync_detailed(
        tenant_id=tenant_id,
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    tenant_id: str,
    *,
    client: AuthenticatedClient,
    body: PackageUploadForm,
) -> Response[ErrorBody | WorkflowUploadedResponse]:
    """POST /tenants/:tenant_id/workflows — multipart upload of .cloacina source package.

    Args:
        tenant_id (str):
        body (PackageUploadForm): Multipart form for workflow package upload. Spec-only type: the
            handler
            accepts the first file field regardless of name; `file` is the
            conventional field name.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[ErrorBody | WorkflowUploadedResponse]
    """

    kwargs = _get_kwargs(
        tenant_id=tenant_id,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    tenant_id: str,
    *,
    client: AuthenticatedClient,
    body: PackageUploadForm,
) -> ErrorBody | WorkflowUploadedResponse | None:
    """POST /tenants/:tenant_id/workflows — multipart upload of .cloacina source package.

    Args:
        tenant_id (str):
        body (PackageUploadForm): Multipart form for workflow package upload. Spec-only type: the
            handler
            accepts the first file field regardless of name; `file` is the
            conventional field name.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        ErrorBody | WorkflowUploadedResponse
    """

    return (
        await asyncio_detailed(
            tenant_id=tenant_id,
            client=client,
            body=body,
        )
    ).parsed
