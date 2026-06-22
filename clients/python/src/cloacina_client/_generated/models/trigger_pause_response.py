from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="TriggerPauseResponse")


@_attrs_define
class TriggerPauseResponse:
    """`POST /tenants/{tenant_id}/triggers/{name}/pause` and `/resume` response
    (CLOACI-T-0749).

        Attributes:
            id (str): Schedule UUID.
            name (str): The name the schedule was addressed by (trigger or workflow name).
            paused (bool): Current paused state after the operation.
            status (str): `"paused"` or `"resumed"`.
            tenant_id (str):
    """

    id: str
    name: str
    paused: bool
    status: str
    tenant_id: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        id = self.id

        name = self.name

        paused = self.paused

        status = self.status

        tenant_id = self.tenant_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "name": name,
                "paused": paused,
                "status": status,
                "tenant_id": tenant_id,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        id = d.pop("id")

        name = d.pop("name")

        paused = d.pop("paused")

        status = d.pop("status")

        tenant_id = d.pop("tenant_id")

        trigger_pause_response = cls(
            id=id,
            name=name,
            paused=paused,
            status=status,
            tenant_id=tenant_id,
        )

        trigger_pause_response.additional_properties = d
        return trigger_pause_response

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
