from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.list_response_reactor_fire_items_item_inputs import (
        ListResponseReactorFireItemsItemInputs,
    )


T = TypeVar("T", bound="ListResponseReactorFireItemsItem")


@_attrs_define
class ListResponseReactorFireItemsItem:
    """One recorded reactor fire (CLOACI-T-0766) — a row in
    `GET /v1/health/reactors/{name}/fires`. Makes fires observable (outcome +
    duration), not just counted.

        Attributes:
            duration_ms (int): Graph execution wall-clock for this fire, in milliseconds.
            fired_at (str): RFC 3339 time the fire completed.
            ok (bool): Whether the graph execution completed (`false` = errored).
            error (None | str | Unset): Error detail for a failed fire.
            inputs (ListResponseReactorFireItemsItemInputs | Unset): Input boundary values that triggered this fire: source
                name → value
                (CLOACI-T-0775). The graph's I/O history, so a fire reads as more than
                "ran in 0ms".
            manual (bool | Unset): Whether this fire was a manual operator intervention
                (`force_fire`/`fire_with`) rather than a criteria-driven fire over real
                boundary events (CLOACI-T-0776). The UI marks it with a "manual" pill.
            outputs (list[Any] | Unset): Terminal outputs the graph produced for this fire, as JSON
                (CLOACI-T-0775). Empty when the executor can't serialize them (e.g. the
                Python reactor path) or on a failed fire.
    """

    duration_ms: int
    fired_at: str
    ok: bool
    error: None | str | Unset = UNSET
    inputs: ListResponseReactorFireItemsItemInputs | Unset = UNSET
    manual: bool | Unset = UNSET
    outputs: list[Any] | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        duration_ms = self.duration_ms

        fired_at = self.fired_at

        ok = self.ok

        error: None | str | Unset
        if isinstance(self.error, Unset):
            error = UNSET
        else:
            error = self.error

        inputs: dict[str, Any] | Unset = UNSET
        if not isinstance(self.inputs, Unset):
            inputs = self.inputs.to_dict()

        manual = self.manual

        outputs: list[Any] | Unset = UNSET
        if not isinstance(self.outputs, Unset):
            outputs = self.outputs

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "duration_ms": duration_ms,
                "fired_at": fired_at,
                "ok": ok,
            }
        )
        if error is not UNSET:
            field_dict["error"] = error
        if inputs is not UNSET:
            field_dict["inputs"] = inputs
        if manual is not UNSET:
            field_dict["manual"] = manual
        if outputs is not UNSET:
            field_dict["outputs"] = outputs

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.list_response_reactor_fire_items_item_inputs import (
            ListResponseReactorFireItemsItemInputs,
        )

        d = dict(src_dict)
        duration_ms = d.pop("duration_ms")

        fired_at = d.pop("fired_at")

        ok = d.pop("ok")

        def _parse_error(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        error = _parse_error(d.pop("error", UNSET))

        _inputs = d.pop("inputs", UNSET)
        inputs: ListResponseReactorFireItemsItemInputs | Unset
        if isinstance(_inputs, Unset):
            inputs = UNSET
        else:
            inputs = ListResponseReactorFireItemsItemInputs.from_dict(_inputs)

        manual = d.pop("manual", UNSET)

        outputs = cast(list[Any], d.pop("outputs", UNSET))

        list_response_reactor_fire_items_item = cls(
            duration_ms=duration_ms,
            fired_at=fired_at,
            ok=ok,
            error=error,
            inputs=inputs,
            manual=manual,
            outputs=outputs,
        )

        list_response_reactor_fire_items_item.additional_properties = d
        return list_response_reactor_fire_items_item

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
