from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.execution_summary import ExecutionSummary


T = TypeVar("T", bound="SyncPass")


@_attrs_define
class SyncPass:
    """One pass of a sync: its segments' executions, shown as one.

    Attributes:
        executions (list[ExecutionSummary]): The segments, oldest first.
        pass_id (str):
        segments (int):
        started_at (str): RFC 3339: the first segment's start.
        status (str): `Running`, `Completed` or `Failed`.
        completed_at (None | str | Unset): RFC 3339: the last segment's end; `null` while running.
        cursor (None | str | Unset): The cursor the pass's last segment ended on.
    """

    executions: list[ExecutionSummary]
    pass_id: str
    segments: int
    started_at: str
    status: str
    completed_at: None | str | Unset = UNSET
    cursor: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        executions = []
        for executions_item_data in self.executions:
            executions_item = executions_item_data.to_dict()
            executions.append(executions_item)

        pass_id = self.pass_id

        segments = self.segments

        started_at = self.started_at

        status = self.status

        completed_at: None | str | Unset
        if isinstance(self.completed_at, Unset):
            completed_at = UNSET
        else:
            completed_at = self.completed_at

        cursor: None | str | Unset
        if isinstance(self.cursor, Unset):
            cursor = UNSET
        else:
            cursor = self.cursor

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "executions": executions,
                "pass_id": pass_id,
                "segments": segments,
                "started_at": started_at,
                "status": status,
            }
        )
        if completed_at is not UNSET:
            field_dict["completed_at"] = completed_at
        if cursor is not UNSET:
            field_dict["cursor"] = cursor

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        from ..models.execution_summary import ExecutionSummary

        d = dict(src_dict)
        executions = []
        _executions = d.pop("executions")
        for executions_item_data in _executions:
            executions_item = ExecutionSummary.from_dict(executions_item_data)

            executions.append(executions_item)

        pass_id = d.pop("pass_id")

        segments = d.pop("segments")

        started_at = d.pop("started_at")

        status = d.pop("status")

        def _parse_completed_at(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        completed_at = _parse_completed_at(d.pop("completed_at", UNSET))

        def _parse_cursor(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        cursor = _parse_cursor(d.pop("cursor", UNSET))

        sync_pass = cls(
            executions=executions,
            pass_id=pass_id,
            segments=segments,
            started_at=started_at,
            status=status,
            completed_at=completed_at,
            cursor=cursor,
        )

        sync_pass.additional_properties = d
        return sync_pass

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
