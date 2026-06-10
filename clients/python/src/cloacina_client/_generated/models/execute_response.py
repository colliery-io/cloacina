from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="ExecuteResponse")


@_attrs_define
class ExecuteResponse:
    """`202 Accepted` body for a scheduled workflow execution.

    Attributes:
        execution_id (str): UUID of the scheduled execution.
        status (str): Always `"scheduled"` at accept time.
        tenant_id (str):
        workflow_name (str):
    """

    execution_id: str
    status: str
    tenant_id: str
    workflow_name: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        execution_id = self.execution_id

        status = self.status

        tenant_id = self.tenant_id

        workflow_name = self.workflow_name

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "execution_id": execution_id,
                "status": status,
                "tenant_id": tenant_id,
                "workflow_name": workflow_name,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        execution_id = d.pop("execution_id")

        status = d.pop("status")

        tenant_id = d.pop("tenant_id")

        workflow_name = d.pop("workflow_name")

        execute_response = cls(
            execution_id=execution_id,
            status=status,
            tenant_id=tenant_id,
            workflow_name=workflow_name,
        )

        execute_response.additional_properties = d
        return execute_response

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
