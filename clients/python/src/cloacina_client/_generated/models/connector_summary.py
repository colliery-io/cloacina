from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="ConnectorSummary")


@_attrs_define
class ConnectorSummary:
    """One connector a server can run: compiled (a provider package) or
    declarative (a connector definition on the REST runtime).

        Attributes:
            kind (str): `compiled` or `declarative`.
            name (str): The name a sync names it by: `*_provider` for a compiled connector,
                `*_definition` for a declarative one.
            provider (str): The provider package that runs it.
            roles (list[str]): `Source`, `Destination`.
            streams (list[str]): A definition's streams, or a destination's objects. Empty for a
                compiled connector, whose streams are what `discover` finds.
            sync_modes (list[str]):
            version (None | str | Unset):
    """

    kind: str
    name: str
    provider: str
    roles: list[str]
    streams: list[str]
    sync_modes: list[str]
    version: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        kind = self.kind

        name = self.name

        provider = self.provider

        roles = self.roles

        streams = self.streams

        sync_modes = self.sync_modes

        version: None | str | Unset
        if isinstance(self.version, Unset):
            version = UNSET
        else:
            version = self.version

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "kind": kind,
                "name": name,
                "provider": provider,
                "roles": roles,
                "streams": streams,
                "sync_modes": sync_modes,
            }
        )
        if version is not UNSET:
            field_dict["version"] = version

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        kind = d.pop("kind")

        name = d.pop("name")

        provider = d.pop("provider")

        roles = cast(list[str], d.pop("roles"))

        streams = cast(list[str], d.pop("streams"))

        sync_modes = cast(list[str], d.pop("sync_modes"))

        def _parse_version(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        version = _parse_version(d.pop("version", UNSET))

        connector_summary = cls(
            kind=kind,
            name=name,
            provider=provider,
            roles=roles,
            streams=streams,
            sync_modes=sync_modes,
            version=version,
        )

        connector_summary.additional_properties = d
        return connector_summary

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
