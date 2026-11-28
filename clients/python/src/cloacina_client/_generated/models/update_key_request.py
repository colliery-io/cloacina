from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="UpdateKeyRequest")


@_attrs_define
class UpdateKeyRequest:
    """Request body for `PATCH /tenants/{tenant_id}/keys/{key_id}`
    (CLOACINA-T-1123): the highest priority the key may set.

        Attributes:
            max_priority (int | None | Unset): 0 (the highest) to 10; `null` clears it, and the key has the default
                of its role again. Not better than the caller's own limit (403);
                refused on a `read` or `agent` key (400).
    """

    max_priority: int | None | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        max_priority: int | None | Unset
        if isinstance(self.max_priority, Unset):
            max_priority = UNSET
        else:
            max_priority = self.max_priority

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if max_priority is not UNSET:
            field_dict["max_priority"] = max_priority

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)

        def _parse_max_priority(data: object) -> int | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(int | None | Unset, data)

        max_priority = _parse_max_priority(d.pop("max_priority", UNSET))

        update_key_request = cls(
            max_priority=max_priority,
        )

        update_key_request.additional_properties = d
        return update_key_request

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
