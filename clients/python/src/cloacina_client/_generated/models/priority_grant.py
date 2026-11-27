from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="PriorityGrant")


@_attrs_define
class PriorityGrant:
    """A priority that was asked for and the priority that was given.

    `given` is `asked` held to the limit of the principal that asked: the
    highest priority that principal may set. A principal that asks for more
    than it may set is not refused: it gets its limit, and the answer says so.

        Attributes:
            asked (int): The priority that was asked for.
            given (int): The priority the work runs at.
    """

    asked: int
    given: int
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        asked = self.asked

        given = self.given

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "asked": asked,
                "given": given,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        asked = d.pop("asked")

        given = d.pop("given")

        priority_grant = cls(
            asked=asked,
            given=given,
        )

        priority_grant.additional_properties = d
        return priority_grant

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
