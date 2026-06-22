from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.graph_topology import GraphTopology


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
            fires (int | Unset): Total graph fires since load — the reactor's live fire counter
                (CLOACI-I-0124 / WS-10). The UI derives recent throughput from the delta
                across successive polls.
            input_strategy (None | str | Unset): Input strategy of the bound reactor: `"latest"` | `"sequential"`.
            last_fired_at (None | str | Unset): RFC 3339 timestamp of the last graph fire; `null` if it hasn't fired yet.
            reaction_mode (None | str | Unset): Reaction mode of the bound reactor: `"when_any"` | `"when_all"`.
            reactor (None | str | Unset): Name of the reactor this graph is bound to (the trigger that fires it).
            source_package (None | str | Unset): Package whose retained source defines this graph's nodes/reactor, so the
                UI can fetch it via `GET /workflows/{package}/source` and show node code
                (CLOACI-T-0773). `None` when the package can't be resolved (e.g. a graph
                declaring no typed surface). Populated on the single-graph detail endpoint
                only; the list leaves it `None`.
            topology (GraphTopology | None | Unset):
    """

    accumulators: list[str]
    health: Any
    name: str
    paused: bool
    fires: int | Unset = UNSET
    input_strategy: None | str | Unset = UNSET
    last_fired_at: None | str | Unset = UNSET
    reaction_mode: None | str | Unset = UNSET
    reactor: None | str | Unset = UNSET
    source_package: None | str | Unset = UNSET
    topology: GraphTopology | None | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.graph_topology import GraphTopology

        accumulators = self.accumulators

        health = self.health

        name = self.name

        paused = self.paused

        fires = self.fires

        input_strategy: None | str | Unset
        if isinstance(self.input_strategy, Unset):
            input_strategy = UNSET
        else:
            input_strategy = self.input_strategy

        last_fired_at: None | str | Unset
        if isinstance(self.last_fired_at, Unset):
            last_fired_at = UNSET
        else:
            last_fired_at = self.last_fired_at

        reaction_mode: None | str | Unset
        if isinstance(self.reaction_mode, Unset):
            reaction_mode = UNSET
        else:
            reaction_mode = self.reaction_mode

        reactor: None | str | Unset
        if isinstance(self.reactor, Unset):
            reactor = UNSET
        else:
            reactor = self.reactor

        source_package: None | str | Unset
        if isinstance(self.source_package, Unset):
            source_package = UNSET
        else:
            source_package = self.source_package

        topology: dict[str, Any] | None | Unset
        if isinstance(self.topology, Unset):
            topology = UNSET
        elif isinstance(self.topology, GraphTopology):
            topology = self.topology.to_dict()
        else:
            topology = self.topology

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
        if fires is not UNSET:
            field_dict["fires"] = fires
        if input_strategy is not UNSET:
            field_dict["input_strategy"] = input_strategy
        if last_fired_at is not UNSET:
            field_dict["last_fired_at"] = last_fired_at
        if reaction_mode is not UNSET:
            field_dict["reaction_mode"] = reaction_mode
        if reactor is not UNSET:
            field_dict["reactor"] = reactor
        if source_package is not UNSET:
            field_dict["source_package"] = source_package
        if topology is not UNSET:
            field_dict["topology"] = topology

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.graph_topology import GraphTopology

        d = dict(src_dict)
        accumulators = cast(list[str], d.pop("accumulators"))

        health = d.pop("health")

        name = d.pop("name")

        paused = d.pop("paused")

        fires = d.pop("fires", UNSET)

        def _parse_input_strategy(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        input_strategy = _parse_input_strategy(d.pop("input_strategy", UNSET))

        def _parse_last_fired_at(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        last_fired_at = _parse_last_fired_at(d.pop("last_fired_at", UNSET))

        def _parse_reaction_mode(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        reaction_mode = _parse_reaction_mode(d.pop("reaction_mode", UNSET))

        def _parse_reactor(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        reactor = _parse_reactor(d.pop("reactor", UNSET))

        def _parse_source_package(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        source_package = _parse_source_package(d.pop("source_package", UNSET))

        def _parse_topology(data: object) -> GraphTopology | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                topology_type_1 = GraphTopology.from_dict(data)

                return topology_type_1
            except (TypeError, ValueError, AttributeError, KeyError):
                pass
            return cast(GraphTopology | None | Unset, data)

        topology = _parse_topology(d.pop("topology", UNSET))

        list_response_graph_status_items_item = cls(
            accumulators=accumulators,
            health=health,
            name=name,
            paused=paused,
            fires=fires,
            input_strategy=input_strategy,
            last_fired_at=last_fired_at,
            reaction_mode=reaction_mode,
            reactor=reactor,
            source_package=source_package,
            topology=topology,
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
