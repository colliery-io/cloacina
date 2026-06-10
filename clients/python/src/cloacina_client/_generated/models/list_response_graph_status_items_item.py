from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="ListResponseGraphStatusItemsItem")


@_attrs_define
class ListResponseGraphStatusItemsItem:
    """One row in `GET /v1/health/graphs`, and the `GET /v1/health/graphs/{name}`
    response body.

        Attributes:
            accumulators (list[str]): Names of the accumulators feeding this graph.
            health (Any): Graph health snapshot; `{"state": "running" | "stopped"}` when no
                detailed health is available. Free-form JSON for now.
            name (str):
            paused (bool): Pause state of the graph's reactor.
    """

    accumulators: list[str]
    health: Any
    name: str
    paused: bool
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        accumulators = self.accumulators

        health = self.health

        name = self.name

        paused = self.paused

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "accumulators": accumulators,
                "health": health,
                "name": name,
                "paused": paused,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        accumulators = cast(list[str], d.pop("accumulators"))

        health = d.pop("health")

        name = d.pop("name")

        paused = d.pop("paused")

        list_response_graph_status_items_item = cls(
            accumulators=accumulators,
            health=health,
            name=name,
            paused=paused,
        )

        list_response_graph_status_items_item.additional_properties = d
        return list_response_graph_status_items_item

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
