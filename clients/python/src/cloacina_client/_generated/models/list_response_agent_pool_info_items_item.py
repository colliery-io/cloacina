from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Self, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.agent_pool_settings_info import AgentPoolSettingsInfo


T = TypeVar("T", bound="ListResponseAgentPoolInfoItemsItem")


@_attrs_define
class ListResponseAgentPoolInfoItemsItem:
    """One agent pool of a tenant, as the pool routes return it.

    Attributes:
        desired_agents (int): How many agents the server keeps for the pool: the tenant's desired
            count for `default`, the pool's `max_agents` for another pool.
        live_agents (int): How many agents of the pool are registered and alive now.
        pool (str): Pool name. Every tenant has `default`.
        settings (AgentPoolSettingsInfo): The settings of one agent pool, as the pool routes return them. The same
            fields and the same JSON as [`AgentPoolSettings`], but an unknown field is
            ignored: a client reads an answer of a newer server (CLOACINA-T-1149).
        tenant_id (str):
    """

    desired_agents: int
    live_agents: int
    pool: str
    settings: AgentPoolSettingsInfo
    tenant_id: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        desired_agents = self.desired_agents

        live_agents = self.live_agents

        pool = self.pool

        settings = self.settings.to_dict()

        tenant_id = self.tenant_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "desired_agents": desired_agents,
                "live_agents": live_agents,
                "pool": pool,
                "settings": settings,
                "tenant_id": tenant_id,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        from ..models.agent_pool_settings_info import AgentPoolSettingsInfo

        d = dict(src_dict)
        desired_agents = d.pop("desired_agents")

        live_agents = d.pop("live_agents")

        pool = d.pop("pool")

        settings = AgentPoolSettingsInfo.from_dict(d.pop("settings"))

        tenant_id = d.pop("tenant_id")

        list_response_agent_pool_info_items_item = cls(
            desired_agents=desired_agents,
            live_agents=live_agents,
            pool=pool,
            settings=settings,
            tenant_id=tenant_id,
        )

        list_response_agent_pool_info_items_item.additional_properties = d
        return list_response_agent_pool_info_items_item

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
