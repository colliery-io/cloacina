from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define

from ..types import UNSET, Unset

T = TypeVar("T", bound="AgentEgressPort")


@_attrs_define
class AgentEgressPort:
    """One port of an [`AgentEgressRule`].

    Attributes:
        port (int): Port number, 1 to 65535.
        protocol (None | str | Unset): `TCP` (the default), `UDP` or `SCTP`.
    """

    port: int
    protocol: None | str | Unset = UNSET

    def to_dict(self) -> dict[str, Any]:
        port = self.port

        protocol: None | str | Unset
        if isinstance(self.protocol, Unset):
            protocol = UNSET
        else:
            protocol = self.protocol

        field_dict: dict[str, Any] = {}

        field_dict.update(
            {
                "port": port,
            }
        )
        if protocol is not UNSET:
            field_dict["protocol"] = protocol

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        port = d.pop("port")

        def _parse_protocol(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        protocol = _parse_protocol(d.pop("protocol", UNSET))

        agent_egress_port = cls(
            port=port,
            protocol=protocol,
        )

        return agent_egress_port
