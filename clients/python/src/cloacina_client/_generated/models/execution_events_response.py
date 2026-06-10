from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.execution_event import ExecutionEvent


T = TypeVar("T", bound="ExecutionEventsResponse")


@_attrs_define
class ExecutionEventsResponse:
    """`GET /tenants/{tenant_id}/executions/{id}/events` response.

    Attributes:
        events (list[ExecutionEvent]):
        execution_id (str):
        tenant_id (str):
    """

    events: list[ExecutionEvent]
    execution_id: str
    tenant_id: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        events = []
        for events_item_data in self.events:
            events_item = events_item_data.to_dict()
            events.append(events_item)

        execution_id = self.execution_id

        tenant_id = self.tenant_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "events": events,
                "execution_id": execution_id,
                "tenant_id": tenant_id,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.execution_event import ExecutionEvent

        d = dict(src_dict)
        events = []
        _events = d.pop("events")
        for events_item_data in _events:
            events_item = ExecutionEvent.from_dict(events_item_data)

            events.append(events_item)

        execution_id = d.pop("execution_id")

        tenant_id = d.pop("tenant_id")

        execution_events_response = cls(
            events=events,
            execution_id=execution_id,
            tenant_id=tenant_id,
        )

        execution_events_response.additional_properties = d
        return execution_events_response

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
