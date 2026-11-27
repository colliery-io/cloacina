from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.priority_grant import PriorityGrant


T = TypeVar("T", bound="ExecuteResponse")


@_attrs_define
class ExecuteResponse:
    """`202 Accepted` body for a scheduled workflow execution.

    Attributes:
        execution_id (str): UUID of the scheduled execution.
        status (str): Always `"scheduled"` at accept time.
        tenant_id (str):
        workflow_name (str):
        priority (None | PriorityGrant | Unset):
        priority_notice (None | str | Unset): Set when the given priority is lower than the one asked for: one
            sentence that says what was asked, what was given and why.
    """

    execution_id: str
    status: str
    tenant_id: str
    workflow_name: str
    priority: None | PriorityGrant | Unset = UNSET
    priority_notice: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.priority_grant import PriorityGrant

        execution_id = self.execution_id

        status = self.status

        tenant_id = self.tenant_id

        workflow_name = self.workflow_name

        priority: dict[str, Any] | None | Unset
        if isinstance(self.priority, Unset):
            priority = UNSET
        elif isinstance(self.priority, PriorityGrant):
            priority = self.priority.to_dict()
        else:
            priority = self.priority

        priority_notice: None | str | Unset
        if isinstance(self.priority_notice, Unset):
            priority_notice = UNSET
        else:
            priority_notice = self.priority_notice

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
        if priority is not UNSET:
            field_dict["priority"] = priority
        if priority_notice is not UNSET:
            field_dict["priority_notice"] = priority_notice

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        from ..models.priority_grant import PriorityGrant

        d = dict(src_dict)
        execution_id = d.pop("execution_id")

        status = d.pop("status")

        tenant_id = d.pop("tenant_id")

        workflow_name = d.pop("workflow_name")

        def _parse_priority(data: object) -> None | PriorityGrant | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                priority_type_0 = PriorityGrant.from_dict(data)

                return priority_type_0
            except (TypeError, ValueError, AttributeError, KeyError):
                pass
            return cast(None | PriorityGrant | Unset, data)

        priority = _parse_priority(d.pop("priority", UNSET))

        def _parse_priority_notice(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        priority_notice = _parse_priority_notice(d.pop("priority_notice", UNSET))

        execute_response = cls(
            execution_id=execution_id,
            status=status,
            tenant_id=tenant_id,
            workflow_name=workflow_name,
            priority=priority,
            priority_notice=priority_notice,
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
