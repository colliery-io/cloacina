from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="WorkflowVersionActivatedResponse")


@_attrs_define
class WorkflowVersionActivatedResponse:
    """`POST /tenants/{tenant_id}/workflows/{name}/versions/{version}/activate`
    response (CLOACINA-T-1018).

        Attributes:
            id (str): Package UUID of the version that is now active.
            package_name (str):
            status (str): Always `"active"`.
            tenant_id (str):
            version (str):
    """

    id: str
    package_name: str
    status: str
    tenant_id: str
    version: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        id = self.id

        package_name = self.package_name

        status = self.status

        tenant_id = self.tenant_id

        version = self.version

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "package_name": package_name,
                "status": status,
                "tenant_id": tenant_id,
                "version": version,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        id = d.pop("id")

        package_name = d.pop("package_name")

        status = d.pop("status")

        tenant_id = d.pop("tenant_id")

        version = d.pop("version")

        workflow_version_activated_response = cls(
            id=id,
            package_name=package_name,
            status=status,
            tenant_id=tenant_id,
            version=version,
        )

        workflow_version_activated_response.additional_properties = d
        return workflow_version_activated_response

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
