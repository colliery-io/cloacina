from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.discovered_field import DiscoveredField


T = TypeVar("T", bound="SyncStream")


@_attrs_define
class SyncStream:
    """A stream of a sync: its committed cursor and its captured schema.

    Attributes:
        exhausted (bool):
        fields (list[DiscoveredField]): The captured schema's fields; empty before the first write, or after
            an accept.
        stream (str):
        broken (None | str | Unset): Why the schema is broken; `null` when it stands.
        cursor (None | str | Unset):
        partition (str | Unset):
        updated_at (None | str | Unset): RFC 3339: when the cursor last moved.
    """

    exhausted: bool
    fields: list[DiscoveredField]
    stream: str
    broken: None | str | Unset = UNSET
    cursor: None | str | Unset = UNSET
    partition: str | Unset = UNSET
    updated_at: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        exhausted = self.exhausted

        fields = []
        for fields_item_data in self.fields:
            fields_item = fields_item_data.to_dict()
            fields.append(fields_item)

        stream = self.stream

        broken: None | str | Unset
        if isinstance(self.broken, Unset):
            broken = UNSET
        else:
            broken = self.broken

        cursor: None | str | Unset
        if isinstance(self.cursor, Unset):
            cursor = UNSET
        else:
            cursor = self.cursor

        partition = self.partition

        updated_at: None | str | Unset
        if isinstance(self.updated_at, Unset):
            updated_at = UNSET
        else:
            updated_at = self.updated_at

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "exhausted": exhausted,
                "fields": fields,
                "stream": stream,
            }
        )
        if broken is not UNSET:
            field_dict["broken"] = broken
        if cursor is not UNSET:
            field_dict["cursor"] = cursor
        if partition is not UNSET:
            field_dict["partition"] = partition
        if updated_at is not UNSET:
            field_dict["updated_at"] = updated_at

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        from ..models.discovered_field import DiscoveredField

        d = dict(src_dict)
        exhausted = d.pop("exhausted")

        fields = []
        _fields = d.pop("fields")
        for fields_item_data in _fields:
            fields_item = DiscoveredField.from_dict(fields_item_data)

            fields.append(fields_item)

        stream = d.pop("stream")

        def _parse_broken(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        broken = _parse_broken(d.pop("broken", UNSET))

        def _parse_cursor(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        cursor = _parse_cursor(d.pop("cursor", UNSET))

        partition = d.pop("partition", UNSET)

        def _parse_updated_at(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        updated_at = _parse_updated_at(d.pop("updated_at", UNSET))

        sync_stream = cls(
            exhausted=exhausted,
            fields=fields,
            stream=stream,
            broken=broken,
            cursor=cursor,
            partition=partition,
            updated_at=updated_at,
        )

        sync_stream.additional_properties = d
        return sync_stream

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
