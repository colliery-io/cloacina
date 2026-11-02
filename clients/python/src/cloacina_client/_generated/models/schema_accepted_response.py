from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="SchemaAcceptedResponse")


@_attrs_define
class SchemaAcceptedResponse:
    """`POST /tenants/{tenant_id}/syncs/{name}/schema/accept` response.

    Attributes:
        accepted (bool):
        name (str):
        stream (str):
    """

    accepted: bool
    name: str
    stream: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        accepted = self.accepted

        name = self.name

        stream = self.stream

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "accepted": accepted,
                "name": name,
                "stream": stream,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        accepted = d.pop("accepted")

        name = d.pop("name")

        stream = d.pop("stream")

        schema_accepted_response = cls(
            accepted=accepted,
            name=name,
            stream=stream,
        )

        schema_accepted_response.additional_properties = d
        return schema_accepted_response

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
