from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="TriggerScheduleSummary")


@_attrs_define
class TriggerScheduleSummary:
    """One row in the trigger list.

    Attributes:
        created_at (str): RFC 3339 timestamp.
        enabled (bool):
        id (str): Schedule UUID.
        schedule_type (str): `cron` or `trigger`.
        workflow_name (str):
        cron_expression (None | str | Unset):
        last_run_at (None | str | Unset): RFC 3339 timestamp.
        next_run_at (None | str | Unset): RFC 3339 timestamp.
        poll_interval_ms (int | None | Unset):
        trigger_name (None | str | Unset):
    """

    created_at: str
    enabled: bool
    id: str
    schedule_type: str
    workflow_name: str
    cron_expression: None | str | Unset = UNSET
    last_run_at: None | str | Unset = UNSET
    next_run_at: None | str | Unset = UNSET
    poll_interval_ms: int | None | Unset = UNSET
    trigger_name: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        created_at = self.created_at

        enabled = self.enabled

        id = self.id

        schedule_type = self.schedule_type

        workflow_name = self.workflow_name

        cron_expression: None | str | Unset
        if isinstance(self.cron_expression, Unset):
            cron_expression = UNSET
        else:
            cron_expression = self.cron_expression

        last_run_at: None | str | Unset
        if isinstance(self.last_run_at, Unset):
            last_run_at = UNSET
        else:
            last_run_at = self.last_run_at

        next_run_at: None | str | Unset
        if isinstance(self.next_run_at, Unset):
            next_run_at = UNSET
        else:
            next_run_at = self.next_run_at

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
                "created_at": created_at,
                "enabled": enabled,
                "id": id,
                "schedule_type": schedule_type,
                "workflow_name": workflow_name,
            }
        )
        if cron_expression is not UNSET:
            field_dict["cron_expression"] = cron_expression
        if last_run_at is not UNSET:
            field_dict["last_run_at"] = last_run_at
        if next_run_at is not UNSET:
            field_dict["next_run_at"] = next_run_at
        if poll_interval_ms is not UNSET:
            field_dict["poll_interval_ms"] = poll_interval_ms
        if trigger_name is not UNSET:
            field_dict["trigger_name"] = trigger_name

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        created_at = d.pop("created_at")

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

        def _parse_last_run_at(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        last_run_at = _parse_last_run_at(d.pop("last_run_at", UNSET))

        def _parse_next_run_at(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        next_run_at = _parse_next_run_at(d.pop("next_run_at", UNSET))

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

        trigger_schedule_summary = cls(
            created_at=created_at,
            enabled=enabled,
            id=id,
            schedule_type=schedule_type,
            workflow_name=workflow_name,
            cron_expression=cron_expression,
            last_run_at=last_run_at,
            next_run_at=next_run_at,
            poll_interval_ms=poll_interval_ms,
            trigger_name=trigger_name,
        )

        trigger_schedule_summary.additional_properties = d
        return trigger_schedule_summary

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
