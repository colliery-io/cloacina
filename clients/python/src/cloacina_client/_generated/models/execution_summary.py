from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="ExecutionSummary")


@_attrs_define
class ExecutionSummary:
    """One row in the executions list.

    Attributes:
        id (str): Execution UUID.
        started_at (str): RFC 3339 timestamp.
        status (str):
        workflow_name (str):
        completed_at (None | str | Unset): RFC 3339 timestamp; `null` while still running.
        trigger_origin (None | str | Unset): How the run was triggered (CLOACI-T-0776): `"manual"` for an operator run
            via the REST execute endpoint; `null` for cron/trigger/reactor-driven runs.
            The UI marks manual runs with a "manual" pill.
    """

    id: str
    started_at: str
    status: str
    workflow_name: str
    completed_at: None | str | Unset = UNSET
    trigger_origin: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        id = self.id

        started_at = self.started_at

        status = self.status

        workflow_name = self.workflow_name

        completed_at: None | str | Unset
        if isinstance(self.completed_at, Unset):
            completed_at = UNSET
        else:
            completed_at = self.completed_at

        trigger_origin: None | str | Unset
        if isinstance(self.trigger_origin, Unset):
            trigger_origin = UNSET
        else:
            trigger_origin = self.trigger_origin

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "started_at": started_at,
                "status": status,
                "workflow_name": workflow_name,
            }
        )
        if completed_at is not UNSET:
            field_dict["completed_at"] = completed_at
        if trigger_origin is not UNSET:
            field_dict["trigger_origin"] = trigger_origin

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        id = d.pop("id")

        started_at = d.pop("started_at")

        status = d.pop("status")

        workflow_name = d.pop("workflow_name")

        def _parse_completed_at(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        completed_at = _parse_completed_at(d.pop("completed_at", UNSET))

        def _parse_trigger_origin(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        trigger_origin = _parse_trigger_origin(d.pop("trigger_origin", UNSET))

        execution_summary = cls(
            id=id,
            started_at=started_at,
            status=status,
            workflow_name=workflow_name,
            completed_at=completed_at,
            trigger_origin=trigger_origin,
        )

        execution_summary.additional_properties = d
        return execution_summary

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
