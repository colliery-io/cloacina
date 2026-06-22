from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="ListResponseAgentInfoItemsItem")


@_attrs_define
class ListResponseAgentInfoItemsItem:
    """One registered execution agent in the in-memory fleet roster.

    Attributes:
        agent_id (str):
        available_capacity (int):
        capabilities (list[str]):
        in_flight (int):
        max_concurrency (int):
        seconds_since_heartbeat (int): Seconds since this agent's last heartbeat — the liveness signal an
            operator reads (the underlying record stores a monotonic `Instant`,
            not a wall-clock time).
        target_triple (str):
        tenant_id (None | str | Unset): Tenant scope the agent registered under, if any.
    """

    agent_id: str
    available_capacity: int
    capabilities: list[str]
    in_flight: int
    max_concurrency: int
    seconds_since_heartbeat: int
    target_triple: str
    tenant_id: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        agent_id = self.agent_id

        available_capacity = self.available_capacity

        capabilities = self.capabilities

        in_flight = self.in_flight

        max_concurrency = self.max_concurrency

        seconds_since_heartbeat = self.seconds_since_heartbeat

        target_triple = self.target_triple

        tenant_id: None | str | Unset
        if isinstance(self.tenant_id, Unset):
            tenant_id = UNSET
        else:
            tenant_id = self.tenant_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "agent_id": agent_id,
                "available_capacity": available_capacity,
                "capabilities": capabilities,
                "in_flight": in_flight,
                "max_concurrency": max_concurrency,
                "seconds_since_heartbeat": seconds_since_heartbeat,
                "target_triple": target_triple,
            }
        )
        if tenant_id is not UNSET:
            field_dict["tenant_id"] = tenant_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        agent_id = d.pop("agent_id")

        available_capacity = d.pop("available_capacity")

        capabilities = cast(list[str], d.pop("capabilities"))

        in_flight = d.pop("in_flight")

        max_concurrency = d.pop("max_concurrency")

        seconds_since_heartbeat = d.pop("seconds_since_heartbeat")

        target_triple = d.pop("target_triple")

        def _parse_tenant_id(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        tenant_id = _parse_tenant_id(d.pop("tenant_id", UNSET))

        list_response_agent_info_items_item = cls(
            agent_id=agent_id,
            available_capacity=available_capacity,
            capabilities=capabilities,
            in_flight=in_flight,
            max_concurrency=max_concurrency,
            seconds_since_heartbeat=seconds_since_heartbeat,
            target_triple=target_triple,
            tenant_id=tenant_id,
        )

        list_response_agent_info_items_item.additional_properties = d
        return list_response_agent_info_items_item

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
