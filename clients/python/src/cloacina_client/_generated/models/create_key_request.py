from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.key_role import KeyRole

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
    """

    name: str
    role: KeyRole
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        name = self.name

        role = self.role.value

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "name": name,
                "role": role,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        name = d.pop("name")

        role = KeyRole(d.pop("role"))

        create_key_request = cls(
            name=name,
            role=role,
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
