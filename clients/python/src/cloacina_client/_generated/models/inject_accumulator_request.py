from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="InjectAccumulatorRequest")


@_attrs_define
class InjectAccumulatorRequest:
    """Request body for `POST /v1/health/accumulators/{name}/inject` (CLOACI-T-0753)
    — push a single typed event into a running accumulator, the operator-facing
    REST analogue of the WS accumulator-push path. The JSON `event` is serialized
    to the boundary wire encoding server-side, so operators never craft raw
    `Vec<u8>`.

        Attributes:
            event (Any): The event payload (any JSON) to push to the accumulator.
    """

    event: Any
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        event = self.event

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "event": event,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        event = d.pop("event")

        inject_accumulator_request = cls(
            event=event,
        )

        inject_accumulator_request.additional_properties = d
        return inject_accumulator_request

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
