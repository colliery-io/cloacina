from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="ReactorFireTimeseries")


@_attrs_define
class ReactorFireTimeseries:
    """`GET /v1/health/reactors/{name}/fires/timeseries` (CLOACI-T-0766): fire counts
    per minute for the last 60 minutes, oldest → newest, gaps filled with 0.

        Attributes:
            buckets (list[int]): 60 per-minute fire counts, oldest first; the last entry is the current minute.
    """

    buckets: list[int]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        buckets = self.buckets

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "buckets": buckets,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        buckets = cast(list[int], d.pop("buckets"))

        reactor_fire_timeseries = cls(
            buckets=buckets,
        )

        reactor_fire_timeseries.additional_properties = d
        return reactor_fire_timeseries

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
