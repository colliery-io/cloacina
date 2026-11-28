from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="WorkflowUploadedResponse")


@_attrs_define
class WorkflowUploadedResponse:
    """`201 Created` body for a workflow package upload
    (`POST /tenants/{tenant_id}/workflows`, multipart).

        Attributes:
            package_id (str): UUID of the registered package.
            tenant_id (str):
            priority_limit (int | None | Unset): The highest priority the runs of this package may get, as its
                publisher may set now (CLOACINA-T-1123). A workflow or task that
                declares a higher priority (a smaller number) in its code runs at
                this one. Absent in an answer from an older server.
            priority_notice (None | str | Unset): Set when the limit is below the highest priority: one sentence that
                says the limit and why. `cloacinactl` prints it as a warning.
    """

    package_id: str
    tenant_id: str
    priority_limit: int | None | Unset = UNSET
    priority_notice: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        package_id = self.package_id

        tenant_id = self.tenant_id

        priority_limit: int | None | Unset
        if isinstance(self.priority_limit, Unset):
            priority_limit = UNSET
        else:
            priority_limit = self.priority_limit

        priority_notice: None | str | Unset
        if isinstance(self.priority_notice, Unset):
            priority_notice = UNSET
        else:
            priority_notice = self.priority_notice

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "package_id": package_id,
                "tenant_id": tenant_id,
            }
        )
        if priority_limit is not UNSET:
            field_dict["priority_limit"] = priority_limit
        if priority_notice is not UNSET:
            field_dict["priority_notice"] = priority_notice

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        package_id = d.pop("package_id")

        tenant_id = d.pop("tenant_id")

        def _parse_priority_limit(data: object) -> int | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(int | None | Unset, data)

        priority_limit = _parse_priority_limit(d.pop("priority_limit", UNSET))

        def _parse_priority_notice(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        priority_notice = _parse_priority_notice(d.pop("priority_notice", UNSET))

        workflow_uploaded_response = cls(
            package_id=package_id,
            tenant_id=tenant_id,
            priority_limit=priority_limit,
            priority_notice=priority_notice,
        )

        workflow_uploaded_response.additional_properties = d
        return workflow_uploaded_response

    @property
    def additional_keys(self) -> list[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> Any:
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: Any) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
