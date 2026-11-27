from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="TrustedKeySummary")


@_attrs_define
class TrustedKeySummary:
    """One trusted key of a tenant. The body of the add, and one row of the list.

    Attributes:
        fingerprint (str): SHA-256 of the public key, as hexadecimal. A signature names its
            signer by this value.
        id (str): Key UUID: the id a revoke takes.
        revoked (bool): True when the key was revoked: no upload passes with it.
        trusted_at (str): RFC 3339 timestamp.
        name (None | str | Unset): The name given at the add, if any.
        revoked_at (None | str | Unset): RFC 3339 timestamp of the revoke; `null` for a live key.
    """

    fingerprint: str
    id: str
    revoked: bool
    trusted_at: str
    name: None | str | Unset = UNSET
    revoked_at: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        fingerprint = self.fingerprint

        id = self.id

        revoked = self.revoked

        trusted_at = self.trusted_at

        name: None | str | Unset
        if isinstance(self.name, Unset):
            name = UNSET
        else:
            name = self.name

        revoked_at: None | str | Unset
        if isinstance(self.revoked_at, Unset):
            revoked_at = UNSET
        else:
            revoked_at = self.revoked_at

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "fingerprint": fingerprint,
                "id": id,
                "revoked": revoked,
                "trusted_at": trusted_at,
            }
        )
        if name is not UNSET:
            field_dict["name"] = name
        if revoked_at is not UNSET:
            field_dict["revoked_at"] = revoked_at

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        fingerprint = d.pop("fingerprint")

        id = d.pop("id")

        revoked = d.pop("revoked")

        trusted_at = d.pop("trusted_at")

        def _parse_name(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        name = _parse_name(d.pop("name", UNSET))

        def _parse_revoked_at(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        revoked_at = _parse_revoked_at(d.pop("revoked_at", UNSET))

        trusted_key_summary = cls(
            fingerprint=fingerprint,
            id=id,
            revoked=revoked,
            trusted_at=trusted_at,
            name=name,
            revoked_at=revoked_at,
        )

        trusted_key_summary.additional_properties = d
        return trusted_key_summary

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
