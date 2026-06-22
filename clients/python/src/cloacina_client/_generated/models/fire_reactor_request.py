from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.fire_mode import FireMode
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.fire_reactor_request_inputs import FireReactorRequestInputs


T = TypeVar("T", bound="FireReactorRequest")


@_attrs_define
class FireReactorRequest:
    """Request body for `POST /v1/health/reactors/{name}/fire` (CLOACI-T-0751).

    Operators supply typed JSON per source; the server serializes each value
    to the boundary wire encoding so callers never deal in raw `Vec<u8>`.

        Attributes:
            inputs (FireReactorRequestInputs | Unset): Per-source typed payloads, keyed by accumulator source name. Each
                JSON value is serialized to the boundary encoding server-side.
                Required (and non-empty) when `mode` is `fire_with`; ignored for
                `force_fire`. Each value may be any JSON.
            mode (FireMode | Unset): How a manual REST fire should populate the reactor's input cache.

                CLOACI-T-0751. Mirrors the two WS write commands (`ForceFire` /
                `FireWith`) but with operator-friendly, typed input.
    """

    inputs: FireReactorRequestInputs | Unset = UNSET
    mode: FireMode | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        inputs: dict[str, Any] | Unset = UNSET
        if not isinstance(self.inputs, Unset):
            inputs = self.inputs.to_dict()

        mode: str | Unset = UNSET
        if not isinstance(self.mode, Unset):
            mode = self.mode.value

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if inputs is not UNSET:
            field_dict["inputs"] = inputs
        if mode is not UNSET:
            field_dict["mode"] = mode

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.fire_reactor_request_inputs import FireReactorRequestInputs

        d = dict(src_dict)
        _inputs = d.pop("inputs", UNSET)
        inputs: FireReactorRequestInputs | Unset
        if isinstance(_inputs, Unset):
            inputs = UNSET
        else:
            inputs = FireReactorRequestInputs.from_dict(_inputs)

        _mode = d.pop("mode", UNSET)
        mode: FireMode | Unset
        if isinstance(_mode, Unset):
            mode = UNSET
        else:
            mode = FireMode(_mode)

        fire_reactor_request = cls(
            inputs=inputs,
            mode=mode,
        )

        fire_reactor_request.additional_properties = d
        return fire_reactor_request

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
