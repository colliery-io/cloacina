from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="ReactorFire")


@_attrs_define
class ReactorFire:
    """One recorded reactor fire (CLOACI-T-0766) — a row in
    `GET /v1/health/reactors/{name}/fires`. Makes fires observable (outcome +
    duration), not just counted.

        Attributes:
            duration_ms (int): Graph execution wall-clock for this fire, in milliseconds.
            fired_at (str): RFC 3339 time the fire completed.
            ok (bool): Whether the graph execution completed (`false` = errored).
            error (None | str | Unset): Error detail for a failed fire.
    """

    duration_ms: int
    fired_at: str
    ok: bool
    error: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        duration_ms = self.duration_ms

        fired_at = self.fired_at

        ok = self.ok

        error: None | str | Unset
        if isinstance(self.error, Unset):
            error = UNSET
        else:
            error = self.error

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "duration_ms": duration_ms,
                "fired_at": fired_at,
                "ok": ok,
            }
        )
        if error is not UNSET:
            field_dict["error"] = error

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        duration_ms = d.pop("duration_ms")

        fired_at = d.pop("fired_at")

        ok = d.pop("ok")

        def _parse_error(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        error = _parse_error(d.pop("error", UNSET))

        reactor_fire = cls(
            duration_ms=duration_ms,
            fired_at=fired_at,
            ok=ok,
            error=error,
        )

        reactor_fire.additional_properties = d
        return reactor_fire

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
