from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="WhoamiResponse")


@_attrs_define
class WhoamiResponse:
    """The caller's own identity + role (CLOACI-T-0803) — lets the UI gate
    write/admin controls to the key's role instead of offering actions that
    would 403.

        Attributes:
            is_admin (bool): God-mode flag (cross-tenant platform admin).
            name (str): The key's display name.
            role (str): Role within the tenant: `read` | `write` | `admin`.
            tenant_id (None | str | Unset): Tenant the key is scoped to (`None` = global/public).
    """

    is_admin: bool
    name: str
    role: str
    tenant_id: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        is_admin = self.is_admin

        name = self.name

        role = self.role

        tenant_id: None | str | Unset
        if isinstance(self.tenant_id, Unset):
            tenant_id = UNSET
        else:
            tenant_id = self.tenant_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "is_admin": is_admin,
                "name": name,
                "role": role,
            }
        )
        if tenant_id is not UNSET:
            field_dict["tenant_id"] = tenant_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        is_admin = d.pop("is_admin")

        name = d.pop("name")

        role = d.pop("role")

        def _parse_tenant_id(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        tenant_id = _parse_tenant_id(d.pop("tenant_id", UNSET))

        whoami_response = cls(
            is_admin=is_admin,
            name=name,
            role=role,
            tenant_id=tenant_id,
        )

        whoami_response.additional_properties = d
        return whoami_response

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
