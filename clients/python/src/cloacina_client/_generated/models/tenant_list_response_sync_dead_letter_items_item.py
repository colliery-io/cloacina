from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="TenantListResponseSyncDeadLetterItemsItem")


@_attrs_define
class TenantListResponseSyncDeadLetterItemsItem:
    """A record a sync set aside, with the reason.

    Attributes:
        created_at (str):
        id (str):
        reason (str):
        record (str):
        stream (str):
        execution_id (None | str | Unset):
    """

    created_at: str
    id: str
    reason: str
    record: str
    stream: str
    execution_id: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        created_at = self.created_at

        id = self.id

        reason = self.reason

        record = self.record

        stream = self.stream

        execution_id: None | str | Unset
        if isinstance(self.execution_id, Unset):
            execution_id = UNSET
        else:
            execution_id = self.execution_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "created_at": created_at,
                "id": id,
                "reason": reason,
                "record": record,
                "stream": stream,
            }
        )
        if execution_id is not UNSET:
            field_dict["execution_id"] = execution_id

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        created_at = d.pop("created_at")

        id = d.pop("id")

        reason = d.pop("reason")

        record = d.pop("record")

        stream = d.pop("stream")

        def _parse_execution_id(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        execution_id = _parse_execution_id(d.pop("execution_id", UNSET))

        tenant_list_response_sync_dead_letter_items_item = cls(
            created_at=created_at,
            id=id,
            reason=reason,
            record=record,
            stream=stream,
            execution_id=execution_id,
        )

        tenant_list_response_sync_dead_letter_items_item.additional_properties = d
        return tenant_list_response_sync_dead_letter_items_item

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
