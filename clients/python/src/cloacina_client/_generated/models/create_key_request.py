from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.key_role import KeyRole
from ..types import UNSET, Unset

T = TypeVar("T", bound="CreateKeyRequest")


@_attrs_define
class CreateKeyRequest:
    """Request body for `POST /auth/keys` and `POST /tenants/{tenant_id}/keys`.

    Attributes:
        name (str):
        role (KeyRole): The role of an API key: a named set of permissions, fixed in the server.

            - `read`: list and inspect.
            - `write`: `read`, plus upload, run and operate.
            - `admin`: `write`, plus keys, accounts, secrets, the audit record and the
              fleet size of its tenant.
            - `agent`: act as an execution agent, and nothing else.
        expires_at (None | str | Unset): Optional. When the key expires, as an RFC 3339 timestamp in the
            future (for example `2027-01-31T00:00:00Z`). After it the key is
            refused with 401 `key_expired`. Not set: the key does not expire.
        platform_admin (bool | Unset): `POST /auth/keys` only: make a platform admin key (no tenant, role
            `admin`, the platform admin flag). Needs `role: admin`. The tenant
            route refuses `true` with 400 (CLOACINA-T-1065).
    """

    name: str
    role: KeyRole
    expires_at: None | str | Unset = UNSET
    platform_admin: bool | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        name = self.name

        role = self.role.value

        expires_at: None | str | Unset
        if isinstance(self.expires_at, Unset):
            expires_at = UNSET
        else:
            expires_at = self.expires_at

        platform_admin = self.platform_admin

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "name": name,
                "role": role,
            }
        )
        if expires_at is not UNSET:
            field_dict["expires_at"] = expires_at
        if platform_admin is not UNSET:
            field_dict["platform_admin"] = platform_admin

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        name = d.pop("name")

        role = KeyRole(d.pop("role"))

        def _parse_expires_at(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        expires_at = _parse_expires_at(d.pop("expires_at", UNSET))

        platform_admin = d.pop("platform_admin", UNSET)

        create_key_request = cls(
            name=name,
            role=role,
            expires_at=expires_at,
            platform_admin=platform_admin,
        )

        create_key_request.additional_properties = d
        return create_key_request

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
