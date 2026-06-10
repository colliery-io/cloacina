from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.trigger_execution import TriggerExecution
    from ..models.trigger_schedule_info import TriggerScheduleInfo


T = TypeVar("T", bound="TriggerDetailResponse")


@_attrs_define
class TriggerDetailResponse:
    """`GET /tenants/{tenant_id}/triggers/{name}` response.

    Attributes:
        recent_executions (list[TriggerExecution]):
        schedule (TriggerScheduleInfo): Schedule fields in the trigger detail response.
        tenant_id (str):
    """

    recent_executions: list[TriggerExecution]
    schedule: TriggerScheduleInfo
    tenant_id: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        recent_executions = []
        for recent_executions_item_data in self.recent_executions:
            recent_executions_item = recent_executions_item_data.to_dict()
            recent_executions.append(recent_executions_item)

        schedule = self.schedule.to_dict()

        tenant_id = self.tenant_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "recent_executions": recent_executions,
                "schedule": schedule,
                "tenant_id": tenant_id,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.trigger_execution import TriggerExecution
        from ..models.trigger_schedule_info import TriggerScheduleInfo

        d = dict(src_dict)
        recent_executions = []
        _recent_executions = d.pop("recent_executions")
        for recent_executions_item_data in _recent_executions:
            recent_executions_item = TriggerExecution.from_dict(
                recent_executions_item_data
            )

            recent_executions.append(recent_executions_item)

        schedule = TriggerScheduleInfo.from_dict(d.pop("schedule"))

        tenant_id = d.pop("tenant_id")

        trigger_detail_response = cls(
            recent_executions=recent_executions,
            schedule=schedule,
            tenant_id=tenant_id,
        )

        trigger_detail_response.additional_properties = d
        return trigger_detail_response

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
