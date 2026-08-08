from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from typing_extensions import Self

if TYPE_CHECKING:
    from ..models.create_secret_request_fields import CreateSecretRequestFields


T = TypeVar("T", bound="CreateSecretRequest")


@_attrs_define
class CreateSecretRequest:
    """Request body for `POST /v1/tenants/{tenant_id}/secrets` — create a secret.

    `fields` is the `{field_name: value}` map. The values are write-only: they
    are encrypted at rest and never returned by any read endpoint.

        Attributes:
            fields (CreateSecretRequestFields): The `{field: value}` map. Values are write-only.
            name (str): The secret's name (unique within the tenant).
    """

    fields: CreateSecretRequestFields
    name: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        fields = self.fields.to_dict()

        name = self.name

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "fields": fields,
                "name": name,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        from ..models.create_secret_request_fields import CreateSecretRequestFields

        d = dict(src_dict)
        fields = CreateSecretRequestFields.from_dict(d.pop("fields"))

        name = d.pop("name")

        create_secret_request = cls(
            fields=fields,
            name=name,
        )

        create_secret_request.additional_properties = d
        return create_secret_request

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
