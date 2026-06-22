from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="TriggerScheduleInfo")


@_attrs_define
class TriggerScheduleInfo:
    """Schedule fields in the trigger detail response.

    Attributes:
        enabled (bool):
        id (str): Schedule UUID.
        schedule_type (str): `cron` or `trigger`.
        workflow_name (str):
        cron_expression (None | str | Unset):
        paused (bool | Unset): Whether the schedule is paused (CLOACI-T-0749).
        paused_at (None | str | Unset): RFC 3339 timestamp of when it was paused, if paused.
        poll_interval_ms (int | None | Unset): Poll interval in milliseconds for `trigger`-type schedules (the
            custom-poll cadence). `None` for cron schedules. Lets the Triggers
            detail view show "polls every Ns" (CLOACI-I-0124 / WS-6).
        trigger_name (None | str | Unset):
    """

    enabled: bool
    id: str
    schedule_type: str
    workflow_name: str
    cron_expression: None | str | Unset = UNSET
    paused: bool | Unset = UNSET
    paused_at: None | str | Unset = UNSET
    poll_interval_ms: int | None | Unset = UNSET
    trigger_name: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        enabled = self.enabled

        id = self.id

        schedule_type = self.schedule_type

        workflow_name = self.workflow_name

        cron_expression: None | str | Unset
        if isinstance(self.cron_expression, Unset):
            cron_expression = UNSET
        else:
            cron_expression = self.cron_expression

        paused = self.paused

        paused_at: None | str | Unset
        if isinstance(self.paused_at, Unset):
            paused_at = UNSET
        else:
            paused_at = self.paused_at

        poll_interval_ms: int | None | Unset
        if isinstance(self.poll_interval_ms, Unset):
            poll_interval_ms = UNSET
        else:
            poll_interval_ms = self.poll_interval_ms

        trigger_name: None | str | Unset
        if isinstance(self.trigger_name, Unset):
            trigger_name = UNSET
        else:
            trigger_name = self.trigger_name

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "enabled": enabled,
                "id": id,
                "schedule_type": schedule_type,
                "workflow_name": workflow_name,
            }
        )
        if cron_expression is not UNSET:
            field_dict["cron_expression"] = cron_expression
        if paused is not UNSET:
            field_dict["paused"] = paused
        if paused_at is not UNSET:
            field_dict["paused_at"] = paused_at
        if poll_interval_ms is not UNSET:
            field_dict["poll_interval_ms"] = poll_interval_ms
        if trigger_name is not UNSET:
            field_dict["trigger_name"] = trigger_name

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        enabled = d.pop("enabled")

        id = d.pop("id")

        schedule_type = d.pop("schedule_type")

        workflow_name = d.pop("workflow_name")

        def _parse_cron_expression(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        cron_expression = _parse_cron_expression(d.pop("cron_expression", UNSET))

        paused = d.pop("paused", UNSET)

        def _parse_paused_at(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        paused_at = _parse_paused_at(d.pop("paused_at", UNSET))

        def _parse_poll_interval_ms(data: object) -> int | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(int | None | Unset, data)

        poll_interval_ms = _parse_poll_interval_ms(d.pop("poll_interval_ms", UNSET))

        def _parse_trigger_name(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        trigger_name = _parse_trigger_name(d.pop("trigger_name", UNSET))

        trigger_schedule_info = cls(
            enabled=enabled,
            id=id,
            schedule_type=schedule_type,
            workflow_name=workflow_name,
            cron_expression=cron_expression,
            paused=paused,
            paused_at=paused_at,
            poll_interval_ms=poll_interval_ms,
            trigger_name=trigger_name,
        )

        trigger_schedule_info.additional_properties = d
        return trigger_schedule_info

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
