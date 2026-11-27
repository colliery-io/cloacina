from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="BackfillFailedInterval")


@_attrs_define
class BackfillFailedInterval:
    """One failed interval of a backfill.

    Attributes:
        interval_end (str): RFC 3339 timestamp: the fire time of the interval.
        interval_start (str): RFC 3339 timestamp.
        status (str): `Failed`, `Cancelled`, or `NotStarted` when no run could be started.
        execution_id (None | str | Unset): The workflow execution of the interval. Absent when no run started.
    """

    interval_end: str
    interval_start: str
    status: str
    execution_id: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        interval_end = self.interval_end

        interval_start = self.interval_start

        status = self.status

        execution_id: None | str | Unset
        if isinstance(self.execution_id, Unset):
            execution_id = UNSET
        else:
            execution_id = self.execution_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "interval_end": interval_end,
                "interval_start": interval_start,
                "status": status,
            }
        )
        if execution_id is not UNSET:
            field_dict["execution_id"] = execution_id

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        interval_end = d.pop("interval_end")

        interval_start = d.pop("interval_start")

        status = d.pop("status")

        def _parse_execution_id(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        execution_id = _parse_execution_id(d.pop("execution_id", UNSET))

        backfill_failed_interval = cls(
            interval_end=interval_end,
            interval_start=interval_start,
            status=status,
            execution_id=execution_id,
        )

        backfill_failed_interval.additional_properties = d
        return backfill_failed_interval

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
