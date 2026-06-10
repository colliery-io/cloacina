from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.list_response_key_info_items_item import ListResponseKeyInfoItemsItem


T = TypeVar("T", bound="ListResponseKeyInfo")


@_attrs_define
class ListResponseKeyInfo:
    """Unified list envelope (CLOACI-T-0594 / API-03): every list endpoint
    returns `{items, total}`. `total` is best-effort — it equals the
    returned page size when the server doesn't run a separate COUNT.

        Attributes:
            items (list[ListResponseKeyInfoItemsItem]):
            total (int):
    """

    items: list[ListResponseKeyInfoItemsItem]
    total: int
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        items = []
        for items_item_data in self.items:
            items_item = items_item_data.to_dict()
            items.append(items_item)

        total = self.total

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "items": items,
                "total": total,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.list_response_key_info_items_item import (
            ListResponseKeyInfoItemsItem,
        )

        d = dict(src_dict)
        items = []
        _items = d.pop("items")
        for items_item_data in _items:
            items_item = ListResponseKeyInfoItemsItem.from_dict(items_item_data)

            items.append(items_item)

        total = d.pop("total")

        list_response_key_info = cls(
            items=items,
            total=total,
        )

        list_response_key_info.additional_properties = d
        return list_response_key_info

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
