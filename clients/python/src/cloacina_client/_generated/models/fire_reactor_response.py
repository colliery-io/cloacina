from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.fire_mode import FireMode

T = TypeVar("T", bound="FireReactorResponse")


@_attrs_define
class FireReactorResponse:
    """Response body for a successful manual reactor fire (CLOACI-T-0751).

    Attributes:
        mode (FireMode): How a manual REST fire should populate the reactor's input cache.

            CLOACI-T-0751. Mirrors the two WS write commands (`ForceFire` /
            `FireWith`) but with operator-friendly, typed input.
        reactor (str): Echoes the reactor name that was fired.
        sources_injected (list[str]): Source names whose values were injected (empty for `force_fire`).
    """

    mode: FireMode
    reactor: str
    sources_injected: list[str]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        mode = self.mode.value

        reactor = self.reactor

        sources_injected = self.sources_injected

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "mode": mode,
                "reactor": reactor,
                "sources_injected": sources_injected,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        mode = FireMode(d.pop("mode"))

        reactor = d.pop("reactor")

        sources_injected = cast(list[str], d.pop("sources_injected"))

        fire_reactor_response = cls(
            mode=mode,
            reactor=reactor,
            sources_injected=sources_injected,
        )

        fire_reactor_response.additional_properties = d
        return fire_reactor_response

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
