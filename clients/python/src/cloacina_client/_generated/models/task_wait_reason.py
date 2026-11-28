from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="TaskWaitReason")


@_attrs_define
class TaskWaitReason:
    """Why a task waits (CLOACINA-T-1148): a short code and the same in words.

    The codes are an open set: a client shows an unknown code with its
    message. The codes of this release: `plugin_interface`, `no_agent`,
    `platform`, `too_large`, `no_capacity`, `agent_refused`,
    `artifact_fetch`, `dispatch_retry`.

        Attributes:
            code (str): A short code, for example `no_capacity`.
            message (str): The reason in words, with the tenant, the pool and the numbers.
    """

    code: str
    message: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        code = self.code

        message = self.message

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "code": code,
                "message": message,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        code = d.pop("code")

        message = d.pop("message")

        task_wait_reason = cls(
            code=code,
            message=message,
        )

        task_wait_reason.additional_properties = d
        return task_wait_reason

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
