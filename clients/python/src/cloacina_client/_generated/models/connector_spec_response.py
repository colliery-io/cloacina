from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="ConnectorSpecResponse")


@_attrs_define
class ConnectorSpecResponse:
    """What a compiled connector says about itself (CLOACINA-T-1006): its config
    schema, roles and sync modes, from its `spec()`.

        Attributes:
            config_schema (Any): The JSON Schema of the connector's config.
            connector_version (str):
            name (str):
            roles (list[str]):
            sync_modes (list[str]):
    """

    config_schema: Any
    connector_version: str
    name: str
    roles: list[str]
    sync_modes: list[str]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        config_schema = self.config_schema

        connector_version = self.connector_version

        name = self.name

        roles = self.roles

        sync_modes = self.sync_modes

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "config_schema": config_schema,
                "connector_version": connector_version,
                "name": name,
                "roles": roles,
                "sync_modes": sync_modes,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        config_schema = d.pop("config_schema")

        connector_version = d.pop("connector_version")

        name = d.pop("name")

        roles = cast(list[str], d.pop("roles"))

        sync_modes = cast(list[str], d.pop("sync_modes"))

        connector_spec_response = cls(
            config_schema=config_schema,
            connector_version=connector_version,
            name=name,
            roles=roles,
            sync_modes=sync_modes,
        )

        connector_spec_response.additional_properties = d
        return connector_spec_response

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
