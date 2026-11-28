from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Self, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.agent_egress_port_info import AgentEgressPortInfo


T = TypeVar("T", bound="AgentEgressRuleInfo")


@_attrs_define
class AgentEgressRuleInfo:
    """One extra egress rule, as the pool routes return it. An unknown field is
    ignored (CLOACINA-T-1149).

        Attributes:
            cidr (str): IPv4 or IPv6 CIDR block.
            ports (list[AgentEgressPortInfo] | Unset): Ports the rule allows. Empty: every port.
    """

    cidr: str
    ports: list[AgentEgressPortInfo] | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        cidr = self.cidr

        ports: list[dict[str, Any]] | Unset = UNSET
        if not isinstance(self.ports, Unset):
            ports = []
            for ports_item_data in self.ports:
                ports_item = ports_item_data.to_dict()
                ports.append(ports_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
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
        from ..models.agent_egress_port_info import AgentEgressPortInfo

        d = dict(src_dict)
        cidr = d.pop("cidr")

        _ports = d.pop("ports", UNSET)
        ports: list[AgentEgressPortInfo] | Unset = UNSET
        if _ports is not UNSET:
            ports = []
            for ports_item_data in _ports:
                ports_item = AgentEgressPortInfo.from_dict(ports_item_data)

                ports.append(ports_item)

        agent_egress_rule_info = cls(
            cidr=cidr,
            ports=ports,
        )

        agent_egress_rule_info.additional_properties = d
        return agent_egress_rule_info

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
