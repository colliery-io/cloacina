from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="ReactorStatus")


@_attrs_define
class ReactorStatus:
    """One row in `GET /v1/health/reactors` (CLOACI-T-0742). Reactor-first view:
    reactors are standalone (a graph binds to a reactor, not vice versa), so a
    reactor with no graph bound appears here but not in `GET /v1/health/graphs`.

        Attributes:
            accumulators (list[str]): Accumulators this reactor consumes (its inputs).
            health (Any): Reactor health snapshot; `{"state": "running" | "stopped"}` when no
                detailed health is available. Free-form JSON, mirroring `GraphStatus`.
            name (str):
            paused (bool): Pause state of the reactor.
            bound_graphs (list[str] | Unset): Graphs bound to this reactor; empty when the reactor has no graph yet.
            fires (int | Unset): Total fires since load (the reactor's live fire counter, WS-10).
            input_strategy (None | str | Unset): Input strategy: `"latest"` | `"sequential"`.
            last_fired_at (None | str | Unset): RFC 3339 timestamp of the last fire; `null` if it hasn't fired yet.
            reaction_mode (None | str | Unset): Firing criteria: `"when_any"` | `"when_all"`.
    """

    accumulators: list[str]
    health: Any
    name: str
    paused: bool
    bound_graphs: list[str] | Unset = UNSET
    fires: int | Unset = UNSET
    input_strategy: None | str | Unset = UNSET
    last_fired_at: None | str | Unset = UNSET
    reaction_mode: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        accumulators = self.accumulators

        health = self.health

        name = self.name

        paused = self.paused

        bound_graphs: list[str] | Unset = UNSET
        if not isinstance(self.bound_graphs, Unset):
            bound_graphs = self.bound_graphs

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
        if bound_graphs is not UNSET:
            field_dict["bound_graphs"] = bound_graphs
        if fires is not UNSET:
            field_dict["fires"] = fires
        if input_strategy is not UNSET:
            field_dict["input_strategy"] = input_strategy
        if last_fired_at is not UNSET:
            field_dict["last_fired_at"] = last_fired_at
        if reaction_mode is not UNSET:
            field_dict["reaction_mode"] = reaction_mode

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        accumulators = cast(list[str], d.pop("accumulators"))

        health = d.pop("health")

        name = d.pop("name")

        paused = d.pop("paused")

        bound_graphs = cast(list[str], d.pop("bound_graphs", UNSET))

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

        reactor_status = cls(
            accumulators=accumulators,
            health=health,
            name=name,
            paused=paused,
            bound_graphs=bound_graphs,
            fires=fires,
            input_strategy=input_strategy,
            last_fired_at=last_fired_at,
            reaction_mode=reaction_mode,
        )

        reactor_status.additional_properties = d
        return reactor_status

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
