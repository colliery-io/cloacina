from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.backfill_progress import BackfillProgress
    from ..models.priority_grant import PriorityGrant


T = TypeVar("T", bound="TenantListResponseBackfillSummaryItemsItem")


@_attrs_define
class TenantListResponseBackfillSummaryItemsItem:
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
            priority (None | PriorityGrant | Unset):
            priority_notice (None | str | Unset): Set when `given` is lower than `asked`: what was asked, what is
                given, and why.
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
    priority: None | PriorityGrant | Unset = UNSET
    priority_notice: None | str | Unset = UNSET
    retry_of: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.priority_grant import PriorityGrant

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

        priority: dict[str, Any] | None | Unset
        if isinstance(self.priority, Unset):
            priority = UNSET
        elif isinstance(self.priority, PriorityGrant):
            priority = self.priority.to_dict()
        else:
            priority = self.priority

        priority_notice: None | str | Unset
        if isinstance(self.priority_notice, Unset):
            priority_notice = UNSET
        else:
            priority_notice = self.priority_notice

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
        if priority is not UNSET:
            field_dict["priority"] = priority
        if priority_notice is not UNSET:
            field_dict["priority_notice"] = priority_notice
        if retry_of is not UNSET:
            field_dict["retry_of"] = retry_of

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        from ..models.backfill_progress import BackfillProgress
        from ..models.priority_grant import PriorityGrant

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

        def _parse_priority(data: object) -> None | PriorityGrant | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                priority_type_0 = PriorityGrant.from_dict(data)

                return priority_type_0
            except (TypeError, ValueError, AttributeError, KeyError):
                pass
            return cast(None | PriorityGrant | Unset, data)

        priority = _parse_priority(d.pop("priority", UNSET))

        def _parse_priority_notice(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        priority_notice = _parse_priority_notice(d.pop("priority_notice", UNSET))

        def _parse_retry_of(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        retry_of = _parse_retry_of(d.pop("retry_of", UNSET))

        tenant_list_response_backfill_summary_items_item = cls(
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
            priority=priority,
            priority_notice=priority_notice,
            retry_of=retry_of,
        )

        tenant_list_response_backfill_summary_items_item.additional_properties = d
        return tenant_list_response_backfill_summary_items_item

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
