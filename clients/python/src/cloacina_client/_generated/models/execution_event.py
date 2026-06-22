from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="ExecutionEvent")


@_attrs_define
class ExecutionEvent:
    """One row in the execution event log.

    Attributes:
        created_at (str): RFC 3339 timestamp.
        event_type (str):
        id (str): Event UUID.
        sequence_num (int):
        event_data (None | str | Unset): JSON-encoded additional data for the event; `null` when absent.
        task_name (None | str | Unset): Local name of the task this event is about, resolved from the event's
            `task_execution_id`. `null` for workflow-scoped events (e.g.
            `workflow_completed`) or when the task can't be resolved
            (CLOACI-I-0124 / WS-9).
    """

    created_at: str
    event_type: str
    id: str
    sequence_num: int
    event_data: None | str | Unset = UNSET
    task_name: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        created_at = self.created_at

        event_type = self.event_type

        id = self.id

        sequence_num = self.sequence_num

        event_data: None | str | Unset
        if isinstance(self.event_data, Unset):
            event_data = UNSET
        else:
            event_data = self.event_data

        task_name: None | str | Unset
        if isinstance(self.task_name, Unset):
            task_name = UNSET
        else:
            task_name = self.task_name

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "created_at": created_at,
                "event_type": event_type,
                "id": id,
                "sequence_num": sequence_num,
            }
        )
        if event_data is not UNSET:
            field_dict["event_data"] = event_data
        if task_name is not UNSET:
            field_dict["task_name"] = task_name

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        created_at = d.pop("created_at")

        event_type = d.pop("event_type")

        id = d.pop("id")

        sequence_num = d.pop("sequence_num")

        def _parse_event_data(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        event_data = _parse_event_data(d.pop("event_data", UNSET))

        def _parse_task_name(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        task_name = _parse_task_name(d.pop("task_name", UNSET))

        execution_event = cls(
            created_at=created_at,
            event_type=event_type,
            id=id,
            sequence_num=sequence_num,
            event_data=event_data,
            task_name=task_name,
        )

        execution_event.additional_properties = d
        return execution_event

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
