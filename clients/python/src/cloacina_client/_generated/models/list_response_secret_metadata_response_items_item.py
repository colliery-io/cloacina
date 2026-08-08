from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from typing_extensions import Self

T = TypeVar("T", bound="ListResponseSecretMetadataResponseItemsItem")


@_attrs_define
class ListResponseSecretMetadataResponseItemsItem:
    """Metadata view of a secret — the ONLY shape a read returns. Carries names +
    timestamps; **never** a plaintext or ciphertext value.

        Attributes:
            created_at (str): RFC 3339 timestamp.
            field_names (list[str]): The declared field names (no values).
            id (str): Secret UUID.
            name (str): The secret's name.
            updated_at (str): RFC 3339 timestamp.
    """

    created_at: str
    field_names: list[str]
    id: str
    name: str
    updated_at: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        created_at = self.created_at

        field_names = self.field_names

        id = self.id

        name = self.name

        updated_at = self.updated_at

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "created_at": created_at,
                "field_names": field_names,
                "id": id,
                "name": name,
                "updated_at": updated_at,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        created_at = d.pop("created_at")

        field_names = cast(list[str], d.pop("field_names"))

        id = d.pop("id")

        name = d.pop("name")

        updated_at = d.pop("updated_at")

        list_response_secret_metadata_response_items_item = cls(
            created_at=created_at,
            field_names=field_names,
            id=id,
            name=name,
            updated_at=updated_at,
        )

        list_response_secret_metadata_response_items_item.additional_properties = d
        return list_response_secret_metadata_response_items_item

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
