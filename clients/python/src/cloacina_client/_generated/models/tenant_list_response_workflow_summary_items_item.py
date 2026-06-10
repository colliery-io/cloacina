from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="TenantListResponseWorkflowSummaryItemsItem")


@_attrs_define
class TenantListResponseWorkflowSummaryItemsItem:
    """One row in the workflow list (`GET /tenants/{tenant_id}/workflows`).

    Attributes:
        created_at (str): RFC 3339 timestamp.
        id (str): Package UUID.
        package_name (str):
        tasks (list[str]): Task IDs included in this package.
        version (str):
        description (None | str | Unset):
    """

    created_at: str
    id: str
    package_name: str
    tasks: list[str]
    version: str
    description: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        created_at = self.created_at

        id = self.id

        package_name = self.package_name

        tasks = self.tasks

        version = self.version

        description: None | str | Unset
        if isinstance(self.description, Unset):
            description = UNSET
        else:
            description = self.description

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "created_at": created_at,
                "id": id,
                "package_name": package_name,
                "tasks": tasks,
                "version": version,
            }
        )
        if description is not UNSET:
            field_dict["description"] = description

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        created_at = d.pop("created_at")

        id = d.pop("id")

        package_name = d.pop("package_name")

        tasks = cast(list[str], d.pop("tasks"))

        version = d.pop("version")

        def _parse_description(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        description = _parse_description(d.pop("description", UNSET))

        tenant_list_response_workflow_summary_items_item = cls(
            created_at=created_at,
            id=id,
            package_name=package_name,
            tasks=tasks,
            version=version,
            description=description,
        )

        tenant_list_response_workflow_summary_items_item.additional_properties = d
        return tenant_list_response_workflow_summary_items_item

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
