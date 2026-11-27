from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="TaskAttemptErrorDetail")


@_attrs_define
class TaskAttemptErrorDetail:
    """The error of one failed attempt of a task (CLOACINA-T-1100).

    Attributes:
        attempt (int): The attempt that failed, from 1.
        failed_at (str): RFC 3339 timestamp of the failure.
        message (str): The error message of the attempt.
        details (None | str | Unset): The details of the failure (a Python traceback), at most 16 KiB; a
            cut keeps the end.
    """

    attempt: int
    failed_at: str
    message: str
    details: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        attempt = self.attempt

        failed_at = self.failed_at

        message = self.message

        details: None | str | Unset
        if isinstance(self.details, Unset):
            details = UNSET
        else:
            details = self.details

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "attempt": attempt,
                "failed_at": failed_at,
                "message": message,
            }
        )
        if details is not UNSET:
            field_dict["details"] = details

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        attempt = d.pop("attempt")

        failed_at = d.pop("failed_at")

        message = d.pop("message")

        def _parse_details(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        details = _parse_details(d.pop("details", UNSET))

        task_attempt_error_detail = cls(
            attempt=attempt,
            failed_at=failed_at,
            message=message,
            details=details,
        )

        task_attempt_error_detail.additional_properties = d
        return task_attempt_error_detail

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
