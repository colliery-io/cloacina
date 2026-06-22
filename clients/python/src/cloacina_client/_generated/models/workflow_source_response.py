from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.workflow_source_file import WorkflowSourceFile


T = TypeVar("T", bound="WorkflowSourceResponse")


@_attrs_define
class WorkflowSourceResponse:
    """`GET /tenants/{tenant_id}/workflows/{name}/source` response — the original
    source retained in the package's `.cloacina` archive, surfaced read-only
    (CLOACI-T-0750). The source is independent of build state, so it is
    available even for packages that are still building or failed to build.

        Attributes:
            files (list[WorkflowSourceFile]): Source files in the package, sorted by path. Binary and oversized files
                are omitted.
            id (str): Package UUID.
            package_name (str):
            tenant_id (str):
            version (str):
            workflow_name (str): Executable workflow name (see `WorkflowSummary::workflow_name`).
    """

    files: list[WorkflowSourceFile]
    id: str
    package_name: str
    tenant_id: str
    version: str
    workflow_name: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        files = []
        for files_item_data in self.files:
            files_item = files_item_data.to_dict()
            files.append(files_item)

        id = self.id

        package_name = self.package_name

        tenant_id = self.tenant_id

        version = self.version

        workflow_name = self.workflow_name

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "files": files,
                "id": id,
                "package_name": package_name,
                "tenant_id": tenant_id,
                "version": version,
                "workflow_name": workflow_name,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.workflow_source_file import WorkflowSourceFile

        d = dict(src_dict)
        files = []
        _files = d.pop("files")
        for files_item_data in _files:
            files_item = WorkflowSourceFile.from_dict(files_item_data)

            files.append(files_item)

        id = d.pop("id")

        package_name = d.pop("package_name")

        tenant_id = d.pop("tenant_id")

        version = d.pop("version")

        workflow_name = d.pop("workflow_name")

        workflow_source_response = cls(
            files=files,
            id=id,
            package_name=package_name,
            tenant_id=tenant_id,
            version=version,
            workflow_name=workflow_name,
        )

        workflow_source_response.additional_properties = d
        return workflow_source_response

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
