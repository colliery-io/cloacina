from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.input_slot import InputSlot


T = TypeVar("T", bound="DeclaredSurface")


@_attrs_define
class DeclaredSurface:
    """A declared injectable surface other than the workflow itself — a computation
    graph, reactor, or accumulator (CLOACI-I-0128 Task D). Carries the surface's
    declared input slots so the server can validate operator injections
    (reactor fire / accumulator inject) and the UI can render typed forms.

    Sourced from the package's `get_input_interface` FFI entrypoint at build
    success and stored alongside the package metadata. An undeclared / untyped
    surface has `slots` whose schemas are permissive (`{}`).

        Attributes:
            kind (str): Surface kind: `"graph"`, `"reactor"`, or `"accumulator"`.
            name (str): Surface name (graph name / reactor name / accumulator name).
            slots (list[InputSlot]): The surface's declared input slots.
    """

    kind: str
    name: str
    slots: list[InputSlot]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        kind = self.kind

        name = self.name

        slots = []
        for slots_item_data in self.slots:
            slots_item = slots_item_data.to_dict()
            slots.append(slots_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "kind": kind,
                "name": name,
                "slots": slots,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.input_slot import InputSlot

        d = dict(src_dict)
        kind = d.pop("kind")

        name = d.pop("name")

        slots = []
        _slots = d.pop("slots")
        for slots_item_data in _slots:
            slots_item = InputSlot.from_dict(slots_item_data)

            slots.append(slots_item)

        declared_surface = cls(
            kind=kind,
            name=name,
            slots=slots,
        )

        declared_surface.additional_properties = d
        return declared_surface

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
