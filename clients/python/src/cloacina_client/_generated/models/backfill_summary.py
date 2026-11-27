from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.backfill_progress import BackfillProgress


T = TypeVar("T", bound="BackfillSummary")


@_attrs_define
class BackfillSummary:
    """One backfill: a row in the list, and the response of create, cancel and
    retry.

        Attributes:
            created_at (str): RFC 3339 timestamp.
            cron_expression (str):
            end (str): End of the range, RFC 3339.
            id (str): Backfill id (a UUID). Each run has it in its context as `backfill_id`.
            max_active_runs (int): The limit on active runs of this backfill.
            progress (BackfillProgress): How far a backfill is.
            start (str): Start of the range, RFC 3339.
            status (str): `running`, `paused`, `completed` or `cancelled`. `completed` means
                each interval got a run and each run ended; see `progress.failed`
                for the runs that failed.
            timezone (str):
            workflow_name (str):
            params (Any | Unset): Parameter values for each run, as stored.
            retry_of (None | str | Unset): The id of the backfill whose failed intervals this one runs again.
    """

    created_at: str
    cron_expression: str
    end: str
    id: str
    max_active_runs: int
    progress: BackfillProgress
    start: str
    status: str
    timezone: str
    workflow_name: str
    params: Any | Unset = UNSET
    retry_of: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        created_at = self.created_at

        cron_expression = self.cron_expression

        end = self.end

        id = self.id

        max_active_runs = self.max_active_runs

        progress = self.progress.to_dict()

        start = self.start

        status = self.status

        timezone = self.timezone

        workflow_name = self.workflow_name

        params = self.params

        retry_of: None | str | Unset
        if isinstance(self.retry_of, Unset):
            retry_of = UNSET
        else:
            retry_of = self.retry_of

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "created_at": created_at,
                "cron_expression": cron_expression,
                "end": end,
                "id": id,
                "max_active_runs": max_active_runs,
                "progress": progress,
                "start": start,
                "status": status,
                "timezone": timezone,
                "workflow_name": workflow_name,
            }
        )
        if params is not UNSET:
            field_dict["params"] = params
        if retry_of is not UNSET:
            field_dict["retry_of"] = retry_of

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        from ..models.backfill_progress import BackfillProgress

        d = dict(src_dict)
        created_at = d.pop("created_at")

        cron_expression = d.pop("cron_expression")

        end = d.pop("end")

        id = d.pop("id")

        max_active_runs = d.pop("max_active_runs")

        progress = BackfillProgress.from_dict(d.pop("progress"))

        start = d.pop("start")

        status = d.pop("status")

        timezone = d.pop("timezone")

        workflow_name = d.pop("workflow_name")

        params = d.pop("params", UNSET)

        def _parse_retry_of(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        retry_of = _parse_retry_of(d.pop("retry_of", UNSET))

        backfill_summary = cls(
            created_at=created_at,
            cron_expression=cron_expression,
            end=end,
            id=id,
            max_active_runs=max_active_runs,
            progress=progress,
            start=start,
            status=status,
            timezone=timezone,
            workflow_name=workflow_name,
            params=params,
            retry_of=retry_of,
        )

        backfill_summary.additional_properties = d
        return backfill_summary

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
