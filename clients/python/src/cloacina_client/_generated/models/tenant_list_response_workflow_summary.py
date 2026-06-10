from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.tenant_list_response_workflow_summary_items_item import (
        TenantListResponseWorkflowSummaryItemsItem,
    )


T = TypeVar("T", bound="TenantListResponseWorkflowSummary")


@_attrs_define
class TenantListResponseWorkflowSummary:
    """List envelope variant that retains a top-level `tenant_id`, used by
    tenant-scoped list endpoints for backward compatibility with operator
    dashboards that key off it.

        Attributes:
            items (list[TenantListResponseWorkflowSummaryItemsItem]):
            tenant_id (str):
            total (int):
    """

    items: list[TenantListResponseWorkflowSummaryItemsItem]
    tenant_id: str
    total: int
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        items = []
        for items_item_data in self.items:
            items_item = items_item_data.to_dict()
            items.append(items_item)

        tenant_id = self.tenant_id

        total = self.total

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "items": items,
                "tenant_id": tenant_id,
                "total": total,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.tenant_list_response_workflow_summary_items_item import (
            TenantListResponseWorkflowSummaryItemsItem,
        )

        d = dict(src_dict)
        items = []
        _items = d.pop("items")
        for items_item_data in _items:
            items_item = TenantListResponseWorkflowSummaryItemsItem.from_dict(
                items_item_data
            )

            items.append(items_item)

        tenant_id = d.pop("tenant_id")

        total = d.pop("total")

        tenant_list_response_workflow_summary = cls(
            items=items,
            tenant_id=tenant_id,
            total=total,
        )

        tenant_list_response_workflow_summary.additional_properties = d
        return tenant_list_response_workflow_summary

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
