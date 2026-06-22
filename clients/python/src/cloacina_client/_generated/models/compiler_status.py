from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="CompilerStatus")


@_attrs_define
class CompilerStatus:
    """Build-pipeline state, derived from the build queue in the database — the
    same rows the compiler's own `/v1/status` reports. The server reads them
    directly, so this needs no HTTP coupling to the compiler service.

        Attributes:
            building (int): Packages currently building.
            pending (int): Packages awaiting compilation.
            status (str): Coarse roll-up: `"building"` (work in flight), `"backlogged"` (packages
                pending but none building — the compiler may be down), or `"idle"`
                (nothing queued; liveness is undeterminable from the queue alone).
            last_failure_at (None | str | Unset): RFC 3339 timestamp of the most recent failed build, if any.
            last_success_at (None | str | Unset): RFC 3339 timestamp of the most recent successful build, if any.
            seconds_since_heartbeat (int | None | Unset): Seconds since the compiler last claimed a build (its DB-visible
                heartbeat). Only meaningful while a build is in flight.
    """

    building: int
    pending: int
    status: str
    last_failure_at: None | str | Unset = UNSET
    last_success_at: None | str | Unset = UNSET
    seconds_since_heartbeat: int | None | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        building = self.building

        pending = self.pending

        status = self.status

        last_failure_at: None | str | Unset
        if isinstance(self.last_failure_at, Unset):
            last_failure_at = UNSET
        else:
            last_failure_at = self.last_failure_at

        last_success_at: None | str | Unset
        if isinstance(self.last_success_at, Unset):
            last_success_at = UNSET
        else:
            last_success_at = self.last_success_at

        seconds_since_heartbeat: int | None | Unset
        if isinstance(self.seconds_since_heartbeat, Unset):
            seconds_since_heartbeat = UNSET
        else:
            seconds_since_heartbeat = self.seconds_since_heartbeat

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "building": building,
                "pending": pending,
                "status": status,
            }
        )
        if last_failure_at is not UNSET:
            field_dict["last_failure_at"] = last_failure_at
        if last_success_at is not UNSET:
            field_dict["last_success_at"] = last_success_at
        if seconds_since_heartbeat is not UNSET:
            field_dict["seconds_since_heartbeat"] = seconds_since_heartbeat

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        building = d.pop("building")

        pending = d.pop("pending")

        status = d.pop("status")

        def _parse_last_failure_at(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        last_failure_at = _parse_last_failure_at(d.pop("last_failure_at", UNSET))

        def _parse_last_success_at(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        last_success_at = _parse_last_success_at(d.pop("last_success_at", UNSET))

        def _parse_seconds_since_heartbeat(data: object) -> int | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(int | None | Unset, data)

        seconds_since_heartbeat = _parse_seconds_since_heartbeat(
            d.pop("seconds_since_heartbeat", UNSET)
        )

        compiler_status = cls(
            building=building,
            pending=pending,
            status=status,
            last_failure_at=last_failure_at,
            last_success_at=last_success_at,
            seconds_since_heartbeat=seconds_since_heartbeat,
        )

        compiler_status.additional_properties = d
        return compiler_status

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
