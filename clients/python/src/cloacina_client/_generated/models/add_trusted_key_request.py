from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="AddTrustedKeyRequest")


@_attrs_define
class AddTrustedKeyRequest:
    """Request body for `POST /v1/tenants/{tenant_id}/trusted-keys`.

    Give the key in exactly one of the two forms.

        Attributes:
            name (None | str | Unset): A name for the key, to tell the keys of the list apart.
            public_key (None | str | Unset): The raw 32-byte Ed25519 public key as 64 hexadecimal characters.
            public_key_pem (None | str | Unset): The public key as PEM (`-----BEGIN PUBLIC KEY-----`, Ed25519
                SubjectPublicKeyInfo).
    """

    name: None | str | Unset = UNSET
    public_key: None | str | Unset = UNSET
    public_key_pem: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        name: None | str | Unset
        if isinstance(self.name, Unset):
            name = UNSET
        else:
            name = self.name

        public_key: None | str | Unset
        if isinstance(self.public_key, Unset):
            public_key = UNSET
        else:
            public_key = self.public_key

        public_key_pem: None | str | Unset
        if isinstance(self.public_key_pem, Unset):
            public_key_pem = UNSET
        else:
            public_key_pem = self.public_key_pem

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if name is not UNSET:
            field_dict["name"] = name
        if public_key is not UNSET:
            field_dict["public_key"] = public_key
        if public_key_pem is not UNSET:
            field_dict["public_key_pem"] = public_key_pem

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)

        def _parse_name(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        name = _parse_name(d.pop("name", UNSET))

        def _parse_public_key(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        public_key = _parse_public_key(d.pop("public_key", UNSET))

        def _parse_public_key_pem(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        public_key_pem = _parse_public_key_pem(d.pop("public_key_pem", UNSET))

        add_trusted_key_request = cls(
            name=name,
            public_key=public_key,
            public_key_pem=public_key_pem,
        )

        add_trusted_key_request.additional_properties = d
        return add_trusted_key_request

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
