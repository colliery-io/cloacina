from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.needs_new_build import NeedsNewBuild


T = TypeVar("T", bound="WorkflowSummary")


@_attrs_define
class WorkflowSummary:
    """One row in the workflow list (`GET /tenants/{tenant_id}/workflows`).

    Attributes:
        created_at (str): RFC 3339 timestamp.
        id (str): Package UUID.
        package_name (str):
        tasks (list[str]): Task IDs included in this package.
        version (str):
        workflow_name (str): Executable workflow name (the identifier to execute by). Differs from
            `package_name` under the standard convention (package `demo-slow-rust`
            → workflow `demo_slow_workflow`). Falls back to `package_name` for
            packages predating workflow-name persistence. (CLOACI-T-0671)
        description (None | str | Unset):
        needs_new_build (NeedsNewBuild | None | Unset):
        paused (bool | Unset): Whether this workflow is paused (CLOACI-T-0749). Paused workflows refuse
            new executions until resumed.
        plugin_interface_version (int | None | Unset): The plugin interface version of the package's Rust build
            (CLOACINA-T-1085). Absent for a Python package and for a build whose
            interface is not recorded yet.
        uploaded_by (None | str | Unset): Who uploaded the package (CLOACINA-T-1024): the login identity, or
            the name of the API key. Read from the audit record; `null` for a
            package uploaded before the audit record existed.
    """

    created_at: str
    id: str
    package_name: str
    tasks: list[str]
    version: str
    workflow_name: str
    description: None | str | Unset = UNSET
    needs_new_build: NeedsNewBuild | None | Unset = UNSET
    paused: bool | Unset = UNSET
    plugin_interface_version: int | None | Unset = UNSET
    uploaded_by: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.needs_new_build import NeedsNewBuild

        created_at = self.created_at

        id = self.id

        package_name = self.package_name

        tasks = self.tasks

        version = self.version

        workflow_name = self.workflow_name

        description: None | str | Unset
        if isinstance(self.description, Unset):
            description = UNSET
        else:
            description = self.description

        needs_new_build: dict[str, Any] | None | Unset
        if isinstance(self.needs_new_build, Unset):
            needs_new_build = UNSET
        elif isinstance(self.needs_new_build, NeedsNewBuild):
            needs_new_build = self.needs_new_build.to_dict()
        else:
            needs_new_build = self.needs_new_build

        paused = self.paused

        plugin_interface_version: int | None | Unset
        if isinstance(self.plugin_interface_version, Unset):
            plugin_interface_version = UNSET
        else:
            plugin_interface_version = self.plugin_interface_version

        uploaded_by: None | str | Unset
        if isinstance(self.uploaded_by, Unset):
            uploaded_by = UNSET
        else:
            uploaded_by = self.uploaded_by

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "created_at": created_at,
                "id": id,
                "package_name": package_name,
                "tasks": tasks,
                "version": version,
                "workflow_name": workflow_name,
            }
        )
        if description is not UNSET:
            field_dict["description"] = description
        if needs_new_build is not UNSET:
            field_dict["needs_new_build"] = needs_new_build
        if paused is not UNSET:
            field_dict["paused"] = paused
        if plugin_interface_version is not UNSET:
            field_dict["plugin_interface_version"] = plugin_interface_version
        if uploaded_by is not UNSET:
            field_dict["uploaded_by"] = uploaded_by

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        from ..models.needs_new_build import NeedsNewBuild

        d = dict(src_dict)
        created_at = d.pop("created_at")

        id = d.pop("id")

        package_name = d.pop("package_name")

        tasks = cast(list[str], d.pop("tasks"))

        version = d.pop("version")

        workflow_name = d.pop("workflow_name")

        def _parse_description(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        description = _parse_description(d.pop("description", UNSET))

        def _parse_needs_new_build(data: object) -> NeedsNewBuild | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                needs_new_build_type_0 = NeedsNewBuild.from_dict(data)

                return needs_new_build_type_0
            except (TypeError, ValueError, AttributeError, KeyError):
                pass
            return cast(NeedsNewBuild | None | Unset, data)

        needs_new_build = _parse_needs_new_build(d.pop("needs_new_build", UNSET))

        paused = d.pop("paused", UNSET)

        def _parse_plugin_interface_version(data: object) -> int | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(int | None | Unset, data)

        plugin_interface_version = _parse_plugin_interface_version(
            d.pop("plugin_interface_version", UNSET)
        )

        def _parse_uploaded_by(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        uploaded_by = _parse_uploaded_by(d.pop("uploaded_by", UNSET))

        workflow_summary = cls(
            created_at=created_at,
            id=id,
            package_name=package_name,
            tasks=tasks,
            version=version,
            workflow_name=workflow_name,
            description=description,
            needs_new_build=needs_new_build,
            paused=paused,
            plugin_interface_version=plugin_interface_version,
            uploaded_by=uploaded_by,
        )

        workflow_summary.additional_properties = d
        return workflow_summary

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
