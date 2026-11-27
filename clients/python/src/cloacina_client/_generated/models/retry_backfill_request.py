from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="RetryBackfillRequest")


@_attrs_define
class RetryBackfillRequest:
    """Body for `POST /tenants/{tenant_id}/workflows/{name}/backfills/{id}/retry`.

    Attributes:
        max_active_runs (int | None | Unset): The limit on active runs of the new backfill. Defaults to the limit
            of the backfill that is retried.
    """

    max_active_runs: int | None | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        max_active_runs: int | None | Unset
        if isinstance(self.max_active_runs, Unset):
            max_active_runs = UNSET
        else:
            max_active_runs = self.max_active_runs

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if max_active_runs is not UNSET:
            field_dict["max_active_runs"] = max_active_runs

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)

        def _parse_max_active_runs(data: object) -> int | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(int | None | Unset, data)

        max_active_runs = _parse_max_active_runs(d.pop("max_active_runs", UNSET))

        retry_backfill_request = cls(
            max_active_runs=max_active_runs,
        )

        retry_backfill_request.additional_properties = d
        return retry_backfill_request

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
