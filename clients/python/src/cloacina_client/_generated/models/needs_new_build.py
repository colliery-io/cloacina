from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="NeedsNewBuild")


@_attrs_define
class NeedsNewBuild:
    """Why a package needs a new publish (CLOACINA-T-1085).

    After a minor release that changes the plugin interface, a Rust package
    built for the old interface does not run on the new agents. The compiler
    rebuilds it once from its kept source; when the package pins the old
    minor (in Cargo.toml or Cargo.lock), the rebuild has the old interface
    again. The team changes the `cloacina` pin to the new minor and publishes
    again. The package stays in service until then: agents of the old
    interface still run it during a roll.

        Attributes:
            current (int): The plugin interface version this release loads.
            reason (str): The reason and the step to take.
            built_for (int | None | Unset): The plugin interface version the package was built for. Absent when
                it is not known: a library of another fidius ABI does not open.
    """

    current: int
    reason: str
    built_for: int | None | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        current = self.current

        reason = self.reason

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
                "reason": reason,
            }
        )
        if built_for is not UNSET:
            field_dict["built_for"] = built_for

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        current = d.pop("current")

        reason = d.pop("reason")

        def _parse_built_for(data: object) -> int | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(int | None | Unset, data)

        built_for = _parse_built_for(d.pop("built_for", UNSET))

        needs_new_build = cls(
            current=current,
            reason=reason,
            built_for=built_for,
        )

        needs_new_build.additional_properties = d
        return needs_new_build

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
