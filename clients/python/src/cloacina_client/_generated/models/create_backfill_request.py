from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="CreateBackfillRequest")


@_attrs_define
class CreateBackfillRequest:
    """Body for `POST /tenants/{tenant_id}/workflows/{name}/backfills`.

    Attributes:
        end (str): End of the range, an RFC 3339 timestamp. For one interval, set
            `start` and `end` to its fire time.
        start (str): Start of the range, an RFC 3339 timestamp. The backfill takes each
            fire time `F` of the expression with `start <= F <= end`.
        cron (None | str | Unset): Cron expression that gives the intervals. When omitted: the
            expression and timezone of the cron schedule of the workflow, when it
            has exactly one.
        max_active_runs (int | None | Unset): The most runs of this backfill that may be active at once, 1 or more.
            Defaults to 1. The limit is apart from the limit of any other
            schedule of the workflow.
        params (Any | Unset): Parameter values for each run, validated against the workflow's
            declared `params(...)` slots.
        priority (int | None | Unset): The workflow priority of each run, 0 (the highest) to 10
            (CLOACINA-T-1123). Each run is held to the creator's limit when it
            starts; a priority above the limit is not refused. Not set: the
            priority the workflow declares.
        timezone (None | str | Unset): IANA timezone for `cron`. Defaults to `UTC` when `cron` is given.
    """

    end: str
    start: str
    cron: None | str | Unset = UNSET
    max_active_runs: int | None | Unset = UNSET
    params: Any | Unset = UNSET
    priority: int | None | Unset = UNSET
    timezone: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        end = self.end

        start = self.start

        cron: None | str | Unset
        if isinstance(self.cron, Unset):
            cron = UNSET
        else:
            cron = self.cron

        max_active_runs: int | None | Unset
        if isinstance(self.max_active_runs, Unset):
            max_active_runs = UNSET
        else:
            max_active_runs = self.max_active_runs

        params = self.params

        priority: int | None | Unset
        if isinstance(self.priority, Unset):
            priority = UNSET
        else:
            priority = self.priority

        timezone: None | str | Unset
        if isinstance(self.timezone, Unset):
            timezone = UNSET
        else:
            timezone = self.timezone

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "end": end,
                "start": start,
            }
        )
        if cron is not UNSET:
            field_dict["cron"] = cron
        if max_active_runs is not UNSET:
            field_dict["max_active_runs"] = max_active_runs
        if params is not UNSET:
            field_dict["params"] = params
        if priority is not UNSET:
            field_dict["priority"] = priority
        if timezone is not UNSET:
            field_dict["timezone"] = timezone

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        end = d.pop("end")

        start = d.pop("start")

        def _parse_cron(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        cron = _parse_cron(d.pop("cron", UNSET))

        def _parse_max_active_runs(data: object) -> int | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(int | None | Unset, data)

        max_active_runs = _parse_max_active_runs(d.pop("max_active_runs", UNSET))

        params = d.pop("params", UNSET)

        def _parse_priority(data: object) -> int | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(int | None | Unset, data)

        priority = _parse_priority(d.pop("priority", UNSET))

        def _parse_timezone(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        timezone = _parse_timezone(d.pop("timezone", UNSET))

        create_backfill_request = cls(
            end=end,
            start=start,
            cron=cron,
            max_active_runs=max_active_runs,
            params=params,
            priority=priority,
            timezone=timezone,
        )

        create_backfill_request.additional_properties = d
        return create_backfill_request

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
