from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="ConnectorProbeRequest")


@_attrs_define
class ConnectorProbeRequest:
    """Body for `POST /tenants/{tenant_id}/connectors/check` and
    `POST /tenants/{tenant_id}/connectors/discover` (CLOACINA-T-1002): one side
    of a sync, as a sync definition gives it. A side names a compiled
    connector by `provider` and `member`, or a connector definition by
    `definition`.

        Attributes:
            side (str): `source` or `destination`. Discover reads a source.
            config (Any | Unset): The connector's config, or a definition's user config.
            credential (Any | Unset): The side's credential, a credential spec. A definition supplies its own.
            definition (None | str | Unset):
            grants (Any | Unset): The side's capability grants. A definition supplies its own.
            member (None | str | Unset):
            provider (None | str | Unset):
            secret (None | str | Unset): The stored secret a definition's auth reads.
            sync (None | str | Unset): The sync being edited, if any: a config value that is the placeholder
                for a stored secret takes that sync's stored value (CLOACINA-T-1008).
    """

    side: str
    config: Any | Unset = UNSET
    credential: Any | Unset = UNSET
    definition: None | str | Unset = UNSET
    grants: Any | Unset = UNSET
    member: None | str | Unset = UNSET
    provider: None | str | Unset = UNSET
    secret: None | str | Unset = UNSET
    sync: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        side = self.side

        config = self.config

        credential = self.credential

        definition: None | str | Unset
        if isinstance(self.definition, Unset):
            definition = UNSET
        else:
            definition = self.definition

        grants = self.grants

        member: None | str | Unset
        if isinstance(self.member, Unset):
            member = UNSET
        else:
            member = self.member

        provider: None | str | Unset
        if isinstance(self.provider, Unset):
            provider = UNSET
        else:
            provider = self.provider

        secret: None | str | Unset
        if isinstance(self.secret, Unset):
            secret = UNSET
        else:
            secret = self.secret

        sync: None | str | Unset
        if isinstance(self.sync, Unset):
            sync = UNSET
        else:
            sync = self.sync

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "side": side,
            }
        )
        if config is not UNSET:
            field_dict["config"] = config
        if credential is not UNSET:
            field_dict["credential"] = credential
        if definition is not UNSET:
            field_dict["definition"] = definition
        if grants is not UNSET:
            field_dict["grants"] = grants
        if member is not UNSET:
            field_dict["member"] = member
        if provider is not UNSET:
            field_dict["provider"] = provider
        if secret is not UNSET:
            field_dict["secret"] = secret
        if sync is not UNSET:
            field_dict["sync"] = sync

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        side = d.pop("side")

        config = d.pop("config", UNSET)

        credential = d.pop("credential", UNSET)

        def _parse_definition(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        definition = _parse_definition(d.pop("definition", UNSET))

        grants = d.pop("grants", UNSET)

        def _parse_member(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        member = _parse_member(d.pop("member", UNSET))

        def _parse_provider(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        provider = _parse_provider(d.pop("provider", UNSET))

        def _parse_secret(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        secret = _parse_secret(d.pop("secret", UNSET))

        def _parse_sync(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        sync = _parse_sync(d.pop("sync", UNSET))

        connector_probe_request = cls(
            side=side,
            config=config,
            credential=credential,
            definition=definition,
            grants=grants,
            member=member,
            provider=provider,
            secret=secret,
            sync=sync,
        )

        connector_probe_request.additional_properties = d
        return connector_probe_request

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
