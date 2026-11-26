from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="WorkflowInstanceSummary")


@_attrs_define
class WorkflowInstanceSummary:
    """One row in the instance list, and the body of a single-instance GET.

    Attributes:
        created_at (str): RFC 3339 timestamp.
        enabled (bool):
        id (str): Underlying schedule UUID.
        instance_name (str):
        workflow_name (str):
        catchup_policy (None | str | Unset): `skip` or `run_all`. Absent in a response from an older server.
        cron_expression (None | str | Unset):
        last_run_at (None | str | Unset): RFC 3339 timestamp.
        max_active_runs (int | None | Unset): The declared limit on active runs. Absent when none is declared.
        next_run_at (None | str | Unset): RFC 3339 timestamp.
        params (Any | Unset): Bound parameter values, as stored.
        paused (bool | Unset): Whether the schedule is paused (distinct from `enabled`).
        timezone (None | str | Unset):
    """

    created_at: str
    enabled: bool
    id: str
    instance_name: str
    workflow_name: str
    catchup_policy: None | str | Unset = UNSET
    cron_expression: None | str | Unset = UNSET
    last_run_at: None | str | Unset = UNSET
    max_active_runs: int | None | Unset = UNSET
    next_run_at: None | str | Unset = UNSET
    params: Any | Unset = UNSET
    paused: bool | Unset = UNSET
    timezone: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        created_at = self.created_at

        enabled = self.enabled

        id = self.id

        instance_name = self.instance_name

        workflow_name = self.workflow_name

        catchup_policy: None | str | Unset
        if isinstance(self.catchup_policy, Unset):
            catchup_policy = UNSET
        else:
            catchup_policy = self.catchup_policy

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

        max_active_runs: int | None | Unset
        if isinstance(self.max_active_runs, Unset):
            max_active_runs = UNSET
        else:
            max_active_runs = self.max_active_runs

        next_run_at: None | str | Unset
        if isinstance(self.next_run_at, Unset):
            next_run_at = UNSET
        else:
            next_run_at = self.next_run_at

        params = self.params

        paused = self.paused

        timezone: None | str | Unset
        if isinstance(self.timezone, Unset):
            timezone = UNSET
        else:
            timezone = self.timezone

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "created_at": created_at,
                "enabled": enabled,
                "id": id,
                "instance_name": instance_name,
                "workflow_name": workflow_name,
            }
        )
        if catchup_policy is not UNSET:
            field_dict["catchup_policy"] = catchup_policy
        if cron_expression is not UNSET:
            field_dict["cron_expression"] = cron_expression
        if last_run_at is not UNSET:
            field_dict["last_run_at"] = last_run_at
        if max_active_runs is not UNSET:
            field_dict["max_active_runs"] = max_active_runs
        if next_run_at is not UNSET:
            field_dict["next_run_at"] = next_run_at
        if params is not UNSET:
            field_dict["params"] = params
        if paused is not UNSET:
            field_dict["paused"] = paused
        if timezone is not UNSET:
            field_dict["timezone"] = timezone

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        created_at = d.pop("created_at")

        enabled = d.pop("enabled")

        id = d.pop("id")

        instance_name = d.pop("instance_name")

        workflow_name = d.pop("workflow_name")

        def _parse_catchup_policy(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        catchup_policy = _parse_catchup_policy(d.pop("catchup_policy", UNSET))

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

        def _parse_max_active_runs(data: object) -> int | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(int | None | Unset, data)

        max_active_runs = _parse_max_active_runs(d.pop("max_active_runs", UNSET))

        def _parse_next_run_at(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        next_run_at = _parse_next_run_at(d.pop("next_run_at", UNSET))

        params = d.pop("params", UNSET)

        paused = d.pop("paused", UNSET)

        def _parse_timezone(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        timezone = _parse_timezone(d.pop("timezone", UNSET))

        workflow_instance_summary = cls(
            created_at=created_at,
            enabled=enabled,
            id=id,
            instance_name=instance_name,
            workflow_name=workflow_name,
            catchup_policy=catchup_policy,
            cron_expression=cron_expression,
            last_run_at=last_run_at,
            max_active_runs=max_active_runs,
            next_run_at=next_run_at,
            params=params,
            paused=paused,
            timezone=timezone,
        )

        workflow_instance_summary.additional_properties = d
        return workflow_instance_summary

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
