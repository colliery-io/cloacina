from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="InjectAccumulatorResponse")


@_attrs_define
class InjectAccumulatorResponse:
    """Response body for a successful accumulator inject (CLOACI-T-0753).

    Attributes:
        accumulator (str): Echoes the accumulator name the event was pushed to.
        delivered (int): Number of receivers the event was delivered to.
    """

    accumulator: str
    delivered: int
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        accumulator = self.accumulator

        delivered = self.delivered

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "accumulator": accumulator,
                "delivered": delivered,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        accumulator = d.pop("accumulator")

        delivered = d.pop("delivered")

        inject_accumulator_response = cls(
            accumulator=accumulator,
            delivered=delivered,
        )

        inject_accumulator_response.additional_properties = d
        return inject_accumulator_response

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
