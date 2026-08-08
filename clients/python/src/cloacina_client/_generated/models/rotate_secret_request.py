from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from typing_extensions import Self

if TYPE_CHECKING:
    from ..models.rotate_secret_request_fields import RotateSecretRequestFields


T = TypeVar("T", bound="RotateSecretRequest")


@_attrs_define
class RotateSecretRequest:
    """Request body for `PUT|POST /v1/tenants/{tenant_id}/secrets/{name}` — rotate.

    Replaces the secret's field map in place (D-8/OQ-5: in-place, no versioning).

        Attributes:
            fields (RotateSecretRequestFields): The new `{field: value}` map. Values are write-only.
    """

    fields: RotateSecretRequestFields
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        fields = self.fields.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "fields": fields,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        from ..models.rotate_secret_request_fields import RotateSecretRequestFields

        d = dict(src_dict)
        fields = RotateSecretRequestFields.from_dict(d.pop("fields"))

        rotate_secret_request = cls(
            fields=fields,
        )

        rotate_secret_request.additional_properties = d
        return rotate_secret_request

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
