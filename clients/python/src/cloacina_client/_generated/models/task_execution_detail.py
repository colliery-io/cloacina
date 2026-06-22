from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="TaskExecutionDetail")


@_attrs_define
class TaskExecutionDetail:
    """One per-task row of an execution (CLOACI-I-0124 / WS-1).

    Attributes:
        attempt (int):
        created_at (str): Row-created timestamp (RFC 3339) — always present; a fallback "start"
            when `started_at` is null (some runner configs don't stamp it).
        id (str): Task execution UUID.
        max_attempts (int):
        status (str):
        task_name (str): Task identifier within the workflow.
        updated_at (str): Row-updated timestamp (RFC 3339) — always present; a fallback "end".
        completed_at (None | str | Unset): RFC 3339 timestamp; `null` while still running.
        error_details (None | str | Unset): Structured error details, when present.
        last_error (None | str | Unset): Last error message for the most recent failed attempt, when present.
        started_at (None | str | Unset): RFC 3339 timestamp; `null` until the task starts.
        sub_status (None | str | Unset): `sub_status` qualifier (e.g. deferral), when present.
    """

    attempt: int
    created_at: str
    id: str
    max_attempts: int
    status: str
    task_name: str
    updated_at: str
    completed_at: None | str | Unset = UNSET
    error_details: None | str | Unset = UNSET
    last_error: None | str | Unset = UNSET
    started_at: None | str | Unset = UNSET
    sub_status: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        attempt = self.attempt

        created_at = self.created_at

        id = self.id

        max_attempts = self.max_attempts

        status = self.status

        task_name = self.task_name

        updated_at = self.updated_at

        completed_at: None | str | Unset
        if isinstance(self.completed_at, Unset):
            completed_at = UNSET
        else:
            completed_at = self.completed_at

        error_details: None | str | Unset
        if isinstance(self.error_details, Unset):
            error_details = UNSET
        else:
            error_details = self.error_details

        last_error: None | str | Unset
        if isinstance(self.last_error, Unset):
            last_error = UNSET
        else:
            last_error = self.last_error

        started_at: None | str | Unset
        if isinstance(self.started_at, Unset):
            started_at = UNSET
        else:
            started_at = self.started_at

        sub_status: None | str | Unset
        if isinstance(self.sub_status, Unset):
            sub_status = UNSET
        else:
            sub_status = self.sub_status

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "attempt": attempt,
                "created_at": created_at,
                "id": id,
                "max_attempts": max_attempts,
                "status": status,
                "task_name": task_name,
                "updated_at": updated_at,
            }
        )
        if completed_at is not UNSET:
            field_dict["completed_at"] = completed_at
        if error_details is not UNSET:
            field_dict["error_details"] = error_details
        if last_error is not UNSET:
            field_dict["last_error"] = last_error
        if started_at is not UNSET:
            field_dict["started_at"] = started_at
        if sub_status is not UNSET:
            field_dict["sub_status"] = sub_status

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        attempt = d.pop("attempt")

        created_at = d.pop("created_at")

        id = d.pop("id")

        max_attempts = d.pop("max_attempts")

        status = d.pop("status")

        task_name = d.pop("task_name")

        updated_at = d.pop("updated_at")

        def _parse_completed_at(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        completed_at = _parse_completed_at(d.pop("completed_at", UNSET))

        def _parse_error_details(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        error_details = _parse_error_details(d.pop("error_details", UNSET))

        def _parse_last_error(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        last_error = _parse_last_error(d.pop("last_error", UNSET))

        def _parse_started_at(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        started_at = _parse_started_at(d.pop("started_at", UNSET))

        def _parse_sub_status(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        sub_status = _parse_sub_status(d.pop("sub_status", UNSET))

        task_execution_detail = cls(
            attempt=attempt,
            created_at=created_at,
            id=id,
            max_attempts=max_attempts,
            status=status,
            task_name=task_name,
            updated_at=updated_at,
            completed_at=completed_at,
            error_details=error_details,
            last_error=last_error,
            started_at=started_at,
            sub_status=sub_status,
        )

        task_execution_detail.additional_properties = d
        return task_execution_detail

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
