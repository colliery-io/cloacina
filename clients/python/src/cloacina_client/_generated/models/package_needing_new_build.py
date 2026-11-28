from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="PackageNeedingNewBuild")


@_attrs_define
class PackageNeedingNewBuild:
    """One package that needs a new publish, in the list across tenants
    (`GET /v1/packages/needs-new-build`, platform admin, CLOACINA-T-1085).

        Attributes:
            current (int): The plugin interface version this release loads.
            id (str): Package UUID.
            package_name (str):
            reason (str): The reason and the step to take.
            tenant_id (str):
            version (str):
            built_for (int | None | Unset): The plugin interface version the package was built for. Absent when
                it is not known.
    """

    current: int
    id: str
    package_name: str
    reason: str
    tenant_id: str
    version: str
    built_for: int | None | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        current = self.current

        id = self.id

        package_name = self.package_name

        reason = self.reason

        tenant_id = self.tenant_id

        version = self.version

        built_for: int | None | Unset
        if isinstance(self.built_for, Unset):
            built_for = UNSET
        else:
            built_for = self.built_for

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "current": current,
                "id": id,
                "package_name": package_name,
                "reason": reason,
                "tenant_id": tenant_id,
                "version": version,
            }
        )
        if built_for is not UNSET:
            field_dict["built_for"] = built_for

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        current = d.pop("current")

        id = d.pop("id")

        package_name = d.pop("package_name")

        reason = d.pop("reason")

        tenant_id = d.pop("tenant_id")

        version = d.pop("version")

        def _parse_built_for(data: object) -> int | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(int | None | Unset, data)

        built_for = _parse_built_for(d.pop("built_for", UNSET))

        package_needing_new_build = cls(
            current=current,
            id=id,
            package_name=package_name,
            reason=reason,
            tenant_id=tenant_id,
            version=version,
            built_for=built_for,
        )

        package_needing_new_build.additional_properties = d
        return package_needing_new_build

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
