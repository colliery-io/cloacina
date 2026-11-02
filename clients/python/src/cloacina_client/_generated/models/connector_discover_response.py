from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Self, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.discovered_stream import DiscoveredStream


T = TypeVar("T", bound="ConnectorDiscoverResponse")


@_attrs_define
class ConnectorDiscoverResponse:
    """What a source's `discover` found.

    Attributes:
        streams (list[DiscoveredStream]):
    """

    streams: list[DiscoveredStream]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        streams = []
        for streams_item_data in self.streams:
            streams_item = streams_item_data.to_dict()
            streams.append(streams_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "streams": streams,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        from ..models.discovered_stream import DiscoveredStream

        d = dict(src_dict)
        streams = []
        _streams = d.pop("streams")
        for streams_item_data in _streams:
            streams_item = DiscoveredStream.from_dict(streams_item_data)

            streams.append(streams_item)

        connector_discover_response = cls(
            streams=streams,
        )

        connector_discover_response.additional_properties = d
        return connector_discover_response

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
