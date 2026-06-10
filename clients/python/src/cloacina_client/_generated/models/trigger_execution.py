from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="TriggerExecution")


@_attrs_define
class TriggerExecution:
    """One row in `recent_executions` of the trigger detail response.

    Attributes:
        id (str): Schedule-execution UUID.
        started_at (str): RFC 3339 timestamp.
        completed_at (None | str | Unset): RFC 3339 timestamp.
        scheduled_time (None | str | Unset): RFC 3339 timestamp.
    """

    id: str
    started_at: str
    completed_at: None | str | Unset = UNSET
    scheduled_time: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        id = self.id

        started_at = self.started_at

        completed_at: None | str | Unset
        if isinstance(self.completed_at, Unset):
            completed_at = UNSET
        else:
            completed_at = self.completed_at

        scheduled_time: None | str | Unset
        if isinstance(self.scheduled_time, Unset):
            scheduled_time = UNSET
        else:
            scheduled_time = self.scheduled_time

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "started_at": started_at,
            }
        )
        if completed_at is not UNSET:
            field_dict["completed_at"] = completed_at
        if scheduled_time is not UNSET:
            field_dict["scheduled_time"] = scheduled_time

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        id = d.pop("id")

        started_at = d.pop("started_at")

        def _parse_completed_at(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        completed_at = _parse_completed_at(d.pop("completed_at", UNSET))

        def _parse_scheduled_time(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        scheduled_time = _parse_scheduled_time(d.pop("scheduled_time", UNSET))

        trigger_execution = cls(
            id=id,
            started_at=started_at,
            completed_at=completed_at,
            scheduled_time=scheduled_time,
        )

        trigger_execution.additional_properties = d
        return trigger_execution

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
