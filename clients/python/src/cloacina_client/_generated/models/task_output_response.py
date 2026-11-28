from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Self, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.task_attempt_output_detail import TaskAttemptOutputDetail


T = TypeVar("T", bound="TaskOutputResponse")


@_attrs_define
class TaskOutputResponse:
    """`GET /tenants/{tenant_id}/executions/{id}/tasks/{task_id}/output`
    response (CLOACINA-T-1091): the stored output of each attempt of the
    task, in attempt order. Empty for a task whose output was not kept.

        Attributes:
            attempts (list[TaskAttemptOutputDetail]):
            execution_id (str):
            task_id (str):
            tenant_id (str):
    """

    attempts: list[TaskAttemptOutputDetail]
    execution_id: str
    task_id: str
    tenant_id: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        attempts = []
        for attempts_item_data in self.attempts:
            attempts_item = attempts_item_data.to_dict()
            attempts.append(attempts_item)

        execution_id = self.execution_id

        task_id = self.task_id

        tenant_id = self.tenant_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "attempts": attempts,
                "execution_id": execution_id,
                "task_id": task_id,
                "tenant_id": tenant_id,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        from ..models.task_attempt_output_detail import TaskAttemptOutputDetail

        d = dict(src_dict)
        attempts = []
        _attempts = d.pop("attempts")
        for attempts_item_data in _attempts:
            attempts_item = TaskAttemptOutputDetail.from_dict(attempts_item_data)

            attempts.append(attempts_item)

        execution_id = d.pop("execution_id")

        task_id = d.pop("task_id")

        tenant_id = d.pop("tenant_id")

        task_output_response = cls(
            attempts=attempts,
            execution_id=execution_id,
            task_id=task_id,
            tenant_id=tenant_id,
        )

        task_output_response.additional_properties = d
        return task_output_response

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
