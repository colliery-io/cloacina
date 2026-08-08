from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from typing_extensions import Self

from ..types import UNSET, Unset

T = TypeVar("T", bound="CreateInstanceRequest")


@_attrs_define
class CreateInstanceRequest:
    """Body for `POST /tenants/{tenant_id}/workflows/{name}/instances`.

    Attributes:
        instance_name (str): Instance name, unique per `(workflow_name, instance_name)` within the
            tenant.
        cron (None | str | Unset): Cron expression. When omitted the instance is created **unscheduled** —
            a durable named param binding that never fires on its own.
        enabled (bool | None | Unset): Whether the schedule is enabled on creation. Defaults to `true`.
        params (Any | Unset): Parameter values bound to this instance, validated against the
            workflow's declared `params(...)` slots.
        timezone (None | str | Unset): IANA timezone for `cron`. Defaults to `UTC`.
    """

    instance_name: str
    cron: None | str | Unset = UNSET
    enabled: bool | None | Unset = UNSET
    params: Any | Unset = UNSET
    timezone: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        instance_name = self.instance_name

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
        field_dict.update(
            {
                "instance_name": instance_name,
            }
        )
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
        instance_name = d.pop("instance_name")

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

        create_instance_request = cls(
            instance_name=instance_name,
            cron=cron,
            enabled=enabled,
            params=params,
            timezone=timezone,
        )

        create_instance_request.additional_properties = d
        return create_instance_request

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
