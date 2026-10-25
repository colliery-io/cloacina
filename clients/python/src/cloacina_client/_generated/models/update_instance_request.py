from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="UpdateInstanceRequest")


@_attrs_define
class UpdateInstanceRequest:
    """Body for `PUT /tenants/{tenant_id}/workflows/{name}/instances/{instance}`
    (CLOACINA-T-1001): the instance's new params, cron and timezone, which
    replace the old ones.

        Attributes:
            cron (None | str | Unset): The cron expression. Omitted makes the instance unscheduled.
            enabled (bool | None | Unset): Whether the schedule is enabled. Omitted keeps it as it is.
            params (Any | Unset): The params, validated as on create. Omitted means none.
            timezone (None | str | Unset): IANA timezone for `cron`. Defaults to `UTC`.
    """

    cron: None | str | Unset = UNSET
    enabled: bool | None | Unset = UNSET
    params: Any | Unset = UNSET
    timezone: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        cron: None | str | Unset
        if isinstance(self.cron, Unset):
            cron = UNSET
        else:
            cron = self.cron

        enabled: bool | None | Unset
        if isinstance(self.enabled, Unset):
            enabled = UNSET
        else:
            enabled = self.enabled

        params = self.params

        timezone: None | str | Unset
        if isinstance(self.timezone, Unset):
            timezone = UNSET
        else:
            timezone = self.timezone

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if cron is not UNSET:
            field_dict["cron"] = cron
        if enabled is not UNSET:
            field_dict["enabled"] = enabled
        if params is not UNSET:
            field_dict["params"] = params
        if timezone is not UNSET:
            field_dict["timezone"] = timezone

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)

        def _parse_cron(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        cron = _parse_cron(d.pop("cron", UNSET))

        def _parse_enabled(data: object) -> bool | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(bool | None | Unset, data)

        enabled = _parse_enabled(d.pop("enabled", UNSET))

        params = d.pop("params", UNSET)

        def _parse_timezone(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        timezone = _parse_timezone(d.pop("timezone", UNSET))

        update_instance_request = cls(
            cron=cron,
            enabled=enabled,
            params=params,
            timezone=timezone,
        )

        update_instance_request.additional_properties = d
        return update_instance_request

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
