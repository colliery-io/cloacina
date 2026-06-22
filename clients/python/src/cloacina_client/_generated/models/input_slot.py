from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.input_slot_default_type_0 import InputSlotDefaultType0
    from ..models.input_slot_schema import InputSlotSchema


T = TypeVar("T", bound="InputSlot")


@_attrs_define
class InputSlot:
    """One declared input slot of an injectable surface: a named, typed value the
    surface accepts. `schema` is a JSON Schema fragment (the type descriptor —
    `schemars`-derived for Rust, type-hint-derived for Python) that the UI can
    render a form from and the server can validate an injection against.

    `required` slots must be supplied; `default` (when present) is applied when a
    slot is omitted. A surface with no declared interface exposes an empty slot
    list (the "undeclared" state) and accepts free-form input.

        Attributes:
            name (str): Slot name — the context key (workflows) or source/event name
                (accumulators/reactors).
            required (bool): Whether this slot must be supplied for the injection to be accepted.
            schema (InputSlotSchema): JSON Schema fragment describing the accepted value's type.
            default (InputSlotDefaultType0 | None | Unset): Optional default applied when the slot is omitted.
    """

    name: str
    required: bool
    schema: InputSlotSchema
    default: InputSlotDefaultType0 | None | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.input_slot_default_type_0 import InputSlotDefaultType0

        name = self.name

        required = self.required

        schema = self.schema.to_dict()

        default: dict[str, Any] | None | Unset
        if isinstance(self.default, Unset):
            default = UNSET
        elif isinstance(self.default, InputSlotDefaultType0):
            default = self.default.to_dict()
        else:
            default = self.default

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "name": name,
                "required": required,
                "schema": schema,
            }
        )
        if default is not UNSET:
            field_dict["default"] = default

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.input_slot_default_type_0 import InputSlotDefaultType0
        from ..models.input_slot_schema import InputSlotSchema

        d = dict(src_dict)
        name = d.pop("name")

        required = d.pop("required")

        schema = InputSlotSchema.from_dict(d.pop("schema"))

        def _parse_default(data: object) -> InputSlotDefaultType0 | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                default_type_0 = InputSlotDefaultType0.from_dict(data)

                return default_type_0
            except (TypeError, ValueError, AttributeError, KeyError):
                pass
            return cast(InputSlotDefaultType0 | None | Unset, data)

        default = _parse_default(d.pop("default", UNSET))

        input_slot = cls(
            name=name,
            required=required,
            schema=schema,
            default=default,
        )

        input_slot.additional_properties = d
        return input_slot

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
