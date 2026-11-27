from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="BackfillProgress")


@_attrs_define
class BackfillProgress:
    """How far a backfill is.

    Attributes:
        completed (int): Intervals whose run ended `Completed`. Counted from the run history:
            a run that the retention job deleted is not counted.
        failed (int): Intervals whose run ended `Failed` or `Cancelled`, or could not be
            started. Counted from the run history, like `completed`.
        remaining (int): Intervals with no run yet.
        running (int): Intervals whose run has not ended.
        total (int): Intervals in the backfill.
        last_interval_end (None | str | Unset): The position: the end of the last interval that got a run. RFC 3339.
            Absent before the first run.
        next_interval_end (None | str | Unset): The end of the next interval to get a run. RFC 3339. Absent when no
            interval remains.
    """

    completed: int
    failed: int
    remaining: int
    running: int
    total: int
    last_interval_end: None | str | Unset = UNSET
    next_interval_end: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        completed = self.completed

        failed = self.failed

        remaining = self.remaining

        running = self.running

        total = self.total

        last_interval_end: None | str | Unset
        if isinstance(self.last_interval_end, Unset):
            last_interval_end = UNSET
        else:
            last_interval_end = self.last_interval_end

        next_interval_end: None | str | Unset
        if isinstance(self.next_interval_end, Unset):
            next_interval_end = UNSET
        else:
            next_interval_end = self.next_interval_end

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "completed": completed,
                "failed": failed,
                "remaining": remaining,
                "running": running,
                "total": total,
            }
        )
        if last_interval_end is not UNSET:
            field_dict["last_interval_end"] = last_interval_end
        if next_interval_end is not UNSET:
            field_dict["next_interval_end"] = next_interval_end

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        completed = d.pop("completed")

        failed = d.pop("failed")

        remaining = d.pop("remaining")

        running = d.pop("running")

        total = d.pop("total")

        def _parse_last_interval_end(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        last_interval_end = _parse_last_interval_end(d.pop("last_interval_end", UNSET))

        def _parse_next_interval_end(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        next_interval_end = _parse_next_interval_end(d.pop("next_interval_end", UNSET))

        backfill_progress = cls(
            completed=completed,
            failed=failed,
            remaining=remaining,
            running=running,
            total=total,
            last_interval_end=last_interval_end,
            next_interval_end=next_interval_end,
        )

        backfill_progress.additional_properties = d
        return backfill_progress

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
