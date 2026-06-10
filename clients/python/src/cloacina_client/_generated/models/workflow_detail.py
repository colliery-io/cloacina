from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="WorkflowDetail")


@_attrs_define
class WorkflowDetail:
    """`GET /tenants/{tenant_id}/workflows/{name}` response — summary fields
    plus real build state (pending/building/failed/success).

        Attributes:
            build_status (str):
            created_at (str): RFC 3339 timestamp.
            id (str): Package UUID.
            package_name (str):
            tasks (list[str]): Task IDs included in this package.
            tenant_id (str):
            version (str):
            build_error (None | str | Unset):
            description (None | str | Unset):
    """

    build_status: str
    created_at: str
    id: str
    package_name: str
    tasks: list[str]
    tenant_id: str
    version: str
    build_error: None | str | Unset = UNSET
    description: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        build_status = self.build_status

        created_at = self.created_at

        id = self.id

        package_name = self.package_name

        tasks = self.tasks

        tenant_id = self.tenant_id

        version = self.version

        build_error: None | str | Unset
        if isinstance(self.build_error, Unset):
            build_error = UNSET
        else:
            build_error = self.build_error

        description: None | str | Unset
        if isinstance(self.description, Unset):
            description = UNSET
        else:
            description = self.description

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "build_status": build_status,
                "created_at": created_at,
                "id": id,
                "package_name": package_name,
                "tasks": tasks,
                "tenant_id": tenant_id,
                "version": version,
            }
        )
        if build_error is not UNSET:
            field_dict["build_error"] = build_error
        if description is not UNSET:
            field_dict["description"] = description

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        build_status = d.pop("build_status")

        created_at = d.pop("created_at")

        id = d.pop("id")

        package_name = d.pop("package_name")

        tasks = cast(list[str], d.pop("tasks"))

        tenant_id = d.pop("tenant_id")

        version = d.pop("version")

        def _parse_build_error(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        build_error = _parse_build_error(d.pop("build_error", UNSET))

        def _parse_description(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        description = _parse_description(d.pop("description", UNSET))

        workflow_detail = cls(
            build_status=build_status,
            created_at=created_at,
            id=id,
            package_name=package_name,
            tasks=tasks,
            tenant_id=tenant_id,
            version=version,
            build_error=build_error,
            description=description,
        )

        workflow_detail.additional_properties = d
        return workflow_detail

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
