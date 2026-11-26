from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Self, TypeVar

from attrs import define as _attrs_define

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.agent_egress_port import AgentEgressPort


T = TypeVar("T", bound="AgentEgressRule")


@_attrs_define
class AgentEgressRule:
    """One extra egress rule: a CIDR block the agent pods may reach, on the listed
    ports. No ports means every port.

        Attributes:
            cidr (str): IPv4 or IPv6 CIDR block, with no host bits set (`198.51.100.0/24`).
            ports (list[AgentEgressPort] | Unset): Ports the rule allows. Empty: every port.
    """

    cidr: str
    ports: list[AgentEgressPort] | Unset = UNSET

    def to_dict(self) -> dict[str, Any]:
        cidr = self.cidr

        ports: list[dict[str, Any]] | Unset = UNSET
        if not isinstance(self.ports, Unset):
            ports = []
            for ports_item_data in self.ports:
                ports_item = ports_item_data.to_dict()
                ports.append(ports_item)

        field_dict: dict[str, Any] = {}

        field_dict.update(
            {
                "cidr": cidr,
            }
        )
        if ports is not UNSET:
            field_dict["ports"] = ports

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        from ..models.agent_egress_port import AgentEgressPort

        d = dict(src_dict)
        cidr = d.pop("cidr")

        _ports = d.pop("ports", UNSET)
        ports: list[AgentEgressPort] | Unset = UNSET
        if _ports is not UNSET:
            ports = []
            for ports_item_data in _ports:
                ports_item = AgentEgressPort.from_dict(ports_item_data)

                ports.append(ports_item)

        agent_egress_rule = cls(
            cidr=cidr,
            ports=ports,
        )

        return agent_egress_rule
