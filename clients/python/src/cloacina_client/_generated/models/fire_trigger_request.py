from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="FireTriggerRequest")


@_attrs_define
class FireTriggerRequest:
    """`POST /tenants/{tenant_id}/triggers/{name}/fire` request (CLOACI-T-0777).
    Manually push an event to a trigger; it fans out to every subscribed workflow.

        Attributes:
            event (Any | Unset): Optional typed event merged into each fired workflow's context, validated
                against the trigger's declared params (CLOACI-T-0777 P2). Omit to fire with
                just the trigger metadata.
    """

    event: Any | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        event = self.event

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if event is not UNSET:
            field_dict["event"] = event

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        event = d.pop("event", UNSET)

        fire_trigger_request = cls(
            event=event,
        )

        fire_trigger_request.additional_properties = d
        return fire_trigger_request

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
