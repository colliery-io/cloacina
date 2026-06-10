from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="ListResponseKeyInfoItemsItem")


@_attrs_define
class ListResponseKeyInfoItemsItem:
    """One row in the key list (`GET /auth/keys`). No hashes or plaintext.

    Attributes:
        created_at (str): RFC 3339 timestamp.
        id (str): Key UUID.
        is_admin (bool):
        name (str):
        permissions (str): Role string: `read` | `write` | `admin`.
        revoked (bool):
        tenant_id (None | str | Unset): Tenant scope; `null` for global keys.
    """

    created_at: str
    id: str
    is_admin: bool
    name: str
    permissions: str
    revoked: bool
    tenant_id: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        created_at = self.created_at

        id = self.id

        is_admin = self.is_admin

        name = self.name

        permissions = self.permissions

        revoked = self.revoked

        tenant_id: None | str | Unset
        if isinstance(self.tenant_id, Unset):
            tenant_id = UNSET
        else:
            tenant_id = self.tenant_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "created_at": created_at,
                "id": id,
                "is_admin": is_admin,
                "name": name,
                "permissions": permissions,
                "revoked": revoked,
            }
        )
        if tenant_id is not UNSET:
            field_dict["tenant_id"] = tenant_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        created_at = d.pop("created_at")

        id = d.pop("id")

        is_admin = d.pop("is_admin")

        name = d.pop("name")

        permissions = d.pop("permissions")

        revoked = d.pop("revoked")

        def _parse_tenant_id(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        tenant_id = _parse_tenant_id(d.pop("tenant_id", UNSET))

        list_response_key_info_items_item = cls(
            created_at=created_at,
            id=id,
            is_admin=is_admin,
            name=name,
            permissions=permissions,
            revoked=revoked,
            tenant_id=tenant_id,
        )

        list_response_key_info_items_item.additional_properties = d
        return list_response_key_info_items_item

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
