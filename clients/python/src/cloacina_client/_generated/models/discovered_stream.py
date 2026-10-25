from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.discovered_field import DiscoveredField


T = TypeVar("T", bound="DiscoveredStream")


@_attrs_define
class DiscoveredStream:
    """A stream a source offers.

    Attributes:
        name (str):
        sync_modes (list[str]): `FullRefresh`, `Incremental` or `Cdc`.
        default_cursor_field (None | str | Unset):
        fields (list[DiscoveredField] | Unset): The stream's typed fields, when the source declares them. A compiled
            connector's are empty for now.
        namespace (None | str | Unset):
        primary_key (list[str] | None | Unset):
    """

    name: str
    sync_modes: list[str]
    default_cursor_field: None | str | Unset = UNSET
    fields: list[DiscoveredField] | Unset = UNSET
    namespace: None | str | Unset = UNSET
    primary_key: list[str] | None | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        name = self.name

        sync_modes = self.sync_modes

        default_cursor_field: None | str | Unset
        if isinstance(self.default_cursor_field, Unset):
            default_cursor_field = UNSET
        else:
            default_cursor_field = self.default_cursor_field

        fields: list[dict[str, Any]] | Unset = UNSET
        if not isinstance(self.fields, Unset):
            fields = []
            for fields_item_data in self.fields:
                fields_item = fields_item_data.to_dict()
                fields.append(fields_item)

        namespace: None | str | Unset
        if isinstance(self.namespace, Unset):
            namespace = UNSET
        else:
            namespace = self.namespace

        primary_key: list[str] | None | Unset
        if isinstance(self.primary_key, Unset):
            primary_key = UNSET
        elif isinstance(self.primary_key, list):
            primary_key = self.primary_key

        else:
            primary_key = self.primary_key

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "name": name,
                "sync_modes": sync_modes,
            }
        )
        if default_cursor_field is not UNSET:
            field_dict["default_cursor_field"] = default_cursor_field
        if fields is not UNSET:
            field_dict["fields"] = fields
        if namespace is not UNSET:
            field_dict["namespace"] = namespace
        if primary_key is not UNSET:
            field_dict["primary_key"] = primary_key

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        from ..models.discovered_field import DiscoveredField

        d = dict(src_dict)
        name = d.pop("name")

        sync_modes = cast(list[str], d.pop("sync_modes"))

        def _parse_default_cursor_field(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        default_cursor_field = _parse_default_cursor_field(
            d.pop("default_cursor_field", UNSET)
        )

        _fields = d.pop("fields", UNSET)
        fields: list[DiscoveredField] | Unset = UNSET
        if _fields is not UNSET:
            fields = []
            for fields_item_data in _fields:
                fields_item = DiscoveredField.from_dict(fields_item_data)

                fields.append(fields_item)

        def _parse_namespace(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        namespace = _parse_namespace(d.pop("namespace", UNSET))

        def _parse_primary_key(data: object) -> list[str] | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                primary_key_type_0 = cast(list[str], data)

                return primary_key_type_0
            except (TypeError, ValueError, AttributeError, KeyError):
                pass
            return cast(list[str] | None | Unset, data)

        primary_key = _parse_primary_key(d.pop("primary_key", UNSET))

        discovered_stream = cls(
            name=name,
            sync_modes=sync_modes,
            default_cursor_field=default_cursor_field,
            fields=fields,
            namespace=namespace,
            primary_key=primary_key,
        )

        discovered_stream.additional_properties = d
        return discovered_stream

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
