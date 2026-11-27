from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="WorkflowVersionSummary")


@_attrs_define
class WorkflowVersionSummary:
    """One stored version of a package, in
    `GET /tenants/{tenant_id}/workflows/{name}/versions` (CLOACINA-T-1018).

        Attributes:
            active (bool): `true` for the version in service: built and not superseded. At most
                one version of a package is active.
            build_status (str): `pending`, `building`, `success` or `failed`.
            content_hash (str): SHA-256 of the uploaded archive.
            created_at (str): Upload time (RFC 3339).
            id (str): Package UUID of this version.
            package_name (str):
            superseded (bool): `true` once another version replaced this one, or a newer upload
                replaced it before it was built.
            version (str):
            build_error (None | str | Unset):
            compiled_at (None | str | Unset): Time the build succeeded (RFC 3339).
            describe_error (None | str | Unset): Why the compiler could not write the metadata document of this built
                version (CLOACINA-T-1074). Absent when the version is described, or
                has not been tried yet.
    """

    active: bool
    build_status: str
    content_hash: str
    created_at: str
    id: str
    package_name: str
    superseded: bool
    version: str
    build_error: None | str | Unset = UNSET
    compiled_at: None | str | Unset = UNSET
    describe_error: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        active = self.active

        build_status = self.build_status

        content_hash = self.content_hash

        created_at = self.created_at

        id = self.id

        package_name = self.package_name

        superseded = self.superseded

        version = self.version

        build_error: None | str | Unset
        if isinstance(self.build_error, Unset):
            build_error = UNSET
        else:
            build_error = self.build_error

        compiled_at: None | str | Unset
        if isinstance(self.compiled_at, Unset):
            compiled_at = UNSET
        else:
            compiled_at = self.compiled_at

        describe_error: None | str | Unset
        if isinstance(self.describe_error, Unset):
            describe_error = UNSET
        else:
            describe_error = self.describe_error

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "active": active,
                "build_status": build_status,
                "content_hash": content_hash,
                "created_at": created_at,
                "id": id,
                "package_name": package_name,
                "superseded": superseded,
                "version": version,
            }
        )
        if build_error is not UNSET:
            field_dict["build_error"] = build_error
        if compiled_at is not UNSET:
            field_dict["compiled_at"] = compiled_at
        if describe_error is not UNSET:
            field_dict["describe_error"] = describe_error

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        active = d.pop("active")

        build_status = d.pop("build_status")

        content_hash = d.pop("content_hash")

        created_at = d.pop("created_at")

        id = d.pop("id")

        package_name = d.pop("package_name")

        superseded = d.pop("superseded")

        version = d.pop("version")

        def _parse_build_error(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        build_error = _parse_build_error(d.pop("build_error", UNSET))

        def _parse_compiled_at(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        compiled_at = _parse_compiled_at(d.pop("compiled_at", UNSET))

        def _parse_describe_error(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        describe_error = _parse_describe_error(d.pop("describe_error", UNSET))

        workflow_version_summary = cls(
            active=active,
            build_status=build_status,
            content_hash=content_hash,
            created_at=created_at,
            id=id,
            package_name=package_name,
            superseded=superseded,
            version=version,
            build_error=build_error,
            compiled_at=compiled_at,
            describe_error=describe_error,
        )

        workflow_version_summary.additional_properties = d
        return workflow_version_summary

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
