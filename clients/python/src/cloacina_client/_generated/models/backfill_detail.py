from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Self, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.backfill_failed_interval import BackfillFailedInterval
    from ..models.backfill_summary import BackfillSummary


T = TypeVar("T", bound="BackfillDetail")


@_attrs_define
class BackfillDetail:
    """Response of `GET /tenants/{tenant_id}/workflows/{name}/backfills/{id}`.

    Attributes:
        backfill (BackfillSummary): One backfill: a row in the list, and the response of create, cancel and
            retry.
        failed_intervals (list[BackfillFailedInterval]): The failed intervals, oldest first, at most 1000.
        failed_intervals_truncated (bool): True when the backfill has more failed intervals than the list holds.
    """

    backfill: BackfillSummary
    failed_intervals: list[BackfillFailedInterval]
    failed_intervals_truncated: bool
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        backfill = self.backfill.to_dict()

        failed_intervals = []
        for failed_intervals_item_data in self.failed_intervals:
            failed_intervals_item = failed_intervals_item_data.to_dict()
            failed_intervals.append(failed_intervals_item)

        failed_intervals_truncated = self.failed_intervals_truncated

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "backfill": backfill,
                "failed_intervals": failed_intervals,
                "failed_intervals_truncated": failed_intervals_truncated,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        from ..models.backfill_failed_interval import BackfillFailedInterval
        from ..models.backfill_summary import BackfillSummary

        d = dict(src_dict)
        backfill = BackfillSummary.from_dict(d.pop("backfill"))

        failed_intervals = []
        _failed_intervals = d.pop("failed_intervals")
        for failed_intervals_item_data in _failed_intervals:
            failed_intervals_item = BackfillFailedInterval.from_dict(
                failed_intervals_item_data
            )

            failed_intervals.append(failed_intervals_item)

        failed_intervals_truncated = d.pop("failed_intervals_truncated")

        backfill_detail = cls(
            backfill=backfill,
            failed_intervals=failed_intervals,
            failed_intervals_truncated=failed_intervals_truncated,
        )

        backfill_detail.additional_properties = d
        return backfill_detail

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
