from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.task_execution_detail import TaskExecutionDetail


T = TypeVar("T", bound="ExecutionTasksResponse")


@_attrs_define
class ExecutionTasksResponse:
    """`GET /tenants/{tenant_id}/executions/{id}/tasks` response.

    Attributes:
        execution_id (str):
        tasks (list[TaskExecutionDetail]):
        tenant_id (str):
    """

    execution_id: str
    tasks: list[TaskExecutionDetail]
    tenant_id: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        execution_id = self.execution_id

        tasks = []
        for tasks_item_data in self.tasks:
            tasks_item = tasks_item_data.to_dict()
            tasks.append(tasks_item)

        tenant_id = self.tenant_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "execution_id": execution_id,
                "tasks": tasks,
                "tenant_id": tenant_id,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.task_execution_detail import TaskExecutionDetail

        d = dict(src_dict)
        execution_id = d.pop("execution_id")

        tasks = []
        _tasks = d.pop("tasks")
        for tasks_item_data in _tasks:
            tasks_item = TaskExecutionDetail.from_dict(tasks_item_data)

            tasks.append(tasks_item)

        tenant_id = d.pop("tenant_id")

        execution_tasks_response = cls(
            execution_id=execution_id,
            tasks=tasks,
            tenant_id=tenant_id,
        )

        execution_tasks_response.additional_properties = d
        return execution_tasks_response

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
