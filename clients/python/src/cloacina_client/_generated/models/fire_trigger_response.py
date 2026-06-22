from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.fired_execution import FiredExecution


T = TypeVar("T", bound="FireTriggerResponse")


@_attrs_define
class FireTriggerResponse:
    """`POST /tenants/{tenant_id}/triggers/{name}/fire` response (CLOACI-T-0777).

    Attributes:
        executions (list[FiredExecution]): The started executions: `(workflow_name, execution_id)`.
        fired (int): How many subscribed workflows were fired (the fan-out count).
        tenant_id (str):
        trigger (str): The trigger name fired.
    """

    executions: list[FiredExecution]
    fired: int
    tenant_id: str
    trigger: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        executions = []
        for executions_item_data in self.executions:
            executions_item = executions_item_data.to_dict()
            executions.append(executions_item)

        fired = self.fired

        tenant_id = self.tenant_id

        trigger = self.trigger

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "executions": executions,
                "fired": fired,
                "tenant_id": tenant_id,
                "trigger": trigger,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.fired_execution import FiredExecution

        d = dict(src_dict)
        executions = []
        _executions = d.pop("executions")
        for executions_item_data in _executions:
            executions_item = FiredExecution.from_dict(executions_item_data)

            executions.append(executions_item)

        fired = d.pop("fired")

        tenant_id = d.pop("tenant_id")

        trigger = d.pop("trigger")

        fire_trigger_response = cls(
            executions=executions,
            fired=fired,
            tenant_id=tenant_id,
            trigger=trigger,
        )

        fire_trigger_response.additional_properties = d
        return fire_trigger_response

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
